use std::sync::Arc;

use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::{self, MediaEngine};
use webrtc::api::{APIBuilder, API};
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::track::track_remote::TrackRemote;
use webrtc::Error;

pub struct RTCClient {
    peer_connection: Arc<RTCPeerConnection>,
    api: API,
}

impl RTCClient {
    pub async fn new() -> Result<Self, Error> {

        let mut registry = Registry::new();
        
        
        let mut media_engine = MediaEngine::default();
        registry = register_default_interceptors(registry, &mut media_engine)?;
        let api = APIBuilder::new().with_media_engine(media_engine).with_interceptor_registry(registry).build();
        let config = RTCConfiguration::default();
        let peer_connection = Arc::new(api.new_peer_connection(config).await?);

        Ok(Self { peer_connection, api })
    }

    // 处理远程音频轨道
    async fn handle_track(&self, track: Arc<TrackRemote>) {
        // ... 省略其他代码 ...
        
        // 处理音频数据
        let mut buff = vec![0u8; 1920]; // 假设采样率48000, 20ms的PCM数据
        
        loop {
            match track.read(&mut buff).await {
                Ok((n, _)) => {
                    if n.payload.len() > 0 {
                        // 在这里处理PCM音频数据
                        // buff[..n] 包含了原始PCM数据
                        println!("收到 {} 字节的音频数据", n);
                    }
                }
                Err(err) => {
                    println!("读取音频数据出错: {}", err);
                    break;
                }
            }
        }
    }

    pub async fn handle_offer(&mut self, offer_sdp: String) -> Result<String, Error> {
        // 设置远程描述(Offer)
        let offer = webrtc::peer_connection::sdp::session_description::RTCSessionDescription::offer(offer_sdp)?;
        self.peer_connection.set_remote_description(offer).await?;

        // 监听音频轨道
        let pc = Arc::new(self.peer_connection.clone());
        self.peer_connection.on_track(Box::new(move |track, _, _| {
            let pc2 = Arc::clone(&pc);
            Box::pin(async move {
                if track.kind() == webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecType::Audio {
                    pc2.handle_track(track).await;
                }
            })
        }));

        // 创建Answer
        let answer = self.peer_connection.create_answer(None).await?;
        self.peer_connection.set_local_description(answer.clone()).await?;

        Ok(answer.sdp)
    }
}



