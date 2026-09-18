use tiny_skia::Pixmap;

pub struct SadFilter;

impl SadFilter {
    /// Detect width of right vertical scrollbar (if present) by looking for columns
    /// on the right edge whose pixels changed independently of document motion.
    /// Matches macshot's detectRightMargin: scans rows 20%..80%, maxScanCols min(50, w/8).
    /// If scrollbarWidth is in 3..=40, returns scrollbarWidth + 4 padding pixels.
    pub fn detect_scrollbar_margin(prev: &Pixmap, curr: &Pixmap) -> usize {
        let w = prev.width() as usize;
        let h = prev.height() as usize;

        if w < 100 || h < 50 || curr.width() as usize != w || curr.height() as usize != h {
            return 0;
        }

        let max_margin = 50usize.min(w / 8);
        let prev_data = prev.data();
        let curr_data = curr.data();

        let row_start = h * 2 / 10;
        let row_end = h * 8 / 10;
        let row_step = ((row_end - row_start) / 40).max(1);

        let mut scrollbar_width = 0usize;

        for col_from_right in 0..max_margin {
            let x = w - 1 - col_from_right;
            let mut diff_sum = 0u64;
            let mut samples = 0usize;

            for y in (row_start..row_end).step_by(row_step) {
                let idx = (y * w + x) * 4;
                let dr = (prev_data[idx] as i32 - curr_data[idx] as i32).abs();
                let dg = (prev_data[idx + 1] as i32 - curr_data[idx + 1] as i32).abs();
                let db = (prev_data[idx + 2] as i32 - curr_data[idx + 2] as i32).abs();
                diff_sum += (dr + dg + db) as u64;
                samples += 3;
            }

            if samples > 0 {
                let avg_sad = (diff_sum as f32) / (samples as f32);
                if avg_sad > 8.0 {
                    scrollbar_width = col_from_right + 1;
                } else if scrollbar_width > 0 {
                    break;
                }
            }
        }

        if (3..=40).contains(&scrollbar_width) {
            scrollbar_width + 4
        } else {
            0
        }
    }

    /// Detect height of a static/frozen sticky header at the top of the viewport.
    /// When content has scrolled by `shift_px`, rows in the sticky header remain identical
    /// between previous and current frames (SAD <= 8.0).
    pub fn detect_sticky_header(prev: &Pixmap, curr: &Pixmap, shift_px: usize) -> usize {
        let w = prev.width() as usize;
        let h = prev.height() as usize;

        if shift_px < 10 || h < 80 || curr.width() as usize != w || curr.height() as usize != h {
            return 0;
        }

        let max_header_scan = (h / 2).min(shift_px.saturating_sub(5));
        let prev_data = prev.data();
        let curr_data = curr.data();

        let mut sticky_rows = 0usize;

        for y in 0..max_header_scan {
            let row_offset = y * w * 4;
            let mut diff_sum = 0u64;
            let mut samples = 0usize;

            // Sample every 4th column
            for x in (0..w).step_by(4) {
                let idx = row_offset + x * 4;
                let dr = (prev_data[idx] as i32 - curr_data[idx] as i32).abs();
                let dg = (prev_data[idx + 1] as i32 - curr_data[idx + 1] as i32).abs();
                let db = (prev_data[idx + 2] as i32 - curr_data[idx + 2] as i32).abs();
                diff_sum += (dr + dg + db) as u64;
                samples += 3;
            }

            let avg_sad = (diff_sum as f32) / (samples as f32);
            if avg_sad <= 8.0 {
                sticky_rows += 1;
            } else {
                break;
            }
        }

        if sticky_rows >= 10 {
            sticky_rows
        } else {
            0
        }
    }
}
