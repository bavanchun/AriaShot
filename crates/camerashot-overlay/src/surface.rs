use crate::hud::{FloatingToolbar, Loupe, ResolutionBox};
use crate::state_machine::{OverlayState, SelectionController};
use camerashot_core::annotation::{Annotation, AnnotationTool};
use camerashot_core::beautify::{BeautifyConfig, BeautifyRenderer};
use camerashot_core::geometry::{Point, Rect};
use camerashot_core::renderer::AnnotationRenderer;
use camerashot_core::snap_index::BoundarySnapIndex;
use camerashot_core::undo::UndoStack;
use tiny_skia::{
    FillRule, Paint, PathBuilder, Pixmap, PixmapPaint, Stroke, StrokeDash, Transform,
};

pub struct OverlaySurface {
    pub width: u32,
    pub height: u32,
    pub base_image: Pixmap,
    pub snap_index: Option<BoundarySnapIndex>,
    pub controller: SelectionController,
    pub annotations: Vec<Annotation>,
    pub undo_stack: UndoStack,
    pub active_tool: AnnotationTool,
    pub beautify_config: Option<BeautifyConfig>,
}

impl OverlaySurface {
    pub fn new(width: u32, height: u32, rgba_pixels: &[u8]) -> Option<Self> {
        let mut base_image = Pixmap::new(width, height)?;
        base_image.data_mut().copy_from_slice(rgba_pixels);

        let draw_rect = Rect::new(0.0, 0.0, width as f64, height as f64);
        let snap_index = BoundarySnapIndex::build(width as usize, height as usize, rgba_pixels, draw_rect);

        Some(Self {
            width,
            height,
            base_image,
            snap_index,
            controller: SelectionController::new(),
            annotations: Vec::new(),
            undo_stack: UndoStack::default(),
            active_tool: AnnotationTool::Pencil,
            beautify_config: None,
        })
    }

    pub fn on_mouse_move(&mut self, pt: Point) {
        self.controller.on_mouse_move(pt, self.snap_index.as_ref());
    }

    pub fn on_mouse_down(&mut self, pt: Point) {
        self.controller.on_mouse_down(pt);
    }

    pub fn on_mouse_up(&mut self) {
        self.controller.on_mouse_up();
    }

    /// Render the complete composited frame into target_pixmap.
    pub fn render_frame(&self, target: &mut Pixmap) {
        // 1. Blit base screenshot
        target.draw_pixmap(
            0,
            0,
            self.base_image.as_ref(),
            &PixmapPaint::default(),
            Transform::identity(),
            None,
        );

        let selection_opt = self.controller.current_selection_rect();

        // 2. Dim outside selection if selecting or selected
        if let Some(sel) = selection_opt {
            let mut dim_pb = PathBuilder::new();
            if let Some(r) = tiny_skia::Rect::from_xywh(0.0, 0.0, self.width as f32, self.height as f32) {
                dim_pb.push_rect(r);
            }
            if let Some(r) = tiny_skia::Rect::from_xywh(
                sel.min_x() as f32,
                sel.min_y() as f32,
                sel.width() as f32,
                sel.height() as f32,
            ) {
                dim_pb.push_rect(r);
            }
            if let Some(path) = dim_pb.finish() {
                let mut dim_paint = Paint::default();
                dim_paint.set_color_rgba8(0, 0, 0, 110);
                target.fill_path(&path, &dim_paint, FillRule::EvenOdd, Transform::identity(), None);
            }

            // Selection outline border
            let mut border_pb = PathBuilder::new();
            if let Some(r) = tiny_skia::Rect::from_xywh(
                sel.min_x() as f32,
                sel.min_y() as f32,
                sel.width() as f32,
                sel.height() as f32,
            ) {
                border_pb.push_rect(r);
            }
            if let Some(border_path) = border_pb.finish() {
                let mut border_paint = Paint::default();
                border_paint.set_color_rgba8(0, 140, 255, 255);
                border_paint.anti_alias = true;
                let mut stroke = Stroke::default();
                stroke.width = 1.5;
                if let Some(dash) = StrokeDash::new(vec![6.0, 4.0], 0.0) {
                    stroke.dash = Some(dash);
                }
                target.stroke_path(&border_path, &border_paint, &stroke, Transform::identity(), None);
            }

            // Resolution Box HUD
            ResolutionBox::render(&mut target.as_mut(), sel);

            // Floating Toolbar if Selected
            if matches!(self.controller.state, OverlayState::Selected { .. }) {
                FloatingToolbar::render(&mut target.as_mut(), sel, self.active_tool);
            }
        } else if let OverlayState::Idle { cursor } = self.controller.state {
            // In Idle, render Loupe
            Loupe::render(&mut target.as_mut(), self.base_image.as_ref(), cursor);
        }

        // 3. Render annotations
        AnnotationRenderer::render_annotations(&mut target.as_mut(), &self.annotations);
    }

    /// Crop selected region into a new Pixmap.
    pub fn export_selection_pixmap(&self) -> Option<Pixmap> {
        let sel = self.controller.current_selection_rect()?;
        let x = (sel.min_x().round() as u32).min(self.width - 1);
        let y = (sel.min_y().round() as u32).min(self.height - 1);
        let w = (sel.width().round() as u32).min(self.width - x);
        let h = (sel.height().round() as u32).min(self.height - y);

        if w == 0 || h == 0 {
            return None;
        }

        let mut composited = Pixmap::new(self.width, self.height)?;
        self.render_frame(&mut composited);

        let mut cropped = Pixmap::new(w, h)?;
        cropped.draw_pixmap(
            -(x as i32),
            -(y as i32),
            composited.as_ref(),
            &PixmapPaint::default(),
            Transform::identity(),
            None,
        );

        if let Some(ref config) = self.beautify_config {
            BeautifyRenderer::render(&cropped, config)
        } else {
            Some(cropped)
        }
    }
}
