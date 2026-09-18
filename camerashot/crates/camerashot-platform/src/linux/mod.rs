pub mod wayland;
pub mod x11;

pub use wayland::WaylandCaptureBackend;
pub use x11::X11CaptureBackend;

use crate::traits::{CaptureBackend, FrameBuffer, PlatformDisplay, PlatformError};
use camerashot_core::geometry::Rect;

pub enum LinuxDisplayServer {
    Wayland,
    X11,
}

impl LinuxDisplayServer {
    pub fn detect() -> Self {
        if std::env::var_os("WAYLAND_DISPLAY").is_some() {
            Self::Wayland
        } else {
            Self::X11
        }
    }
}

pub struct LinuxCaptureBackend {
    inner: Box<dyn CaptureBackend>,
}

impl LinuxCaptureBackend {
    pub fn new() -> Result<Self, PlatformError> {
        let server = LinuxDisplayServer::detect();
        let inner: Box<dyn CaptureBackend> = match server {
            LinuxDisplayServer::Wayland => Box::new(WaylandCaptureBackend::new()?),
            LinuxDisplayServer::X11 => Box::new(X11CaptureBackend::new()?),
        };
        Ok(Self { inner })
    }
}

impl CaptureBackend for LinuxCaptureBackend {
    fn enumerate_displays(&self) -> Result<Vec<PlatformDisplay>, PlatformError> {
        self.inner.enumerate_displays()
    }

    fn capture_display(&self, display_id: u32) -> Result<FrameBuffer, PlatformError> {
        self.inner.capture_display(display_id)
    }

    fn capture_rect(&self, rect: Rect) -> Result<FrameBuffer, PlatformError> {
        self.inner.capture_rect(rect)
    }
}
