//! Screen recording capture thread — owns `RecordingConfig`, downscale, and `RecordingHandle`.

use camerashot_media::{
    AudioConfig, RecordingSession, ScreenRecorder, VideoEncoderConfig,
};
use camerashot_platform::traits::CaptureBackend;
use image::imageops::FilterType;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Tuneable recording parameters — single source of truth.
#[derive(Debug, Clone, Copy)]
pub struct RecordingConfig {
    /// Target frames per second.
    pub fps: u32,
    /// Maximum output width in pixels (height scales to preserve aspect ratio).
    pub max_width: u32,
    /// Maximum recording duration in seconds; recording auto-stops at this limit.
    pub max_secs: u32,
}

impl Default for RecordingConfig {
    fn default() -> Self {
        Self {
            fps: 10,
            max_width: 960,
            max_secs: 30,
        }
    }
}

/// Downscale RGBA data if `width > max_width`, preserving aspect ratio.
///
/// Returns `(rgba_data, new_width, new_height)`. If no downscale needed, returns input as-is.
pub fn downscale_rgba(
    rgba: Vec<u8>,
    width: u32,
    height: u32,
    max_width: u32,
) -> (Vec<u8>, u32, u32) {
    if width <= max_width {
        return (rgba, width, height);
    }

    let new_w = max_width;
    let new_h = ((height as f64) * (max_width as f64) / (width as f64)).round() as u32;
    // Ensure even dimensions (nice for GIF)
    let new_h = if new_h % 2 == 1 { new_h + 1 } else { new_h };

    let img = image::RgbaImage::from_raw(width, height, rgba)
        .expect("downscale_rgba: invalid buffer size");
    let resized = image::imageops::resize(&img, new_w, new_h, FilterType::Triangle);
    (resized.into_raw(), new_w, new_h)
}

/// Handle to a running recording thread. Provides stop, elapsed, and join.
pub struct RecordingHandle {
    stop_flag: Arc<AtomicBool>,
    started: Instant,
    join: Option<JoinHandle<Result<RecordingSession, String>>>,
}

impl RecordingHandle {
    /// Start a recording on the primary display.
    ///
    /// `backend` is used inside the capture thread; `display_id` selects which display.
    pub fn start(
        backend: Box<dyn CaptureBackend>,
        display_id: u32,
        config: RecordingConfig,
    ) -> Self {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let started = Instant::now();
        let flag = stop_flag.clone();

        let join = std::thread::spawn(move || {
            let video_config = VideoEncoderConfig {
                width: config.max_width,
                height: 540, // placeholder; actual size comes from capture
                fps: config.fps,
                bitrate_kbps: 4000,
                hardware_accelerated: false,
            };
            let mut recorder = ScreenRecorder::new(video_config, AudioConfig::default());
            recorder.start().map_err(|e| format!("Recorder start: {e}"))?;

            let frame_interval = Duration::from_secs_f64(1.0 / config.fps as f64);
            let max_duration = Duration::from_secs(config.max_secs as u64);

            loop {
                if flag.load(Ordering::Relaxed) {
                    break;
                }
                if started.elapsed() >= max_duration {
                    tracing::info!("Recording auto-stopped at {} s", config.max_secs);
                    break;
                }

                let frame_start = Instant::now();

                // Capture
                match backend.capture_display(display_id) {
                    Ok(fb) => {
                        let rgba = fb.to_rgba8();
                        let (data, w, h) =
                            downscale_rgba(rgba, fb.width as u32, fb.height as u32, config.max_width);
                        if let Err(e) = recorder.push_video_frame(data, w, h) {
                            tracing::warn!("Frame push error: {e}");
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Capture error: {e}");
                    }
                }

                // Pace to target FPS
                let elapsed = frame_start.elapsed();
                if elapsed < frame_interval {
                    std::thread::sleep(frame_interval - elapsed);
                }
            }

            recorder
                .stop()
                .map_err(|e| format!("Recorder stop: {e}"))
        });

        Self {
            stop_flag,
            started,
            join: Some(join),
        }
    }

    /// Elapsed time since recording started.
    pub fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }

    /// Signal the recording thread to stop.
    pub fn signal_stop(&self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }

    /// Check if the recording thread has finished.
    pub fn is_finished(&self) -> bool {
        self.join
            .as_ref()
            .map_or(true, |j| j.is_finished())
    }

    /// Stop recording and join the capture thread, returning the `RecordingSession`.
    pub fn stop_and_join(mut self) -> Result<RecordingSession, String> {
        self.signal_stop();
        self.join
            .take()
            .expect("already joined")
            .join()
            .map_err(|_| "Recording thread panicked".to_string())?
    }
}
