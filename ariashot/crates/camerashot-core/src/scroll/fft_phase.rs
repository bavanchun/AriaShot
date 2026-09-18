use tiny_skia::Pixmap;

/// Deterministic vertical displacement estimator for continuous scrolling content.
/// Evaluates multi-column Sum of Absolute Differences (SAD) with multi-scale refinement.
pub struct VerticalShiftEstimator;

impl VerticalShiftEstimator {
    /// Minimum vertical displacement to avoid noise jitter.
    pub const MIN_SHIFT_PX: usize = 10;
    /// Maximum acceptable mean error per pixel (0..255) to qualify as a valid match.
    pub const MAX_ACCEPTABLE_SAD: f32 = 18.0;

    /// Estimate the vertical scroll shift in pixels from `previous_frame` to `current_frame`.
    /// `right_margin_px` is the width of the detected right scrollbar to exclude from comparison.
    pub fn estimate_shift(prev: &Pixmap, curr: &Pixmap, right_margin_px: usize) -> Option<usize> {
        let w = prev.width() as usize;
        let h = prev.height() as usize;

        if w < 10 || h < 40 || curr.width() as usize != w || curr.height() as usize != h {
            return None;
        }

        let usable_w = w.saturating_sub(right_margin_px).max(10);
        let prev_data = prev.data();
        let curr_data = curr.data();

        // 1. Select 8 evenly spaced test columns across usable width
        let num_sample_cols = 8usize;
        let mut sample_cols = Vec::with_capacity(num_sample_cols);
        for i in 1..=num_sample_cols {
            let x = (usable_w * i) / (num_sample_cols + 1);
            sample_cols.push(x);
        }

        let max_shift = h.saturating_sub(15);
        let min_shift = (h / 20).max(Self::MIN_SHIFT_PX);

        if min_shift >= max_shift {
            return None;
        }

        let mut best_shift = 0usize;
        let mut min_sad = f32::MAX;

        // 2. Coarse search across candidate shifts
        for shift in min_shift..=max_shift {
            let overlap_h = h - shift;
            if overlap_h < 20 {
                continue;
            }

            let mut total_diff = 0u64;
            let mut sample_count = 0usize;

            // Sample every 2nd row in overlap region for speed
            for y_overlap in (0..overlap_h).step_by(2) {
                let y_prev = y_overlap + shift;
                let y_curr = y_overlap;

                let row_prev = y_prev * w * 4;
                let row_curr = y_curr * w * 4;

                for &x in &sample_cols {
                    let idx_prev = row_prev + x * 4;
                    let idx_curr = row_curr + x * 4;

                    let dr = (prev_data[idx_prev] as i32 - curr_data[idx_curr] as i32).abs();
                    let dg =
                        (prev_data[idx_prev + 1] as i32 - curr_data[idx_curr + 1] as i32).abs();
                    let db =
                        (prev_data[idx_prev + 2] as i32 - curr_data[idx_curr + 2] as i32).abs();

                    total_diff += (dr + dg + db) as u64;
                    sample_count += 3;
                }
            }

            if sample_count > 0 {
                let sad = (total_diff as f32) / (sample_count as f32);
                if sad < min_sad {
                    min_sad = sad;
                    best_shift = shift;
                }
            }
        }

        if min_sad <= Self::MAX_ACCEPTABLE_SAD && best_shift > 0 {
            // 3. Fine refinement pass (+/- 2 pixels with every row)
            let fine_lo = best_shift.saturating_sub(2).max(min_shift);
            let fine_hi = (best_shift + 2).min(max_shift);

            let mut refined_shift = best_shift;
            let mut refined_min_sad = min_sad;

            for shift in fine_lo..=fine_hi {
                let overlap_h = h - shift;
                let mut total_diff = 0u64;
                let mut sample_count = 0usize;

                for y_overlap in 0..overlap_h {
                    let y_prev = y_overlap + shift;
                    let y_curr = y_overlap;

                    let row_prev = y_prev * w * 4;
                    let row_curr = y_curr * w * 4;

                    for &x in &sample_cols {
                        let idx_prev = row_prev + x * 4;
                        let idx_curr = row_curr + x * 4;

                        let dr = (prev_data[idx_prev] as i32 - curr_data[idx_curr] as i32).abs();
                        let dg =
                            (prev_data[idx_prev + 1] as i32 - curr_data[idx_curr + 1] as i32).abs();
                        let db =
                            (prev_data[idx_prev + 2] as i32 - curr_data[idx_curr + 2] as i32).abs();

                        total_diff += (dr + dg + db) as u64;
                        sample_count += 3;
                    }
                }

                let sad = (total_diff as f32) / (sample_count as f32);
                if sad < refined_min_sad {
                    refined_min_sad = sad;
                    refined_shift = shift;
                }
            }

            Some(refined_shift)
        } else {
            None
        }
    }
}
