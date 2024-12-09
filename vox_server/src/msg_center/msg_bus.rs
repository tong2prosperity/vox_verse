use crate::{bot::bot::Bot, config::CONFIG, msg_center::signaling_msgs::SignalingMessage};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use anyhow::Result;
use super::*;
use once_cell::sync::Lazy;

const DEFAULT_CHANNEL_SIZE: usize = 100;



// 消息路由表,管理所有的发送端
#[derive(Default)]
struct MessageRouter {
    // client_id/bot_id -> sender
    bots_senders: HashMap<String, mpsc::Sender<SignalingMessage>>,
    bots: HashMap<String, Bot>,
}

impl MessageRouter {
    pub async fn add_route(&mut self, id: &String, sender: mpsc::Sender<SignalingMessage>) {
        // 检查当前是否存在sender
        if self.bots_senders.contains_key(id) {
            error!("correlated bot already exists for user id: {}", id);
            return;
        }

        let (message_tx, message_rx) = mpsc::channel(DEFAULT_CHANNEL_SIZE);

        let mut bot = Bot::new(CONFIG.read().await.clone(),id.clone(),sender.clone(),message_rx).await.unwrap();
        bot.setup_audio_processor().await;

        self.bots_senders.insert(id.clone(), message_tx);
        self.bots.insert(id.clone(), bot);
    }

    fn remove_route(&mut self, id: &str) {
        self.bots_senders.remove(id);
        self.bots.remove(id);
    }

    fn get_sender(&self, id: &str) -> Option<mpsc::Sender<SignalingMessage>> {
        self.bots_senders.get(id).cloned()
    }

    fn get_bot(&self, id: &str) -> Option<&Bot> {
        self.bots.get(id)
    }
}

#[derive(Clone)]
pub struct MessageBus {
    router: Arc<RwLock<MessageRouter>>,
    back2ws_sender: mpsc::Sender<SignalingMessage>,
}

impl MessageBus {
    pub fn new(websocket_sender: mpsc::Sender<SignalingMessage>) -> Self {
        Self {
            router: Arc::new(RwLock::new(MessageRouter::default())),
            back2ws_sender: websocket_sender,
        }
    }

    // 注册一个新的消息通道
    pub async fn register(&self, id: String){
        self.router.write().await.add_route(&id, self.back2ws_sender.clone());
    }

    // 注销消息通道
    pub async fn unregister(&self, id: &str) {
        self.router.write().await.remove_route(id);
    }

    // 发送消息到指定目标
    pub async fn send_from(&self, from: &str, message: SignalingMessage) -> Result<()> {
        if let Some(sender) = self.router.read().await.get_sender(from) {
            sender.send(message).await
                .map_err(|e| anyhow::anyhow!("Failed to send message: {}", e))?;
        } else {
            warn!("No route found for target: {}", from);
        }
        Ok(())
    }

    pub async fn run(mut msg_recv: mpsc::Receiver<SignalingMessage>){
        let mut bus  = Self::new(mpsc::channel(DEFAULT_CHANNEL_SIZE).0);
        while let Some(message) = msg_recv.recv().await {
            match message {
                SignalingMessage::Call { ref from, .. } => {
                    bus.register(from.clone()).await;
                }
                SignalingMessage::Offer { ref from, ref to, ref sdp } => {
                    bus.send_from(&from, message.clone()).await;
                }
                SignalingMessage::IceCandidate { ref from, ref to, ref candidate } => {
                    bus.send_from(&from, message.clone()).await;
                }
                SignalingMessage::Answer { ref from, ref to, ref sdp } => {
                    bus.send_from(&from, message.clone()).await;
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
