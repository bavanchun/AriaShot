//! Integration tests for `camerashot-cli`.
//!
//! Uses a `MockBackend` that returns a fixture image (`pii-sample.png`)
//! from the OCR crate's test fixtures to verify the full CLI pipeline
//! without needing actual screen capture.

use camerashot_cli::{parse_crop, Cli, RedactStyle, EXIT_OK};
use camerashot_core::geometry::Rect;
use camerashot_platform::traits::{CaptureBackend, FrameBuffer, PlatformDisplay, PlatformError, PixelFormat};
use clap::Parser;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Mock capture backend returning the pii-sample fixture
// ---------------------------------------------------------------------------

fn fixture_path() -> PathBuf {
    // Fixture lives in the sibling crate camerashot-ocr
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../camerashot-ocr/tests/fixtures/pii-sample.png")
}

struct MockBackend {
    fb: FrameBuffer,
}

impl MockBackend {
    fn new() -> Self {
        let img = image::open(fixture_path())
            .expect("cannot open pii-sample.png fixture");
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        let raw = rgba.into_raw();
        let stride = w as usize * 4;
        Self {
            fb: FrameBuffer::new(w as usize, h as usize, stride, PixelFormat::Rgba8, raw),
        }
    }
}

impl CaptureBackend for MockBackend {
    fn enumerate_displays(&self) -> Result<Vec<PlatformDisplay>, PlatformError> {
        Ok(vec![PlatformDisplay {
            id: 1,
            name: "Mock Display".to_string(),
            bounds: Rect::new(0.0, 0.0, self.fb.width as f64, self.fb.height as f64),
            scale_factor: 1.0,
            is_primary: true,
        }])
    }

    fn capture_display(&self, _display_id: u32) -> Result<FrameBuffer, PlatformError> {
        Ok(self.fb.clone())
    }

    fn capture_rect(&self, _rect: Rect) -> Result<FrameBuffer, PlatformError> {
        Ok(self.fb.clone())
    }
}

// ---------------------------------------------------------------------------
// Parse tests
// ---------------------------------------------------------------------------

#[test]
fn parse_crop_valid() {
    let r = parse_crop("10,20,300,200").unwrap();
    assert!((r.origin.x - 10.0).abs() < f64::EPSILON);
    assert!((r.origin.y - 20.0).abs() < f64::EPSILON);
    assert!((r.size.width - 300.0).abs() < f64::EPSILON);
    assert!((r.size.height - 200.0).abs() < f64::EPSILON);
}

#[test]
fn parse_crop_valid_float() {
    let r = parse_crop("1.5,2.5,100.0,50.0").unwrap();
    assert!((r.origin.x - 1.5).abs() < f64::EPSILON);
    assert!((r.origin.y - 2.5).abs() < f64::EPSILON);
}

#[test]
fn parse_crop_zero_width() {
    assert!(parse_crop("1,2,0,5").is_err());
}

#[test]
fn parse_crop_zero_height() {
    assert!(parse_crop("1,2,5,0").is_err());
}

#[test]
fn parse_crop_negative_width() {
    assert!(parse_crop("1,2,-3,5").is_err());
}

#[test]
fn parse_crop_non_numeric() {
    assert!(parse_crop("a,b,c,d").is_err());
}

#[test]
fn parse_crop_wrong_count() {
    assert!(parse_crop("1,2,3").is_err());
    assert!(parse_crop("1,2,3,4,5").is_err());
}

// ---------------------------------------------------------------------------
// Clap argument validation tests
// ---------------------------------------------------------------------------

#[test]
fn clap_missing_target_fails() {
    let result = Cli::try_parse_from(["ariashot"]);
    assert!(result.is_err(), "should fail without --full or --crop");
}

#[test]
fn clap_full_and_crop_conflict() {
    let result = Cli::try_parse_from(["ariashot", "--full", "--crop", "10,20,300,200"]);
    assert!(result.is_err(), "should fail when both --full and --crop");
}

#[test]
fn clap_display_requires_full() {
    let result = Cli::try_parse_from(["ariashot", "--crop", "10,20,300,200", "--display", "2"]);
    assert!(result.is_err(), "--display should require --full");
}

#[test]
fn clap_full_parses_ok() {
    let cli = Cli::try_parse_from(["ariashot", "--full"]).unwrap();
    assert!(cli.full);
    assert!(cli.crop.is_none());
}

#[test]
fn clap_crop_parses_rect() {
    let cli = Cli::try_parse_from(["ariashot", "--crop", "10,20,300,200"]).unwrap();
    assert!(!cli.full);
    let rect = cli.crop.unwrap();
    assert!((rect.origin.x - 10.0).abs() < f64::EPSILON);
    assert!((rect.size.height - 200.0).abs() < f64::EPSILON);
}

