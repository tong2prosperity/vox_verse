use async_trait::async_trait;


pub trait WebRTCHandler {
    async fn generate_answer(&mut self, offer_sdp: String) -> String;
    async fn handle_candidate(&mut self, candidate: String);
    // 其他 WebRTC 相关方法
}