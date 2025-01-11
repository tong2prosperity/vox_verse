use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::State,
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use msgs::SignalingMessage;
use serde::{de, Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::serv::{RawMessage, ServerEvent};

use super::server_mngr::SERVER_MNGR;
use super::AppState;
use super::*;

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientMsgType {
    Connect,
    Message,
    Offer,
    HangUP,
    Answer,
    Candidate,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClientMsg {
    #[serde(rename = "type")]
    msg_type: ClientMsgType,
    payload: String,
}

#[derive(Debug, Serialize)]
struct ClientResponse {
    #[serde(rename = "type")]
    msg_type: String,
    client_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    server_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

pub async fn client_call_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    debug!("New WebSocket upgrade request received");
    ws.on_upgrade(|socket| handle_client_ws(socket, state))
}

async fn handle_client_ws(socket: WebSocket, state: Arc<AppState>) {
    info!("New WebSocket connection established");
    let (mut sender, mut receiver) = socket.split();
    let (msg_tx, mut msg_rx) = mpsc::channel::<String>(100);

    info!("New client registered with ID: ");
    // 获取client_id 先读取一次信息
    let msg = if let Some(Ok(Message::Text(text))) = receiver.next().await {
        text
    } else {
        warn!("Failed to receive message from client");
        return;
    };
    let (cli_id, server_id) = if let Ok(client_msg) = serde_json::from_str::<SignalingMessage>(&msg)
    {
        match client_msg {
            SignalingMessage::ClientConnect { client_id } => {
                info!("New client registered with ID: {}", &client_id);
                let mut server_mngr = SERVER_MNGR.lock().await;
                server_mngr
                    .register_client(&client_id, msg_tx.clone())
                    .await;
                if let Some(server_id) = server_mngr.assign_server_to_client(&client_id).await {
                    debug!(
                        "Found available server {} for client {}",
                        server_id, client_id
                    );

                    info!(
                        "Successfully assigned server {} to client {}",
                        server_id, client_id
                    );
                    let response = SignalingMessage::ClientConnected {
                        client_id: client_id.clone(),
                        server_id: server_id.clone(),
                    };
                    debug!("Sending server assignment response: {:?}", response);
                    let _ = msg_tx.send(serde_json::to_string(&response).unwrap()).await;

                    server_mngr
                        .forward_to_server_by_client(
                            &client_id.clone(),
                            serde_json::from_str::<SignalingMessage>(&msg).unwrap(),
                        )
                        .await;
                    (client_id.clone(), server_id.clone())
                } else {
                    warn!("No available server found for client {}", client_id);
                    return;
                }
            }
            _ => {
                warn!("Invalid message type from client");
                return;
            }
        }
    } else {
        warn!("Failed to parse message from client for the first frame");
        return;
    };

    let cli_id_copy = cli_id.clone();

    let mut receive_task = tokio::spawn(async move {
        debug!("Starting WebSocket receive task for client");
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            debug!("Received message from client  {}", text);
            if let Ok(msg) = serde_json::from_str::<SignalingMessage>(&text) {
                let mut server_mngr = SERVER_MNGR.lock().await;

                match msg {
                    _ => {
                        let to_pass = text.clone();
                        info!("passing through the message from client {:?}", to_pass);
                        let msg = serde_json::from_str::<SignalingMessage>(&to_pass).unwrap();
                        server_mngr
                            .forward_to_server_by_client(&cli_id.clone(), msg)
                            .await;
                        //let _ = msg_tx.send(to_pass).await;
                    }
                }
            } else {
                warn!("Failed to parse message from client ");
            }
        }
    });

    // Spawn task to send messages to the WebSocket
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = msg_rx.recv().await {
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = (&mut receive_task) => send_task.abort(),
        _ = (&mut send_task) => receive_task.abort(),
    };

    // Clean up when the connection is closed
    let mut server_mngr = SERVER_MNGR.lock().await;
    info!("Cleaning up connection for client {}", cli_id_copy);
    server_mngr.remove_client(&cli_id_copy).await;
    debug!("Client {} removed from server manager", cli_id_copy);
}
