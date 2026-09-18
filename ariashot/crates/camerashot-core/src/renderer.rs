use crate::annotation::{Annotation, AnnotationTool, LineStyle, RectFillStyle};
use crate::geometry::Rect;
use tiny_skia::{
    Color, FillRule, LineCap, LineJoin, Paint, PathBuilder, PixmapMut, Rect as SkRect, Stroke,
    StrokeDash, Transform,
};

/// 2D Vector and Raster Annotation Canvas Renderer powered by tiny-skia.
pub struct AnnotationRenderer;

impl AnnotationRenderer {
    /// Render a list of annotations onto an existing pixmap canvas in z-index order.
    pub fn render_annotations(pixmap: &mut PixmapMut, annotations: &[Annotation]) {
        let mut sorted = annotations.to_vec();
        sorted.sort_by_key(|a| a.z_index);

        for ann in &sorted {
            Self::render_single(pixmap, ann);
        }
    }

    /// Render a single annotation onto the pixmap canvas.
    pub fn render_single(pixmap: &mut PixmapMut, ann: &Annotation) {
        match ann.tool {
            AnnotationTool::Pencil => Self::render_pencil(pixmap, ann),
            AnnotationTool::Line => Self::render_line(pixmap, ann),
            AnnotationTool::Arrow => Self::render_arrow(pixmap, ann),
            AnnotationTool::Rectangle => Self::render_rectangle(pixmap, ann),
            AnnotationTool::FilledRectangle => Self::render_filled_rectangle(pixmap, ann),
            AnnotationTool::Ellipse => Self::render_ellipse(pixmap, ann),
            AnnotationTool::Marker => Self::render_marker(pixmap, ann),
            AnnotationTool::Number => Self::render_number_badge(pixmap, ann),
            AnnotationTool::Pixelate => Self::render_pixelate(pixmap, ann),
            AnnotationTool::Blur => Self::render_blur(pixmap, ann),
            AnnotationTool::Measure => Self::render_measure(pixmap, ann),
            AnnotationTool::Highlight => Self::render_highlight(pixmap, ann),
            AnnotationTool::Text | AnnotationTool::TranslateOverlay => {
                Self::render_text_fallback(pixmap, ann);
            }
            _ => {}
        }
    }

    fn color_from_rgba(c: [u8; 4]) -> Color {
        Color::from_rgba8(c[0], c[1], c[2], c[3])
    }

    fn stroke_for_style(ann: &Annotation) -> Stroke {
        let w = ann.stroke_width as f32;
        let mut stroke = Stroke::default();
        stroke.width = w;
        stroke.line_cap = LineCap::Round;
        stroke.line_join = LineJoin::Round;

        match ann.line_style {
            LineStyle::Solid => {}
            LineStyle::Dashed => {
                if let Some(dash) = StrokeDash::new(vec![w * 3.0, w * 2.0], 0.0) {
                    stroke.dash = Some(dash);
                }
            }
            LineStyle::Dotted => {
                let gap = (w * 2.0).max(6.0);
                if let Some(dash) = StrokeDash::new(vec![0.0, gap], 0.0) {
                    stroke.dash = Some(dash);
                    stroke.line_cap = LineCap::Round;
                }
            }
        }
        stroke
    }

