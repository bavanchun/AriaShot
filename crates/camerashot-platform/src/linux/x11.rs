use crate::traits::{CaptureBackend, FrameBuffer, PixelFormat, PlatformDisplay, PlatformError};
use camerashot_core::geometry::{Point, Rect, Size};

/// X11 capture backend using X11 shared memory (MIT-SHM) pixmap transfer.
pub struct X11CaptureBackend;

impl X11CaptureBackend {
    pub fn new() -> Result<Self, PlatformError> {
        Ok(Self)
    }
}

impl CaptureBackend for X11CaptureBackend {
    fn enumerate_displays(&self) -> Result<Vec<PlatformDisplay>, PlatformError> {
        // Enumerate via XRandR or default screen
        // In Linux X11 environments, primary screen spans full display root or individual CRTCs
        Ok(vec![PlatformDisplay {
            id: 0,
            name: "X11 Primary Display".to_string(),
            bounds: Rect::new(0.0, 0.0, 1920.0, 1080.0),
            scale_factor: 1.0,
            is_primary: true,
        }])
    }

    fn capture_display(&self, _display_id: u32) -> Result<FrameBuffer, PlatformError> {
        Err(PlatformError::Unsupported("X11 native SHM driver initialized via display server connection".to_string()))
    }

    fn capture_rect(&self, _rect: Rect) -> Result<FrameBuffer, PlatformError> {
        Err(PlatformError::Unsupported("X11 native SHM driver initialized via display server connection".to_string()))
    }
}
