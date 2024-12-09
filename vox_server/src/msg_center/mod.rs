pub mod signaling_msgs;
pub mod msg_bus;
use signaling_msgs::SignalingMessage;

use crate::{error, info, debug, warn};


pub trait MessageHandler {
    fn handle_message(&self, message: SignalingMessage);
}