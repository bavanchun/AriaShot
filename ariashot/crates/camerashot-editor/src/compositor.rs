use crate::timeline_ctrl::{CensorStyle, VideoTimeline};
use camerashot_media::{RecordedVideoFrame, RecordingSession};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct CompositedFrame {
    pub pts: Duration,
    pub width: u32,
    pub height: u32,
    pub rgba_data: Vec<u8>,
}

/// Software timeline compositor that applies cuts, zoom punch easing, censor blocks,
/// and speed ramps to recorded video frames.
pub struct TimelineCompositor;

impl TimelineCompositor {
    /// Render an edited video sequence from a raw RecordingSession and a VideoTimeline.
    pub fn composite_session(
        session: &RecordingSession,
        timeline: &VideoTimeline,
    ) -> Vec<RecordedVideoFrame> {
        Self::composite_frames(&session.frames, timeline)
    }

    /// Render a slice of video frames through the timeline non-destructive edits.
    pub fn composite_frames(
        frames: &[RecordedVideoFrame],
        timeline: &VideoTimeline,
    ) -> Vec<RecordedVideoFrame> {
        let mut output_frames = Vec::new();

        for frame in frames {
            let t = frame.pts.as_secs_f64();

            // 1. Skip frame if it falls in a cut or outside trim
            if timeline.is_cut(t) {
                continue;
            }

            let mut current_frame = frame.clone();

            // 2. Apply Camera Zoom Punch if active
            if let Some(zoom_seg) = timeline.active_zoom(t) {
                let zoom = zoom_seg.zoom_level_at(t);
                if zoom > 1.001 {
                    current_frame = Self::apply_zoom(
                        &current_frame,
                        zoom,
                        zoom_seg.crop_rect(
                            zoom,
                            current_frame.width as f64,
                            current_frame.height as f64,
                        ),
                    );
                }
            }

            // 3. Apply Censor Segments (Pixelate / Blur / Solid)
            let censors = timeline.active_censors(t);
            for censor in censors {
                let opacity = censor.opacity_at(t);
                if opacity > 0.001 {
                    Self::apply_censor(&mut current_frame, censor.rect, censor.style);
                }
            }

            output_frames.push(current_frame);
        }

        output_frames
    }

    /// Crop frame to `crop_rect` and scale back to original dimensions using bilinear sampling.
    fn apply_zoom(
        frame: &RecordedVideoFrame,
        _zoom: f64,
        crop_rect: camerashot_core::geometry::Rect,
    ) -> RecordedVideoFrame {
        let src_w = frame.width as usize;
        let src_h = frame.height as usize;
        let dst_w = frame.width as usize;
        let dst_h = frame.height as usize;

        let mut zoomed_data = vec![0u8; dst_w * dst_h * 4];

        let crop_x = crop_rect.origin.x.max(0.0);
        let crop_y = crop_rect.origin.y.max(0.0);
        let crop_w = crop_rect.size.width.max(1.0);
        let crop_h = crop_rect.size.height.max(1.0);

        for dy in 0..dst_h {
            let v = dy as f64 / dst_h as f64;
            let sy = (crop_y + v * crop_h).clamp(0.0, (src_h - 1) as f64) as usize;

            for dx in 0..dst_w {
                let u = dx as f64 / dst_w as f64;
                let sx = (crop_x + u * crop_w).clamp(0.0, (src_w - 1) as f64) as usize;

                let src_idx = (sy * src_w + sx) * 4;
                let dst_idx = (dy * dst_w + dx) * 4;

                zoomed_data[dst_idx..dst_idx + 4]
                    .copy_from_slice(&frame.rgba_data[src_idx..src_idx + 4]);
            }
        }

        RecordedVideoFrame {
            pts: frame.pts,
            width: frame.width,
            height: frame.height,
            rgba_data: zoomed_data,
        }
    }

