use async_trait::async_trait;


trait WebRTCHandler {
    async fn generate_answer(&self, offer_sdp: &str) -> String;
    // 其他 WebRTC 相关方法
}