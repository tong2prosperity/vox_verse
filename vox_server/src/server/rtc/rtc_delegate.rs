use super::{rtc_client::RTCClient, *};
use crate::{
    audio_processor::biz_processor::{AsrProcessor, AudioBizProcessor, VadProcessor},
    server::data,
};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::mpsc;

/// RTCDelegate 负责协调 WebRTC 轨道与音频处理模块之间的数据流
pub struct RTCDelegate {
    // 从远程轨道接收的音频数据发送到处理模块
    //remote_audio_rx: mpsc::Receiver<Vec<i16>>,
    // 从处理模块接收要发送到本地轨道的音频数据
    local_audio_tx: mpsc::Sender<data::AudioData>,
}

impl RTCDelegate {
    /// 创建新的 RTCDelegate 实例
    pub fn new() -> (
        Self,
        //        mpsc::Sender<Vec<i16>>,
        mpsc::Receiver<data::AudioData>,
    ) {
        // 创建用于远程音频数据的通道
        //      let (remote_audio_tx, remote_audio_rx) = mpsc::channel::<Vec<i16>>(100);

        // 创建用于本地音频数据的通道
        let (local_audio_tx, local_audio_rx) = mpsc::channel::<data::AudioData>(100);

        (
            Self {
                //            remote_audio_rx,
                local_audio_tx,
            },
            //      remote_audio_tx,
            local_audio_rx,
        )
    }

    /// 获取本地音频发送器的克隆，用于传递给音频生成模块
    pub fn get_local_audio_tx(&self) -> mpsc::Sender<data::AudioData> {
        self.local_audio_tx.clone()
    }

    /// 发送音频数据到本地轨道
    pub async fn send_audio_to_local_track(&self, audio_data: data::AudioData) -> Result<()> {
        self.local_audio_tx.send(audio_data).await?;
        Ok(())
    }
}
