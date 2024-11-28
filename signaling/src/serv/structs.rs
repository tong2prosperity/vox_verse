use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CallRequest {
    pub user_id: String,
    pub sdp: String,
    pub payload: String,
}
