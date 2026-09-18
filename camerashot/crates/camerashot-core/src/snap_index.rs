use crate::geometry::Rect;

/// Image-edge index for "boundary snap" — snapping the capture selection's
/// dragged edges to strong color boundaries (UI lines, window borders, table
/// rows, etc.) in the captured screenshot.
///
/// Works purely in image space without requiring window manager or compositor
/// privileges, guaranteeing full functionality under Wayland, X11, and macOS.
///
/// Built ONCE per screenshot (off the main thread). During a resize drag, the
/// lookup is a fast scan over a small ±radius window, scored only across the
/// selection's perpendicular span so it snaps to edges that actually run along
/// the dragged side.
#[derive(Debug, Clone)]
pub struct BoundarySnapIndex {
    pub width: usize,
    pub height: usize,
    /// Rect the screenshot is mapped to (overlay-space). Used to map
    /// view points <-> image pixels.
    pub draw_rect: Rect,

    /// Per-pixel vertical edge strength: difference between column x-1 and x.
    /// Indexed `[y * (width + 1) + x_boundary]`, x_boundary in 1..width.
    vertical_diff: Vec<f32>,
    /// Per-pixel horizontal edge strength: difference between row y-1 and y.
    /// Indexed `[y_boundary * width + x]`, y_boundary in 1..height.
    horizontal_diff: Vec<f32>,
}

/// A qualifying snap target.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hit {
    /// Overlay-space coordinate to snap the edge to.
    pub view_position: f64,
    /// Pixel boundary index.
    pub pixel_boundary: usize,
    /// Mean color difference along the tested span.
    pub strength: f32,
}

impl BoundarySnapIndex {
    /// Minimum mean RGB Euclidean distance (0..~441.67) along the tested span to qualify as an edge.
    pub const MIN_MEAN_DIFF: f32 = 28.0;
    /// Minimum fraction of the tested span that must have contrast >= MIN_MEAN_DIFF.
    pub const MIN_SUPPORT_FRACTION: f32 = 0.55;

    /// Build the boundary snap index from raw RGBA8 pixels and the target draw rect.
    /// Returns None for degenerate dimensions or buffer length mismatch.
    pub fn build(width: usize, height: usize, pixels: &[u8], draw_rect: Rect) -> Option<Self> {
        if width < 2 || height < 2 || draw_rect.width() <= 0.0 || draw_rect.height() <= 0.0 {
            return None;
        }

        // Cap work to prevent extreme memory allocation (e.g. >40MP)
        if width * height > 40_000_000 {
            return None;
        }

        let expected_len = width * height * 4;
        if pixels.len() < expected_len {
            return None;
        }

        let bytes_per_row = width * 4;
        let mut v_diff = vec![0.0f32; height * (width + 1)];
        let mut h_diff = vec![0.0f32; height * width];

        // 1. Vertical boundaries: difference between column x and x-1 per row.
        for y in 0..height {
            let row_offset = y * bytes_per_row;
            let v_base = y * (width + 1);
            for x in 1..width {
                let idx_a = row_offset + (x - 1) * 4;
                let idx_b = row_offset + x * 4;
                v_diff[v_base + x] = Self::color_dist(pixels, idx_a, idx_b);
            }
        }

        // 2. Horizontal boundaries: difference between row y and y-1 per column.
        for y in 1..height {
            let row_a = (y - 1) * bytes_per_row;
            let row_b = y * bytes_per_row;
            let h_base = y * width;
            for x in 0..width {
                let idx_a = row_a + x * 4;
                let idx_b = row_b + x * 4;
                h_diff[h_base + x] = Self::color_dist(pixels, idx_a, idx_b);
            }
        }

        Some(Self {
            width,
            height,
            draw_rect,
            vertical_diff: v_diff,
            horizontal_diff: h_diff,
        })
    }

    #[inline(always)]
    fn color_dist(p: &[u8], a: usize, b: usize) -> f32 {
        let dr = (p[a] as f32) - (p[b] as f32);
        let dg = (p[a + 1] as f32) - (p[b + 1] as f32);
        let db = (p[a + 2] as f32) - (p[b + 2] as f32);
        (dr * dr + dg * dg + db * db).sqrt()
    }

    #[inline]
    pub fn scale_x(&self) -> f64 {
        (self.width as f64) / self.draw_rect.width()
    }

    #[inline]
    pub fn scale_y(&self) -> f64 {
        (self.height as f64) / self.draw_rect.height()
    }

    /// Overlay-space X -> pixel boundary (clamped to 0..=width).
    #[inline]
    pub fn pixel_x(&self, view_x: f64) -> usize {
        let px = ((view_x - self.draw_rect.min_x()) * self.scale_x()).round() as isize;
        px.clamp(0, self.width as isize) as usize
    }

