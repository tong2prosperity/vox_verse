use super::*;
use anyhow::Result;
pub struct AudioProcessor {
    pub buff: Vec<u8>,
}



impl AudioProcessor {
    pub fn new() -> Self {
        Self { buff: vec![] }
    }

    pub async fn process(&mut self, track: &TrackRemote) -> Result<()> {
        OK(())
    }
}