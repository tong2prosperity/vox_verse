use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use tokio::sync::{mpsc::Receiver, Mutex};

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
            select! {
                msg = self.ws.recv() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => println!("{}", text),
                        _ => break,
                    }
                }
                inner_msg = self.sig_rx.recv() => {
                    match inner_msg {
                        Ok(msg) => {
                            match msg.event {
                                ServerEvent::OnCalling => {
                                    self.send(msg.payload).await;
                                }
                                _ => {}
                            }
                        }
                        Err(e) => error!("{}", e),
                    }
                }
            }
        }
    }

    pub async fn send(&mut self, msg: String) {
        self.ws.send(Message::Text(msg)).await.unwrap();
    }
}
