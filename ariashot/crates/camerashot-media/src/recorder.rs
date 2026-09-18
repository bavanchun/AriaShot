use crate::audio::{AudioConfig, AudioMixer};
use crate::gif::GifExporter;
use crate::video::{
    MediaError, PtsAccumulator, RecordedVideoFrame, VideoEncoder, VideoEncoderConfig,
};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordingState {
    Idle,
    Recording,
    Paused,
    Stopped,
}

/// The result of a completed screen recording session.
#[derive(Debug, Clone)]
pub struct RecordingSession {
    pub video_config: VideoEncoderConfig,
    pub audio_config: AudioConfig,
    pub frames: Vec<RecordedVideoFrame>,
    pub mixed_audio: Vec<f32>,
    pub duration: Duration,
}

impl RecordingSession {
    /// Export the session's video frames to an animated GIF.
    pub fn export_gif(&self, fps: u32) -> Result<Vec<u8>, MediaError> {
        GifExporter::export_gif(&self.frames, fps)
    }

    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    pub fn duration(&self) -> Duration {
        self.duration
    }
}

/// High-performance screen recorder that coordinates video frame capture,
/// PTS timestamp tracking, and dual-source audio mixing.
pub struct ScreenRecorder {
    state: RecordingState,
    video_config: VideoEncoderConfig,
    audio_config: AudioConfig,
    pts: PtsAccumulator,
    encoder: VideoEncoder,
    mixer: AudioMixer,
    mixed_audio: Vec<f32>,
    dropped_frames: u64,
}

impl ScreenRecorder {
    pub fn new(video_config: VideoEncoderConfig, audio_config: AudioConfig) -> Self {
        let fps = video_config.fps;
        Self {
            state: RecordingState::Idle,
            video_config: video_config.clone(),
            audio_config,
            pts: PtsAccumulator::new(fps),
            encoder: VideoEncoder::new(video_config),
            mixer: AudioMixer::new(audio_config),
            mixed_audio: Vec::new(),
            dropped_frames: 0,
        }
    }

    pub fn state(&self) -> RecordingState {
        self.state
    }

    pub fn is_recording(&self) -> bool {
        self.state == RecordingState::Recording
    }

    pub fn is_paused(&self) -> bool {
        self.state == RecordingState::Paused
    }

    pub fn elapsed(&self) -> Duration {
        self.pts.recorded_duration()
    }

    pub fn dropped_frames(&self) -> u64 {
        self.dropped_frames
    }

    pub fn mixer(&self) -> &AudioMixer {
        &self.mixer
    }

    pub fn mixer_mut(&mut self) -> &mut AudioMixer {
        &mut self.mixer
    }

    pub fn start(&mut self) -> Result<(), MediaError> {
        if self.state == RecordingState::Recording {
            return Ok(());
        }
        self.encoder.clear();
        self.mixed_audio.clear();
        self.dropped_frames = 0;
        self.pts.start();
        self.state = RecordingState::Recording;
        tracing::info!("Screen recording started at {} FPS", self.video_config.fps);
        Ok(())
    }

    pub fn pause(&mut self) -> Result<(), MediaError> {
        if self.state != RecordingState::Recording {
            return Ok(());
        }
        self.pts.pause();
        self.state = RecordingState::Paused;
        tracing::info!(
            "Screen recording paused at PTS {:?}",
            self.pts.current_pts()
        );
        Ok(())
    }

    pub fn resume(&mut self) -> Result<(), MediaError> {
        if self.state != RecordingState::Paused {
            return Ok(());
        }
        self.pts.resume();
        self.state = RecordingState::Recording;
        tracing::info!(
            "Screen recording resumed at PTS {:?}",
            self.pts.current_pts()
        );
        Ok(())
    }

    /// Push an uncompressed RGBA video frame from the capture source.
    pub fn push_video_frame(
        &mut self,
        rgba_data: Vec<u8>,
        width: u32,
        height: u32,
    ) -> Result<(), MediaError> {
        if self.state != RecordingState::Recording {
            // Drop frame when paused or idle
            return Ok(());
        }

        let expected_size = (width * height * 4) as usize;
        if rgba_data.len() != expected_size {
            self.dropped_frames += 1;
            return Err(MediaError::EncodingFailed(format!(
                "Frame buffer size mismatch: expected {} bytes, got {}",
                expected_size,
                rgba_data.len()
            )));
        }

        let pts = self.pts.current_pts();
        let frame = RecordedVideoFrame {
            pts,
            width,
            height,
            rgba_data,
        };

        self.encoder.push_frame(frame);
        Ok(())
    }

    /// Push audio samples from system loopback and/or microphone.
    pub fn push_audio_chunk(
        &mut self,
        system_audio: Option<&[f32]>,
        mic_audio: Option<&[f32]>,
    ) -> Result<(), MediaError> {
        if self.state != RecordingState::Recording {
            return Ok(());
        }

        let sys = system_audio.unwrap_or(&[]);
        let mic = mic_audio.unwrap_or(&[]);
        self.mixer.mix(sys, mic, &mut self.mixed_audio);
        Ok(())
    }

    /// Stop recording and produce a complete `RecordingSession`.
    pub fn stop(&mut self) -> Result<RecordingSession, MediaError> {
        if self.state == RecordingState::Idle || self.state == RecordingState::Stopped {
            return Err(MediaError::EncodingFailed(
                "Cannot stop recording: recorder is not running".to_string(),
            ));
        }

        let total_duration = self.pts.recorded_duration();
        self.state = RecordingState::Stopped;

        let frames = self.encoder.take_frames();
        let mixed_audio = std::mem::take(&mut self.mixed_audio);

        tracing::info!(
            "Screen recording stopped: {} frames captured, duration {:?}",
            frames.len(),
            total_duration
        );

        Ok(RecordingSession {
            video_config: self.video_config.clone(),
            audio_config: self.audio_config,
            frames,
            mixed_audio,
            duration: total_duration,
        })
    }
}
