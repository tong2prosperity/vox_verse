use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use std::sync::Arc;
use std::collections::HashMap;

use super::*;

pub struct ServerMngr {
    // server id -> rtc server
    rtc_server_map: HashMap<String, RtcServer>,
}

impl ServerMngr {
    pub fn new() -> Self {
        Self
    }
    pub async fn server_mngr(&mut self, socket: WebSocket, state: Arc<AppState>) -> impl IntoResponse {
        let (mut sender, mut receiver) = socket.split();

        let rtc_server_map = &mut self.rtc_server_map;
        
        let mut recv_task = tokio::spawn(async move {
            while let Some(Ok(Message::Text(text))) = receiver.next().await {
                // 解析注册消息
                if let Ok(server_msg) = serde_json::from_str::<ServerMsg>(&text) {
                    match server_msg.event {
                        ServerEvent::Register => {
                            rtc_server_map.insert(server_msg.server_id.clone(), RtcServer::new(server_msg.server_id));
                        }
                        ServerEvent::Unregister => {
                            rtc_server_map.remove(&server_msg.server_id);
                        }
                    }
                }
            }
        });

        // 等待接收任务完成
        recv_task.await.unwrap();

        
    }




}



