use en_decoder::{CodecType, VoxDecoder};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use webrtc::{
    ice_transport::ice_candidate::RTCIceCandidate, interceptor::report::receiver, media::Sample,
    peer_connection::sdp::session_description::RTCSessionDescription,
};

use crate::{
    msg_center::signaling_msgs::SignalingMessage,
    server::{
        data,
        signal_cli::{
            msgs::{ServerEvent, ServerMsg},
            SERVER_ID,
        },
    },
};

use super::{en_decoder::create_encoder, *};

pub struct RTCClient {
    peer_connection: Arc<RTCPeerConnection>,
    api: API,
    track_id: String,
    rtp_sender: Option<Arc<RTCRtpSender>>,
    audio_track: Option<Arc<TrackLocalStaticSample>>,
    remote_audio_tx: Option<mpsc::Sender<Vec<i16>>>,
    local_audio_rx: Option<mpsc::Receiver<data::AudioData>>,
    ws_tx: mpsc::Sender<SignalingMessage>,
    client_id: String,
    bot_id: String,
    cached_candidates: Arc<Mutex<Vec<RTCIceCandidate>>>,
}

impl RTCClient {
    pub async fn new(
        client_id: String,
        bot_id: String,
        ws_tx: mpsc::Sender<SignalingMessage>,
    ) -> Result<Self> {
        let mut registry = Registry::new();
        let mut media_engine = MediaEngine::default();
        media_engine.register_default_codecs()?;
        registry = register_default_interceptors(registry, &mut media_engine)?;
        let api = APIBuilder::new()
            .with_media_engine(media_engine)
            .with_interceptor_registry(registry)
            .build();
        let config = RTCConfiguration::default();
        let peer_connection = Arc::new(api.new_peer_connection(config).await?);

        let mut client = Self {
            peer_connection,
            api,
            track_id: uuid::Uuid::new_v4().to_string(),
            rtp_sender: None,
            remote_audio_tx: None,
            ws_tx,
            client_id,
            bot_id,
            cached_candidates: Arc::new(Mutex::new(Vec::new())),
            audio_track: None,
            local_audio_rx: None,
        };
        Ok(client)
    }

    pub fn set_local_audio_rx(&mut self, local_audio_rx: mpsc::Receiver<data::AudioData>) {
        self.local_audio_rx = Some(local_audio_rx);
    }

    pub async fn handle_offer(&mut self, offer_sdp: String) -> Result<String> {
        // 设置远程描述(Offer)
        let offer = match serde_json::from_str::<RTCSessionDescription>(&offer_sdp) {
            Ok(s) => s,
            Err(err) => panic!("{}", err),
        };

        self.peer_connection.set_remote_description(offer).await?;
        debug!("Bot set remote description ok");

        //let mut gather_complete = self.peer_connection.gathering_complete_promise().await;

        // 创建Answer
        let answer = self.peer_connection.create_answer(None).await?;
        info!("Bot creating answer, {:?}", answer);
        self.peer_connection
            .set_local_description(answer.clone())
            .await?;

        //let _ = gather_complete.recv().await;

        let local_description = self.peer_connection.local_description().await.unwrap();
        info!("Bot setting local description, {:?}", local_description);
        self.ws_tx
            .send(SignalingMessage::Answer {
                from: self.bot_id.clone(),
                to: self.client_id.clone(),
                sdp: serde_json::to_string(&local_description).unwrap(),
            })
            .await?;

        // 在设置远程描述后发送缓存的候选项
        let cached = {
            let mut cache = self.cached_candidates.lock().unwrap();
            std::mem::take(&mut *cache)
        };

        for candidate in cached {
            let msg = SignalingMessage::IceCandidate {
                from: self.bot_id.clone(),
                to: self.client_id.clone(),
                candidate: serde_json::to_string(&candidate.to_json().unwrap()).unwrap(),
            };
            self.ws_tx.send(msg).await?;
        }

        Ok(local_description.sdp)
    }

