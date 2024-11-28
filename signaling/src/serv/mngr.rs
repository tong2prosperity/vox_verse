use tokio::sync::mpsc;

use super::*;

lazy_static! {
    pub static ref SERVER_MNGR: Mutex<ServerMngr> = Mutex::new(ServerMngr::new());
}

pub struct ManagedServer {
    pub sig_tx: mpsc::Sender<ServerMsg>,
    pub connected_users: u32,
}

pub struct ServerMngr {
    // server id -> rtc server
    //rtc_server_map: HashMap<String, Arc<Mutex<RtcServer>>>,
    mngr_server_map: HashMap<String, ManagedServer>,
    room_server_map: HashMap<String, String>, // room_id -> server_id
}

impl ServerMngr {
    pub fn new() -> Self {
        Self {
            //      rtc_server_map: HashMap::new(),
            mngr_server_map: HashMap::new(),
            room_server_map: HashMap::new(),
        }
    }

    pub async fn select_server(&self) -> Option<String> {
        // find the server with the least rooms using loop
        let mut min_users = u32::MAX;
        let mut min_server_id = None;
        for (server_id, svr) in self.mngr_server_map.iter() {
            if svr.connected_users < min_users {
                min_users = svr.connected_users;
                min_server_id = Some(server_id.clone());
            }
        }
        min_server_id
    }

    pub async fn assign_room(&mut self, room_id: String) -> Option<String> {
        if let Some(server_id) = self.select_server().await {
            if let Some(svr) = self.get_server(&server_id) {
                svr.connected_users += 1;
                self.room_server_map.insert(room_id, server_id.clone());
                return Some(server_id);
            }
        }
        None
    }

    pub fn remove_rtc_server(&mut self, server_id: &str) {
        let svr = self.mngr_server_map.remove(server_id);
        if let Some(svr) = svr {
            info!("remove rtc server");
            //self.room_server_map.remove(&svr.room_id);
        }
    }

    pub fn get_server(&mut self, server_id: &str) -> Option<&mut ManagedServer> {
        self.mngr_server_map.get_mut(server_id)
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
        let server_id = rtc_server.server_id.clone();
        {
            let mut server_mngr = SERVER_MNGR.lock().await;
            server_mngr.mngr_server_map.insert(
                rtc_server.server_id.clone(),
                ManagedServer {
                    sig_tx,
                    connected_users: 0,
                },
            );
        }

        debug!("Server mngr registered server: {:?}", &server_id);
        rtc_server.process().await;
        debug!("one rtc server process exited, server_id: {:?}", server_id);
    };

    // 等待接收任务完成
}
