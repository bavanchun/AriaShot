pub mod audio;
pub mod gif;
pub mod recorder;
pub mod video;

pub use audio::{AudioBuffer, AudioConfig, AudioMixer, CoreAudioBackend, PipeWireAudioBackend};
pub use gif::GifExporter;
pub use recorder::{RecordingSession, RecordingState, ScreenRecorder};
pub use video::{MediaError, PtsAccumulator, RecordedVideoFrame, VideoEncoder, VideoEncoderConfig};
