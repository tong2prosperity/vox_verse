
use mngr::SERVER_MNGR;
use structs::CallRequest;

use super::*;

pub async fn assign_room_handler(
    State(state): State<Arc<AppState>>,
    Json(call_req): Json<CallRequest>,
) -> impl IntoResponse {
    let mut server_mngr = SERVER_MNGR.lock().await;

    let room_id = xid::new().to_string();
    
    match server_mngr.assign_room(room_id.clone()).await {
        Some(server_id) => {
            // 通知选中的RTC服务器
            if let Some(server) = server_mngr.get_server(&server_id) {
                let msg = RTCCallRequest {
                    user_id: call_req.user_id,
                    sdp: call_req.sdp,
                    payload: call_req.payload,
                    event: RTCCallEvent::Call,
                };
                
                let mut server = server.lock().await;
                server.send(serde_json::to_string(&msg).unwrap()).await;
                
                Json(RoomAssignResponse {
                    success: true,
                    server_id: Some(server_id),
                    error: None,
                })
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