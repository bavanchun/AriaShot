use crate::traits::{CaptureBackend, FrameBuffer, PixelFormat, PlatformDisplay, PlatformError};
use camerashot_core::geometry::Rect;
use core_graphics::display::{
    kCGWindowImageBestResolution, kCGWindowListOptionOnScreenOnly, CGDisplay,
};

pub struct MacOSCaptureBackend;

impl MacOSCaptureBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MacOSCaptureBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl CaptureBackend for MacOSCaptureBackend {
    fn enumerate_displays(&self) -> Result<Vec<PlatformDisplay>, PlatformError> {
        let display_ids = CGDisplay::active_displays()
            .map_err(|e| PlatformError::CaptureFailed(format!("Failed to list active displays: {:?}", e)))?;

        let main_id = CGDisplay::main().id;
        let mut displays = Vec::with_capacity(display_ids.len());

        for id in display_ids {
            let display = CGDisplay::new(id);
            let bounds = display.bounds();
            let is_primary = id == main_id;

            // In CoreGraphics, pixels_wide / bounds.size.width gives scale factor (e.g. 2.0 for Retina)
            let pixels_wide = display.pixels_wide();
            let scale_factor = if bounds.size.width > 0.0 {
                (pixels_wide as f64) / bounds.size.width
            } else {
                1.0
            };

            displays.push(PlatformDisplay {
                id,
                name: format!("Display {}", id),
                bounds: Rect::new(
                    bounds.origin.x,
                    bounds.origin.y,
                    bounds.size.width,
                    bounds.size.height,
                ),
                scale_factor,
                is_primary,
            });
        }

        Ok(displays)
    }

    fn capture_display(&self, display_id: u32) -> Result<FrameBuffer, PlatformError> {
        let display = CGDisplay::new(display_id);
        let image = display
            .image()
            .ok_or_else(|| PlatformError::CaptureFailed(format!("Failed to capture CGDisplay {}", display_id)))?;

        let width = image.width();
        let height = image.height();
        let bytes_per_row = image.bytes_per_row();
        let data = image.data();
        let bytes = data.bytes().to_vec();

        // macOS CGDisplay::image() produces 32-bit BGRA or RGBA
        Ok(FrameBuffer::new(
            width,
            height,
            bytes_per_row,
            PixelFormat::Bgra8,
            bytes,
        ))
    }

    fn capture_rect(&self, rect: Rect) -> Result<FrameBuffer, PlatformError> {
        let cg_rect = core_graphics::geometry::CGRect::new(
            &core_graphics::geometry::CGPoint::new(rect.origin.x, rect.origin.y),
            &core_graphics::geometry::CGSize::new(rect.size.width, rect.size.height),
        );

        let image = CGDisplay::screenshot(
            cg_rect,
            kCGWindowListOptionOnScreenOnly,
            0,
            kCGWindowImageBestResolution,
        )
        .ok_or_else(|| PlatformError::CaptureFailed(format!("Failed to capture rect {:?}", rect)))?;

        let width = image.width();
        let height = image.height();
        let bytes_per_row = image.bytes_per_row();
        let data = image.data();
        let bytes = data.bytes().to_vec();

        Ok(FrameBuffer::new(
            width,
            height,
            bytes_per_row,
            PixelFormat::Bgra8,
            bytes,
        ))
    }
}
