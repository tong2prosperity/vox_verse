use lazy_static::lazy_static;
use std::collections::HashMap;
use tokio::sync::{mpsc, Mutex};
use xid;

use super::*;
use crate::serv::msgs::SignalingMessage;

lazy_static! {
    pub static ref SERVER_MNGR: Mutex<ServerMngr> = Mutex::new(ServerMngr::new());
}

// 服务器节点信息
pub struct ServerNode {
    pub sig_tx: mpsc::Sender<SignalingMessage>,  // 发送消息到服务器的channel
    pub connected_users: u32,                    // 当前连接的用户数
    pub client_ids: Vec<String>,                 // 该服务器管理的客户端ID列表
}

// 客户端信息
pub struct ClientInfo {
    pub client_tx: mpsc::Sender<String>,         // 发送消息到客户端的channel
    pub server_id: Option<String>,               // 分配的服务器ID
}

pub struct ServerMngr {
    server_nodes: HashMap<String, ServerNode>,    // server_id -> ServerNode
    client_info: HashMap<String, ClientInfo>,     // client_id -> ClientInfo
}

impl ServerMngr {
    pub fn new() -> Self {
        Self {
            server_nodes: HashMap::new(),
            client_info: HashMap::new(),
        }
    }

    // 注册新的服务器节点
    pub async fn register_server(&mut self, server_id: String, sig_tx: mpsc::Sender<SignalingMessage>) {
        self.server_nodes.insert(server_id, ServerNode {
            sig_tx,
            connected_users: 0,
            client_ids: Vec::new(),
        });
    }

    // 注册新的客户端
    pub async fn register_client(&mut self, client_id: &str, client_tx: mpsc::Sender<String>) {
        self.client_info.insert(client_id.to_string(), ClientInfo {
            client_tx,
            server_id: None,
        });
    }

    // 为客户端分配服务器（负载均衡）
    pub async fn assign_server_to_client(&mut self, client_id: &str) -> Option<String> {
        // 找到负载最小的服务器
        let selected_server = self.server_nodes.iter_mut()
            .min_by_key(|(_, node)| node.connected_users)
            .map(|(id, node)| {
                node.connected_users += 1;
                node.client_ids.push(client_id.to_string());
                id.clone()
            });

        // 更新客户端信息
        if let Some(server_id) = &selected_server {
            if let Some(client) = self.client_info.get_mut(client_id) {
                client.server_id = Some(server_id.clone());
            }
        }

        selected_server
    }

    // 转发消息到服务器
    pub async fn forward_to_server(&self, server_id: &str, msg: SignalingMessage) -> bool {
        if let Some(server) = self.server_nodes.get(server_id) {
            match server.sig_tx.send(msg).await {
                Ok(_) => true,
                Err(e) => {
                    error!("Failed to forward message to server: {}", e);
                    false
                }
            }
        } else {
            false
        }
    }

    // 转发消息到客户端
    pub async fn forward_to_client(&self, client_id: &str, msg: String) -> bool {
        if let Some(client) = self.client_info.get(client_id) {
            match client.client_tx.send(msg).await {
                Ok(_) => true,
                Err(e) => {
                    error!("Failed to forward message to client: {}", e);
                    false
                }
            }
        } else {
            false
        }
    }

    // 通过client_id转发消息到对应的server
    pub async fn forward_to_server_by_client(&self, client_id: &str, msg: SignalingMessage) -> bool {
        if let Some(client) = self.client_info.get(client_id) {
            if let Some(server_id) = &client.server_id {
                return self.forward_to_server(server_id, msg).await;
            }
        }
        false
    }

    // 移除客户端
    pub async fn remove_client(&mut self, client_id: &str) {
        if let Some(client) = self.client_info.remove(client_id) {
            if let Some(server_id) = client.server_id {
                if let Some(server) = self.server_nodes.get_mut(&server_id) {
                    server.connected_users = server.connected_users.saturating_sub(1);
                    server.client_ids.retain(|id| id != client_id);
                }
            }
        }
    }

    // 移除服务器
    pub async fn remove_server(&mut self, server_id: &str) {
        let mut client_id_to_remove = None;
        if let Some(server) = self.server_nodes.remove(server_id) {
            // 清理该服务器关联的所有客户端
            for client_id in server.client_ids {
                if let Some(client) = self.client_info.get_mut(&client_id) {
                    client.server_id = None;
                    client_id_to_remove = Some(client_id.clone());
                }
            }
        }
        if let Some(client_id) = client_id_to_remove {
            debug!("remove client: {}", client_id);
            self.remove_client(&client_id).await;
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
    debug!("Server mngr connected");

    let mut rtc_server = None;

    let (sig_tx, sig_rx) = mpsc::channel::<SignalingMessage>(100);
    if let Some(msg) = assert_msg::<SignalingMessage>(&mut socket, "ServerRegistered").await {
        match msg {
            SignalingMessage::ServerRegister { server_id } => {
                rtc_server = Some(RtcServer::new(server_id, socket, sig_rx));
            }
            _ => {
                error!("Unexpected message type, expected ServerRegistered");
                return;
            }
        }
    } else {
        error!("Failed to receive ServerRegistered message");
        return;
    }

    if let Some(rtc_server) = rtc_server {
        let server_id = rtc_server.server_id.clone();
        {
            let mut server_mngr = SERVER_MNGR.lock().await;
            server_mngr
                .register_server(rtc_server.server_id.clone(), sig_tx)
                .await;
        }

        debug!("Server mngr registered server: {:?}", &server_id);
        rtc_server.process().await;
        debug!("one rtc server process exited, server_id: {:?}", server_id);

        {
            let mut server_mngr = SERVER_MNGR.lock().await;
            server_mngr.remove_server(&server_id).await;
        }
        debug!("Server cleaned up: {:?}", server_id);
    }
}

async fn assert_msg<T>(socket: &mut WebSocket, expected_type: &str) -> Option<T> 
where 
    T: serde::de::DeserializeOwned,
{
    if let Some(Ok(Message::Text(text))) = socket.recv().await {
        match serde_json::from_str::<T>(&text) {
            Ok(msg) => {
                debug!("Received expected {} message", expected_type);
                Some(msg)
            }
            Err(e) => {
                error!("Failed to parse {} message: {}", expected_type, e);
                None
            }
        }
    } else {
        error!("Failed to receive {} message", expected_type);
        None
    }
}