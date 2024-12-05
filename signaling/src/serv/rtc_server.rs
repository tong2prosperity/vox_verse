use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use log::{debug, error};
use tokio::sync::{mpsc::Receiver, Mutex};

use crate::serv::ServerEvent;
use crate::serv::mngr::SERVER_MNGR;

use super::ServerMsg;



pub struct RtcServer {
    ws: WebSocket,
    sig_rx: Receiver<ServerMsg>,
    pub server_id: String,
    pub managed_rooms: Vec<String>,
}

impl RtcServer {
    pub fn new(server_id: String, ws: WebSocket, sig_rx: Receiver<ServerMsg>) -> Self {
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
                        Some(Ok(Message::Text(text))) => debug!("{}", text),
                        Some(Err(_)) | None => {
                            // WebSocket connection closed or error occurred
                            self.cleanup().await;
                            return;
                        }
                        _ => {}
                    }
                }
                inner_msg = self.sig_rx.recv() => {
                    match inner_msg {
                        Some(msg) => {
                            match msg.event {
                                ServerEvent::Calling => {
                                    debug!("recv oncalling msg: {}", serde_json::to_string(&msg).unwrap());
                                    self.send(serde_json::to_string(&msg).unwrap()).await;
                                }
                                _ => {
                                    debug!("recv msg: {}", serde_json::to_string(&msg).unwrap());
                                }
                            }
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
