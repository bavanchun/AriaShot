use crate::vision_macos::OcrBlock;

pub struct RapidOnnxOcr;

impl RapidOnnxOcr {
    pub fn new() -> Self {
        Self
    }

    /// Perform OCR on image pixel buffer using embedded ONNX Runtime (DBNet text detection + SVTR text recognition).
    pub fn recognize_text(
        &self,
        _rgba_data: &[u8],
        _width: u32,
        _height: u32,
    ) -> Result<Vec<OcrBlock>, String> {
        Err("OCR is not supported on Linux yet; use macOS".into())
    }
}
