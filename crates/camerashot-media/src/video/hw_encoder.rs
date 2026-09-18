use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MediaError {
    #[error("Video encoding error: {0}")]
    EncodingFailed(String),

    #[error("Audio device or pipeline error: {0}")]
    AudioFailed(String),

    #[error("I/O error during export: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoEncoderConfig {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub bitrate_kbps: u32,
    pub hardware_accelerated: bool,
}

impl Default for VideoEncoderConfig {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            fps: 60,
            bitrate_kbps: 6000,
            hardware_accelerated: true,
        }
    }
}

/// In-memory frame packet containing video pixel payload and continuous PTS.
#[derive(Debug, Clone)]
pub struct RecordedVideoFrame {
    pub pts: Duration,
    pub width: u32,
    pub height: u32,
    pub rgba_data: Vec<u8>,
}

pub struct VideoEncoder {
    pub config: VideoEncoderConfig,
    frames: Vec<RecordedVideoFrame>,
}

impl VideoEncoder {
    pub fn new(config: VideoEncoderConfig) -> Self {
        Self {
            config,
            frames: Vec::new(),
        }
    }

    pub fn push_frame(&mut self, frame: RecordedVideoFrame) {
        self.frames.push(frame);
    }

    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    pub fn frames(&self) -> &[RecordedVideoFrame] {
        &self.frames
    }

    pub fn frames_mut(&mut self) -> &mut [RecordedVideoFrame] {
        &mut self.frames
    }

    pub fn take_frames(&mut self) -> Vec<RecordedVideoFrame> {
        std::mem::take(&mut self.frames)
    }

    pub fn clear(&mut self) {
        self.frames.clear();
    }
}
