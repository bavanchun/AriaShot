use camerashot_core::annotation::AnnotationTool;
use camerashot_core::geometry::Rect;
use tiny_skia::{FillRule, Paint, PathBuilder, PixmapMut, Stroke, Transform};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolbarAction {
    SelectTool(AnnotationTool),
    ToggleBeautify,
    Undo,
    Redo,
    CopyToClipboard,
    SaveToFile,
    OpenInEditor,
    Close,
}

pub struct FloatingToolbar;

impl FloatingToolbar {
    /// Render the floating tool bar below the selection rectangle.
    pub fn render(pixmap: &mut PixmapMut, selection: Rect, _active_tool: AnnotationTool) {
        if selection.width() <= 0.0 || selection.height() <= 0.0 {
            return;
        }

        let bar_w = 420.0f32;
        let bar_h = 36.0f32;

        let bar_x = ((selection.min_x() + selection.width() / 2.0) as f32) - bar_w / 2.0;
        let bar_x = bar_x.clamp(8.0, (pixmap.width() as f32) - bar_w - 8.0);

        let bar_y = if selection.max_y() + (bar_h as f64) + 16.0 <= pixmap.height() as f64 {
            (selection.max_y() as f32) + 8.0
        } else {
            (selection.min_y() as f32) - bar_h - 8.0
        };

        // Dark translucent capsule bar
        let mut pb = PathBuilder::new();
        let r = 8.0f32;
        pb.move_to(bar_x + r, bar_y);
        pb.line_to(bar_x + bar_w - r, bar_y);
        pb.quad_to(bar_x + bar_w, bar_y, bar_x + bar_w, bar_y + r);
        pb.line_to(bar_x + bar_w, bar_y + bar_h - r);
        pb.quad_to(bar_x + bar_w, bar_y + bar_h, bar_x + bar_w - r, bar_y + bar_h);
        pb.line_to(bar_x + r, bar_y + bar_h);
        pb.quad_to(bar_x, bar_y + bar_h, bar_x, bar_y + bar_h - r);
        pb.line_to(bar_x, bar_y + r);
        pb.quad_to(bar_x, bar_y, bar_x + r, bar_y);
        pb.close();

        if let Some(path) = pb.finish() {
            let mut bg_paint = Paint::default();
            bg_paint.set_color_rgba8(28, 28, 32, 235);
            bg_paint.anti_alias = true;
            pixmap.fill_path(&path, &bg_paint, FillRule::Winding, Transform::identity(), None);

            let mut border_paint = Paint::default();
            border_paint.set_color_rgba8(255, 255, 255, 30);
            border_paint.anti_alias = true;
            let mut stroke = Stroke::default();
            stroke.width = 1.0;
            pixmap.stroke_path(&path, &border_paint, &stroke, Transform::identity(), None);
        }
    }
}
