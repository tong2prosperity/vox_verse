use serde_json::json;
use tokio_tungstenite::connect_async;
use tokio::net::TcpStream;
use tungstenite::protocol::Message;
use tungstenite::client::IntoClientRequest;
use futures_util::{SinkExt, StreamExt};
use super::*;

use url::Url;

use crate::{bot::bot::Bot, config::CONFIG};

use super::rtc::traits::WebRTCHandler;


async fn run_websocket_client() {
    // WebSocket 服务器的 URL
    let url = Url::parse("wss://your-signaling-server.com/room").unwrap();
    let url_str = url.as_str().into_client_request().unwrap();


    // 连接到 WebSocket 服务器
    let (ws_stream, _) = connect_async(url_str).await.expect("Failed to connect");

    info!("Connected to the server");

    // 分离 WebSocket 流为发送和接收部分
    let  (mut write, read) = ws_stream.split();

    // 设定角色
    let role = "bot"; // 可以是 "bot" 或其他角色

    // 发送角色信息到服务器
    let role_message = Message::Text(format!("{{\"role\": \"{}\"}}", role));
    write.send(role_message).await.expect("Failed to send role");

    // 接收消息
    tokio::spawn(async move {
        read.for_each(|message| async {
            match message {
                Ok(msg) => {
                    if let Message::Text(text) = msg {
                        println!("Received: {}", text);
                        // 处理接收到的 WebRTC offer 并生成 answer
                        // 这里预留接口
                    }
                }
                Err(e) => {
                    eprintln!("Error receiving message: {}", e);
                }
            }
        }).await;
    });

    // 其他逻辑...
}

async fn handle_server_message(ws_stream: &mut tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>, msg: &str) {
    let data: serde_json::Value = serde_json::from_str(msg).unwrap();

    match data.get("type").and_then(|t| t.as_str()) {
        Some("offer") => {
            // 处理 offer，生成 answer
            if let Some(sdp) = data.get("sdp") {
                let sdp_str = sdp.as_str().unwrap();
                let mut bot = Bot::new(CONFIG.read().await.clone()).await.unwrap();
                let answer = bot.generate_answer(sdp_str.to_owned()).await;
                // 发送 answer 回服务器
                let answer_msg = json!({
                    "type": "answer",
                    "room": data.get("room").and_then(|r| r.as_str()).unwrap(),
                    "sdp": answer
                });
                ws_stream.send(Message::Text(answer_msg.to_string())).await.unwrap();
            }
        }
        Some("candidate") => {
            if let Some(candidate) = data.get("candidate") {
                let candidate_str = candidate.as_str().unwrap();
                let mut bot = Bot::new(CONFIG.read().await.clone()).await.unwrap();
                bot.handle_candidate(candidate_str.to_owned()).await;
            }
        }
        _ => {}
    }
}


// async fn reconnect(url: Url, webrtc_handler: &dyn WebRTCHandler) {
//     loop {
//         match connect_async(url.clone()).await {
//             Ok((mut ws_stream, _)) => {
//                 // 重新加入房间并设置角色
//                 let join_msg = json!({
//                     "action": "join",
//                     "room": "room1",
//                     "role": "bot"
//                 });
//                 ws_stream.send(Message::Text(join_msg.to_string())).await.unwrap();

//                 // 处理消息
//                 while let Some(msg) = ws_stream.next().await {
//                     match msg {
//                         Ok(Message::Text(text)) => {
//                             handle_server_message(&mut ws_stream, &text, webrtc_handler).await;
//                         }
//                         _ => {}
//                     }
//                 }
//             }
//             Err(e) => {
//                 eprintln!("连接失败，正在重试: {:?}", e);
//                 tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
//             }
//         }
//     }
// }

#[tokio::main]
async fn main() {
    run_websocket_client().await;
}
