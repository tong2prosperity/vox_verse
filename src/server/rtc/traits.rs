use async_trait::async_trait;


trait WebRTCHandler {
    async fn generate_answer(&self, offer_sdp: &str) -> String;
    // 其他 WebRTC 相关方法
}

struct MyWebRTCHandler;

//
impl WebRTCHandler for MyWebRTCHandler {
    async fn generate_answer(&self, offer_sdp: &str) -> String {
        // 实现 SDP 处理逻辑
        "answer SDP".to_string()
    }
}