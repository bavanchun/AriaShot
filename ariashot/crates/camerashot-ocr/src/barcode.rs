use camerashot_core::geometry::Rect;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BarcodeType {
    QrCode,
    DataMatrix,
    Aztec,
    Code128,
    Ean13,
    UpcA,
    Unknown,
}

impl BarcodeType {
    pub fn name(&self) -> &'static str {
        match self {
            Self::QrCode => "QR Code",
            Self::DataMatrix => "Data Matrix",
            Self::Aztec => "Aztec",
            Self::Code128 => "Code 128",
            Self::Ean13 => "EAN-13",
            Self::UpcA => "UPC-A",
            Self::Unknown => "Barcode",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarcodeResult {
    pub payload: String,
    pub barcode_type: BarcodeType,
    pub bounds: Rect,
}

pub struct BarcodeDetector;

impl BarcodeDetector {
    /// Detect barcode symbols from image pixel buffer.
    pub fn detect_from_rgba(
        _rgba: &[u8],
        _width: u32,
        _height: u32,
    ) -> Result<Vec<BarcodeResult>, String> {
        // High-level integration hook for zxing-cpp / Apple Vision
        Ok(Vec::new())
    }
}
