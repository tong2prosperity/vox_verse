// create a struct for json serialize and deserialize

use serde::{Serialize, Deserialize};


#[repr(i32)]
#[derive(Serialize, Deserialize)]
pub enum MessageType {
    Offer,
    Answer,
}


#[derive(Serialize, Deserialize)]
pub struct RoomMessage {
    pub message_type: MessageType,
    pub room_id: String,
    pub user_id: String,
    pub sdp: String,
}