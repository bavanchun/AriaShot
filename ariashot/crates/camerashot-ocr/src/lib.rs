pub mod barcode;
pub mod onnx_linux;
pub mod redactor;
pub mod vision_macos;
pub mod pipeline;

pub use barcode::{BarcodeDetector, BarcodeResult, BarcodeType};
pub use onnx_linux::RapidOnnxOcr;
pub use redactor::{PiiCategory, PiiDetector, PiiMatch, PiiRedactor};
pub use vision_macos::{AppleVisionOcr, OcrBlock};
pub use pipeline::{blocks_to_text, detect_redactions, mask_pii_in_text, redactions_from_blocks};

/// Cross-platform OCR dispatcher that selects AppleVision on macOS and RapidOCR ONNX on Linux.
pub struct OcrEngine {
    #[cfg(target_os = "macos")]
    backend: AppleVisionOcr,
    #[cfg(not(target_os = "macos"))]
    backend: RapidOnnxOcr,
}

impl Default for OcrEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl OcrEngine {
    pub fn new() -> Self {
        #[cfg(target_os = "macos")]
        {
            Self {
                backend: AppleVisionOcr::new(),
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            Self {
                backend: RapidOnnxOcr::new(),
            }
        }
    }

    pub fn recognize_text(
        &self,
        rgba_data: &[u8],
        width: u32,
        height: u32,
    ) -> Result<Vec<OcrBlock>, String> {
        self.backend.recognize_text(rgba_data, width, height)
    }
}
