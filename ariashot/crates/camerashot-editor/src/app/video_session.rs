//! Pure-Rust video editing session — no Slint dependency, fully testable.
//!
//! Wraps a `RecordingSession` with a `VideoTimeline` for non-destructive edits.

use crate::compositor::TimelineCompositor;
use crate::timeline_ctrl::{
    CensorStyle, VideoCensorSegment, VideoCutSegment, VideoTimeline, VideoZoomSegment,
};
use camerashot_core::geometry::{Point, Rect};
use camerashot_media::{MediaError, RecordedVideoFrame, RecordingSession};

/// Holds the state for a video editing session.
pub struct VideoSession {
    /// The raw recorded frames.
    pub session: RecordingSession,
    /// Non-destructive edits.
    pub timeline: VideoTimeline,
    /// Current playback position in seconds.
    pub current_time: f64,
    /// Click focus point in normalized [0,1] coordinates (for zoom/censor placement).
    pub focus: Point,
    /// Recording FPS.
    pub fps: u32,
}

impl VideoSession {
    /// Create a `VideoSession` from a completed recording.
    pub fn from_recording(session: RecordingSession, fps: u32) -> Self {
        let dur = session.duration.as_secs_f64();
        Self {
            timeline: VideoTimeline::new(dur),
            session,
            current_time: 0.0,
            focus: Point::new(0.5, 0.5),
            fps,
        }
    }

    /// Total duration of the recording in seconds.
    pub fn duration(&self) -> f64 {
        self.session.duration.as_secs_f64()
    }

    /// Find the index of the frame whose PTS is the largest value ≤ `t`.
    /// Returns `None` if no frames exist or `t < 0`.
    pub fn frame_index_at(&self, t: f64) -> Option<usize> {
        if self.session.frames.is_empty() || t < 0.0 {
            return None;
        }
        // Binary search for the last frame with pts <= t
        let frames = &self.session.frames;
        let mut lo = 0usize;
        let mut hi = frames.len();
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if frames[mid].pts.as_secs_f64() <= t {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        if lo == 0 {
            None
        } else {
            Some(lo - 1)
        }
    }

    /// Render the composited frame at time `t`.
    /// Returns `None` if no frame matches.
    pub fn render_at(&self, t: f64) -> Option<RecordedVideoFrame> {
        let idx = self.frame_index_at(t)?;
        let frame = &self.session.frames[idx];
        // Composite through the timeline (cuts, zooms, censors)
        let composited =
            TimelineCompositor::composite_frames(std::slice::from_ref(frame), &self.timeline);
        composited.into_iter().next()
    }

    /// Compute the next playback time after `t` stepping by `dt`,
    /// skipping over cut regions. Returns `None` if past the end.
    pub fn next_play_time(&self, t: f64, dt: f64) -> Option<f64> {
        let next = t + dt;
        let dur = self.duration();
        if next >= dur {
            return None;
        }
        // If next falls in a cut, jump to the end of that cut
        let kept = self.timeline.kept_ranges();
        for &(start, end) in &kept {
            if next >= start && next < end {
                return Some(next);
            }
        }
        // We're in a cut region — find the next kept range
        for &(start, _) in &kept {
            if start > next {
                return if start < dur { Some(start) } else { None };
            }
        }
        None
    }

    /// Add a 1-second cut starting at `t`.
    pub fn add_cut(&mut self, t: f64) {
        let end = (t + 1.0).min(self.duration());
        self.timeline.cuts.push(VideoCutSegment::new(t, end));
    }

    /// Add a 2x zoom at the current focus, lasting 2 seconds from `t`.
    pub fn add_zoom(&mut self, t: f64) {
        let end = (t + 2.0).min(self.duration());
        self.timeline
            .zooms
            .push(VideoZoomSegment::new(t, end, 2.0, self.focus));
    }

    /// Add a pixelate censor box (20%×12% of frame) centered at `focus`, lasting 2 seconds.
    pub fn add_censor(&mut self, t: f64) {
        let end = (t + 2.0).min(self.duration());
        // Censor rect in normalized coordinates
        let cx = self.focus.x;
        let cy = self.focus.y;
        let w = 0.20;
        let h = 0.12;
        let rect = Rect::new((cx - w / 2.0).max(0.0), (cy - h / 2.0).max(0.0), w, h);
        self.timeline
            .censors
            .push(VideoCensorSegment::new(t, end, rect, CensorStyle::Pixelate));
    }

    /// Get the counts of each edit type for UI display.
    pub fn counts(&self) -> (usize, usize, usize) {
        (
            self.timeline.cuts.len(),
            self.timeline.zooms.len(),
            self.timeline.censors.len(),
        )
    }

    /// Export all composited frames as an animated GIF.
    pub fn export_gif(&self) -> Result<Vec<u8>, MediaError> {
        let composited = TimelineCompositor::composite_session(&self.session, &self.timeline);
        camerashot_media::GifExporter::export_gif(&composited, self.fps)
    }
}
