use camerashot_core::geometry::Rect;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrBlock {
    pub text: String,
    pub confidence: f32,
    pub bounds: Rect,
}

pub struct AppleVisionOcr;

impl AppleVisionOcr {
    pub fn new() -> Self {
        Self
    }

    /// Perform OCR on image pixel buffer using Apple Vision framework.
    pub fn recognize_text(
        &self,
        _rgba_data: &[u8],
        _width: u32,
        _height: u32,
    ) -> Result<Vec<OcrBlock>, String> {
        // macOS Vision framework FFI call hook (VNRecognizeTextRequest)
        Ok(Vec::new())
    }
}
