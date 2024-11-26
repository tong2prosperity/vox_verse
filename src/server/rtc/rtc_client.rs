use std::sync::Arc;
use super::traits::WebRTCHandler;

use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::{self, MediaEngine, MIME_TYPE_OPUS};
use webrtc::api::{APIBuilder, API};
use webrtc::ice_transport::ice_connection_state::RTCIceConnectionState;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
use webrtc::rtp_transceiver::rtp_sender::RTCRtpSender;
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;
use webrtc::track::track_remote::TrackRemote;
use webrtc::Error;

use anyhow::Result;

pub struct RTCClient {
    peer_connection: Arc<RTCPeerConnection>,
    api: API,
    track_id: String,
    rtp_sender: Option<Arc<RTCRtpSender>>,
}

impl RTCClient {
    pub async fn new() -> Result<Self, Error> {

        let mut registry = Registry::new();
        
        
        let mut media_engine = MediaEngine::default();
        registry = register_default_interceptors(registry, &mut media_engine)?;
        let api = APIBuilder::new().with_media_engine(media_engine).with_interceptor_registry(registry).build();
        let config = RTCConfiguration::default();
        let peer_connection = Arc::new(api.new_peer_connection(config).await?);

        Ok(Self { peer_connection, api, track_id: uuid::Uuid::new_v4().to_string(), rtp_sender: None })
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


    async fn setup_media(&mut self) -> Result<()> {
        let audio_track = Arc::new(TrackLocalStaticSample::new(
            RTCRtpCodecCapability {
                mime_type: MIME_TYPE_OPUS.to_owned(),
                clock_rate: 24000,
                channels: 1,
                ..Default::default()
            },
            "audio_tts_res_track".to_owned(),
            self.track_id.to_owned(),
        ));
        
        let rtp_sender = self.peer_connection.add_track(audio_track).await?;
        tokio::spawn(Self::tts_audio_rtcp_handler(rtp_sender.clone()));


        self.rtp_sender = Some(rtp_sender);
        
        Ok(())
    }


    fn setup_pc_other_handler(&mut self) -> Result<()> {
        self.peer_connection.on_ice_connection_state_change(Box::new(
            move |connection_state: RTCIceConnectionState| {
                println!("Connection State has changed {connection_state}");
                if connection_state == RTCIceConnectionState::Connected {
                    // 通知上层 notify_tx.send()
                }
                Box::pin(async {})
            },
        ));
    
        // Set the handler for Peer connection state
        // This will notify you when the peer has connected/disconnected
        self.peer_connection.on_peer_connection_state_change(Box::new(move |s: RTCPeerConnectionState| {
            println!("Peer Connection State has changed: {s}");
    
            if s == RTCPeerConnectionState::Failed {
                // Wait until PeerConnection has had no network activity for 30 seconds or another failure. It may be reconnected using an ICE Restart.
                // Use webrtc.PeerConnectionStateDisconnected if you are interested in detecting faster timeout.
                // Note that the PeerConnection may come back from PeerConnectionStateDisconnected.
                println!("Peer Connection has gone to failed exiting");
                // 通知上层  notify_tx.send()
            }
    
            Box::pin(async {})
        }));
        Ok(())
    }



    


    async fn tts_audio_rtcp_handler(sender: Arc<RTCRtpSender>) {
        let mut buff = vec![0u8; 1500]; //  just the rtcp packet
        
        loop {
            match sender.read(&mut buff).await {
                Ok((n, _)) => {
                    if n.len() > 0 {
                        // 在这里处理PCM音频数据
                        println!("收到 {} 字节的音频数据", &n.len());
                    }
                }
                Err(err) => {
                    println!("读取音频数据出错: {}", err);
                    break;
                }
            }
        }
    }
}


impl WebRTCHandler for RTCClient {
    async fn generate_answer(&self, offer_sdp: &str) -> String {
        self.handle_offer(offer_sdp).await.unwrap()
    }
}