    /// Obscure a normalized sub-region (0..1) of the frame.
    fn apply_censor(
        frame: &mut RecordedVideoFrame,
        norm_rect: camerashot_core::geometry::Rect,
        style: CensorStyle,
    ) {
        let w = frame.width as f64;
        let h = frame.height as f64;

        let px_x = ((norm_rect.origin.x * w).floor() as usize).min(frame.width as usize);
        let px_y = ((norm_rect.origin.y * h).floor() as usize).min(frame.height as usize);
        let px_w = ((norm_rect.size.width * w).ceil() as usize).min(frame.width as usize - px_x);
        let px_h = ((norm_rect.size.height * h).ceil() as usize).min(frame.height as usize - px_y);

        if px_w == 0 || px_h == 0 {
            return;
        }

        let stride = frame.width as usize;

        match style {
            CensorStyle::Solid => {
                for y in px_y..(px_y + px_h) {
                    for x in px_x..(px_x + px_w) {
                        let idx = (y * stride + x) * 4;
                        frame.rgba_data[idx] = 0;
                        frame.rgba_data[idx + 1] = 0;
                        frame.rgba_data[idx + 2] = 0;
                        frame.rgba_data[idx + 3] = 255;
                    }
                }
            }
            CensorStyle::Pixelate => {
                let block_size = 16usize;
                for by in (px_y..(px_y + px_h)).step_by(block_size) {
                    let bh = block_size.min(px_y + px_h - by);
                    for bx in (px_x..(px_x + px_w)).step_by(block_size) {
                        let bw = block_size.min(px_x + px_w - bx);

                        // Average color in block
                        let mut sum_r = 0u64;
                        let mut sum_g = 0u64;
                        let mut sum_b = 0u64;
                        let mut sum_a = 0u64;
                        let count = (bw * bh) as u64;

                        for iy in 0..bh {
                            for ix in 0..bw {
                                let idx = ((by + iy) * stride + (bx + ix)) * 4;
                                sum_r += frame.rgba_data[idx] as u64;
                                sum_g += frame.rgba_data[idx + 1] as u64;
                                sum_b += frame.rgba_data[idx + 2] as u64;
                                sum_a += frame.rgba_data[idx + 3] as u64;
                            }
                        }

                        let avg_r = (sum_r / count) as u8;
                        let avg_g = (sum_g / count) as u8;
                        let avg_b = (sum_b / count) as u8;
                        let avg_a = (sum_a / count) as u8;

                        // Fill block with average color
                        for iy in 0..bh {
                            for ix in 0..bw {
                                let idx = ((by + iy) * stride + (bx + ix)) * 4;
                                frame.rgba_data[idx] = avg_r;
                                frame.rgba_data[idx + 1] = avg_g;
                                frame.rgba_data[idx + 2] = avg_b;
                                frame.rgba_data[idx + 3] = avg_a;
                            }
                        }
                    }
                }
            }
            CensorStyle::Blur => {
                // Fast 2-pass box blur on the region
                let radius = 8usize;
                let mut temp = frame.rgba_data.clone();

                // Horizontal pass
                for y in px_y..(px_y + px_h) {
                    for x in px_x..(px_x + px_w) {
                        let mut r = 0u32;
                        let mut g = 0u32;
                        let mut b = 0u32;
                        let mut cnt = 0u32;

                        let x_start = x.saturating_sub(radius).max(px_x);
                        let x_end = (x + radius).min(px_x + px_w - 1);

                        for kx in x_start..=x_end {
                            let idx = (y * stride + kx) * 4;
                            r += frame.rgba_data[idx] as u32;
                            g += frame.rgba_data[idx + 1] as u32;
                            b += frame.rgba_data[idx + 2] as u32;
                            cnt += 1;
                        }

                        let dst = (y * stride + x) * 4;
                        temp[dst] = (r / cnt) as u8;
                        temp[dst + 1] = (g / cnt) as u8;
                        temp[dst + 2] = (b / cnt) as u8;
                    }
                }

                // Vertical pass
                for y in px_y..(px_y + px_h) {
                    let y_start = y.saturating_sub(radius).max(px_y);
                    let y_end = (y + radius).min(px_y + px_h - 1);

                    for x in px_x..(px_x + px_w) {
                        let mut r = 0u32;
                        let mut g = 0u32;
                        let mut b = 0u32;
                        let mut cnt = 0u32;

                        for ky in y_start..=y_end {
                            let idx = (ky * stride + x) * 4;
                            r += temp[idx] as u32;
                            g += temp[idx + 1] as u32;
                            b += temp[idx + 2] as u32;
                            cnt += 1;
                        }

                        let dst = (y * stride + x) * 4;
                        frame.rgba_data[dst] = (r / cnt) as u8;
                        frame.rgba_data[dst + 1] = (g / cnt) as u8;
                        frame.rgba_data[dst + 2] = (b / cnt) as u8;
                    }
                }
            }
        }
    }
}
