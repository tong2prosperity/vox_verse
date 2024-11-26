use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use futures::{sink::SinkExt, stream::StreamExt};
use std::{
    collections::HashSet,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::sync::broadcast;

#[derive(Clone)]
struct AppState {
    sender: broadcast::Sender<String>,
}

#[tokio::main]
async fn main() {
    // 创建广播通道
    let (sender, _) = broadcast::channel(16);
    let app_state = Arc::new(AppState { sender });

    // 创建路由
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(app_state);

    // 启动服务器
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    println!("Listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();

    // 订阅广播通道
    let mut rx = state.sender.subscribe();

    // 处理接收到的消息
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    // 处理发送的消息
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            // 广播消息给所有客户端
            if state.sender.send(text).is_err() {
                break;
            }
        }
    });

    // 等待任一任务完成
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };
}