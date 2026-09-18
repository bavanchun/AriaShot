use camerashot_core::geometry::Point;
use tiny_skia::{
    FillRule, Mask, Paint, PathBuilder, Pixmap, PixmapMut, PixmapPaint, PixmapRef,
    Rect as SkRect, Stroke, Transform,
};

pub struct Loupe;

/// Loupe radius in pixels (circle diameter = 2 * RADIUS).
const RADIUS: u32 = 48;
/// Magnification factor.
const MAG: u32 = 2;

/// Build a magnified pixmap of size `(2*radius) × (2*radius)` by sampling from `bg`
/// around `cursor` with nearest-neighbor 2x zoom.
///
/// Each source pixel maps to a `MAG × MAG` block in the output.
/// Pixels outside the source image are filled with transparent black.
pub fn sample_magnified(bg: PixmapRef, cursor: Point, radius: u32) -> Pixmap {
    let diameter = radius * 2;
    let mut out = Pixmap::new(diameter, diameter).expect("diameter > 0");
    let data = out.data_mut();

    // Number of source pixels that map into the output
    let src_half = (radius / MAG) as i32;
    let cx = cursor.x.round() as i32;
    let cy = cursor.y.round() as i32;

    let bw = bg.width() as i32;
    let bh = bg.height() as i32;
    let bg_data = bg.data();

    for sy_off in -src_half..src_half {
        for sx_off in -src_half..src_half {
            let src_x = cx + sx_off;
            let src_y = cy + sy_off;

            // Destination block top-left in the output pixmap
            let dx = ((sx_off + src_half) as u32) * MAG;
            let dy = ((sy_off + src_half) as u32) * MAG;

            let (r, g, b, a) = if src_x >= 0 && src_x < bw && src_y >= 0 && src_y < bh {
                let idx = ((src_y as usize) * (bw as usize) + (src_x as usize)) * 4;
                (bg_data[idx], bg_data[idx + 1], bg_data[idx + 2], bg_data[idx + 3])
            } else {
                (0, 0, 0, 0)
            };

            // Fill the MAG×MAG block
            for by in 0..MAG {
                for bx in 0..MAG {
                    let px = dx + bx;
                    let py = dy + by;
                    if px < diameter && py < diameter {
                        let oi = ((py as usize) * (diameter as usize) + (px as usize)) * 4;
                        data[oi] = r;
                        data[oi + 1] = g;
                        data[oi + 2] = b;
                        data[oi + 3] = a;
                    }
                }
            }
        }
    }

    out
}

