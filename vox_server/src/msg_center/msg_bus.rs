use super::*;
use crate::{
    bot::bot::Bot, config::CONFIG, msg_center::signaling_msgs::SignalingMessage,
    server::rtc::traits::WebRTCHandler,
};
use anyhow::Result;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};

const DEFAULT_CHANNEL_SIZE: usize = 100;

// 消息路由表,管理所有的发送端
#[derive(Default)]
struct MessageRouter {
    // client_id/bot_id -> sender
    bots_senders: HashMap<String, mpsc::Sender<SignalingMessage>>,
    //bots: HashMap<String, Bot>,
}

impl MessageRouter {
    pub async fn add_route(&mut self, id: &String, sender: mpsc::Sender<SignalingMessage>) {
        debug!("add route for bot: {}", id);
        // 检查当前是否存在sender
        if self.bots_senders.contains_key(id) {
            error!("correlated bot already exists for user id: {}", id);
            return;
        }

        let (message_tx, message_rx) = mpsc::channel(DEFAULT_CHANNEL_SIZE);

        let mut bot = Bot::new(
            CONFIG.read().await.clone(),
            id.clone(),
            sender.clone(),
            message_rx,
        )
        .await
        .unwrap();

        bot.setup_audio_processor().await;
        tokio::spawn(async move {
            bot.handle_message().await;
        });
        self.bots_senders.insert(id.clone(), message_tx);
        // self.bots.insert(id.clone(), bot);
    }

    fn remove_route(&mut self, id: &str) {
        self.bots_senders.remove(id);
        // self.bots.remove(id);
    }

    fn get_sender(&self, id: &str) -> Option<mpsc::Sender<SignalingMessage>> {
        self.bots_senders.get(id).cloned()
    }

    // fn get_bot(&self, id: &str) -> Option<&Bot> {
    //     self.bots.get(id)
    // }
}

pub struct BotManager {
    bots: HashMap<String, mpsc::Sender<SignalingMessage>>,
}

impl BotManager {
    pub fn new() -> Self {
        Self {
            bots: HashMap::new(),
        }
    }

    pub async fn create_bot(
        &mut self,
        id: String,
        sender: mpsc::Sender<SignalingMessage>,
    ) -> Result<mpsc::Sender<SignalingMessage>> {
        let (message_tx, message_rx) = mpsc::channel(DEFAULT_CHANNEL_SIZE);

        let mut bot = Bot::new(CONFIG.read().await.clone(), id.clone(), sender, message_rx).await?;

        bot.setup_audio_processor().await;

        tokio::spawn(async move {
            debug!("start bot handle message with id: {}", &bot.bot_id);
            bot.handle_message().await;
        });

        self.bots.insert(id, message_tx.clone());

        Ok(message_tx)
    }

    pub fn remove_bot(&mut self, id: &str) {
        self.bots.remove(id);
    }
}

#[derive(Clone)]
pub struct MessageBus {
    router: Arc<RwLock<MessageRouter>>,
    bot_manager: Arc<RwLock<BotManager>>,
    back2ws_sender: mpsc::Sender<SignalingMessage>,
}

impl MessageBus {
    pub fn new(websocket_sender: mpsc::Sender<SignalingMessage>) -> Self {
        Self {
            router: Arc::new(RwLock::new(MessageRouter::default())),
            bot_manager: Arc::new(RwLock::new(BotManager::new())),
            back2ws_sender: websocket_sender,
        }
    }

    // 注册一个新的消息通道
    pub async fn register(&self, id: String) {
        if self.router.read().await.get_sender(&id).is_some() {
            return;
        }

        let ws_sender = self.back2ws_sender.clone();

        // 先创建Bot
        let message_tx = self
            .bot_manager
            .write()
            .await
            .create_bot(id.clone(), ws_sender)
            .await
            .expect("Failed to create bot");

        // 然后添加路由
        self.router.write().await.add_route(&id, message_tx).await;

        // let message_tx = self.bot_manager.write().await.create_bot(id, self.back2ws_sender.clone()).await?;
        // self.router
        //     .write()
        //     .await
        //     .add_route(&id, self.back2ws_sender.clone()).await;
    }

    // 注销消息通道
    pub async fn unregister(&self, id: &str) {
        self.router.write().await.remove_route(id);
    }

    // 发送消息到指定目标
    pub async fn send_from(&self, from: &str, message: SignalingMessage) -> Result<()> {
        if let Some(sender) = self.router.read().await.get_sender(from) {
            debug!("send message to bot: {}", from);
            sender
                .send(message)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to send message: {}", e))?;
        } else {
            warn!("No route found for target: {}", from);
        }
        Ok(())
    }