    pub async fn setup_pc_handlers(&mut self) -> Result<()> {
        let pc: Arc<RTCPeerConnection> = Arc::clone(&self.peer_connection);
        let client_id = self.client_id.clone();
        let bot_id = self.bot_id.clone();
        let ws_tx = self.ws_tx.clone();
        let cached_candidates: Arc<Mutex<Vec<RTCIceCandidate>>> =
            Arc::clone(&self.cached_candidates);

        // ICE Candidate 处理
        // 不做ice trickle
        self.peer_connection.on_ice_candidate(Box::new(move |c| {
            info!("rtc client received ice candidate, {:?}", c);
            let pc2 = Arc::clone(&pc);
            let client_id = client_id.clone();
            let bot_id = bot_id.clone();
            let ws_tx = ws_tx.clone();
            let cached_candidates = Arc::clone(&cached_candidates);

            Box::pin(async move {
                if let Some(candidate) = c {
                    if pc2.remote_description().await.is_none() {
                        // 如果还没有远程描述，缓存候选项
                        cached_candidates.lock().unwrap().push(candidate);
                    } else {
                        // 已有远程描述，直接发送
                        let msg = SignalingMessage::IceCandidate {
                            from: bot_id,
                            to: client_id,
                            candidate: serde_json::to_string(&candidate.to_json().unwrap())
                                .unwrap(),
                        };
                        debug!("rtc client send ice candidate: {:?}", msg);
                        if let Err(e) = ws_tx.send(msg).await {
                            error!("Failed to send ICE candidate: {}", e);
                        }
                    }
                }
            })
        }));
        // 监听音频轨道

        let audio_tx = self.remote_audio_tx.take().unwrap();
        self.peer_connection
            .on_track(Box::new(move |track, _receiver, _transceiver| {
                let audio_tx = audio_tx.clone();
                Box::pin(async move {
                    info!("Bot received track, {:?}", track);
                    if track.kind() == RTPCodecType::Audio {
                        Self::handle_track(track, audio_tx).await;
                    }
                })
            }));

        // 连接状态变化处理
        self.peer_connection
            .on_peer_connection_state_change(Box::new(move |s: RTCPeerConnectionState| {
                let state = s.clone();
                Box::pin(async move {
                    info!("Peer Connection State has changed: {}", state);
                    if state == RTCPeerConnectionState::Failed {
                        error!("Peer Connection has failed");
                    }
                })
            }));

        Ok(())
    }

    pub fn setup_pc_other_handler(&mut self) -> Result<()> {
        self.peer_connection
            .on_data_channel(Box::new(move |channel| {
                debug!("data channel opened");
                Box::pin(async move {
                    // 处理数据通道
                })
            }));

        self.peer_connection
            .on_ice_connection_state_change(Box::new(
                move |connection_state: RTCIceConnectionState| {
                    debug!("Connection State has changed {connection_state}");
                    match connection_state {
                        RTCIceConnectionState::Unspecified => {
                            info!("rtc client ice connection unspecify");
                        }
                        RTCIceConnectionState::New => {
                            info!("rtc client ice connection new");
                        }
                        RTCIceConnectionState::Checking => {
                            info!("rtc client ice connection checking");
                        }
                        RTCIceConnectionState::Connected => {
                            info!("rtc client ice connection connected");
                        }
                        RTCIceConnectionState::Completed => {
                            info!("rtc client ice connection completed");
                        }
                        RTCIceConnectionState::Disconnected => {
                            info!("rtc client ice connection disconnected");
                        }
                        RTCIceConnectionState::Failed => {
                            info!("rtc client ice connection failed");
                        }
                        RTCIceConnectionState::Closed => {
                            info!("rtc client ice connection closed");
                        }
                    }
                    if connection_state == RTCIceConnectionState::Connected {
                        // 通知上层 notify_tx.send()
                    }
                    Box::pin(async {})
                },
            ));

        // Set the handler for Peer connection state
        // This will notify you when the peer has connected/disconnected
        self.peer_connection
            .on_peer_connection_state_change(Box::new(move |s: RTCPeerConnectionState| {
                println!("Peer Connection State has changed: {s}");

                if s == RTCPeerConnectionState::Failed {
                    // Wait until PeerConnection has had no network activity for 30 seconds or another failure. It may be reconnected using an ICE Restart.
                    // Use webrtc.PeerConnectionStateDisconnected if you are interested in detecting faster timeout.
                    // Note that the PeerConnection may come back from PeerConnectionStateDisconnected.
                    warn!("Peer Connection has gone to failed exiting");
                    // 通知上层  notify_tx.send()
                }

                Box::pin(async {})
            }));
        Ok(())
    }

