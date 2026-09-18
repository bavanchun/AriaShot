use camerashot_core::geometry::Point;
use tiny_skia::{
    FillRule, Paint, PathBuilder, PixmapMut, PixmapRef, Rect as SkRect, Stroke, Transform,
};

pub struct Loupe;

impl Loupe {
    /// Render high-DPI magnified Loupe around the cursor showing pixel grid and color.
    pub fn render(pixmap: &mut PixmapMut, bg_frame: PixmapRef, cursor: Point) {
        let lens_radius = 48.0f32;
        let cx = (cursor.x as f32) + lens_radius + 16.0;
        let cy = (cursor.y as f32) + lens_radius + 16.0;

        // Ensure loupe stays on screen
        let cx = if cx + lens_radius > pixmap.width() as f32 {
            (cursor.x as f32) - lens_radius - 16.0
        } else {
            cx
        };
        let cy = if cy + lens_radius > pixmap.height() as f32 {
            (cursor.y as f32) - lens_radius - 16.0
        } else {
            cy
        };

        // Draw dark background lens circle
        let mut pb = PathBuilder::new();
        pb.push_circle(cx, cy, lens_radius);
        if let Some(path) = pb.finish() {
            let mut bg_paint = Paint::default();
            bg_paint.set_color_rgba8(30, 30, 34, 240);
            bg_paint.anti_alias = true;
            pixmap.fill_path(&path, &bg_paint, FillRule::Winding, Transform::identity(), None);

            // Sample center pixel color
            let px_x = (cursor.x.round() as usize).min(bg_frame.width() as usize - 1);
            let px_y = (cursor.y.round() as usize).min(bg_frame.height() as usize - 1);
            let idx = (px_y * bg_frame.width() as usize + px_x) * 4;
            let data = bg_frame.data();
            let r = data[idx];
            let g = data[idx + 1];
            let b = data[idx + 2];

            // Render center zoomed block
            let mut center_pb = PathBuilder::new();
            if let Some(r) = SkRect::from_xywh(cx - 8.0, cy - 8.0, 16.0, 16.0) {
                center_pb.push_rect(r);
            }
            if let Some(c_path) = center_pb.finish() {
                let mut c_paint = Paint::default();
                c_paint.set_color_rgba8(r, g, b, 255);
                pixmap.fill_path(&c_path, &c_paint, FillRule::Winding, Transform::identity(), None);
            }

            // Lens border ring
            let mut border_paint = Paint::default();
            border_paint.set_color_rgba8(255, 255, 255, 220);
            border_paint.anti_alias = true;
            let mut stroke = Stroke::default();
            stroke.width = 2.5;
            pixmap.stroke_path(&path, &border_paint, &stroke, Transform::identity(), None);

            // Crosshair
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
                pixmap.stroke_path(&cross_path, &cross_paint, &cross_stroke, Transform::identity(), None);
            }
        }
    }
}
