use crate::geometry::Point;
use serde::{Deserialize, Serialize};

/// Pencil curve smoothing modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PencilSmoothMode {
    /// Mode 0: Raw coordinates without smoothing.
    None = 0,
    /// Mode 1: 2 passes of Chaikin subdivision on stroke finish.
    Smooth = 1,
    /// Mode 2: Trailing moving average ($N=8$) + endpoint padding + 2 passes of Chaikin polish.
    #[default]
    Refined = 2,
}

pub const DEFAULT_SMOOTH_WINDOW_SIZE: usize = 8;

/// Retroactive trailing moving average: replicates the same smoothing that live-smoothing
/// does incrementally (trailing window of N points), but applied to the full buffer at once.
/// Produces identical output with zero drawing lag.
pub fn moving_average_smooth(pts: &[Point], window_size: usize) -> Vec<Point> {
    if pts.len() <= 2 || window_size <= 1 {
        return pts.to_vec();
    }
    let mut result = Vec::with_capacity(pts.len());
    for i in 0..pts.len() {
        let lo = i.saturating_sub(window_size - 1);
        let mut avg_x = 0.0;
        let mut avg_y = 0.0;
        for pt in &pts[lo..=i] {
            avg_x += pt.x;
            avg_y += pt.y;
        }
        let n = (i - lo + 1) as f64;
        result.push(Point::new(avg_x / n, avg_y / n));
    }
    result
}

/// Moving average for scalar values (such as tablet pressure), matching `moving_average_smooth`.
pub fn moving_average_smooth_values(vals: &[f64], window_size: usize) -> Vec<f64> {
    if vals.len() <= 2 || window_size <= 1 {
        return vals.to_vec();
    }
    let mut result = Vec::with_capacity(vals.len());
    for i in 0..vals.len() {
        let lo = i.saturating_sub(window_size - 1);
        let mut sum = 0.0;
        for &val in &vals[lo..=i] {
            sum += val;
        }
        let n = (i - lo + 1) as f64;
        result.push(sum / n);
    }
    result
}

/// Chaikin corner-cutting: each iteration replaces every segment with two points
/// at 25% and 75% along it, keeping endpoints fixed.
/// 2 passes gives natural, gentle smoothing without clipping sharp corners excessively.
pub fn chaikin_smooth(pts: &[Point], iterations: usize) -> Vec<Point> {
    if pts.len() <= 2 || iterations == 0 {
        return pts.to_vec();
    }
    let mut current = pts.to_vec();
    for _ in 0..iterations {
        let mut next = Vec::with_capacity(current.len() * 2);
        next.push(current[0]);
        for i in 0..current.len() - 1 {
            let p0 = current[i];
            let p1 = current[i + 1];
            next.push(Point::new(
                0.75 * p0.x + 0.25 * p1.x,
                0.75 * p0.y + 0.25 * p1.y,
            ));
            next.push(Point::new(
                0.25 * p0.x + 0.75 * p1.x,
                0.25 * p0.y + 0.75 * p1.y,
            ));
        }
        next.push(current[current.len() - 1]);
        current = next;
    }
    current
}

/// Linearly interpolate a values array to a target count.
/// Preserves first and last values exactly; intermediate values are lerped.
pub fn interpolate_to_count(values: &[f64], target_count: usize) -> Vec<f64> {
    if values.len() < 2 || target_count < 2 {
        return values.to_vec();
    }
    if values.len() == target_count {
        return values.to_vec();
    }

    let mut result = Vec::with_capacity(target_count);
    let max_in_idx = (values.len() - 1) as f64;
    let denom = (target_count - 1) as f64;

    for i in 0..target_count {
        let t = (i as f64) / denom * max_in_idx;
        let lo = (t.floor() as usize).min(values.len() - 1);
        let hi = (lo + 1).min(values.len() - 1);
        let frac = t - (lo as f64);
        result.push(values[lo] * (1.0 - frac) + values[hi] * frac);
    }
    result
}

/// Apply full pencil stroke processing pipeline matching macshot's PencilToolHandler.
pub fn smooth_pencil_stroke(
    raw_points: &[Point],
    raw_pressures: Option<&[f64]>,
    mode: PencilSmoothMode,
) -> (Vec<Point>, Option<Vec<f64>>) {
    if raw_points.is_empty() {
        return (Vec::new(), None);
    }

    // Single or double click tap: synthesize 3 close points so line cap creates a round dot
    if raw_points.len() < 3 {
        let p = raw_points[0];
        let synthetic_points = vec![p, Point::new(p.x + 0.5, p.y), Point::new(p.x + 0.5, p.y)];
        let synthetic_pressures = raw_pressures.map(|pr| {
            let p_val = pr.first().copied().unwrap_or(1.0);
            vec![p_val, p_val, p_val]
        });
        return (synthetic_points, synthetic_pressures);
    }

    match mode {
        PencilSmoothMode::None => (raw_points.to_vec(), raw_pressures.map(|p| p.to_vec())),
        PencilSmoothMode::Smooth => {
            let final_pts = chaikin_smooth(raw_points, 2);
            let final_pressures = raw_pressures.map(|p| interpolate_to_count(p, final_pts.len()));
            (final_pts, final_pressures)
        }
        PencilSmoothMode::Refined => {
            let last_pt = *raw_points.last().unwrap();
            let pad_count = DEFAULT_SMOOTH_WINDOW_SIZE.saturating_sub(1);
            let mut padded_pts = Vec::with_capacity(raw_points.len() + pad_count);
            padded_pts.extend_from_slice(raw_points);
            for _ in 0..pad_count {
                padded_pts.push(last_pt);
            }

            let smoothed_pts = moving_average_smooth(&padded_pts, DEFAULT_SMOOTH_WINDOW_SIZE);
            let final_pts = chaikin_smooth(&smoothed_pts, 2);

            let final_pressures = raw_pressures.map(|p| {
                let last_pr = *p.last().unwrap_or(&1.0);
                let mut padded_p = Vec::with_capacity(p.len() + pad_count);
                padded_p.extend_from_slice(p);
                for _ in 0..pad_count {
                    padded_p.push(last_pr);
                }
                let smoothed_p = moving_average_smooth_values(
                    &padded_p,
                    (DEFAULT_SMOOTH_WINDOW_SIZE / 2).max(3),
                );
                interpolate_to_count(&smoothed_p, final_pts.len())
            });

            (final_pts, final_pressures)
        }
    }
}