    async fn audio_track_rtcp_handler(sender: Arc<RTCRtpSender>) {
        let mut buff = vec![0u8; 1500]; //  just the rtcp packet
        loop {
            match sender.read(&mut buff).await {
                Ok((n, _)) => {
                    if n.len() > 0 {
                        // 在这里处理PCM音频数据
                        info!("收到 {} 字节的音频数据", &n.len());
                    }
                }
                Err(err) => {
                    error!("读取音频数据出错: {}", err);
                    break;
                }
            }
        }
    }

    async fn audio_send_handler(
        mut receiver: mpsc::Receiver<data::AudioData>,
        audio_track: Arc<TrackLocalStaticSample>,
    ) -> Result<()> {
        let mut encoder = match create_encoder(CodecType::Opus, 48000, 1) {
            Ok(p) => Box::pin(p),
            Err(e) => {
                error!("创建音频处理器失败: {}", e);
                return Err(e);
            }
        };

        loop {
            let audio_data = receiver.recv().await;
            if audio_data.is_none() {
                error!("receive generated audio data error");
                continue;
            }

            let audio_data = audio_data.unwrap();
            let duration = audio_data.duration;
            let audio_data = encoder.encode(&audio_data.data).await?;

            let sample = Sample {
                data: audio_data,
                duration: duration,
                ..Default::default()
            };
            audio_track.write_sample(&sample).await?;
        }
        Ok(())
    }

    pub async fn add_ice_candidate(&self, candidate: String) -> Result<()> {
        self.peer_connection
            .add_ice_candidate(webrtc::ice_transport::ice_candidate::RTCIceCandidateInit {
                candidate,
                ..Default::default()
            })
            .await
            .map_err(|e| anyhow::anyhow!("Failed to add ICE candidate: {:?}", e))
    }

    pub(crate) fn set_remote_audio_tx(&mut self, audio_tx: mpsc::Sender<Vec<i16>>) -> () {
        self.remote_audio_tx = Some(audio_tx);
    }

    // pub(crate) fn set_audio_rx(&mut self, audio_rx: mpsc::Receiver<Vec<i16>>) -> () {
    //     self.audio_rx = Some(audio_rx);
    // }
}

impl RTCClient {
    // 处理远程音频轨道
    pub async fn handle_track(track: Arc<TrackRemote>, audio_tx: mpsc::Sender<Vec<i16>>) {
        debug!("handle_track start");
        let mut decoder = match VoxDecoder::new(CodecType::Opus, 48000, 1) {
            Ok(p) => Box::pin(p),
            Err(e) => {
                error!("创建音频处理器失败: {}", e);
                return;
            }
        };

        let mut buff = vec![0u8; 1920];

        loop {
            match track.read(&mut buff).await {
                Ok((n, _)) => {
                    if n.payload.len() > 0 {
                        match decoder.decode(&n.payload).await {
                            Ok(pcm_data) => {
                                info!("收到 {} 字节的音频数据", &pcm_data.len());
                                if let Err(e) = audio_tx.send(pcm_data).await {
                                    error!("send audio data to bot failed: {}", e);
                                    break;
                                }
                            }
                            Err(e) => error!("decode error: {}", e),
                        }
                    }
                }
                Err(err) => {
                    error!("读取音频数据出错: {}", err);
                    break;
                }
            }
        }
    }

    pub async fn setup_media(&mut self) -> Result<()> {
        let audio_track = Arc::new(TrackLocalStaticSample::new(
            RTCRtpCodecCapability {
                mime_type: MIME_TYPE_OPUS.to_owned(),
                clock_rate: 48000,
                channels: 1,
                ..Default::default()
            },
            "audio".to_owned(),
            "webrtc-rs".to_owned(),
        ));

        let rtp_sender = self.peer_connection.add_track(audio_track.clone()).await?;
        tokio::spawn(Self::audio_track_rtcp_handler(rtp_sender.clone()));
        tokio::spawn(Self::audio_send_handler(
            self.local_audio_rx.take().unwrap(),
            audio_track.clone(),
        ));

        self.rtp_sender = Some(rtp_sender);
        self.audio_track = Some(audio_track);
        Ok(())
    }
}
