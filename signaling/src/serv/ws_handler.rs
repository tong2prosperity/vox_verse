use std::sync::Arc;
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    extract::State,
};
use tokio::sync::mpsc;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use super::mngr::SERVER_MNGR;
use super::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct WsMessage {
    #[serde(rename = "type")]
    msg_type: String,
    payload: serde_json::Value,
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

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();
    let (msg_tx, mut msg_rx) = mpsc::channel::<String>(100);

    // Register the client
    let client_id = {
        let mut server_mngr = SERVER_MNGR.lock().await;
        server_mngr.add_client(msg_tx.clone()).await
    };

    // Send client ID back to the client
    let connect_response = ClientResponse {
        msg_type: "connected".to_string(),
        client_id: client_id.clone(),
        server_id: None,
        error: None,
    };
    if let Err(e) = sender.send(Message::Text(serde_json::to_string(&connect_response).unwrap())).await {
        error!("Failed to send client ID: {}", e);
        return;
    }

    // Spawn task to receive messages from the WebSocket
    let client_id_clone = client_id.clone();
    let mut receive_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            if let Ok(msg) = serde_json::from_str::<WsMessage>(&text) {
                let mut server_mngr = SERVER_MNGR.lock().await;
                
                match msg.msg_type.as_str() {
                    "connect" => {
                        if let Some(server_id) = server_mngr.find_available_server().await {
                            if server_mngr.assign_server_to_client(&client_id_clone, server_id.clone()).await {
                                let response = ClientResponse {
                                    msg_type: "server_assigned".to_string(),
                                    client_id: client_id_clone.clone(),
                                    server_id: Some(server_id.clone()),
                                    error: None,
                                };
                                let _ = msg_tx.send(serde_json::to_string(&response).unwrap()).await;
                            }
                        }
                    },
                    "message" => {
                        if let Some(server_id) = server_mngr.get_client_server(&client_id_clone).await {
                            if let Some(server) = server_mngr.get_server(&server_id) {
                                if let Err(e) = server.sig_tx.send(ServerMsg {
                                    server_type: "rtc".to_string(),
                                    server_id,
                                    payload: msg.payload,
                                    event: ServerEvent::Message,
                                }).await {
                                    error!("Failed to forward message to server: {}", e);
                                    let response = ClientResponse {
                                        msg_type: "error".to_string(),
                                        client_id: client_id_clone.clone(),
                                        server_id: None,
                                        error: Some(format!("Failed to send message: {}", e)),
                                    };
                                    let _ = msg_tx.send(serde_json::to_string(&response).unwrap()).await;
                                }
                            }
                        }
                    },
                    _ => {
                        error!("Unknown message type: {}", msg.msg_type);
                        let response = ClientResponse {
                            msg_type: "error".to_string(),
                            client_id: client_id_clone.clone(),
                            server_id: None,
                            error: Some(format!("Unknown message type: {}", msg.msg_type)),
                        };
                        let _ = msg_tx.send(serde_json::to_string(&response).unwrap()).await;
                    }
                }
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
    server_mngr.remove_client(&client_id).await;
}
