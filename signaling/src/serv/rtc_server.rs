use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
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
                        Some(Ok(Message::Text(text))) => println!("{}", text),
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
                                ServerEvent::OnCalling => {
                                    self.send(msg.payload).await;
                                }
                                _ => {}
                            }
                        }
                        None => break,
                    }
                }
            }
        }
    }

    pub async fn send(&mut self, msg: String) {
        self.ws.send(Message::Text(msg)).await.unwrap();
    }

    async fn cleanup(&mut self) {
        // Clear managed rooms
        self.managed_rooms.clear();

        // Remove the server from the global manager
        let mut server_mngr = SERVER_MNGR.lock().await;
        server_mngr.remove_rtc_server(&self.server_id);
    }
}
