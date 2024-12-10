pub mod msg_bus;
pub mod signaling_msgs;
use signaling_msgs::SignalingMessage;

use crate::{debug, error, info, warn};

pub trait MessageHandler {
    fn handle_message(&self, message: SignalingMessage);
}
