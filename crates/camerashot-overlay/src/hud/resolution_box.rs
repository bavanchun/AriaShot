use camerashot_core::geometry::Rect;
use tiny_skia::{FillRule, Paint, PathBuilder, PixmapMut, Transform};

pub struct ResolutionBox;

impl ResolutionBox {
    /// Render floating resolution HUD badge above or inside the selection rect.
    pub fn render(pixmap: &mut PixmapMut, rect: Rect) {
        if rect.width() <= 0.0 || rect.height() <= 0.0 {
            return;
        }

        let badge_w = 90.0f32;
        let badge_h = 24.0f32;

        let badge_x = (rect.min_x() as f32).max(4.0);
        let badge_y = if rect.min_y() >= (badge_h + 8.0) as f64 {
            (rect.min_y() as f32) - badge_h - 4.0
        } else {
            (rect.min_y() as f32) + 4.0
        };

        // Dark background capsule
        let mut pb = PathBuilder::new();
        let r = 4.0f32;
        pb.move_to(badge_x + r, badge_y);
        pb.line_to(badge_x + badge_w - r, badge_y);
        pb.quad_to(badge_x + badge_w, badge_y, badge_x + badge_w, badge_y + r);
        pb.line_to(badge_x + badge_w, badge_y + badge_h - r);
        pb.quad_to(
            badge_x + badge_w,
            badge_y + badge_h,
            badge_x + badge_w - r,
            badge_y + badge_h,
        );
        pb.line_to(badge_x + r, badge_y + badge_h);
        pb.quad_to(badge_x, badge_y + badge_h, badge_x, badge_y + badge_h - r);
        pb.line_to(badge_x, badge_y + r);
        pb.quad_to(badge_x, badge_y, badge_x + r, badge_y);
        pb.close();

        if let Some(path) = pb.finish() {
            let mut paint = Paint::default();
            paint.set_color_rgba8(24, 24, 28, 220);
            paint.anti_alias = true;
            pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
        }
    }
}
