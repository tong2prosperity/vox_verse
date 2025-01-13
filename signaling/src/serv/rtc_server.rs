use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use broadcast::error;
use log::{debug, error};
use serde::Deserialize;
use tokio::sync::{mpsc::Receiver, Mutex};

use crate::serv::{server_mngr::SERVER_MNGR, ServerEvent};

use super::*;
use super::{msgs::SignalingMessage, RawMessage};

pub struct RtcServer {
    ws: WebSocket,
    msg_bus: Receiver<SignalingMessage>,
    pub server_id: String,
    pub managed_rooms: Vec<String>,
}

impl RtcServer {
    pub fn new(server_id: String, ws: WebSocket, bus_rx: Receiver<SignalingMessage>) -> Self {
        Self {
            server_id,
            ws,
            msg_bus: bus_rx,
            managed_rooms: Vec::new(),
        }
    }

    pub async fn process(mut self) {
        loop {
            tokio::select! {
                // websocket 接收消息 发送给messageBus
                msg = self.ws.recv() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            if let Ok(signaling_msg) = serde_json::from_str::<SignalingMessage>(&text) {
                                info!("recv signaling msg: {}", text);
                                // rtc server 收到消息后，需要发给对应的client！！！！
                                self.handle_message(signaling_msg).await;
                            }
                        },
                        Some(Err(_)) | None => {
                            debug!("server disconnected");
                            self.cleanup().await;
                            return;
                        }
                        _ => {
                            debug!("recv msg: {:?}", msg);
                        }
                    }
                }
                // messageBus 接收消息 发送给websocket
                inner_msg = self.msg_bus.recv() => {
                    match inner_msg {
                        Some(msg) => {
                            self.send(serde_json::to_string(&msg).unwrap()).await;
                            // match msg {
                            //     SignalingMessage::Call {from} => {
                            //         self.send(serde_json::to_string(&msg).unwrap()).await;
                            //     }
                            //     SignalingMessage::Offer {from, to, sdp} => {
                            //         debug!("recv onmessage msg: {}", serde_json::to_string(&msg).unwrap());
                            //         self.send(serde_json::to_string(&msg).unwrap()).await;
                            //     }
                            //     _ => {
                            //         debug!("recv msg: {}", serde_json::to_string(&msg).unwrap());
                            //     }
                            // }
                        }
                        None => break,
                    }
                }
            }
        }
    }

    pub async fn handle_message(&mut self, msg: SignalingMessage) {
        let msg_str = serde_json::to_string(&msg).unwrap();
        match msg {
            SignalingMessage::Answer { from, to, sdp } => {
                self.forward_to_client(&to, msg_str.clone())
                    .await;
            }
            SignalingMessage::IceCandidate {
                from,
                to,
                candidate,
            } => {
                self.forward_to_client(&to, msg_str.clone())
                    .await;
            }
            SignalingMessage::Error { code, message } => {
                error!("rtc server handle message error: {}", message);
            }
            _ => {
                warn!("unsupported msg");
            }
        }
    }

    pub async fn forward_to_client(&mut self, client_id: &str, msg: String) {
        let server_mngr = SERVER_MNGR.lock().await;
        let result = server_mngr.forward_to_client(client_id, msg).await;
        if result {
            info!("forward to client success");
        } else {
            error!("forward to client failed");
        }
    }

    pub async fn send(&mut self, msg: String) {
        debug!("send msg: {}", msg);
        match self.ws.send(Message::Text(msg)).await {
            Ok(_) => debug!("send msg success"),
            Err(e) => error!("send msg error: {}", e),
        }
    }

    async fn cleanup(&mut self) {
        // Clear managed rooms
        self.managed_rooms.clear();

        // Remove the server from the global manager
        let mut server_mngr = SERVER_MNGR.lock().await;
        server_mngr.remove_server(&self.server_id).await;
    }
}
