use structs::SignalingMessage;
use tokio::sync::mpsc;
use std::collections::HashMap;
use tokio::sync::Mutex;
use lazy_static::lazy_static;
use xid;

use super::*;

lazy_static! {
    pub static ref SERVER_MNGR: Mutex<ServerMngr> = Mutex::new(ServerMngr::new());
}

pub struct ManagedServer {
    pub sig_tx: mpsc::Sender<SignalingMessage>,
    pub connected_users: u32,
}

pub struct ClientInfo {
    pub client_tx: mpsc::Sender<String>,
    pub server_id: Option<String>,
}

pub struct ServerMngr {
    mngr_server_map: HashMap<String, ManagedServer>,
    client_server_map: HashMap<String, String>, // room_id -> server_id
    client_map: HashMap<String, ClientInfo>,  // client_id -> ClientInfo
}

impl ServerMngr {
    pub fn new() -> Self {
        Self {
            mngr_server_map: HashMap::new(),
            client_server_map: HashMap::new(),
            client_map: HashMap::new(),
        }
    }

    pub fn generate_client_id() -> String {
        xid::new().to_string()
    }

    pub async fn find_available_server(&mut self) -> Option<String> {
        self.select_server().await
    }

    pub async fn select_server(&self) -> Option<String> {
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

    pub async fn add_client(&mut self,client_id: &str, client_tx: mpsc::Sender<String>) {
        self.client_map.insert(client_id.to_string(), ClientInfo {
            client_tx,
            server_id: None,
        });
    
    }

    pub async fn assign_server_to_client(&mut self, client_id: &str, server_id: String) -> bool {
        if let Some(client_info) = self.client_map.get_mut(client_id) {
            client_info.server_id = Some(server_id.clone());
            if let Some(server) = self.mngr_server_map.get_mut(&server_id) {
                server.connected_users += 1;
                return true;
            }
        }
        false
    }

    pub async fn remove_client(&mut self, client_id: &str) {
        if let Some(client_info) = self.client_map.remove(client_id) {
            if let Some(server_id) = client_info.server_id {
                if let Some(server) = self.mngr_server_map.get_mut(&server_id) {
                    server.connected_users = server.connected_users.saturating_sub(1);
                }
            }
        }
    }

    pub async fn get_client_server(&self, client_id: &str) -> Option<String> {
        self.client_map.get(client_id).and_then(|info| info.server_id.clone())
    }

    pub async fn get_client_tx(&self, client_id: &str) -> Option<mpsc::Sender<String>> {
        self.client_map.get(client_id).map(|info| info.client_tx.clone())
    }

    pub async fn user_calling(&mut self, client_id: String) -> Option<String> {
        if let Some(server_id) = self.select_server().await {
            if let Some(svr) = self.get_server(&server_id) {
                svr.connected_users += 1;
                self.client_server_map.insert(client_id, server_id.clone());
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

    let (sig_tx, sig_rx) = mpsc::channel::<SignalingMessage>(100);
    while let Some(Ok(Message::Text(text))) = socket.recv().await {
        // 解析注册消息
        match serde_json::from_str::<SignalingMessage>(&text) {
            Ok(server_msg) => {
                debug!("Server mngr received message: {:?}", server_msg);
                match server_msg {
                    SignalingMessage::ServerAssigned { server_id } => {
                        rtc_server = Some(RtcServer::new(server_id, socket, sig_rx));
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
