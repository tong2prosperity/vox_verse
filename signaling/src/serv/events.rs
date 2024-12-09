
use serde::{Serialize, Deserialize};

#[repr(i32)]
#[derive(Debug, Serialize, Deserialize)]
pub enum ServerEvent {
    Register,
    Unregister,
    Calling,

    Message,

    Answer,
    Candidate,
}



#[derive(Debug, Serialize, Deserialize)]
pub struct RawMessage {
    pub server_type: String,
    pub server_id: String,
    pub payload: String,
    pub event: ServerEvent,
}


#[repr(i32)]
#[derive(Debug, Serialize, Deserialize)]
pub enum RTCCallEvent {
    Call,
    Answer,
    Reject,
}


#[derive(Debug, Serialize, Deserialize)]
pub struct RTCCallRequest {
    pub user_id: String,
    pub sdp: String,
    pub payload: String,
    pub event: RTCCallEvent,
}




#[derive(Debug, Serialize, Deserialize)]
pub struct RoomAssignResponse {
    pub success: bool,
    pub server_id: Option<String>,
    pub error: Option<String>,
}
