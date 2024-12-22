use super::*;
use crate::audio_processor::biz_processor::{AsrProcessor, AudioBizProcessor, VadProcessor};
use crate::config::AppConfig;
use crate::error;
use crate::msg_center::signaling_msgs::SignalingMessage;
use crate::server::rtc::rtc_client::RTCClient;
use crate::server::rtc::traits::WebRTCHandler;
use crate::server::signal_cli::SERVER_ID;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub struct Bot {
    pub bot_id: String,
    rtc: RTCClient,
    cfg: AppConfig,
    audio_processor: Option<AudioBizProcessor>,
    audio_tx: Option<mpsc::Sender<Vec<i16>>>,
    message_rx: mpsc::Receiver<SignalingMessage>,
    ws_tx: mpsc::Sender<SignalingMessage>,

    processor_handle: Option<JoinHandle<()>>,
    client_id: String,
}

impl Bot {
    pub async fn new(
        cfg: AppConfig,
        client_id: String,
        ws_tx: mpsc::Sender<SignalingMessage>,
        message_rx: mpsc::Receiver<SignalingMessage>,
    ) -> Result<Self> {
        let bot_id = format!("bot_{}", xid::new().to_string());
        let rtc = RTCClient::new(client_id.clone(), bot_id.clone(), ws_tx.clone()).await?;
        Ok(Self {
            bot_id,
            rtc,
            cfg,
            audio_processor: None,
            audio_tx: None,
            message_rx,
            ws_tx: ws_tx.clone(),

            processor_handle: None,
            client_id,
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

    

    pub async fn handle_message(mut self) {

        // 通过ws_tx发送BotConnected消息
        //self.ws_tx.send(SignalingMessage::ClientConnected { client_id: self.client_id.clone(), server_id: SERVER_ID.clone() }).await.unwrap();

        
        tokio::select! {
            control_msg = self.message_rx.recv() => {
                info!("Bot received message, {:?}", control_msg);
                if let Some(msg) = control_msg {
                    match msg {

                        SignalingMessage::Offer {from, to, sdp } => {
                            match self.rtc.handle_offer(sdp, self.audio_tx.unwrap().clone()).await {
                                Ok(answer_sdp) => {
                                    self.ws_tx.send(SignalingMessage::Answer {from:to, to:from, sdp: answer_sdp}).await.unwrap();
                                }
                                Err(e) => {
                                    error!("Failed to handle offer: {:?}", e);
                                }
                            }
                        }
                        // SignalingMessage::Answer { room_id, from, to, sdp } => todo!(),
                        SignalingMessage::IceCandidate {from, to, candidate } => {
                            self.rtc.add_ice_candidate(candidate).await.unwrap();
                        }
                        _ => {
                            error!("Bot received unknown message: {:?}", msg);
                        }
                    }
                }
            }

        }
    }
}

impl WebRTCHandler for Bot {
    async fn generate_answer(&mut self, offer_sdp: String) -> String {
        let audio_tx = self.audio_tx.take().unwrap();
        self.rtc.handle_offer(offer_sdp, audio_tx).await.unwrap()
    }

    async fn handle_candidate(&mut self, candidate: String) {
        if let Err(e) = self.rtc.add_ice_candidate(candidate).await {
            error!("Failed to add ICE candidate: {:?}", e);
        }
    }
}
