use std::time::{Duration, Instant};

/// Presentation Timestamp (PTS) accumulator for high-FPS video recording.
/// Correctly adjusts timestamps across pause/resume cycles to eliminate black pauses or sync drift.
#[derive(Debug)]
pub struct PtsAccumulator {
    start_time: Option<Instant>,
    total_pause_duration: Duration,
    pause_start: Option<Instant>,
    fps: u32,
}

impl PtsAccumulator {
    pub fn new(fps: u32) -> Self {
        Self {
            start_time: None,
            total_pause_duration: Duration::ZERO,
            pause_start: None,
            fps: fps.clamp(15, 120),
        }
    }

    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
        self.total_pause_duration = Duration::ZERO;
        self.pause_start = None;
    }

    pub fn pause(&mut self) {
        if self.pause_start.is_none() && self.start_time.is_some() {
            self.pause_start = Some(Instant::now());
        }
    }

    pub fn resume(&mut self) {
        if let Some(paused_at) = self.pause_start.take() {
            self.total_pause_duration += paused_at.elapsed();
        }
    }

    pub fn is_paused(&self) -> bool {
        self.pause_start.is_some()
    }

    /// Calculate the monotonic active recorded duration excluding pause intervals.
    pub fn recorded_duration(&self) -> Duration {
        let start = match self.start_time {
            Some(s) => s,
            None => return Duration::ZERO,
        };

        let current_elapsed = match self.pause_start {
            Some(paused_at) => paused_at.duration_since(start),
            None => start.elapsed(),
        };

        current_elapsed.saturating_sub(self.total_pause_duration)
    }

    /// Calculate continuous video Presentation Timestamp (PTS) for current frame.
    pub fn current_pts(&self) -> Duration {
        self.recorded_duration()
    }

    /// Calculate the target discrete frame index based on configured FPS.
    pub fn current_frame_index(&self) -> u64 {
        let secs = self.recorded_duration().as_secs_f64();
        (secs * (self.fps as f64)).floor() as u64
    }
}
