pub mod clipboard;
pub mod input;
pub mod traits;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "linux")]
pub mod linux;

pub use input::SyntheticScroll;
pub use traits::{CaptureBackend, FrameBuffer, PixelFormat, PlatformDisplay, PlatformError};

#[cfg(target_os = "macos")]
pub use macos::MacOSCaptureBackend as DefaultCaptureBackend;

#[cfg(target_os = "linux")]
pub use linux::LinuxCaptureBackend as DefaultCaptureBackend;

/// Create the native capture backend appropriate for the running operating system and display server.
pub fn create_default_backend() -> Result<Box<dyn CaptureBackend>, PlatformError> {
    #[cfg(target_os = "macos")]
    {
        Ok(Box::new(macos::MacOSCaptureBackend::new()))
    }
    #[cfg(target_os = "linux")]
    {
        Ok(Box::new(linux::LinuxCaptureBackend::new()?))
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Err(PlatformError::Unsupported(
            "Camerashot currently supports macOS and Linux (Wayland/X11)".to_string(),
        ))
    }
}
