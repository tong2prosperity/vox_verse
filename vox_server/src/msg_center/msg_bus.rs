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
    pub async fn add_route(&mut self, client_id: &String, sender: mpsc::Sender<SignalingMessage>) {
        debug!("add route for bot: {}", client_id);
        // 检查当前是否存在sender
        if self.bots_senders.contains_key(client_id) {
            error!("correlated bot already exists for user id: {}", client_id);
            return;
        }

        // let (message_tx, message_rx) = mpsc::channel(DEFAULT_CHANNEL_SIZE);

        // let mut bot = Bot::new(
        //     CONFIG.read().await.clone(),
        //     client_id.clone(),
        //     sender.clone(),
        //     message_rx,
        // )
        // .await
        // .unwrap();

        // bot.setup_audio_processor().await;
        // tokio::spawn(async move {
        //     bot.handle_message().await;
        // });
        self.bots_senders.insert(client_id.clone(), sender);
        info!("Successfully added route for client: {}", client_id);
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
        client_id: String,
        ws_sender: mpsc::Sender<SignalingMessage>,
    ) -> Result<mpsc::Sender<SignalingMessage>> {
        let (message_tx, message_rx) = mpsc::channel(DEFAULT_CHANNEL_SIZE);

        let mut bot = Bot::new(
            CONFIG.read().await.clone(),
            client_id.clone(),
            ws_sender,
            message_rx,
        )
        .await?;

        bot.setup_audio_processor().await;

        tokio::spawn(async move {
            debug!("start bot handle message with id: {}", &bot.bot_id);
            bot.handle_message().await;
        });

        self.bots.insert(client_id, message_tx.clone());

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
    pub async fn register(&self, client_id: String) {
        info!("Attempting to register client with ID: {}", client_id);
        if self.router.read().await.get_sender(&client_id).is_some() {
            warn!(
                "Client {} already registered, skipping registration",
                client_id
            );
            return;
        }

        let ws_sender = self.back2ws_sender.clone();
        debug!("Creating bot for client: {}", client_id);

        let message_tx = match self
            .bot_manager
            .write()
            .await
            .create_bot(client_id.clone(), ws_sender)
            .await
        {
            Ok(tx) => {
                info!("Successfully created bot for client: {}", client_id);
                tx
            }
            Err(e) => {
                error!("Failed to create bot for client {}: {}", client_id, e);
                return;
            }
        };

        debug!("Adding route for client: {}", client_id);
        self.router
            .write()
            .await
            .add_route(&client_id, message_tx)
            .await;
        info!("Successfully registered client: {}", client_id);
    }

    // 注销消息通道
    pub async fn unregister(&self, id: &str) {
        self.router.write().await.remove_route(id);
    }

    // 发送消息到指定目标
    pub async fn send_from(&self, from: &str, message: SignalingMessage) -> Result<()> {
        debug!("Attempting to send message from {}: {:?}", from, message);
        if let Some(sender) = self.router.read().await.get_sender(from) {
            info!("Found sender for {}, sending message", from);
            sender.send(message).await.map_err(|e| {
                error!("Failed to send message from {}: {}", from, e);
                anyhow::anyhow!("Failed to send message: {}", e)
            })?;
            debug!("Successfully sent message from {}", from);
        } else {
            warn!("No route found for sender: {}", from);
        }
        Ok(())
    }

    // 处理消息传递。
    // 接收client消息，创建，发送给bot。
    // 接收bot消息，发送给client。
    pub async fn run(
        mut msg_recv: mpsc::Receiver<SignalingMessage>,
        ws_tx: mpsc::Sender<SignalingMessage>,
    ) {
        info!("Starting MessageBus...");
        let mut bus = Self::new(ws_tx);

        while let Some(message) = msg_recv.recv().await {
            debug!("MessageBus received message: {:?}", message);
            match message {
                SignalingMessage::ClientConnect { client_id } => {
                    info!("Received client connect message for client: {}", client_id);
                    bus.register(client_id.clone()).await;
                }
                SignalingMessage::Offer {
                    ref from,
                    ref to,
                    ref sdp,
                } => {
                    info!("Received offer message from: {} to: {}", from, to);
                    match bus.send_from(&from, message.clone()).await {
                        Ok(_) => info!("Successfully sent offer message to bot: {}", from),
                        Err(e) => error!(
                            "Failed to send offer message to bot: {}, error: {}",
                            from, e
                        ),
                    }
                }
                SignalingMessage::IceCandidate {
                    ref from,
                    ref to,
                    ref candidate,
                } => {
                    info!("Received ICE candidate from: {} to: {}", from, to);
                    match bus.send_from(&from, message.clone()).await {
                        Ok(_) => info!("Successfully sent ICE candidate to bot: {}", from),
                        Err(e) => error!(
                            "Failed to send ICE candidate to bot: {}, error: {}",
                            from, e
                        ),
                    }
                }
                SignalingMessage::Answer {
                    ref from,
                    ref to,
                    ref sdp,
                } => {
                    info!("Received answer message from: {} to: {}", from, to);
                    match bus.send_from(&from, message.clone()).await {
                        Ok(_) => info!("Successfully sent answer message to bot: {}", from),
                        Err(e) => error!(
                            "Failed to send answer message to bot: {}, error: {}",
                            from, e
                        ),
                    }
                }
                _ => {
                    warn!("Received unknown message type: {:?}", message);
                }
            }
        }
        info!("MessageBus stopped");
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
