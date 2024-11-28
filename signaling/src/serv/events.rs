
use serde::{Serialize, Deserialize};

#[repr(i32)]
#[derive(Debug, Serialize, Deserialize)]
pub enum ServerEvent {
    Register,
    Unregister,
}



#[derive(Debug, Serialize, Deserialize)]
pub struct ServerMsg {
    pub server_id: String,
    pub event: ServerEvent,
}