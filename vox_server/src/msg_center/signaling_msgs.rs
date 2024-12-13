use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum SignalingMessage {
    // 连接管理
    ClientConnect {
        client_id: String,
    },
    Call {
        from: String,
    },
    ServerAssigned {
        server_id: String,
    },

    // WebRTC 信令
    Offer {
        from: String,
        to: String,
        sdp: String,
    },
    Answer {
        from: String,
        to: String,
        sdp: String,
    },
    IceCandidate {
        //    room_id: String,
        from: String,
        to: String,
        candidate: String,
    },

    // 房间事件
    RoomCreated {
        room_id: String,
    },
    UserJoined {
        room_id: String,
        user_id: String,
    },
    UserLeft {
        room_id: String,
        user_id: String,
    },

    // 音频处理
    AudioData {
        room_id: String,
        user_id: String,
        data: Vec<i16>,
    },
    AudioProcessingResult {
        room_id: String,
        user_id: String,
        result: AudioProcessingResult,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AudioProcessingResult {
    VadResult(bool),
    AsrResult(String),
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
        let message = SignalingMessage::Call {
            from: "client_123".to_string(),
        };

        let json = serde_json::to_string(&message).unwrap();
        println!("ClientConnect JSON: {}", json);

        let message = SignalingMessage::ServerAssigned {
            server_id: "server_456".to_string(),
        };

        let json = serde_json::to_string(&message).unwrap();
        println!("ServerAssigned JSON: {}", json);

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
