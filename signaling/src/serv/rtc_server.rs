use axum::extract::ws::{Message, WebSocket};



pub struct RtcServer {
    ws: WebSocket,
    pub server_id: String,
    managed_rooms: Vec<String>,
}

impl RtcServer {
    pub fn new(server_id: String, ws: WebSocket) -> Self {
        Self {
            server_id,
            ws,
            managed_rooms: Vec::new(),
        }
    }


    pub async fn process(&mut self) {
        while let Some(Ok(Message::Text(text))) = self.ws.recv().await {
            println!("{}", text);
        }
    }

    pub async fn send(&mut self, msg: String) {
        self.ws.send(Message::Text(msg)).await.unwrap();
    }
}
