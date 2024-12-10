use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use log::{debug, error};
use serde::Deserialize;
use tokio::sync::{mpsc::Receiver, Mutex};

use crate::serv::{mngr::SERVER_MNGR, ServerEvent};

use super::*;
use super::{structs::SignalingMessage, RawMessage};

pub struct RtcServer {
    ws: WebSocket,
    sig_rx: Receiver<SignalingMessage>,
    pub server_id: String,
    pub managed_rooms: Vec<String>,
}

impl RtcServer {
    pub fn new(server_id: String, ws: WebSocket, sig_rx: Receiver<SignalingMessage>) -> Self {
        Self {
            server_id,
            ws,
            sig_rx,
            managed_rooms: Vec::new(),
        }
    }

    pub async fn process(mut self) {
        loop {
            tokio::select! {
                msg = self.ws.recv() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            if let Ok(signaling_msg) = serde_json::from_str::<SignalingMessage>(&text) {
                                info!("recv signaling msg: {}", text);
                                self.ws.send(Message::Text(serde_json::to_string(&signaling_msg).unwrap())).await.unwrap();
                            }

                            if let Ok(msg) = serde_json::from_str::<RawMessage>(&text) {
                                info!("recv raw msg: {}", text);
                                match msg.event {
                                    ServerEvent::Answer => {
                                        // 转发 answer 给对应的客户端
                                        if let Ok(payload) = serde_json::from_str::<serde_json::Value>(&msg.payload) {
                                            if let Some(user_id) = payload.get("user_id").and_then(|v| v.as_str()) {
                                                let mut server_mngr = SERVER_MNGR.lock().await;
                                                if let Some(client_tx) = server_mngr.get_client_tx(user_id).await {
                                                    let _ = client_tx.send(text).await;
                                                }
                                            }
                                        }
                                    },
                                    _ => debug!("recv msg: {}", text),
                                }
                            }
                        },
                        Some(Err(_)) | None => {
                            self.cleanup().await;
                            return;
                        }
                        _ => {}
                    }
                }
                inner_msg = self.sig_rx.recv() => {
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
        server_mngr.remove_rtc_server(&self.server_id);
    }
}
