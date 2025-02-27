use std::path::PathBuf;
use std::sync::Arc;

use super::*;
use crate::audio_processor::AudioSink;
use crate::server::data::AudioData;
use crate::{debug, info, warn};

use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::Mutex;
use vad_rs::ort_vad::speech_state::SpeechState;
use vad_rs::ort_vad::utils::SampleRate;
use vad_rs::ort_vad::vad_iter::VadIter;
use vad_rs::ort_vad::{silero::Silero, utils::VadParams};

pub struct AudioCache {
    buffer: Vec<Vec<i16>>,
    state: SpeechState,
    max_cache_size: usize,
}

impl AudioCache {
    pub fn new(max_cache_size: usize) -> Self {
        Self {
            buffer: Vec::new(),
            state: SpeechState::Silent,
            max_cache_size,
        }
    }

    pub fn push(&mut self, frame: &[i16]) -> bool {
        if self.buffer.len() >= self.max_cache_size {
            return false;
        }
        self.buffer.push(frame.to_vec());
        true
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

pub struct NopClient {}

impl NopClient {
    pub fn new() -> Self {
        Self {}
    }
}

pub struct VadProcessor {
    audio_frame_rx: UnboundedReceiver<AudioData>,
    vad_iter: VadIter,
    // 用于缓存音频数据的buffer
    audio_buffer: Vec<i16>,
    // 32ms 数据需要的采样点数 (16000 * 0.032 = 512)
    samples_per_frame: usize,
    cli: Arc<Mutex<NopClient>>,
    audio_cache: AudioCache,
    last_state: SpeechState,
    audio_sink: Option<Box<dyn AudioSink>>,
}

impl VadProcessor {
    pub fn new(audio_frame_rx: UnboundedReceiver<AudioData>, cli: Arc<Mutex<NopClient>>) -> Self {
        let silero = Silero::new(SampleRate::SixteenkHz, "").unwrap();
        let params = VadParams::default();
        let vad_iter = VadIter::new(silero, params);
        // let file = FileSink::new(PathBuf::from("./user_audios"));
        // let file_sink = Some(Box::new(file));

        Self {
            audio_frame_rx,
            vad_iter,
            audio_buffer: Vec::new(),
            samples_per_frame: 512, // 16000Hz * 32ms = 512 samples
            cli,
            audio_cache: AudioCache::new(100),
            last_state: SpeechState::Silent,
            audio_sink: None,
        }
    }

    async fn handle_speech_state(
        &mut self,
        speech_state: SpeechState,
        current_frame: Option<&[i16]>,
    ) {
        match speech_state {
            SpeechState::StartSpeaking => {
                if self.last_state == SpeechState::Silent {
                    info!("检测到开始说话");
                    if let Some(frame) = current_frame {
                        self.audio_cache.push(frame);
                        if let Some(sink) = &mut self.audio_sink {
                            if let Err(e) = sink.write(frame) {
                                warn!("写入音频文件失败: {}", e);
                            }
                        }
                    }
                }
            }

            SpeechState::Speaking => {
                if self.last_state == SpeechState::StartSpeaking {
                    info!("确认持续说话，发送缓存的音频");
                    let mut cli = self.cli.lock().await;
                    for frame in self.audio_cache.buffer.iter() {
                        if let Err(e) = cli.send_audio_frame(frame.as_slice()).await {
                            warn!("发送缓存音频帧失败: {}", e);
                        }
                    }
                }

                if let Some(frame) = current_frame {
                    let mut cli = self.cli.lock().await;
                    if let Err(e) = cli.send_audio_frame(frame).await {
                        warn!("发送当前音频帧失败: {}", e);
                    }
                    if let Some(sink) = &mut self.audio_sink {
                        if let Err(e) = sink.write(frame) {
                            warn!("写入音频文件失败: {}", e);
                        }
                    }
                }
            }

            SpeechState::StopSpeaking => {
                info!("检测到说话结束");
                if let Some(frame) = current_frame {
                    let mut cli = self.cli.lock().await;
                    if let Err(e) = cli.send_audio_frame(frame).await {
                        warn!("发送最后音频帧失败: {}", e);
                    }
                    if let Err(e) = cli.commit_audio_frame().await {
                        warn!("提交音频帧失败: {}", e);
                    }
                    if let Some(sink) = &mut self.audio_sink {
                        if let Err(e) = sink.write(frame) {
                            warn!("写入最后音频帧失败: {}", e);
                        }
                        if let Err(e) = sink.finish() {
                            warn!("完成音频文件写入失败: {}", e);
                        }
                    }

                    info!("发送音频帧结束");
                }

                self.audio_cache.clear();
            }

            SpeechState::Silent => {
                self.audio_cache.clear();
            }

            _ => {}
        }

        self.last_state = speech_state;
    }

    async fn process_buffer(&mut self, audio_buffer: &mut Vec<i16>) {
        while audio_buffer.len() >= self.samples_per_frame {
            let frame = &audio_buffer[..self.samples_per_frame];

            match self.vad_iter.process(frame) {
                Ok(speech_state) => {
                    self.handle_speech_state(speech_state, Some(frame)).await;
                }
                Err(e) => {
                    warn!("VAD process error: {}", e);
                }
            }

            // 移除已处理的数据
            audio_buffer.drain(..self.samples_per_frame);
        }
    }

    pub async fn handle(&mut self) {
        // 移除已处理的数据
        info!("VAD processor handle start");
        let mut audio_buffer = Vec::new();

        while let Some(frame) = self.audio_frame_rx.recv().await {
            let audio_data: Vec<i16> = frame
                .data
                .chunks_exact(2)
                .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
                // 将音频数据转换为i16格式并添加到缓冲区
                .collect();

            //self.audio_buffer.extend(audio_data);
            audio_buffer.extend(audio_data);

            self.process_buffer(&mut audio_buffer).await;
        }
        info!("VAD processor handle end");
    }
}
// 处理缓冲区中的数据

// impl super::AudioProcessor for VadProcessor {
//     async fn process_frame(&mut self, frame: AudioFrameRS) {
//         let audio_data: Vec<i16> = frame
//             .data
//             .chunks_exact(2)
//             .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
//             // 将音频数据转换为i16格式并添加到缓冲区
//             .collect();

//         self.audio_buffer.extend(audio_data);

//         self.process_buffer(&mut self.audio_buffer).await;
//     }

//     async fn handle(&mut self) {
//         self.handle().await;
//         // 处理缓冲区中的数据
//     }
// }
