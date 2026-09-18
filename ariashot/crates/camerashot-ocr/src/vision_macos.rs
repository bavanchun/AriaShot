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

    /// Perform OCR on an RGBA pixel buffer using Apple Vision framework.
    ///
    /// Returns `OcrBlock`s with bounds in **pixel coordinates, top-left origin**.
    /// Vision natively uses normalised coordinates with a bottom-left origin,
    /// so this function flips the Y axis for the caller.
    pub fn recognize_text(
        &self,
        rgba_data: &[u8],
        width: u32,
        height: u32,
    ) -> Result<Vec<OcrBlock>, String> {
        #[cfg(target_os = "macos")]
        {
            recognize_text_vision(rgba_data, width, height)
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (rgba_data, width, height);
            Err("Apple Vision OCR is only available on macOS".into())
        }
    }
}

#[cfg(target_os = "macos")]
fn recognize_text_vision(
    rgba_data: &[u8],
    width: u32,
    height: u32,
) -> Result<Vec<OcrBlock>, String> {
    use image::codecs::png::PngEncoder;
    use image::{ColorType, ImageEncoder};
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2::AnyThread;
    use objc2_foundation::{NSArray, NSData, NSDictionary, NSString};
    use objc2_vision::{
        VNImageRequestHandler, VNRecognizeTextRequest, VNRequestTextRecognitionLevel,
    };

    // 1. Encode RGBA → PNG in memory
    let mut png_buf: Vec<u8> = Vec::new();
    {
        let encoder = PngEncoder::new(&mut png_buf);
        encoder
            .write_image(rgba_data, width, height, ColorType::Rgba8.into())
            .map_err(|e| format!("PNG encode failed: {e}"))?;
    }

    // 2. Run Vision OCR inside an autorelease pool
    objc2::rc::autoreleasepool(|_pool| {
        let ns_data = NSData::with_bytes(&png_buf);

        let empty_dict: Retained<NSDictionary<NSString, AnyObject>> = NSDictionary::new();
        let handler = VNImageRequestHandler::initWithData_options(
            VNImageRequestHandler::alloc(),
            &ns_data,
            &empty_dict,
        );

        // SAFETY: VNRecognizeTextRequest::init is safe to call with a fresh allocation.
        let request = unsafe { VNRecognizeTextRequest::init(VNRecognizeTextRequest::alloc()) };
        request.setRecognitionLevel(VNRequestTextRecognitionLevel::Accurate);
        request.setUsesLanguageCorrection(true);

        // Upcast to VNRequest
        let request_as_vn: Retained<objc2_vision::VNRequest> =
            unsafe { Retained::cast_unchecked(Retained::clone(&request)) };
        let requests = NSArray::from_retained_slice(&[request_as_vn]);

        handler
            .performRequests_error(&requests)
            .map_err(|e| format!("Vision performRequests failed: {e}"))?;

        let Some(observations) = request.results() else {
            return Ok(Vec::new());
        };

        let w = width as f64;
        let h = height as f64;
        let mut blocks = Vec::new();

        for obs in observations.iter() {
            let candidates = obs.topCandidates(1);
            if candidates.len() == 0 {
                continue;
            }
            let candidate = candidates.objectAtIndex(0);

            let text = candidate.string().to_string();
            let confidence = candidate.confidence();

            // boundingBox is normalised coords with bottom-left origin
            let bbox = unsafe { obs.boundingBox() };
            let bx = bbox.origin.x;
            let by = bbox.origin.y;
            let bw = bbox.size.width;
            let bh = bbox.size.height;

            // Convert to pixel coords, top-left origin
            let px_x = bx * w;
            let px_y = (1.0 - by - bh) * h;
            let px_w = bw * w;
            let px_h = bh * h;

            blocks.push(OcrBlock {
                text,
                confidence: confidence as f32,
                bounds: Rect::new(px_x, px_y, px_w, px_h),
            });
        }

        Ok(blocks)
    })
}
