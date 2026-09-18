use crate::audio::mixer::{AudioBuffer, AudioConfig};
use crate::video::MediaError;

pub struct CoreAudioBackend;

impl CoreAudioBackend {
    pub fn new(_config: AudioConfig) -> Result<Self, MediaError> {
        Ok(Self)
    }

    pub fn capture_system_chunk(&mut self) -> AudioBuffer {
        AudioBuffer::new()
    }

    pub fn capture_mic_chunk(&mut self) -> AudioBuffer {
        AudioBuffer::new()
    }
}
