use std::time::Duration;

#[derive(Debug, Clone)]
pub struct AudioData {
    pub data: Vec<i16>,
    pub duration: Duration,
}
