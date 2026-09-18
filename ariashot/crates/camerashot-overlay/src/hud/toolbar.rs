use camerashot_core::annotation::AnnotationTool;
use camerashot_core::geometry::{Point, Rect};
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

/// Computed toolbar layout, shared between `render` and `hit_test`.
pub struct ToolbarLayout {
    pub bar_x: f32,
    pub bar_y: f32,
    pub bar_w: f32,
    pub bar_h: f32,
    pub buttons: [(ToolbarAction, [f32; 4]); 2],
}

const BAR_W: f32 = 96.0;
const BAR_H: f32 = 36.0;
const BTN_SIZE: f32 = 24.0;
const BTN_PAD: f32 = 8.0;
const BAR_RADIUS: f32 = 8.0;

impl ToolbarLayout {
    /// Compute toolbar position and button rects from the selection and canvas size.
    pub fn compute(selection: Rect, canvas_w: u32, canvas_h: u32) -> Self {
        let bar_x = ((selection.min_x() + selection.width() / 2.0) as f32) - BAR_W / 2.0;
        let bar_x = bar_x.clamp(8.0, (canvas_w as f32) - BAR_W - 8.0);

        let bar_y = if selection.max_y() + (BAR_H as f64) + 16.0 <= canvas_h as f64 {
            (selection.max_y() as f32) + 8.0
        } else {
            (selection.min_y() as f32) - BAR_H - 8.0
        };

        // Two buttons: Copy (left) and Close (right), centered vertically
        let btn_y = bar_y + (BAR_H - BTN_SIZE) / 2.0;
        let copy_x = bar_x + BTN_PAD;
        let close_x = bar_x + BAR_W - BTN_PAD - BTN_SIZE;

        ToolbarLayout {
            bar_x,
            bar_y,
            bar_w: BAR_W,
            bar_h: BAR_H,
            buttons: [
                (
                    ToolbarAction::CopyToClipboard,
                    [copy_x, btn_y, BTN_SIZE, BTN_SIZE],
                ),
                (ToolbarAction::Close, [close_x, btn_y, BTN_SIZE, BTN_SIZE]),
            ],
        }
    }
}

pub struct FloatingToolbar;

impl FloatingToolbar {
    /// Render the floating tool bar below the selection rectangle.
    pub fn render(pixmap: &mut PixmapMut, selection: Rect, _active_tool: AnnotationTool) {
        if selection.width() <= 0.0 || selection.height() <= 0.0 {
            return;
        }

        let layout = ToolbarLayout::compute(selection, pixmap.width(), pixmap.height());

        // Dark translucent capsule bar
        let mut pb = PathBuilder::new();
        let r = BAR_RADIUS;
        let bx = layout.bar_x;
        let by = layout.bar_y;
        let bw = layout.bar_w;
        let bh = layout.bar_h;

        pb.move_to(bx + r, by);
        pb.line_to(bx + bw - r, by);
        pb.quad_to(bx + bw, by, bx + bw, by + r);
        pb.line_to(bx + bw, by + bh - r);
        pb.quad_to(bx + bw, by + bh, bx + bw - r, by + bh);
        pb.line_to(bx + r, by + bh);
        pb.quad_to(bx, by + bh, bx, by + bh - r);
        pb.line_to(bx, by + r);
        pb.quad_to(bx, by, bx + r, by);
        pb.close();

        if let Some(path) = pb.finish() {
            let mut bg_paint = Paint::default();
            bg_paint.set_color_rgba8(28, 28, 32, 235);
            bg_paint.anti_alias = true;
            pixmap.fill_path(
                &path,
                &bg_paint,
                FillRule::Winding,
                Transform::identity(),
                None,
            );

            let mut border_paint = Paint::default();
            border_paint.set_color_rgba8(255, 255, 255, 30);
            border_paint.anti_alias = true;
            let stroke = Stroke {
                width: 1.0,
                ..Stroke::default()
            };
            pixmap.stroke_path(&path, &border_paint, &stroke, Transform::identity(), None);
        }

        // Draw button icons
        for &(action, [bx, by, bw, bh]) in &layout.buttons {
            match action {
                ToolbarAction::CopyToClipboard => {
                    draw_copy_icon(pixmap, bx, by, bw, bh);
                }
                ToolbarAction::Close => {
                    draw_close_icon(pixmap, bx, by, bw, bh);
                }
                _ => {}
            }
        }
    }

    /// Hit-test a point against toolbar buttons. Returns the action if a button was clicked.
    pub fn hit_test(
        selection: Rect,
        canvas_w: u32,
        canvas_h: u32,
        pt: Point,
    ) -> Option<ToolbarAction> {
        let layout = ToolbarLayout::compute(selection, canvas_w, canvas_h);
        for &(action, [bx, by, bw, bh]) in &layout.buttons {
            let px = pt.x as f32;
            let py = pt.y as f32;
            if px >= bx && px <= bx + bw && py >= by && py <= by + bh {
                return Some(action);
            }
        }
        None
    }
}

/// Draw a Copy icon: two overlapping rectangles (document stack).
fn draw_copy_icon(pixmap: &mut PixmapMut, bx: f32, by: f32, bw: f32, bh: f32) {
    let mut paint = Paint::default();
    paint.set_color_rgba8(220, 220, 225, 230);
    paint.anti_alias = true;
    let stroke = Stroke {
        width: 1.5,
        ..Stroke::default()
    };

    let inset = 4.0;
    let offset = 3.0;

    // Back rectangle (offset)
    let mut pb1 = PathBuilder::new();
    if let Some(r) = tiny_skia::Rect::from_xywh(
        bx + inset + offset,
        by + inset,
        bw - inset * 2.0 - offset,
        bh - inset * 2.0 - offset,
    ) {
        pb1.push_rect(r);
    }
    if let Some(p) = pb1.finish() {
        pixmap.stroke_path(&p, &paint, &stroke, Transform::identity(), None);
    }

    // Front rectangle
    let mut pb2 = PathBuilder::new();
    if let Some(r) = tiny_skia::Rect::from_xywh(
        bx + inset,
        by + inset + offset,
        bw - inset * 2.0 - offset,
        bh - inset * 2.0 - offset,
    ) {
        pb2.push_rect(r);
    }
    if let Some(p) = pb2.finish() {
        // Fill with bar background so front covers back
        let mut fill_paint = Paint::default();
        fill_paint.set_color_rgba8(28, 28, 32, 235);
        pixmap.fill_path(
            &p,
            &fill_paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
        pixmap.stroke_path(&p, &paint, &stroke, Transform::identity(), None);
    }
}

/// Draw a Close icon: X mark.
fn draw_close_icon(pixmap: &mut PixmapMut, bx: f32, by: f32, bw: f32, bh: f32) {
    let mut paint = Paint::default();
    paint.set_color_rgba8(220, 220, 225, 230);
    paint.anti_alias = true;
    let stroke = Stroke {
        width: 2.0,
        ..Stroke::default()
    };

    let inset = 6.0;
    let mut pb = PathBuilder::new();
    pb.move_to(bx + inset, by + inset);
    pb.line_to(bx + bw - inset, by + bh - inset);
    pb.move_to(bx + bw - inset, by + inset);
    pb.line_to(bx + inset, by + bh - inset);
    if let Some(path) = pb.finish() {
        pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }
}
