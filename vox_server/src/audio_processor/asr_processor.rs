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
<<<<<<< HEAD
=======

// impl AsrProcessor {
//     pub async fn new() -> Result<Self> {

//         let mut asr_service = TencentASR::new(config);
//         asr_service.connect().await?;

//         let (data_tx, result_rx) = asr_service.start().await?;
//         Ok(Self {
//             asr_service,
//             data_tx,
//             result_rx,
//         })
//     }

//     pub async fn process(&mut self, pcm_data: &[i16]) -> Result<()> {
//         Ok(())
//     }
// }
>>>>>>> 8980ccf (dange)
