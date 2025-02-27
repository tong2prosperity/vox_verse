pub mod file_sink;
pub mod vad_asr;

use tokio::sync::Notify;

use crate::utils::config::UserConfig;

pub enum PipelineState {
    Idle,
    Asring,
    LLMing,
    TTSing,
}

pub struct Pipeline {
    pub user_conf: UserConfig,
    pub round: u32,
    pub state: PipelineState,
    pub break_notify: Notify,
}

impl Pipeline {
    pub fn new(user_conf: UserConfig, notify: Notify) -> Self {
        Self {
            user_conf,
            round: 0,
            state: PipelineState::Idle,
            break_notify: notify,
        }
    }

    pub async fn run(&mut self) {
        loop {
            // Start in idle state
            self.state = PipelineState::Idle;

            // Start ASR processing
            self.state = PipelineState::Asring;
            let asr_result = self.process_asr().await;

            // Check if we were interrupted during ASR
            if self.check_interrupted() {
                continue;
            }

            // Start LLM processing
            self.state = PipelineState::LLMing;
            let llm_result = self.process_llm(asr_result).await;

            // Check if we were interrupted during LLM
            if self.check_interrupted() {
                continue;
            }

            // Start TTS processing
            self.state = PipelineState::TTSing;
            self.process_tts(llm_result).await;

            // Check if we were interrupted during TTS
            if self.check_interrupted() {
                continue;
            }

            // Increment round counter
            self.round += 1;
        }
    }

    async fn process_asr(&self) -> String {
        // Implementation for ASR processing with VAD
        // Returns the transcribed text
        // TODO: Implement
        String::new()
    }

    async fn process_llm(&self, input: String) -> String {
        // Implementation for LLM processing
        // Returns the generated response
        // TODO: Implement
        String::new()
    }

    async fn process_tts(&self, input: String) {
        // Implementation for TTS processing
        // Outputs the audio
        // TODO: Implement
    }

    fn check_interrupted(&self) -> bool {
        // Check if the break_notify has been triggered
        // TODO: Implement
        false
    }
}
