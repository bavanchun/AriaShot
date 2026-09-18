use camerashot_core::geometry::{Point, Rect};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Hermite cubic smoothstep interpolation: S(x) = x^2 * (3 - 2x) for x in [0, 1].
#[inline(always)]
pub fn smoothstep(x: f64) -> f64 {
    let c = x.clamp(0.0, 1.0);
    c * c * (3.0 - 2.0 * c)
}

/// A cut region of the recorded source video to remove from playback/export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoCutSegment {
    pub id: Uuid,
    pub start_time: f64,
    pub end_time: f64,
}

impl VideoCutSegment {
    pub fn new(start_time: f64, end_time: f64) -> Self {
        Self {
            id: Uuid::new_v4(),
            start_time,
            end_time,
        }
    }

    pub fn duration(&self) -> f64 {
        (self.end_time - self.start_time).max(0.0)
    }

    pub fn overlaps(&self, start: f64, end: f64) -> bool {
        self.start_time < end && self.end_time > start
    }
}

/// A timeline region where the video zooms into a target focus point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoZoomSegment {
    pub id: Uuid,
    pub start_time: f64,
    pub end_time: f64,
    pub zoom_level: f64,
    pub center: Point,
    pub fade_in: f64,
    pub fade_out: f64,
}

impl VideoZoomSegment {
    pub const DEFAULT_FADE: f64 = 0.35;
    pub const MIN_ZOOM: f64 = 1.1;
    pub const MAX_ZOOM: f64 = 5.0;

    pub fn new(start_time: f64, end_time: f64, zoom_level: f64, center: Point) -> Self {
        Self {
            id: Uuid::new_v4(),
            start_time,
            end_time,
            zoom_level: zoom_level.clamp(Self::MIN_ZOOM, Self::MAX_ZOOM),
            center: Self::clamped_center(center, zoom_level),
            fade_in: Self::DEFAULT_FADE,
            fade_out: Self::DEFAULT_FADE,
        }
    }

    pub fn duration(&self) -> f64 {
        (self.end_time - self.start_time).max(0.0)
    }

    pub fn effective_fade_in(&self) -> f64 {
        let cap = (self.duration() / 2.0 - 0.001).max(0.0);
        self.fade_in.clamp(0.0, cap)
    }

    pub fn effective_fade_out(&self) -> f64 {
        let cap = (self.duration() / 2.0 - 0.001).max(0.0);
        self.fade_out.clamp(0.0, cap)
    }

    /// Interpolated zoom magnification level at time `t` (seconds).
    pub fn zoom_level_at(&self, t: f64) -> f64 {
        if t < self.start_time || t > self.end_time || self.duration() <= 0.0 {
            return 1.0;
        }

        let f_in = self.effective_fade_in();
        let f_out = self.effective_fade_out();
        let into = t - self.start_time;
        let to_end = self.end_time - t;

        if into < f_in && f_in > 0.0 {
            1.0 + (self.zoom_level - 1.0) * smoothstep(into / f_in)
        } else if to_end < f_out && f_out > 0.0 {
            1.0 + (self.zoom_level - 1.0) * smoothstep(to_end / f_out)
        } else {
            self.zoom_level
        }
    }

    /// Clamp normalized center (0..1) so the zoomed window stays inside video frame.
    pub fn clamped_center(c: Point, zoom: f64) -> Point {
        let half = 1.0 / (2.0 * zoom.max(1.0001));
        Point::new(c.x.clamp(half, 1.0 - half), c.y.clamp(half, 1.0 - half))
    }

    /// Calculate crop rectangle in video pixel coordinates for given frame dimensions.
    pub fn crop_rect(&self, zoom: f64, video_width: f64, video_height: f64) -> Rect {
        let crop_w = video_width / zoom;
        let crop_h = video_height / zoom;

        let center = Self::clamped_center(self.center, zoom);
        let cx = center.x * video_width;
        let cy = center.y * video_height;

        let origin_x = (cx - crop_w / 2.0).clamp(0.0, video_width - crop_w);
        let origin_y = (cy - crop_h / 2.0).clamp(0.0, video_height - crop_h);

        Rect::new(origin_x, origin_y, crop_w, crop_h)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CensorStyle {
    Solid,
    Pixelate,
    Blur,
}

/// A timeline region where a rectangular sub-area of the video is obscured.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoCensorSegment {
    pub id: Uuid,
    pub start_time: f64,
    pub end_time: f64,
    pub rect: Rect, // Normalized coordinates (0..1)
    pub style: CensorStyle,
    pub fade_in: f64,
    pub fade_out: f64,
}

impl VideoCensorSegment {
    pub const DEFAULT_FADE: f64 = 0.25;

