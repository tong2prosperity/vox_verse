pub mod msgs;
use msgs::*;

use std::{collections::HashMap, sync::Arc};

use futures_util::{SinkExt, StreamExt};
use msgs::{ServerEvent, ServerMsg};
use serde_json::json;
use tokio::{select, sync::mpsc};
use tokio_tungstenite::{connect_async, WebSocketStream};
use tungstenite::{client::IntoClientRequest, Message};

use crate::config::CONFIG;


use super::*;

// lazy static a server id
lazy_static::lazy_static! {
    static ref SERVER_ID: String = xid::new().to_string();
}

pub enum BotEvent {
    UserJoin(String, String),        // room_id, user_id
    UserLeave(String, String),       // room_id, user_id
    Message(String, String, String), // room_id, user_id, message
}

pub struct UserBotPair {
    pub user_id: String,
    pub bot_id: String,
    pub room_id: String,
    pub bot_event_tx: mpsc::Sender<BotEvent>,
}

pub struct SignalCli {
    // WebSocket 连接地址
    ws_url: String,
    // 可选的发送器，用于发送消息
    ws_stream: Option<WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>,

    roomer_rx: Option<mpsc::Receiver<String>>,
    // 用户-Bot 映射关系管理
    user_bot_pairs: HashMap<String, UserBotPair>,
}

impl SignalCli {
    pub fn new(ws_url: &String) -> Self {
        Self {
            ws_url: ws_url.clone(),
            ws_stream: None,
            roomer_rx: None,
            user_bot_pairs: HashMap::new(),
        }
    }

    // todo: 需要考虑重连
    pub async fn run(ws_url: String) -> Result<()> {
        let mut client = SignalCli::new(&ws_url);

        loop {
            match client.connect().await {
                Ok(_) => {
                    info!(
                        "connect to ws server success, remote url: {}, start handle message",
                        &ws_url
                    );
                    break;
                }
                Err(e) => {
                    error!("connect to ws server error: {}", e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    continue;
                }
            }
        }
        tokio::spawn(async move { client.handle_message().await });

        Ok(())
    }

    // 连接到 WebSocket 服务器
    pub async fn connect(&mut self) -> Result<()> {
        // 如果已经有连接，先关闭它
        if self.ws_stream.is_some() {
            self.close().await?;
        }

        let (roomer_tx, roomer_rx) = mpsc::channel(100);
        self.roomer_rx = Some(roomer_rx);

        let url = Url::parse(&self.ws_url)?;
        let url_str = url.as_str().into_client_request().unwrap();
        let (ws_stream, resp) = connect_async(url_str).await?;
        debug!("connect to ws server success, resp: {:?}", resp);
        self.ws_stream = Some(ws_stream);

        self.register(SERVER_ID.clone()).await?;

        Ok(())
    }

    pub async fn register(&mut self, server_id: String) -> Result<()> {
        let msg = json!({
            "server_id": server_id,
            "server_type": "rtc",
            "payload": "",
            "event": "register"
        });

        self.send_message(serde_json::to_string(&msg)?).await?;
        Ok(())
    }

    // 发送消息
    pub async fn send_message(&mut self, message: String) -> Result<()> {
        if let Some(ws_stream) = &mut self.ws_stream {
            ws_stream.send(Message::Text(message)).await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("WebSocket not connected"))
        }
    }

    // 关闭连接
    pub async fn close(&mut self) -> Result<()> {
        if let Some(ws_stream) = &mut self.ws_stream {
            ws_stream.close(None).await?;
            self.ws_stream = None;
        }
        Ok(())
    }

    // 创建新的用户-Bot 对
    async fn create_user_bot_pair(&mut self, room_id: String, user_id: String) -> Result<()> {
        // 检查是否已存在该用户的配对
        if self.user_bot_pairs.contains_key(&user_id) {
            return Ok(());
        }

        // 为新的 Bot 创建通信通道
        let (bot_event_tx, bot_event_rx) = mpsc::channel(100);

        // 生成新的 bot_id
        let bot_id = xid::new().to_string();
        let user_id_2_bot = user_id.clone();
        let room_id_2_bot = room_id.clone();
        let bot_id_2_bot = bot_id.clone();

        // 启动新的 Bot
        let bot_event_tx_clone = bot_event_tx.clone();
        let rtc_cfg = CONFIG.read().await.rtc.clone();
        tokio::spawn(async move {
            
        });

        // 存储用户-Bot 配对信息
        let pair = UserBotPair {
            user_id: user_id.clone(),
            bot_id: bot_id.clone(),
            room_id: room_id.clone(),
            bot_event_tx: bot_event_tx_clone,
        };

        self.user_bot_pairs.insert(user_id.clone(), pair);

        Ok(())
    }

    // 移除用户-Bot 对
    async fn remove_user_bot_pair(&mut self, user_id: &str) {
        if let Some(pair) = self.user_bot_pairs.remove(user_id) {
            // 通知 Bot 用户离开
            if let Err(e) = pair
                .bot_event_tx
                .send(BotEvent::UserLeave(pair.room_id, pair.user_id))
                .await
            {
                error!("发送用户离开事件失败: {}", e);
            }
        }
    }

    pub async fn handle_message(mut self) -> Result<()> {
        // 在循环前解构 self，这样我们就不会多次借用 self
        let mut ws_stream = self.ws_stream.take().unwrap();
        let mut mpsc_receiver = self.roomer_rx.take().unwrap();
        //let mut user_bot_pairs = self.user_bot_pairs;

        loop {
            select! {
                msg = mpsc_receiver.recv() => {
                    if let Some(msg) = msg {
                        info!("send msg to ws server: {}", msg);
                        ws_stream.send(Message::Text(msg)).await?;
                    }
                }

                ws_msg = ws_stream.next() => {
                    match ws_msg {
                        Some(msg) => {
                            match msg {
                                Ok(m) => {
                                    if let Message::Text(text) = m {
                                        info!("receive msg from ws server: {}", text);
                                        match serde_json::from_str::<ServerMsg>(&text) {
                                            Ok(server_msg) => {
                                                match server_msg.event {
                                                    ServerEvent::Calling => {
                                                        if let Ok(payload) = serde_json::from_str::<CallingPayload>(&server_msg.payload) {
                                                        info!("收到 OnCalling 事件: room_id={}, user_id={}",
                                                            payload.room_id, payload.user_id);

                                                        // 创建新的用户-Bot 配对
                                                        if let Err(e) = self.create_user_bot_pair(
                                                            payload.room_id.clone(),
                                                            payload.user_id.clone()
                                                        ).await {
                                                            error!("创建用户-Bot配对失败: {}", e);
                                                            continue;
                                                        }

                                                        // 通知对应的 Bot 有用户加入
                                                        if let Some(pair) = self.user_bot_pairs.get(&payload.user_id) {
                                                            if let Err(e) = pair.bot_event_tx.send(BotEvent::UserJoin(
                                                                payload.room_id,
                                                                payload.user_id,
                                                            )).await {
                                                                error!("发送用户加入事件失败: {}", e);
                                                            }
                                                        }
                                                    }
                                                }
                                                _ => {
                                                    info!("other server msg: {:?}", server_msg);
                                                }
                                                }
                                            }
                                        Err(e) => {
                                            error!("receive msg error: {}", e);
                                        }
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("receive msg error: {}", e);
                                    break;
                                }
                            }
                        },
                        None => continue,
                    }
                }
            }
        }

        Ok(())
    }
}