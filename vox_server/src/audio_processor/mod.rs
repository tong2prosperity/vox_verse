pub mod asr_processor;
pub mod biz_processor;

use crate::{debug, error, info, warn};
use anyhow::Result;

pub trait AudioSink: Send {
    fn write(&mut self, audio_data: &[i16]) -> std::io::Result<()>;
    fn finish(&mut self) -> std::io::Result<()>;
}
