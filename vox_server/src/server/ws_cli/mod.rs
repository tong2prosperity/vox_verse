use std::collections::HashMap;

use super::*;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use signal_cli::{
    msgs::{CallingPayload, CandidatePayload, ServerEvent, ServerMsg},
    SERVER_ID,
};
use tokio::{net::TcpStream, sync::mpsc};
use tokio_tungstenite::connect_async;
use tungstenite::client::IntoClientRequest;
use tungstenite::protocol::Message;

use url::Url;

use crate::{bot::bot::Bot, config::CONFIG, msg_center::signaling_msgs::SignalingMessage};

use super::rtc::traits::WebRTCHandler;

async fn run_websocket_client() {
    let url = Url::parse(&CONFIG.read().await.server.signaling_server).unwrap();
    let url_str = url.as_str().into_client_request().unwrap();

    // 连接到 WebSocket 服务器
    let (ws_stream, _) = connect_async(url_str).await.expect("Failed to connect");
    info!("Connected to the server");

    let (mut write, mut read) = ws_stream.split();

    // 注册为 RTC 服务器
    let register_msg = ServerMsg {
        server_type: "rtc".to_string(),
        server_id: SERVER_ID.to_string(),
        payload: "".to_string(),
        event: ServerEvent::Register,
    };

    write
        .send(Message::Text(serde_json::to_string(&register_msg).unwrap()))
        .await
        .expect("Failed to send register message");

    // 管理所有 bot 实例
    let mut bots: HashMap<String, Bot> = HashMap::new();

    // 处理接收到的消息
    while let Some(Ok(msg)) = read.next().await {
        if let Message::Text(text) = msg {
            match serde_json::from_str::<ServerMsg>(&text) {
                Ok(server_msg) => {
                    match server_msg.event {
                        ServerEvent::Calling => {
                            // if let Ok(payload) = serde_json::from_str::<CallingPayload>(&server_msg.payload) {
                            //     // 为新用户创建 bot
                            //     let mut bot = Bot::new(CONFIG.read().await.clone()).await.unwrap();
                            //     bot.setup_audio_processor().await;
                            //     bots.insert(payload.user_id.clone(), bot);

                            //     // 处理 offer
                            //     if let Some(bot) = bots.get_mut(&payload.user_id) {
                            //         let answer = bot.generate_answer(payload.sdp).await;

                            //         // 发送 answer
                            //         let answer_msg = ServerMsg {
                            //             server_type: "rtc".to_string(),
                            //             server_id: SERVER_ID.to_string(),
                            //             payload: serde_json::json!({
                            //                 "type": "answer",
                            //                 "sdp": answer,
                            //                 "user_id": payload.user_id,
                            //             }).to_string(),
                            //             event: ServerEvent::Answer,
                            //         };

                            //         write.send(Message::Text(serde_json::to_string(&answer_msg).unwrap()))
                            //             .await
                            //             .expect("Failed to send answer");
                            //     }
                            // }
                        }
                        ServerEvent::Candidate => {
                            if let Ok(payload) =
                                serde_json::from_str::<CandidatePayload>(&server_msg.payload)
                            {
                                if let Some(bot) = bots.get_mut(&payload.user_id) {
                                    bot.handle_candidate(payload.candidate).await;
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Err(e) => error!("Failed to parse server message: {}", e),
            }
        }
    }
}

pub async fn run_signaling_client(bus_tx: mpsc::Sender<SignalingMessage>) {
    let url = Url::parse(&CONFIG.read().await.server.signaling_server).unwrap();
    let url_str = url.as_str().into_client_request().unwrap();

    // 连接到 WebSocket 服务器
    
    let (ws_stream, _) = connect_async(url_str).await.expect("Failed to connect");
    info!("Connected to the server");

    let (mut write, mut read) = ws_stream.split();

    // 注册为 RTC 服务器
    let register_msg = SignalingMessage::ServerRegister {
        server_id: SERVER_ID.to_string(),
    };

    write
        .send(Message::Text(serde_json::to_string(&register_msg).unwrap()))
        .await
        .expect("Failed to send register message");

    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                let msg = serde_json::from_str::<SignalingMessage>(&text).unwrap();
                bus_tx.send(msg).await.unwrap();
            }
            _ => {
                error!("recv msg: {:?}", msg);
            }
        }
    }
}

// async fn handle_server_message(ws_stream: &mut tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>, msg: &str) {
//     let data: serde_json::Value = serde_json::from_str(msg).unwrap();

//     match data.get("type").and_then(|t| t.as_str()) {
//         Some("offer") => {
//             // 处理 offer，生成 answer
//             if let Some(sdp) = data.get("sdp") {
//                 let sdp_str = sdp.as_str().unwrap();
//                 let mut bot = Bot::new(CONFIG.read().await.clone()).await.unwrap();
//                 let answer = bot.generate_answer(sdp_str.to_owned()).await;
//                 // 发送 answer 回服务器
//                 let answer_msg = json!({
//                     "type": "answer",
//                     "room": data.get("room").and_then(|r| r.as_str()).unwrap(),
//                     "sdp": answer
//                 });
//                 ws_stream.send(Message::Text(answer_msg.to_string())).await.unwrap();
//             }
//         }
//         Some("candidate") => {
//             if let Some(candidate) = data.get("candidate") {
//                 let candidate_str = candidate.as_str().unwrap();
//                 let mut bot = Bot::new(CONFIG.read().await.clone()).await.unwrap();
//                 bot.handle_candidate(candidate_str.to_owned()).await;
//             }
//         }
//         _ => {}
//     }
// }

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
