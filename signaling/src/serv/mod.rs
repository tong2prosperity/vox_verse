pub mod mngr;
pub mod rtc_server;
pub mod events;
pub mod msg_pass;
pub mod structs;

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

use lazy_static::lazy_static;
use tokio::sync::Mutex;