#[test]
fn clap_crop_invalid_value_fails() {
    let result = Cli::try_parse_from(["ariashot", "--crop", "1,2,0,5"]);
    assert!(result.is_err(), "W=0 should fail parse_crop");
}

// ---------------------------------------------------------------------------
// run() integration tests with MockBackend
// ---------------------------------------------------------------------------

#[test]
fn run_full_output_file() {
    let tmp = tempfile::tempdir().unwrap();
    let out_path = tmp.path().join("capture.png");

    let cli = Cli::try_parse_from([
        "ariashot",
        "--full",
        "-o",
        out_path.to_str().unwrap(),
    ])
    .unwrap();

    let backend = MockBackend::new();
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    let code = camerashot_cli::run(&cli, &backend, &mut stdout, &mut stderr);

    assert_eq!(code, EXIT_OK, "exit code should be 0");
    assert!(out_path.exists(), "output file should exist");
    assert!(out_path.metadata().unwrap().len() > 0, "output file should not be empty");
    assert!(stdout.is_empty(), "stdout should be empty (no --ocr)");

    let err_str = String::from_utf8_lossy(&stderr);
    assert!(err_str.contains("saved:"), "stderr should mention saved path");
}

#[test]
fn run_full_creates_parent_dirs() {
    let tmp = tempfile::tempdir().unwrap();
    let out_path = tmp.path().join("deep/nested/dir/capture.png");

    let cli = Cli::try_parse_from([
        "ariashot",
        "--full",
        "-o",
        out_path.to_str().unwrap(),
    ])
    .unwrap();

    let backend = MockBackend::new();
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    let code = camerashot_cli::run(&cli, &backend, &mut stdout, &mut stderr);

    assert_eq!(code, EXIT_OK);
    assert!(out_path.exists(), "output file should exist in nested dir");
}

#[test]
fn run_full_output_dimensions_match_fixture() {
    let tmp = tempfile::tempdir().unwrap();
    let out_path = tmp.path().join("out.png");

    let cli = Cli::try_parse_from([
        "ariashot",
        "--full",
        "-o",
        out_path.to_str().unwrap(),
    ])
    .unwrap();

    let backend = MockBackend::new();
    let expected_w = backend.fb.width as u32;
    let expected_h = backend.fb.height as u32;

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    let code = camerashot_cli::run(&cli, &backend, &mut stdout, &mut stderr);
    assert_eq!(code, EXIT_OK);

    let img = image::open(&out_path).expect("should open output PNG");
    let (w, h) = (img.width(), img.height());
    assert_eq!(w, expected_w, "output width should match fixture");
    assert_eq!(h, expected_h, "output height should match fixture");
}

// ---------------------------------------------------------------------------
// macOS-only OCR tests (Vision framework required)
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
mod macos_ocr_tests {
    use super::*;

    #[test]
    fn run_full_ocr_outputs_email() {
        let cli = Cli::try_parse_from(["ariashot", "--full", "--ocr"]).unwrap();

        let backend = MockBackend::new();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let code = camerashot_cli::run(&cli, &backend, &mut stdout, &mut stderr);

        assert_eq!(code, EXIT_OK, "OCR should succeed on macOS");
        let text = String::from_utf8_lossy(&stdout);
        assert!(
            text.contains("jane.doe@example.com") || text.contains("jane.doe@example"),
            "stdout should contain the email from fixture. Got: {text}"
        );
    }

    #[test]
    fn run_full_ocr_redact_masks_pii_in_text() {
        let tmp = tempfile::tempdir().unwrap();
        let out_path = tmp.path().join("redacted.png");

        let cli = Cli::try_parse_from([
            "ariashot",
            "--full",
            "--ocr",
            "--redact",
            "-o",
            out_path.to_str().unwrap(),
        ])
        .unwrap();

        let backend = MockBackend::new();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let code = camerashot_cli::run(&cli, &backend, &mut stdout, &mut stderr);

        assert_eq!(code, EXIT_OK, "OCR+redact should succeed on macOS");

        let text = String::from_utf8_lossy(&stdout);
        assert!(
            !text.contains("jane.doe@example.com"),
            "stdout should NOT contain raw email when --redact is used. Got: {text}"
        );
        assert!(
            text.contains('█'),
            "stdout should contain masking character █. Got: {text}"
        );

        // Verify image was written
        assert!(out_path.exists(), "redacted image should be saved");

        // Verify pixels in the redacted image differ from the original
        let redacted_img = image::open(&out_path).unwrap().to_rgba8();
        let original_img = image::open(fixture_path()).unwrap().to_rgba8();
        assert_eq!(redacted_img.dimensions(), original_img.dimensions());

        // At least some pixels should differ (the redaction regions)
        let diff_count = redacted_img
            .pixels()
            .zip(original_img.pixels())
            .filter(|(a, b)| a != b)
            .count();
        assert!(
            diff_count > 0,
            "redacted image should have at least some different pixels from original"
        );

        let err_str = String::from_utf8_lossy(&stderr);
        assert!(err_str.contains("redacted"), "stderr should mention redaction count");
    }

