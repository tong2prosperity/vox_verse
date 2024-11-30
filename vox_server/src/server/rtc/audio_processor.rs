use super::*;
use anyhow::Result;
use audiopus::{coder::Decoder, SampleRate, Channels};

pub struct AudioProcessor {
    decoder: Decoder,
    pub buff: Vec<u8>,
}

impl AudioProcessor {
    pub fn new() -> Result<Self> {
        let decoder = Decoder::new(
            SampleRate::Hz48000,
            Channels::Mono
        )?;
        
        Ok(Self { 
            decoder,
            buff: vec![] 
        })
    }

    pub async fn process(&mut self, data: &[u8]) -> Result<Vec<i16>> {
        // 解码 Opus 数据到 PCM
        let mut pcm_data = vec![0i16; 960]; // 20ms at 48kHz
        let samples_per_channel = self.decoder.decode(
            Some(data),
            &mut pcm_data,
            false
        )?;

        // 截取实际解码的数据长度
        pcm_data.truncate(samples_per_channel);
        
        Ok(pcm_data)
    }
}