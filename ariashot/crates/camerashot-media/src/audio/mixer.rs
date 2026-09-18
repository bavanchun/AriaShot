/// Audio stream configuration (standard 48kHz stereo float PCM).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioConfig {
    pub sample_rate: u32,
    pub channels: u16,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48_000,
            channels: 2,
        }
    }
}

/// A chunk of interleaved stereo float PCM samples (L, R, L, R...).
#[derive(Debug, Clone, Default)]
pub struct AudioBuffer {
    pub samples: Vec<f32>,
}

impl AudioBuffer {
    pub fn new() -> Self {
        Self {
            samples: Vec::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            samples: Vec::with_capacity(capacity),
        }
    }
}

/// Dual-source audio mixer that combines system loopback audio and microphone audio
/// with independent gain control and soft-limiting.
pub struct AudioMixer {
    pub config: AudioConfig,
    pub system_volume: f32,
    pub mic_volume: f32,
    pub mute_mic: bool,
    pub mute_system: bool,
}

impl Default for AudioMixer {
    fn default() -> Self {
        Self::new(AudioConfig::default())
    }
}

impl AudioMixer {
    pub fn new(config: AudioConfig) -> Self {
        Self {
            config,
            system_volume: 1.0,
            mic_volume: 1.0,
            mute_mic: false,
            mute_system: false,
        }
    }

    /// Mix system audio and microphone buffers together into output buffer.
    /// Applies soft clipping ($\tanh(x)$) to eliminate harsh digital clipping if sum exceeds $\pm 1.0$.
    pub fn mix(&self, system_audio: &[f32], mic_audio: &[f32], out: &mut Vec<f32>) {
        let max_len = system_audio.len().max(mic_audio.len());
        out.reserve(max_len);

        let sys_gain = if self.mute_system {
            0.0
        } else {
            self.system_volume
        };
        let mic_gain = if self.mute_mic { 0.0 } else { self.mic_volume };

        for i in 0..max_len {
            let sys_sample = system_audio.get(i).copied().unwrap_or(0.0) * sys_gain;
            let mic_sample = mic_audio.get(i).copied().unwrap_or(0.0) * mic_gain;
            let mixed = sys_sample + mic_sample;

            // Soft-clipping function
            let limited = Self::soft_clip(mixed);
            out.push(limited);
        }
    }

    #[inline(always)]
    fn soft_clip(x: f32) -> f32 {
        if x > 1.0 {
            1.0 - (-x + 1.0).exp() * 0.5
        } else if x < -1.0 {
            -1.0 + (x + 1.0).exp() * 0.5
        } else {
            // Cubic saturation in [-1.0, 1.0]
            x - (x * x * x) / 6.0
        }
    }
}