    pub fn new(start_time: f64, end_time: f64, rect: Rect, style: CensorStyle) -> Self {
        Self {
            id: Uuid::new_v4(),
            start_time,
            end_time,
            rect,
            style,
            fade_in: Self::DEFAULT_FADE,
            fade_out: Self::DEFAULT_FADE,
        }
    }

    pub fn duration(&self) -> f64 {
        (self.end_time - self.start_time).max(0.0)
    }

    pub fn effective_fade_in(&self) -> f64 {
        let cap = (self.duration() / 2.0 - 0.001).max(0.0);
        self.fade_in.clamp(0.0, cap)
    }

    pub fn effective_fade_out(&self) -> f64 {
        let cap = (self.duration() / 2.0 - 0.001).max(0.0);
        self.fade_out.clamp(0.0, cap)
    }

    pub fn opacity_at(&self, t: f64) -> f64 {
        if t < self.start_time || t > self.end_time || self.duration() <= 0.0 {
            return 0.0;
        }

        let f_in = self.effective_fade_in();
        let f_out = self.effective_fade_out();
        let into = t - self.start_time;
        let to_end = self.end_time - t;

        if into < f_in && f_in > 0.0 {
            smoothstep(into / f_in)
        } else if to_end < f_out && f_out > 0.0 {
            smoothstep(to_end / f_out)
        } else {
            1.0
        }
    }
}

/// A timeline region played back at non-1.0 speed (0.25x to 10.0x).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoSpeedSegment {
    pub id: Uuid,
    pub start_time: f64,
    pub end_time: f64,
    pub speed_factor: f64,
}

impl VideoSpeedSegment {
    pub const MIN_FACTOR: f64 = 0.25;
    pub const MAX_FACTOR: f64 = 10.0;

    pub fn new(start_time: f64, end_time: f64, speed_factor: f64) -> Self {
        Self {
            id: Uuid::new_v4(),
            start_time,
            end_time,
            speed_factor: speed_factor.clamp(Self::MIN_FACTOR, Self::MAX_FACTOR),
        }
    }

    pub fn source_duration(&self) -> f64 {
        (self.end_time - self.start_time).max(0.0)
    }

    pub fn composition_duration(&self) -> f64 {
        if self.speed_factor <= 0.0 {
            return self.source_duration();
        }
        self.source_duration() / self.speed_factor
    }
}

/// A timeline text callout with fade in/out animations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoTextSegment {
    pub id: Uuid,
    pub start_time: f64,
    pub end_time: f64,
    pub rect: Rect, // Normalized coordinates (0..1)
    pub text: String,
    pub font_size: f64,
    pub font_family: String,
    pub text_color: [u8; 4],
    pub bg_color: [u8; 4],
    pub fade_in: f64,
    pub fade_out: f64,
}

impl VideoTextSegment {
    pub const DEFAULT_FADE: f64 = 0.25;

    pub fn new(start_time: f64, end_time: f64, rect: Rect, text: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            start_time,
            end_time,
            rect,
            text,
            font_size: 48.0,
            font_family: "System".to_string(),
            text_color: [255, 255, 255, 255],
            bg_color: [0, 0, 0, 180],
            fade_in: Self::DEFAULT_FADE,
            fade_out: Self::DEFAULT_FADE,
        }
    }

    pub fn duration(&self) -> f64 {
        (self.end_time - self.start_time).max(0.0)
    }

    pub fn opacity_at(&self, t: f64) -> f64 {
        if t < self.start_time || t > self.end_time || self.duration() <= 0.0 {
            return 0.0;
        }

        let f_in = (self.duration() / 2.0 - 0.001).max(0.0).min(self.fade_in);
        let f_out = (self.duration() / 2.0 - 0.001).max(0.0).min(self.fade_out);
        let into = t - self.start_time;
        let to_end = self.end_time - t;

        if into < f_in && f_in > 0.0 {
            smoothstep(into / f_in)
        } else if to_end < f_out && f_out > 0.0 {
            smoothstep(to_end / f_out)
        } else {
            1.0
        }
    }
}

