use camerashot_core::geometry::Rect;
use camerashot_ocr::{blocks_to_text, mask_pii_in_text, OcrBlock, PiiDetector};

// --- Pure Rust tests (all OS) ---

#[test]
fn test_blocks_to_text_sorts_by_y_then_x() {
    let blocks = vec![
        OcrBlock {
            text: "bottom-right".to_string(),
            confidence: 0.9,
            bounds: Rect::new(200.0, 100.0, 100.0, 20.0),
        },
        OcrBlock {
            text: "top-left".to_string(),
            confidence: 0.9,
            bounds: Rect::new(10.0, 10.0, 100.0, 20.0),
        },
        OcrBlock {
            text: "top-right".to_string(),
            confidence: 0.9,
            bounds: Rect::new(200.0, 10.0, 100.0, 20.0),
        },
        OcrBlock {
            text: "bottom-left".to_string(),
            confidence: 0.9,
            bounds: Rect::new(10.0, 100.0, 100.0, 20.0),
        },
    ];

    let text = blocks_to_text(&blocks);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 4);
    assert_eq!(lines[0], "top-left");
    assert_eq!(lines[1], "top-right");
    assert_eq!(lines[2], "bottom-left");
    assert_eq!(lines[3], "bottom-right");
}

#[test]
fn test_blocks_to_text_empty() {
    let text = blocks_to_text(&[]);
    assert!(text.is_empty());
}

#[test]
fn test_mask_pii_in_text_email() {
    let detector = PiiDetector::new();
    let masked = mask_pii_in_text(&detector, "contact: jane.doe@example.com here");
    assert!(!masked.contains("jane.doe@example.com"));
    assert!(masked.contains('█'));
    assert!(masked.starts_with("contact: "));
    assert!(masked.ends_with(" here"));
}

#[test]
fn test_mask_pii_in_text_credit_card() {
    let detector = PiiDetector::new();
    let masked = mask_pii_in_text(&detector, "card: 4111 1111 1111 1111 end");
    assert!(!masked.contains("4111 1111 1111 1111"));
    assert!(masked.contains('█'));
}

#[test]
fn test_mask_pii_in_text_no_pii() {
    let detector = PiiDetector::new();
    let input = "Hello world, no sensitive data here.";
    let masked = mask_pii_in_text(&detector, input);
    assert_eq!(masked, input);
}

// --- macOS-only Vision tests ---

#[cfg(target_os = "macos")]
mod vision_tests {
    use camerashot_core::annotation::AnnotationTool;
    use camerashot_core::geometry::Rect;
    use camerashot_ocr::{detect_redactions, OcrEngine};

    fn load_fixture() -> (Vec<u8>, u32, u32) {
        let fixture_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/pii-sample.png");
        let img = image::open(&fixture_path)
            .unwrap_or_else(|e| panic!("Failed to open fixture {:?}: {e}", fixture_path));
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        (rgba.into_raw(), w, h)
    }

    #[test]
    fn test_vision_recognizes_email_in_fixture() {
        let (rgba, w, h) = load_fixture();
        let engine = OcrEngine::new();
        let blocks = engine
            .recognize_text(&rgba, w, h)
            .expect("Vision OCR should not fail on fixture");

        // Fixture contains "jane.doe@example.com" — Vision should find it
        let all_text: String = blocks.iter().map(|b| b.text.as_str()).collect::<Vec<_>>().join(" ");
        assert!(
            all_text.contains("jane.doe@example.com") || all_text.contains("jane.doe@example"),
            "Vision did not find email in fixture. Got: {all_text}"
        );

        // Bounds should be within the image
        for block in &blocks {
            assert!(
                block.bounds.origin.x >= 0.0,
                "x should be >= 0, got {}",
                block.bounds.origin.x
            );
            assert!(
                block.bounds.origin.y >= 0.0,
                "y should be >= 0, got {}",
                block.bounds.origin.y
            );
            assert!(
                block.bounds.origin.x + block.bounds.size.width <= w as f64 + 1.0,
                "x+w should be <= image width"
            );
            assert!(
                block.bounds.origin.y + block.bounds.size.height <= h as f64 + 1.0,
                "y+h should be <= image height"
            );
        }
    }

    #[test]
    fn test_detect_redactions_produces_annotations() {
        let (rgba, w, h) = load_fixture();
        let engine = OcrEngine::new();
        let (batch_id, annotations) = detect_redactions(
            &engine,
            &rgba,
            w,
            h,
            AnnotationTool::Pixelate,
            2.0,
        )
        .expect("detect_redactions should not fail on fixture");

        // Fixture has PII (email + credit card) — should produce at least 1 annotation
        assert!(
            !annotations.is_empty(),
            "Expected at least 1 redaction annotation, got 0"
        );

        // All annotations should share the same group_id
        for ann in &annotations {
            assert_eq!(ann.group_id, Some(batch_id));
        }

        // Annotation bounds should be within the image (with padding tolerance)
        let img_rect = Rect::new(-10.0, -10.0, w as f64 + 20.0, h as f64 + 20.0);
        for ann in &annotations {
            let ann_rect = Rect::from_points(ann.start_point, ann.end_point);
            assert!(
                img_rect.intersects(&ann_rect),
                "Annotation rect {:?} should intersect image bounds {:?}",
                ann_rect,
                img_rect
            );
        }
    }
}