impl Loupe {
    /// Render high-DPI magnified Loupe around the cursor showing pixel grid and color.
    pub fn render(pixmap: &mut PixmapMut, bg_frame: PixmapRef, cursor: Point) {
        let lens_radius = RADIUS as f32;
        let diameter = lens_radius * 2.0;

        // Position loupe offset from cursor
        let mut cx = (cursor.x as f32) + lens_radius + 16.0;
        let mut cy = (cursor.y as f32) + lens_radius + 16.0;

        // Flip to stay on screen
        if cx + lens_radius > pixmap.width() as f32 {
            cx = (cursor.x as f32) - lens_radius - 16.0;
        }
        if cy + lens_radius > pixmap.height() as f32 {
            cy = (cursor.y as f32) - lens_radius - 16.0;
        }

        // Build circular clip mask
        let mask = {
            let mut m = Mask::new(pixmap.width(), pixmap.height())
                .expect("mask dimensions match pixmap");
            let mut circle_pb = PathBuilder::new();
            circle_pb.push_circle(cx, cy, lens_radius);
            if let Some(circle_path) = circle_pb.finish() {
                m.fill_path(
                    &circle_path,
                    FillRule::Winding,
                    true,
                    Transform::identity(),
                );
            }
            m
        };

        // Draw dark background through circular mask
        {
            let mut bg_pb = PathBuilder::new();
            if let Some(r) = SkRect::from_xywh(cx - lens_radius, cy - lens_radius, diameter, diameter) {
                bg_pb.push_rect(r);
            }
            if let Some(bg_path) = bg_pb.finish() {
                let mut bg_paint = Paint::default();
                bg_paint.set_color_rgba8(30, 30, 34, 240);
                bg_paint.anti_alias = true;
                pixmap.fill_path(
                    &bg_path,
                    &bg_paint,
                    FillRule::Winding,
                    Transform::identity(),
                    Some(&mask),
                );
            }
        }

        // Sample and draw magnified content
        let mag_pixmap = sample_magnified(bg_frame, cursor, RADIUS);
        pixmap.draw_pixmap(
            (cx - lens_radius) as i32,
            (cy - lens_radius) as i32,
            mag_pixmap.as_ref(),
            &PixmapPaint::default(),
            Transform::identity(),
            Some(&mask),
        );

        // Draw subtle pixel grid lines (every MAG pixels in the magnified image)
        {
            let mut grid_pb = PathBuilder::new();
            let top_x = cx - lens_radius;
            let top_y = cy - lens_radius;
            let step = MAG as f32;
            let count = (RADIUS * 2 / MAG) as u32;

            for i in 1..count {
                let offset = (i as f32) * step;
                // Vertical line
                grid_pb.move_to(top_x + offset, top_y);
                grid_pb.line_to(top_x + offset, top_y + diameter);
                // Horizontal line
                grid_pb.move_to(top_x, top_y + offset);
                grid_pb.line_to(top_x + diameter, top_y + offset);
            }
            if let Some(grid_path) = grid_pb.finish() {
                let mut grid_paint = Paint::default();
                grid_paint.set_color_rgba8(255, 255, 255, 25);
                let mut grid_stroke = Stroke::default();
                grid_stroke.width = 0.5;
                pixmap.stroke_path(
                    &grid_path,
                    &grid_paint,
                    &grid_stroke,
                    Transform::identity(),
                    Some(&mask),
                );
            }
        }

        // Highlight center pixel with a bright border
        {
            let center_x = cx - (MAG as f32) / 2.0;
            let center_y = cy - (MAG as f32) / 2.0;
            let mut cpb = PathBuilder::new();
            if let Some(r) = SkRect::from_xywh(center_x, center_y, MAG as f32, MAG as f32) {
                cpb.push_rect(r);
            }
            if let Some(cp) = cpb.finish() {
                let mut highlight_paint = Paint::default();
                highlight_paint.set_color_rgba8(255, 255, 255, 200);
                highlight_paint.anti_alias = true;
                let mut hs = Stroke::default();
                hs.width = 1.5;
                pixmap.stroke_path(&cp, &highlight_paint, &hs, Transform::identity(), Some(&mask));
            }
        }

        // Lens border ring (no mask — drawn outside)
        {
            let mut border_pb = PathBuilder::new();
            border_pb.push_circle(cx, cy, lens_radius);
            if let Some(border_path) = border_pb.finish() {
                let mut border_paint = Paint::default();
                border_paint.set_color_rgba8(255, 255, 255, 220);
                border_paint.anti_alias = true;
                let mut stroke = Stroke::default();
                stroke.width = 2.5;
                pixmap.stroke_path(
                    &border_path,
                    &border_paint,
                    &stroke,
                    Transform::identity(),
                    None,
                );
            }
        }

        // Crosshair
        {
            let mut cross_pb = PathBuilder::new();
            cross_pb.move_to(cx - 12.0, cy);
            cross_pb.line_to(cx + 12.0, cy);
            cross_pb.move_to(cx, cy - 12.0);
            cross_pb.line_to(cx, cy + 12.0);
            if let Some(cross_path) = cross_pb.finish() {
                let mut cross_stroke = Stroke::default();
                cross_stroke.width = 1.0;
                let mut cross_paint = Paint::default();
                cross_paint.set_color_rgba8(255, 255, 255, 180);
                pixmap.stroke_path(
                    &cross_path,
                    &cross_paint,
                    &cross_stroke,
                    Transform::identity(),
                    None,
                );
            }
        }
    }
}
