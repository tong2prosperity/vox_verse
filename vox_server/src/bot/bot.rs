use super::*;
use crate::audio_processor::biz_processor::{AsrProcessor, AudioBizProcessor, VadProcessor};
use crate::config::AppConfig;
use crate::server::rtc::rtc_client::RTCClient;
use crate::server::rtc::traits::WebRTCHandler;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub struct Bot {
    rtc: RTCClient,
    cfg: AppConfig,
    audio_processor: Option<AudioBizProcessor>,
    audio_tx: Option<mpsc::Sender<Vec<i16>>>,
    processor_handle: Option<JoinHandle<()>>,
}

impl Bot {
    pub async fn new(cfg: AppConfig) -> Result<Self, Error> {
        Ok(Self {
            rtc: RTCClient::new().await?,
            cfg,
            audio_processor: None,
            audio_tx: None,
            processor_handle: None,
        })
    }

    pub async fn setup_audio_processor(&mut self) {
        let (audio_tx, audio_rx) = mpsc::channel(100);
        let mut processor = AudioBizProcessor::new(audio_rx);

        // 添加音频处理能力
        processor.add_capability(Box::new(VadProcessor {}));
        processor.add_capability(Box::new(AsrProcessor {}));

        // 启动处理器
        let processor_handle = tokio::spawn(async move {
            processor.start().await;
        });

        self.audio_tx = Some(audio_tx);
        self.processor_handle = Some(processor_handle);
    }
}

impl WebRTCHandler for Bot {
    async fn generate_answer(&mut self, offer_sdp: String) -> String {
        let audio_tx = self.audio_tx.take().unwrap();
        self.rtc.handle_offer(offer_sdp, audio_tx).await.unwrap()
    }
}