/// Comprehensive video editing timeline coordinating cuts, zoom punches, censors, speed ramps, and text callouts.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VideoTimeline {
    pub trim_start: f64,
    pub trim_end: f64,
    pub cuts: Vec<VideoCutSegment>,
    pub zooms: Vec<VideoZoomSegment>,
    pub censors: Vec<VideoCensorSegment>,
    pub speeds: Vec<VideoSpeedSegment>,
    pub texts: Vec<VideoTextSegment>,
}

impl VideoTimeline {
    pub fn new(total_duration_secs: f64) -> Self {
        Self {
            trim_start: 0.0,
            trim_end: total_duration_secs.max(0.0),
            cuts: Vec::new(),
            zooms: Vec::new(),
            censors: Vec::new(),
            speeds: Vec::new(),
            texts: Vec::new(),
        }
    }

    /// Check if timestamp `t` falls within an active cut or outside the trim bounds.
    pub fn is_cut(&self, t: f64) -> bool {
        if t < self.trim_start || t > self.trim_end {
            return true;
        }
        self.cuts
            .iter()
            .any(|c| t >= c.start_time && t <= c.end_time)
    }

    /// Retrieve the currently active zoom segment at time `t`, if any.
    pub fn active_zoom(&self, t: f64) -> Option<&VideoZoomSegment> {
        self.zooms
            .iter()
            .find(|z| t >= z.start_time && t <= z.end_time)
    }

    /// Retrieve all active censor segments at time `t`.
    pub fn active_censors(&self, t: f64) -> Vec<&VideoCensorSegment> {
        self.censors
            .iter()
            .filter(|c| t >= c.start_time && t <= c.end_time)
            .collect()
    }

    /// Retrieve all active text segments at time `t`.
    pub fn active_texts(&self, t: f64) -> Vec<&VideoTextSegment> {
        self.texts
            .iter()
            .filter(|txt| t >= txt.start_time && t <= txt.end_time)
            .collect()
    }

    /// Calculate non-cut kept ranges `[(start, end)]` inside `[trim_start, trim_end]`.
    pub fn kept_ranges(&self) -> Vec<(f64, f64)> {
        if self.trim_end <= self.trim_start {
            return Vec::new();
        }

        let mut clipped_cuts: Vec<(f64, f64)> = self
            .cuts
            .iter()
            .filter(|c| c.end_time > c.start_time)
            .map(|c| {
                (
                    c.start_time.max(self.trim_start),
                    c.end_time.min(self.trim_end),
                )
            })
            .filter(|(s, e)| s < e)
            .collect();

        clipped_cuts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        // Merge touching / overlapping cuts
        let mut merged: Vec<(f64, f64)> = Vec::new();
        for cut in clipped_cuts {
            if let Some(last) = merged.last_mut() {
                if cut.0 <= last.1 + 0.001 {
                    last.1 = last.1.max(cut.1);
                    continue;
                }
            }
            merged.push(cut);
        }

        // Compliment of cuts
        let mut kept = Vec::new();
        let mut cursor = self.trim_start;
        for (cs, ce) in merged {
            if cs > cursor + 0.001 {
                kept.push((cursor, cs));
            }
            cursor = cursor.max(ce);
        }
        if cursor < self.trim_end - 0.001 {
            kept.push((cursor, self.trim_end));
        }

        kept
    }

    /// Calculate total composition duration across all kept ranges, accounting for speed ramps.
    pub fn composition_duration(&self) -> f64 {
        let kept = self.kept_ranges();
        let mut total = 0.0;

        for (mut start, end) in kept {
            while start < end {
                if let Some(speed_seg) = self
                    .speeds
                    .iter()
                    .find(|s| s.start_time < end && s.end_time > start)
                {
                    let s_start = start.max(speed_seg.start_time);
                    let s_end = end.min(speed_seg.end_time);

                    if s_start > start {
                        total += s_start - start;
                    }
                    total += (s_end - s_start) / speed_seg.speed_factor;
                    start = s_end;
                } else {
                    total += end - start;
                    break;
                }
            }
        }

        total
    }
}