    /// Overlay-space Y -> pixel boundary (clamped to 0..=height) in top-left coordinates.
    #[inline]
    pub fn pixel_y(&self, view_y: f64) -> usize {
        let py = ((view_y - self.draw_rect.min_y()) * self.scale_y()).round() as isize;
        py.clamp(0, self.height as isize) as usize
    }

    #[inline]
    pub fn view_x_of(&self, boundary: usize) -> f64 {
        self.draw_rect.min_x() + (boundary as f64) / self.scale_x()
    }

    #[inline]
    pub fn view_y_of(&self, boundary: usize) -> f64 {
        self.draw_rect.min_y() + (boundary as f64) / self.scale_y()
    }

    /// Find the nearest strong VERTICAL image boundary to `view_x`, scoring edge
    /// strength along the selection's `[y_min_view, y_max_view]` span.
    /// `radius_points` is the search radius in overlay points.
    pub fn nearest_vertical(
        &self,
        view_x: f64,
        y_min_view: f64,
        y_max_view: f64,
        radius_points: f64,
    ) -> Option<Hit> {
        let center = self.pixel_x(view_x);
        let radius_px = ((radius_points * self.scale_x()).round() as usize).max(1);

        let mut y0 = self.pixel_y(y_min_view.min(y_max_view));
        let mut y1 = self.pixel_y(y_min_view.max(y_max_view));
        if y0 > y1 {
            std::mem::swap(&mut y0, &mut y1);
        }
        y0 = y0.min(self.height.saturating_sub(1));
        y1 = y1.min(self.height.saturating_sub(1));
        if y1 < y0 {
            return None;
        }

        let span = (y1 - y0 + 1) as f32;
        let mut best: Option<Hit> = None;
        let mut best_dist = usize::MAX;

        let lo = center.saturating_sub(radius_px).max(1);
        let hi = (center + radius_px).min(self.width.saturating_sub(1));
        if lo > hi {
            return None;
        }

        for b in lo..=hi {
            let mut sum = 0.0f32;
            let mut support = 0usize;

            for y in y0..=y1 {
                let d = self.vertical_diff[y * (self.width + 1) + b];
                sum += d;
                if d >= Self::MIN_MEAN_DIFF {
                    support += 1;
                }
            }

            let mean = sum / span;
            let support_frac = (support as f32) / span;

            if mean >= Self::MIN_MEAN_DIFF && support_frac >= Self::MIN_SUPPORT_FRACTION {
                let dist = (b as isize - center as isize).unsigned_abs();
                if dist < best_dist {
                    best_dist = dist;
                    best = Some(Hit {
                        view_position: self.view_x_of(b),
                        pixel_boundary: b,
                        strength: mean,
                    });
                }
            }
        }

        best
    }

    /// Find the nearest strong HORIZONTAL image boundary to `view_y`, scoring edge
    /// strength along the selection's `[x_min_view, x_max_view]` span.
    /// `radius_points` is the search radius in overlay points.
    pub fn nearest_horizontal(
        &self,
        view_y: f64,
        x_min_view: f64,
        x_max_view: f64,
        radius_points: f64,
    ) -> Option<Hit> {
        let center = self.pixel_y(view_y);
        let radius_px = ((radius_points * self.scale_y()).round() as usize).max(1);

        let mut x0 = self.pixel_x(x_min_view.min(x_max_view));
        let mut x1 = self.pixel_x(x_min_view.max(x_max_view));
        if x0 > x1 {
            std::mem::swap(&mut x0, &mut x1);
        }
        x0 = x0.min(self.width.saturating_sub(1));
        x1 = x1.min(self.width.saturating_sub(1));
        if x1 < x0 {
            return None;
        }

        let span = (x1 - x0 + 1) as f32;
        let mut best: Option<Hit> = None;
        let mut best_dist = usize::MAX;

        let lo = center.saturating_sub(radius_px).max(1);
        let hi = (center + radius_px).min(self.height.saturating_sub(1));
        if lo > hi {
            return None;
        }

        for b in lo..=hi {
            let mut sum = 0.0f32;
            let mut support = 0usize;
            let base = b * self.width;

            for x in x0..=x1 {
                let d = self.horizontal_diff[base + x];
                sum += d;
                if d >= Self::MIN_MEAN_DIFF {
                    support += 1;
                }
            }

            let mean = sum / span;
            let support_frac = (support as f32) / span;

            if mean >= Self::MIN_MEAN_DIFF && support_frac >= Self::MIN_SUPPORT_FRACTION {
                let dist = (b as isize - center as isize).unsigned_abs();
                if dist < best_dist {
                    best_dist = dist;
                    best = Some(Hit {
                        view_position: self.view_y_of(b),
                        pixel_boundary: b,
                        strength: mean,
                    });
                }
            }
        }

        best
    }
}
