pub mod client_handler;
pub mod events;
pub mod server_mngr;
pub mod msg_pass;
pub mod rtc_server;
pub mod msgs;

use events::*;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    Json,
};
use futures::{executor::block_on, StreamExt};
use rtc_server::RtcServer;
use std::collections::HashMap;
use std::sync::Arc;

use crate::app::AppState;

use super::*;
use log::{debug, error, info, warn};

use lazy_static::lazy_static;
use tokio::sync::Mutex;