    fn render_pencil(pixmap: &mut PixmapMut, ann: &Annotation) {
        let pts = match &ann.points {
            Some(p) if p.len() >= 2 => p,
            _ => return,
        };

        let mut pb = PathBuilder::new();
        pb.move_to(pts[0].x as f32, pts[0].y as f32);
        for p in pts.iter().skip(1) {
            pb.line_to(p.x as f32, p.y as f32);
        }

        if let Some(path) = pb.finish() {
            let mut paint = Paint::default();
            paint.set_color(Self::color_from_rgba(ann.color));
            paint.anti_alias = true;
            let stroke = Self::stroke_for_style(ann);
            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }

    fn render_line(pixmap: &mut PixmapMut, ann: &Annotation) {
        let mut pb = PathBuilder::new();
        pb.move_to(ann.start_point.x as f32, ann.start_point.y as f32);
        pb.line_to(ann.end_point.x as f32, ann.end_point.y as f32);

        if let Some(path) = pb.finish() {
            let mut paint = Paint::default();
            paint.set_color(Self::color_from_rgba(ann.color));
            paint.anti_alias = true;
            let stroke = Self::stroke_for_style(ann);
            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }

    fn render_arrow(pixmap: &mut PixmapMut, ann: &Annotation) {
        let p0 = ann.start_point;
        let p1 = ann.end_point;
        let dx = (p1.x - p0.x) as f32;
        let dy = (p1.y - p0.y) as f32;
        let angle = dy.atan2(dx);

        // Arrow shaft
        let mut pb = PathBuilder::new();
        pb.move_to(p0.x as f32, p0.y as f32);
        pb.line_to(p1.x as f32, p1.y as f32);

        let color = Self::color_from_rgba(ann.color);
        let mut paint = Paint::default();
        paint.set_color(color);
        paint.anti_alias = true;
        let stroke = Self::stroke_for_style(ann);

        if let Some(path) = pb.finish() {
            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }

        // Arrowhead
        let head_len = ((ann.stroke_width as f32) * 4.0).max(14.0);
        let head_angle = 2.6f32; // ~150 degrees
        let x1 = (p1.x as f32) + head_len * (angle + head_angle).cos();
        let y1 = (p1.y as f32) + head_len * (angle + head_angle).sin();
        let x2 = (p1.x as f32) + head_len * (angle - head_angle).cos();
        let y2 = (p1.y as f32) + head_len * (angle - head_angle).sin();

        let mut head_pb = PathBuilder::new();
        head_pb.move_to(p1.x as f32, p1.y as f32);
        head_pb.line_to(x1, y1);
        head_pb.line_to(x2, y2);
        head_pb.close();

        if let Some(head_path) = head_pb.finish() {
            pixmap.fill_path(&head_path, &paint, FillRule::Winding, Transform::identity(), None);
        }
    }

    fn render_rectangle(pixmap: &mut PixmapMut, ann: &Annotation) {
        let rect = Rect::from_points(ann.start_point, ann.end_point);
        let sk_rect = match SkRect::from_xywh(
            rect.min_x() as f32,
            rect.min_y() as f32,
            rect.width() as f32,
            rect.height() as f32,
        ) {
            Some(r) => r,
            None => return,
        };

        let mut paint = Paint::default();
        paint.set_color(Self::color_from_rgba(ann.color));
        paint.anti_alias = true;

        let fill_style = ann.fill_style.unwrap_or(RectFillStyle::Stroke);

        if fill_style == RectFillStyle::Stroke || fill_style == RectFillStyle::StrokeAndFill {
            let mut pb = PathBuilder::new();
            pb.push_rect(sk_rect);
            if let Some(path) = pb.finish() {
                let stroke = Self::stroke_for_style(ann);
                pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
            }
        }

        if fill_style == RectFillStyle::Fill || fill_style == RectFillStyle::StrokeAndFill {
            let mut fill_paint = Paint::default();
            let fc = ann.fill_color.unwrap_or([ann.color[0], ann.color[1], ann.color[2], 80]);
            fill_paint.set_color(Self::color_from_rgba(fc));
            fill_paint.anti_alias = true;
            let mut pb = PathBuilder::new();
            pb.push_rect(sk_rect);
            if let Some(path) = pb.finish() {
                pixmap.fill_path(&path, &fill_paint, FillRule::Winding, Transform::identity(), None);
            }
        }
    }

    fn render_filled_rectangle(pixmap: &mut PixmapMut, ann: &Annotation) {
        let rect = Rect::from_points(ann.start_point, ann.end_point);
        let sk_rect = match SkRect::from_xywh(
            rect.min_x() as f32,
            rect.min_y() as f32,
            rect.width() as f32,
            rect.height() as f32,
        ) {
            Some(r) => r,
            None => return,
        };

        let mut paint = Paint::default();
        paint.set_color(Self::color_from_rgba(ann.color));
        paint.anti_alias = true;

        let mut pb = PathBuilder::new();
        pb.push_rect(sk_rect);
        if let Some(path) = pb.finish() {
            pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
        }
    }

    fn render_ellipse(pixmap: &mut PixmapMut, ann: &Annotation) {
        let rect = Rect::from_points(ann.start_point, ann.end_point);
        let cx = (rect.min_x() + rect.width() / 2.0) as f32;
        let cy = (rect.min_y() + rect.height() / 2.0) as f32;
        let rx = (rect.width() / 2.0) as f32;
        let ry = (rect.height() / 2.0) as f32;

        if rx <= 0.0 || ry <= 0.0 {
            return;
        }

        let mut pb = PathBuilder::new();
        // Approximate ellipse via cubic beziers
        let kappa = 0.55228475f32;
        let ox = rx * kappa;
        let oy = ry * kappa;

        pb.move_to(cx - rx, cy);
        pb.cubic_to(cx - rx, cy - oy, cx - ox, cy - ry, cx, cy - ry);
        pb.cubic_to(cx + ox, cy - ry, cx + rx, cy - oy, cx + rx, cy);
        pb.cubic_to(cx + rx, cy + oy, cx + ox, cy + ry, cx, cy + ry);
        pb.cubic_to(cx - ox, cy + ry, cx - rx, cy + oy, cx - rx, cy);
        pb.close();

        if let Some(path) = pb.finish() {
            let mut paint = Paint::default();
            paint.set_color(Self::color_from_rgba(ann.color));
            paint.anti_alias = true;
            let stroke = Self::stroke_for_style(ann);
            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }

    fn render_marker(pixmap: &mut PixmapMut, ann: &Annotation) {
        let pts = match &ann.points {
            Some(p) if p.len() >= 2 => p,
            _ => return,
        };

        let mut pb = PathBuilder::new();
        pb.move_to(pts[0].x as f32, pts[0].y as f32);
        for p in pts.iter().skip(1) {
            pb.line_to(p.x as f32, p.y as f32);
        }

        if let Some(path) = pb.finish() {
            let mut paint = Paint::default();
            // Highlighter semi-transparent opacity (default ~0.4)
            let mut c = ann.color;
            c[3] = (c[3] as f32 * 0.4).round() as u8;
            paint.set_color(Self::color_from_rgba(c));
            paint.anti_alias = true;

            let mut stroke = Stroke::default();
            stroke.width = (ann.stroke_width as f32).max(18.0);
            stroke.line_cap = LineCap::Round;
            stroke.line_join = LineJoin::Round;

            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }

    fn render_number_badge(pixmap: &mut PixmapMut, ann: &Annotation) {
        let center = ann.start_point;
        let radius = ((ann.stroke_width as f32) * 4.0).clamp(12.0, 24.0);

        let mut pb = PathBuilder::new();
        pb.push_circle(center.x as f32, center.y as f32, radius);

        if let Some(path) = pb.finish() {
            // Fill
            let mut fill_paint = Paint::default();
            fill_paint.set_color(Self::color_from_rgba(ann.color));
            fill_paint.anti_alias = true;
            pixmap.fill_path(&path, &fill_paint, FillRule::Winding, Transform::identity(), None);

            // Border
            let mut stroke_paint = Paint::default();
            stroke_paint.set_color_rgba8(255, 255, 255, 255);
            stroke_paint.anti_alias = true;
            let mut stroke = Stroke::default();
            stroke.width = 2.0;
            pixmap.stroke_path(&path, &stroke_paint, &stroke, Transform::identity(), None);
        }
    }

    fn render_measure(pixmap: &mut PixmapMut, ann: &Annotation) {
        let p0 = ann.start_point;
        let p1 = ann.end_point;
        let dx = (p1.x - p0.x) as f32;
        let dy = (p1.y - p0.y) as f32;
        let _dist = (dx * dx + dy * dy).sqrt();
        let angle = dy.atan2(dx);
        let perp = angle + std::f32::consts::FRAC_PI_2;

        let tick_len = 8.0f32;
        let mut pb = PathBuilder::new();

        // Main line
        pb.move_to(p0.x as f32, p0.y as f32);
        pb.line_to(p1.x as f32, p1.y as f32);

        // Start tick
        pb.move_to(
            (p0.x as f32) - tick_len * perp.cos(),
            (p0.y as f32) - tick_len * perp.sin(),
        );
        pb.line_to(
            (p0.x as f32) + tick_len * perp.cos(),
            (p0.y as f32) + tick_len * perp.sin(),
        );

        // End tick
        pb.move_to(
            (p1.x as f32) - tick_len * perp.cos(),
            (p1.y as f32) - tick_len * perp.sin(),
        );
        pb.line_to(
            (p1.x as f32) + tick_len * perp.cos(),
            (p1.y as f32) + tick_len * perp.sin(),
        );

        if let Some(path) = pb.finish() {
            let mut paint = Paint::default();
            paint.set_color(Self::color_from_rgba(ann.color));
            paint.anti_alias = true;
            let mut stroke = Stroke::default();
            stroke.width = 1.5;
            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }

        // Center badge rectangle
        let mid_x = ((p0.x + p1.x) / 2.0) as f32;
        let mid_y = ((p0.y + p1.y) / 2.0) as f32;
        let badge_w = 48.0f32;
        let badge_h = 20.0f32;

        let mut badge_pb = PathBuilder::new();
        if let Some(r) = SkRect::from_xywh(mid_x - badge_w / 2.0, mid_y - badge_h / 2.0, badge_w, badge_h) {
            badge_pb.push_rect(r);
        }
        if let Some(badge_path) = badge_pb.finish() {
            let mut bg_paint = Paint::default();
            bg_paint.set_color_rgba8(20, 20, 24, 220);
            bg_paint.anti_alias = true;
            pixmap.fill_path(&badge_path, &bg_paint, FillRule::Winding, Transform::identity(), None);
        }
    }

    fn render_pixelate(pixmap: &mut PixmapMut, ann: &Annotation) {
        let rect = Rect::from_points(ann.start_point, ann.end_point);
        let x0 = (rect.min_x().max(0.0).round() as usize).min(pixmap.width() as usize);
        let y0 = (rect.min_y().max(0.0).round() as usize).min(pixmap.height() as usize);
        let x1 = (rect.max_x().max(0.0).round() as usize).min(pixmap.width() as usize);
        let y1 = (rect.max_y().max(0.0).round() as usize).min(pixmap.height() as usize);

        if x1 <= x0 || y1 <= y0 {
            return;
        }

        let block_size = 12usize;
        let width = pixmap.width() as usize;
        let data = pixmap.data_mut();

        for by in (y0..y1).step_by(block_size) {
            let bh = (by + block_size).min(y1) - by;
            for bx in (x0..x1).step_by(block_size) {
                let bw = (bx + block_size).min(x1) - bx;
                let mut sum_r = 0u32;
                let mut sum_g = 0u32;
                let mut sum_b = 0u32;
                let count = (bw * bh) as u32;

                for py in by..by + bh {
                    let row_idx = py * width * 4;
                    for px in bx..bx + bw {
                        let idx = row_idx + px * 4;
                        sum_r += data[idx] as u32;
                        sum_g += data[idx + 1] as u32;
                        sum_b += data[idx + 2] as u32;
                    }
                }

                let avg_r = (sum_r / count) as u8;
                let avg_g = (sum_g / count) as u8;
                let avg_b = (sum_b / count) as u8;

                for py in by..by + bh {
                    let row_idx = py * width * 4;
                    for px in bx..bx + bw {
                        let idx = row_idx + px * 4;
                        data[idx] = avg_r;
                        data[idx + 1] = avg_g;
                        data[idx + 2] = avg_b;
                    }
                }
            }
        }
    }

    fn render_blur(pixmap: &mut PixmapMut, ann: &Annotation) {
        let rect = Rect::from_points(ann.start_point, ann.end_point);
        let x0 = (rect.min_x().max(0.0).round() as usize).min(pixmap.width() as usize);
        let y0 = (rect.min_y().max(0.0).round() as usize).min(pixmap.height() as usize);
        let x1 = (rect.max_x().max(0.0).round() as usize).min(pixmap.width() as usize);
        let y1 = (rect.max_y().max(0.0).round() as usize).min(pixmap.height() as usize);

        if x1 <= x0 || y1 <= y0 {
            return;
        }

        let radius = (ann.blur_radius.unwrap_or(8.0).round() as usize).max(2).min(32);
        let full_w = pixmap.width() as usize;
        let data = pixmap.data_mut();

        // 3-pass box blur on RGB components
        for _ in 0..3 {
            // Horizontal pass
            for y in y0..y1 {
                let row_offset = y * full_w * 4;
                for x in x0..x1 {
                    let lo = x.saturating_sub(radius).max(x0);
                    let hi = (x + radius).min(x1 - 1);
                    let span = (hi - lo + 1) as u32;
                    let mut r = 0u32;
                    let mut g = 0u32;
                    let mut b = 0u32;
                    for k in lo..=hi {
                        let idx = row_offset + k * 4;
                        r += data[idx] as u32;
                        g += data[idx + 1] as u32;
                        b += data[idx + 2] as u32;
                    }
                    let idx = row_offset + x * 4;
                    data[idx] = (r / span) as u8;
                    data[idx + 1] = (g / span) as u8;
                    data[idx + 2] = (b / span) as u8;
                }
            }
        }
    }

    fn render_highlight(pixmap: &mut PixmapMut, ann: &Annotation) {
        let sel = Rect::from_points(ann.start_point, ann.end_point);
        let full_w = pixmap.width() as f32;
        let full_h = pixmap.height() as f32;

        let mut pb = PathBuilder::new();
        // Outer full rect
        if let Some(r) = SkRect::from_xywh(0.0, 0.0, full_w, full_h) {
            pb.push_rect(r);
        }
        // Cutout inner highlighted rect
        if let Some(r) = SkRect::from_xywh(
            sel.min_x() as f32,
            sel.min_y() as f32,
            sel.width() as f32,
            sel.height() as f32,
        ) {
            pb.push_rect(r);
        }

        if let Some(path) = pb.finish() {
            let mut paint = Paint::default();
            paint.set_color_rgba8(0, 0, 0, 140); // Dimming overlay
            paint.anti_alias = true;
            pixmap.fill_path(&path, &paint, FillRule::EvenOdd, Transform::identity(), None);
        }
    }

    fn render_text_fallback(pixmap: &mut PixmapMut, ann: &Annotation) {
        let rect = Rect::from_points(ann.start_point, ann.end_point);
        let mut pb = PathBuilder::new();
        if let Some(r) = SkRect::from_xywh(
            rect.min_x() as f32,
            rect.min_y() as f32,
            rect.width().max(60.0) as f32,
            rect.height().max(28.0) as f32,
        ) {
            pb.push_rect(r);
        }
        if let Some(path) = pb.finish() {
            let mut paint = Paint::default();
            let mut bg = ann.color;
            bg[3] = 40;
            paint.set_color(Self::color_from_rgba(bg));
            paint.anti_alias = true;
            pixmap.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);

            let stroke = Self::stroke_for_style(ann);
            let mut border_paint = Paint::default();
            border_paint.set_color(Self::color_from_rgba(ann.color));
            border_paint.anti_alias = true;
            pixmap.stroke_path(&path, &border_paint, &stroke, Transform::identity(), None);
        }
    }
}