    pub async fn run(mut msg_recv: mpsc::Receiver<SignalingMessage>, ws_tx: mpsc::Sender<SignalingMessage>) {
        let mut bus = Self::new(ws_tx);
        // loop {
        //     tokio::select! {
        //         message = msg_recv.recv() => {
        //             if let Some(message) = message {
        //           match message {
        //             SignalingMessage::ClientConnect { client_id } => {
        //                 debug!("recv client connect message: {}", client_id);
        //                 bus.register(client_id.clone()).await;
        //             }
        //             SignalingMessage::Offer {
        //                 ref from,
        //                 ref to,
        //                 ref sdp,
        //             } => {
        //                 debug!("send offer message to bot: {}", from);
        //                 match bus.send_from(&from, message.clone()).await {
        //                     Ok(_) => debug!("send offer message to bot: {} success", from),
        //                     Err(e) => error!("send offer message to bot: {} failed, {}", from, e),
        //                 }
        //             }
        //             SignalingMessage::IceCandidate {
        //                 ref from,
        //                 ref to,
        //                 ref candidate,
        //             } => {
        //                 debug!("send ice candidate message to bot: {}", from);
        //                 match bus.send_from(&from, message.clone()).await {
        //                     Ok(_) => debug!("send ice candidate message to bot: {} success", from),
        //                     Err(e) => error!("send ice candidate message to bot: {} failed, {}", from, e),
        //                 }
        //             }
        //             SignalingMessage::Answer {
        //                 ref from,
        //                 ref to,
        //                 ref sdp,
        //             } => {
        //                 debug!("send answer message to bot: {}", from);
        //                 match bus.send_from(&from, message.clone()).await {
        //                     Ok(_) => debug!("send answer message to bot: {} success", from),
        //                     Err(e) => error!("send answer message to bot: {} failed, {}", from, e),
        //                 }
        //             }
        //             _ => {
        //                 error!("Unknown message: {:?}", message);
        //             }
        //         }
        //           }
        //         }
        //         _ = ws_rx.recv() => {}
        //     }
        // }

        while let Some(message) = msg_recv.recv().await {
            match message {
                SignalingMessage::ClientConnect { client_id } => {
                    debug!("recv client connect message: {}", client_id);
                    bus.register(client_id.clone()).await;
                }
                SignalingMessage::Offer {
                    ref from,
                    ref to,
                    ref sdp,
                } => {
                    debug!("send offer message to bot: {}", from);
                    match bus.send_from(&from, message.clone()).await {
                        Ok(_) => debug!("send offer message to bot: {} success", from),
                        Err(e) => error!("send offer message to bot: {} failed, {}", from, e),
                    }
                }
                SignalingMessage::IceCandidate {
                    ref from,
                    ref to,
                    ref candidate,
                } => {
                    debug!("send ice candidate message to bot: {}", from);
                    match bus.send_from(&from, message.clone()).await {
                        Ok(_) => debug!("send ice candidate message to bot: {} success", from),
                        Err(e) => {
                            error!("send ice candidate message to bot: {} failed, {}", from, e)
                        }
                    }
                }
                SignalingMessage::Answer {
                    ref from,
                    ref to,
                    ref sdp,
                } => {
                    debug!("send answer message to bot: {}", from);
                    match bus.send_from(&from, message.clone()).await {
                        Ok(_) => debug!("send answer message to bot: {} success", from),
                        Err(e) => error!("send answer message to bot: {} failed, {}", from, e),
                    }
                }
                _ => {
                    error!("Unknown message: {:?}", message);
                }
            }
        }
    }
}

// pub struct MessageHandler {
//     bus: MessageBus,
//     component_id: String,
//     receiver: Option<mpsc::Receiver<SignalingMessage>>,
// }

// impl MessageHandler {
//     pub async fn new(bus: MessageBus, component_id: String) -> Self {
//         let receiver = Some(bus.register(component_id.clone()).await);
//         Self {
//             bus,
//             component_id,
//             receiver,
//         }
//     }

//     pub fn take_receiver(&mut self) -> Option<mpsc::Receiver<SignalingMessage>> {
//         self.receiver.take()
//     }

//     pub async fn send_to(&self, target: &str, message: SignalingMessage) -> Result<()> {
//         debug!("Component {} sending message to {}: {:?}",
//             self.component_id, target, message);
//         self.bus.send_from(target, message).await
//     }
// }

// impl Drop for MessageHandler {
//     fn drop(&mut self) {
//         let bus = self.bus.clone();
//         let component_id = self.component_id.clone();
//         tokio::spawn(async move {
//             bus.unregister(&component_id).await;
//         });
//     }
// }
