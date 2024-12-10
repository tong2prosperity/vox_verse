use mngr::SERVER_MNGR;
use structs::{CallRequest, SignalingMessage};

use super::*;

pub async fn caller_handler(
    State(state): State<Arc<AppState>>,
    Json(call_req): Json<CallRequest>,
) -> impl IntoResponse {
    let mut server_mngr = SERVER_MNGR.lock().await;

    let room_id = xid::new().to_string();

    match server_mngr.user_calling(room_id.clone()).await {
        Some(server_id) => {
            // 通知选中的RTC服务器
            if let Some(server) = server_mngr.get_server(&server_id) {
                let msg = SignalingMessage::Call {
                    from: call_req.user_id,
                };

                match server.sig_tx.send(msg).await {
                    Ok(_) => Json(RoomAssignResponse {
                        success: true,
                        server_id: Some(server_id),
                        error: None,
                    }),
                    Err(e) => {
                        error!("send msg to rtc server error: {}", e);
                        Json(RoomAssignResponse {
                            success: false,
                            server_id: None,
                            error: Some(e.to_string()),
                        })
                    }
                }
            } else {
                Json(RoomAssignResponse {
                    success: false,
                    server_id: None,
                    error: Some("Server not found".to_string()),
                })
            }
        }
        None => Json(RoomAssignResponse {
            success: false,
            server_id: None,
            error: Some("No available server".to_string()),
        }),
    }
}
