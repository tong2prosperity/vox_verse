use crate::server::rtc::rtc_client::RTCClient;
use crate::config::AppConfig;

pub struct Bot {
    rtc : RTCClient,
    cfg : AppConfig,
}