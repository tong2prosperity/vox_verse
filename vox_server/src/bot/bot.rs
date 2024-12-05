use crate::server::rtc::rtc_client::RTCClient;
use crate::config::AppConfig;
use crate::server::rtc::traits::WebRTCHandler;
use super::*;

pub struct Bot {
    rtc : RTCClient,
    cfg : AppConfig,
}



impl Bot {
    pub async fn new(cfg: AppConfig) -> Result<Self, Error> {
        Ok(Self { rtc: RTCClient::new().await?, cfg })
    }



}


impl WebRTCHandler for Bot {
    async fn generate_answer(&mut self, offer_sdp: String) -> String {
        self.rtc.handle_offer(offer_sdp).await.unwrap()
    }
}