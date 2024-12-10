use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CallRequest {
    pub user_id: String,
    pub sdp: String,
    pub payload: String,
}

use std::fmt::Debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum SignalingMessage {
    // 连接管理
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