    #[test]
    fn run_full_redact_only_saves_file() {
        let tmp = tempfile::tempdir().unwrap();
        let out_path = tmp.path().join("r.png");

        let cli = Cli::try_parse_from([
            "ariashot",
            "--full",
            "--redact",
            "-o",
            out_path.to_str().unwrap(),
        ])
        .unwrap();

        let backend = MockBackend::new();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let code = camerashot_cli::run(&cli, &backend, &mut stdout, &mut stderr);

        assert_eq!(code, EXIT_OK);
        assert!(out_path.exists());
        // No --ocr, so stdout should be empty
        assert!(stdout.is_empty(), "stdout should be empty when --ocr is not set");
    }
}

// ---------------------------------------------------------------------------
// Linux-only: --redact should fail-closed when OCR is unavailable
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
mod linux_tests {
    use super::*;
    use camerashot_cli::EXIT_OCR;

    #[test]
    fn run_full_redact_fails_closed_on_linux() {
        let tmp = tempfile::tempdir().unwrap();
        let out_path = tmp.path().join("r.png");

        let cli = Cli::try_parse_from([
            "ariashot",
            "--full",
            "--redact",
            "-o",
            out_path.to_str().unwrap(),
        ])
        .unwrap();

        let backend = MockBackend::new();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let code = camerashot_cli::run(&cli, &backend, &mut stdout, &mut stderr);

        assert_eq!(code, EXIT_OCR, "should exit 3 on Linux when OCR fails");
        assert!(
            !out_path.exists(),
            "output file should NOT be written when OCR fails (fail-closed)"
        );
    }
}

// ---------------------------------------------------------------------------
// Format inference tests
// ---------------------------------------------------------------------------

#[test]
fn run_full_output_jpg() {
    let tmp = tempfile::tempdir().unwrap();
    let out_path = tmp.path().join("out.jpg");

    let cli = Cli::try_parse_from([
        "ariashot",
        "--full",
        "-o",
        out_path.to_str().unwrap(),
    ])
    .unwrap();

    let backend = MockBackend::new();
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    let code = camerashot_cli::run(&cli, &backend, &mut stdout, &mut stderr);

    assert_eq!(code, EXIT_OK);
    assert!(out_path.exists(), "JPG output should exist");
    assert!(out_path.metadata().unwrap().len() > 0);
}

#[test]
fn run_full_output_webp() {
    let tmp = tempfile::tempdir().unwrap();
    let out_path = tmp.path().join("out.webp");

    let cli = Cli::try_parse_from([
        "ariashot",
        "--full",
        "-o",
        out_path.to_str().unwrap(),
    ])
    .unwrap();

    let backend = MockBackend::new();
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    let code = camerashot_cli::run(&cli, &backend, &mut stdout, &mut stderr);

    assert_eq!(code, EXIT_OK);
    assert!(out_path.exists(), "WebP output should exist");
    assert!(out_path.metadata().unwrap().len() > 0);
}

// ---------------------------------------------------------------------------
// Default output path (no -o, no --clipboard, no --ocr → default file)
// ---------------------------------------------------------------------------

#[test]
fn run_full_default_output_writes_file() {
    // We can't easily test the exact filename, but we can verify that when
    // no output flags are given, the run succeeds and writes something to stderr.
    let cli = Cli::try_parse_from(["ariashot", "--full"]).unwrap();

    let backend = MockBackend::new();
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    let code = camerashot_cli::run(&cli, &backend, &mut stdout, &mut stderr);

    assert_eq!(code, EXIT_OK);
    let err_str = String::from_utf8_lossy(&stderr);
    assert!(err_str.contains("saved:"), "should report saved file in stderr");

    // Clean up the default file
    let default_file = err_str
        .lines()
        .find(|l| l.starts_with("saved:"))
        .and_then(|l| l.strip_prefix("saved: "))
        .map(|p| PathBuf::from(p.trim()));
    if let Some(p) = default_file {
        let _ = std::fs::remove_file(&p);
    }
}

// ---------------------------------------------------------------------------
// RedactStyle mapping
// ---------------------------------------------------------------------------

#[test]
fn redact_style_to_annotation_tool() {
    use camerashot_core::annotation::AnnotationTool;

    assert_eq!(RedactStyle::Pixelate.to_annotation_tool(), AnnotationTool::Pixelate);
    assert_eq!(RedactStyle::Blur.to_annotation_tool(), AnnotationTool::Blur);
    assert_eq!(RedactStyle::Fill.to_annotation_tool(), AnnotationTool::FilledRectangle);
}
