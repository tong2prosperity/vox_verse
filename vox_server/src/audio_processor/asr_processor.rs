use crate::server::data;
use anyhow::Result;
use llm_audio_toolkit::asr::{
    tencent::{structs::*, *},
    EchoMageConcurr,
};
use tokio::sync::mpsc;

pub struct AsrProcessor {
    asr_service: TencentASR,
    data_tx: mpsc::Sender<structs::AudioData>,
    result_rx: mpsc::Receiver<RecognitionResult>,
}
