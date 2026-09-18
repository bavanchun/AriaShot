use tiny_skia::{Pixmap, PixmapPaint, Transform};

pub struct ScrollStitcher {
    merged: Option<Pixmap>,
    strip_count: usize,
    max_height: usize,
    pub sticky_header_height: usize,
}

impl Default for ScrollStitcher {
    fn default() -> Self {
        Self::new(30_000)
    }
}

impl ScrollStitcher {
    pub fn new(max_height: usize) -> Self {
        Self {
            merged: None,
            strip_count: 0,
            max_height,
            sticky_header_height: 0,
        }
    }

    pub fn strip_count(&self) -> usize {
        self.strip_count
    }

    pub fn stitched_image(&self) -> Option<&Pixmap> {
        self.merged.as_ref()
    }

    pub fn reset(&mut self) {
        self.merged = None;
        self.strip_count = 0;
        self.sticky_header_height = 0;
    }

    /// Set the first settled viewport frame.
    pub fn set_initial_frame(&mut self, frame: Pixmap) {
        self.merged = Some(frame);
        self.strip_count = 1;
    }

    /// Append a newly shifted frame to the stitched document.
    /// Invariant: `safe_offset = (shift_px - 1).max(1)` (-1px seam bias).
    pub fn append_frame(&mut self, frame: &Pixmap, shift_px: usize) -> bool {
        let existing = match &self.merged {
            Some(ex) => ex,
            None => {
                self.set_initial_frame(frame.clone());
                return true;
            }
        };

        let w = existing.width();
        let existing_h = existing.height() as usize;

        // Invariant: -1px seam bias overlap prevents subpixel line artifacts
        let safe_offset = shift_px.saturating_sub(1).max(1);
        let new_total_h = (existing_h + safe_offset).min(self.max_height);

        if new_total_h <= existing_h {
            return false;
        }

        let mut next_merged = match Pixmap::new(w, new_total_h as u32) {
            Some(pm) => pm,
            None => return false,
        };

        // 1. Blit existing content at top
        next_merged.draw_pixmap(
            0,
            0,
            existing.as_ref(),
            &PixmapPaint::default(),
            Transform::identity(),
            None,
        );

        // 2. Blit new frame strip at the bottom
        let frame_h = frame.height() as usize;
        let blit_y = (existing_h as i32) - (frame_h as i32 - safe_offset as i32);

        next_merged.draw_pixmap(
            0,
            blit_y,
            frame.as_ref(),
            &PixmapPaint::default(),
            Transform::identity(),
            None,
        );

        self.merged = Some(next_merged);
        self.strip_count += 1;
        true
    }
}
