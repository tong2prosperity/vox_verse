use serde::{Serialize, Deserialize};

#[repr(i32)]
#[derive(Debug, Serialize, Deserialize)]
pub enum ServerEvent {
    Register,
    Unregister,

    Calling,
    Candidate,
    Answer,
}



#[derive(Debug, Serialize, Deserialize)]
pub struct ServerMsg {
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



#[derive(Debug, Serialize, Deserialize)]
pub struct CallingPayload {
    pub room_id: String,
    pub user_id: String,
    pub sdp: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CandidatePayload {
    pub user_id: String,
    pub candidate: String,
}