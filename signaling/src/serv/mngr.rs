use tokio::sync::mpsc;

use super::*;

lazy_static! {
    pub static ref SERVER_MNGR: Mutex<ServerMngr> = Mutex::new(ServerMngr::new());
}

pub struct ServerMngr {
    // server id -> rtc server
    rtc_server_map: HashMap<String, Arc<Mutex<RtcServer>>>,
    room_server_map: HashMap<String, String>, // room_id -> server_id
}

impl ServerMngr {
    pub fn new() -> Self {
        Self {
            rtc_server_map: HashMap::new(),
            room_server_map: HashMap::new(),
        }
    }

    pub async fn select_server(&self) -> Option<String> {
        // find the server with the least rooms using loop
        let mut min_rooms = usize::MAX;
        let mut min_server_id = None;
        for (server_id, rtc) in self.rtc_server_map.iter() {
            let server = rtc.lock().await;
            if server.managed_rooms.len() < min_rooms {
                min_rooms = server.managed_rooms.len();
                min_server_id = Some(server_id.clone());
            }
        }
        min_server_id
    }

    pub async fn assign_room(&mut self, room_id: String) -> Option<String> {
        if let Some(server_id) = self.select_server().await {
            if let Some(server) = self.rtc_server_map.get(&server_id) {
                let mut server = server.lock().await;
                server.managed_rooms.push(room_id.clone());
                self.room_server_map.insert(room_id, server_id.clone());
                return Some(server_id);
            }
        }
        None
    }

    pub fn get_server(&self, server_id: &str) -> Option<Arc<Mutex<RtcServer>>> {
        self.rtc_server_map.get(server_id).cloned()
    }
}

pub async fn server_mngr_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| server_mngr(socket, state))
}

pub async fn server_mngr(mut socket: WebSocket, state: Arc<AppState>) {
    debug!("Server mngr connected");

    let mut rtc_server = None;

    let (sig_tx, sig_rx) = mpsc::channel::<ServerMsg>(100);
    while let Some(Ok(Message::Text(text))) = socket.recv().await {
        // 解析注册消息
        match serde_json::from_str::<ServerMsg>(&text) {
            Ok(server_msg) => {
                debug!("Server mngr received message: {:?}", server_msg);
                match server_msg.event {
                    ServerEvent::Register => {
                        rtc_server = Some(RtcServer::new(server_msg.server_id, socket, sig_rx));
                        break;
                    }
                    _ => break,
                }
            }
            Err(e) => {
                error!("Server mngr received invalid message: {:?}", e);
            }
        }
    }

    if let Some(rtc_server) = rtc_server {
        let synced_server = Arc::new(Mutex::new(rtc_server));
        {
            let mut server_mngr = SERVER_MNGR.lock().await;
            server_mngr.rtc_server_map.insert(
                synced_server.lock().await.server_id.clone(),
                synced_server.clone(),
            );
        }
        debug!(
            "Server mngr registered server: {:?}",
            synced_server.lock().await.server_id
        );
        synced_server.lock().await.process().await;
        
    };

    // 等待接收任务完成
}
