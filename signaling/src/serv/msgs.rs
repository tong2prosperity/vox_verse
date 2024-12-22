use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CallRequest {
    pub user_id: String,
    pub sdp: String,
    pub payload: String,
}

use std::fmt::Debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum SignalingMessage {
    // 服务器管理
    ServerRegister { server_id: String },
    ServerRegistered { server_id: String },
    ServerDisconnect { server_id: String },
    
    // 客户端管理
    ClientConnect { client_id: String },
    ClientConnected { client_id: String, server_id: String },
    ClientDisconnect { client_id: String },
    
    // WebRTC 信令
    Offer { from: String, to: String, sdp: String },
    Answer { from: String, to: String, sdp: String },
    IceCandidate { from: String, to: String, candidate: String },
    
    // 错误处理
    Error { code: i32, message: String }
}