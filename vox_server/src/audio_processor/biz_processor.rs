use super::*;
use tokio::sync::mpsc;

// 音频处理能力的trait
pub trait AudioCapability: Send + Sync + 'static {
    fn process(&mut self, pcm_data: &[i16]) -> Result<()>;
}

// VAD能力
pub struct VadProcessor {
    // VAD相关配置
}

impl AudioCapability for VadProcessor {
    fn process(&mut self, pcm_data: &[i16]) -> Result<()> {
        // VAD处理逻辑
        Ok(())
    }
}

// ASR能力 
pub struct AsrProcessor {
    // ASR相关配置
}

impl AudioCapability for AsrProcessor {
    fn process(&mut self, pcm_data: &[i16]) -> Result<()> {
        // ASR处理逻辑
        Ok(())
    }
}

pub struct AudioBizProcessor {
    capabilities: Vec<Box<dyn AudioCapability>>,
    audio_rx: mpsc::Receiver<Vec<i16>>,
}

impl AudioBizProcessor {
    pub fn new(audio_rx: mpsc::Receiver<Vec<i16>>) -> Self {
        Self {
            capabilities: Vec::new(),
            audio_rx,
        }
    }

    pub fn add_capability(&mut self, capability: Box<dyn AudioCapability>) {
        self.capabilities.push(capability);
    }

    pub async fn start(&mut self) {
        while let Some(pcm_data) = self.audio_rx.recv().await {
            for capability in &mut self.capabilities {
                if let Err(e) = capability.process(&pcm_data) {
                    error!("处理音频数据失败: {}", e);
                }
            }
        }
    }
}