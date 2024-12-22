use serde::{Deserialize, Serialize};
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
// 用于 WebSocket 传输的消息包装
#[derive(Debug, Serialize, Deserialize)]
pub struct WsMessage {
    pub message_type: SignalingMessage,
    pub payload: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_signaling_message_json() {
        let message = SignalingMessage::ClientConnect {
            client_id: "client_123".to_string(),
        };

        let json = serde_json::to_string(&message).unwrap();
        println!("ClientConnect JSON: {}", json);

        let message = SignalingMessage::Offer {
            from: "client_123".to_string(),
            to: "client_456".to_string(),
            sdp: "sdp_data".to_string(),
        };

        let json = serde_json::to_string(&message).unwrap();
        println!("Offer JSON: {}", json);

        // Add more tests for other variants as needed
    }
}
