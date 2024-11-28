use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures::StreamExt;
use rtc_server::RtcServer;
use std::collections::HashMap;
use std::sync::Arc;

use crate::app::AppState;

use super::*;

use lazy_static::lazy_static;
use tokio::sync::Mutex;

lazy_static! {
    pub static ref SERVER_MNGR: Mutex<ServerMngr> = Mutex::new(ServerMngr {
        rtc_server_map: HashMap::new()
    });
}

pub struct ServerMngr {
    // server id -> rtc server
    rtc_server_map: HashMap<String, Arc<Mutex<RtcServer>>>,
}

impl ServerMngr {
    pub fn new() -> Self {
        Self {
            rtc_server_map: HashMap::new(),
        }
    }
}

pub async fn server_mngr_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| server_mngr(socket, state))
}

pub async fn server_mngr(mut socket: WebSocket, state: Arc<AppState>) {
    //let (mut sender, mut receiver) = socket.split();

    let mut server_mngr = SERVER_MNGR.lock().await;

    let mut rtc_server = None;
    while let Some(Ok(Message::Text(text))) = socket.recv().await {
        // 解析注册消息
        if let Ok(server_msg) = serde_json::from_str::<ServerMsg>(&text) {
            match server_msg.event {
                ServerEvent::Register => {
                    rtc_server = Some(RtcServer::new(server_msg.server_id, socket));
                    break;
                }
                _ => break,
            }
        }
    }

    if let Some(rtc_server) = rtc_server {
        let synced_server = Arc::new(Mutex::new(rtc_server));
        server_mngr
            .rtc_server_map
            .insert(synced_server.lock().await.server_id.clone(), synced_server.clone());

        synced_server.lock().await.process().await;
    };

    // 等待接收任务完成

}
