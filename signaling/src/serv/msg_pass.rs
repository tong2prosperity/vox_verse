use server_mngr::SERVER_MNGR;
use msgs::{CallRequest, SignalingMessage};
use tokio::sync::mpsc;

use super::*;

pub async fn caller_handler(
    State(state): State<Arc<AppState>>,
    Json(call_req): Json<CallRequest>,
) -> impl IntoResponse {
    let mut server_mngr = SERVER_MNGR.lock().await;

    let client_id = xid::new().to_string();

    let (client_tx, client_rx) = mpsc::channel(100);

    server_mngr.register_client(&client_id, client_tx.clone()).await;

    match server_mngr.assign_server_to_client(&client_id).await {
        Some(server_id) => {
            // 通知选中的RTC服务器
            server_mngr.forward_to_server(&server_id, SignalingMessage::ClientConnected {
                client_id: client_id,
                server_id: server_id.clone(),
            }).await;
            Json(RoomAssignResponse {
                success: true,
                server_id: Some(server_id),
                error: None,
            })
        }
        None => Json(RoomAssignResponse {
            success: false,
            server_id: None,
            error: Some("No available server".to_string()),
        }),
    }
}
