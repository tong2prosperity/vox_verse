use crate::audio_processor::AudioSink;

use super::*;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

pub struct FileSink {
    base_path: PathBuf,
    counter: usize,
    current_file: Option<File>,
}

impl FileSink {
    pub fn new(base_path: PathBuf) -> Self {
        //  如果没有目录，就创建
        if !base_path.exists() {
            std::fs::create_dir_all(&base_path).unwrap();
        }
        Self {
            base_path,
            counter: 0,
            current_file: None,
        }
    }

    fn create_new_file(&mut self) -> std::io::Result<()> {
        let file_name = format!("audio_{}.pcm", self.counter);
        let file_path = self.base_path.join(file_name);
        self.current_file = Some(File::create(file_path)?);
        self.counter += 1;
        Ok(())
    }
}

impl AudioSink for FileSink {
    fn write(&mut self, audio_data: &[i16]) -> std::io::Result<()> {
        if self.current_file.is_none() {
            self.create_new_file()?;
        }

        if let Some(file) = &mut self.current_file {
            // 将i16数据转换为字节
            let bytes: Vec<u8> = audio_data
                .iter()
                .flat_map(|&sample| sample.to_le_bytes().to_vec())
                .collect();
            file.write_all(&bytes)?;
        }
        Ok(())
    }

    fn finish(&mut self) -> std::io::Result<()> {
        if let Some(mut file) = self.current_file.take() {
            file.flush()?;
        }
        Ok(())
    }
}
