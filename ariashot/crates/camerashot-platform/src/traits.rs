use camerashot_core::geometry::Rect;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PlatformError {
    #[error("Permission denied for screen recording/capture")]
    PermissionDenied,

    #[error("Display with id {0} not found")]
    DisplayNotFound(u32),

    #[error("Capture failed: {0}")]
    CaptureFailed(String),

    #[error("Unsupported platform or display server: {0}")]
    Unsupported(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Raw pixel formats returned by native grabbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PixelFormat {
    Rgba8,
    Bgra8,
    Rgb8,
    Bgr8,
}

/// In-memory frame buffer holding screen capture raw pixel data.
#[derive(Debug, Clone)]
pub struct FrameBuffer {
    pub width: usize,
    pub height: usize,
    pub stride: usize,
    pub format: PixelFormat,
    pub data: Vec<u8>,
}

impl FrameBuffer {
    pub fn new(
        width: usize,
        height: usize,
        stride: usize,
        format: PixelFormat,
        data: Vec<u8>,
    ) -> Self {
        Self {
            width,
            height,
            stride,
            format,
            data,
        }
    }

    /// Convert to tightly-packed RGBA8 buffer for Skia / Slint / BoundarySnapIndex rendering.
    pub fn to_rgba8(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.width * self.height * 4);
        let bytes_per_pixel = match self.format {
            PixelFormat::Rgba8 | PixelFormat::Bgra8 => 4,
            PixelFormat::Rgb8 | PixelFormat::Bgr8 => 3,
        };

        for y in 0..self.height {
            let row_start = y * self.stride;
            for x in 0..self.width {
                let px_start = row_start + x * bytes_per_pixel;
                match self.format {
                    PixelFormat::Rgba8 => {
                        out.extend_from_slice(&self.data[px_start..px_start + 4]);
                    }
                    PixelFormat::Bgra8 => {
                        let b = self.data[px_start];
                        let g = self.data[px_start + 1];
                        let r = self.data[px_start + 2];
                        let a = self.data[px_start + 3];
                        out.extend_from_slice(&[r, g, b, a]);
                    }
                    PixelFormat::Rgb8 => {
                        let r = self.data[px_start];
                        let g = self.data[px_start + 1];
                        let b = self.data[px_start + 2];
                        out.extend_from_slice(&[r, g, b, 255]);
                    }
                    PixelFormat::Bgr8 => {
                        let b = self.data[px_start];
                        let g = self.data[px_start + 1];
                        let r = self.data[px_start + 2];
                        out.extend_from_slice(&[r, g, b, 255]);
                    }
                }
            }
        }
        out
    }
}

/// Metadata describing a physical or virtual display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformDisplay {
    pub id: u32,
    pub name: String,
    pub bounds: Rect,
    pub scale_factor: f64,
    pub is_primary: bool,
}

/// Abstract screen grabber trait implemented per platform.
pub trait CaptureBackend: Send + Sync {
    /// Enumerate all active displays currently connected.
    fn enumerate_displays(&self) -> Result<Vec<PlatformDisplay>, PlatformError>;

    /// Capture a single display by ID.
    fn capture_display(&self, display_id: u32) -> Result<FrameBuffer, PlatformError>;

    /// Capture all displays simultaneously.
    fn capture_all_displays(&self) -> Result<Vec<(PlatformDisplay, FrameBuffer)>, PlatformError> {
        let displays = self.enumerate_displays()?;
        let mut results = Vec::with_capacity(displays.len());
        for d in displays {
            let fb = self.capture_display(d.id)?;
            results.push((d, fb));
        }
        Ok(results)
    }

    /// Capture an arbitrary rectangle across screens.
    fn capture_rect(&self, rect: Rect) -> Result<FrameBuffer, PlatformError>;
}
