//! `camerashot-cli` library: CLI interface for AriaShot screenshot tool.
//!
//! Provides the `Cli` argument parser and `run()` function that captures
//! screenshots, performs OCR, applies PII redaction, and outputs results.

use camerashot_core::annotation::AnnotationTool;
use camerashot_core::geometry::Rect;
use camerashot_core::renderer::AnnotationRenderer;
use camerashot_ocr::{
    blocks_to_text, mask_pii_in_text, redactions_from_blocks, OcrEngine, PiiDetector, PiiRedactor,
};
use camerashot_platform::clipboard;
use camerashot_platform::traits::{CaptureBackend, FrameBuffer};
use chrono::Local;
use clap::{ArgGroup, Parser, ValueEnum};
use std::io::Write;
use std::path::{Path, PathBuf};
use tiny_skia::Pixmap;

/// Exit codes.
pub const EXIT_OK: i32 = 0;
pub const EXIT_RUNTIME: i32 = 1;
pub const EXIT_SYNTAX: i32 = 2;
pub const EXIT_OCR: i32 = 3;

/// AriaShot – CLI screenshot tool.
///
/// Captures full screen or an arbitrary crop region and optionally performs
/// OCR text extraction and PII redaction.
///
/// Coordinates for `--crop` are in **point-logic** space (the global desktop
/// coordinate system used by CoreGraphics on macOS). On HiDPI displays the
/// resulting image may be larger in pixels than the requested point region.
///
/// On Linux, `--clipboard` blocks until another application reads the
/// clipboard (cooperative ownership). OCR is currently only supported on
/// macOS; on other platforms `--ocr` / `--redact` will fail with exit code 3.
#[derive(Parser, Debug)]
#[command(name = "ariashot", version, about)]
#[command(group(
    ArgGroup::new("target")
        .required(true)
        .args(["full", "crop"])
))]
pub struct Cli {
    /// Capture the full display.
    #[arg(long, group = "target")]
    pub full: bool,

    /// Capture a rectangular region: X,Y,W,H (point-logic coordinates, W and H must be > 0).
    #[arg(long, group = "target", value_parser = parse_crop)]
    pub crop: Option<Rect>,

    /// Display ID to capture (only with --full; defaults to the primary display).
    #[arg(long, requires = "full", conflicts_with = "crop")]
    pub display: Option<u32>,

    /// Output file path. Format is inferred from the extension (png/jpg/webp).
    /// Defaults to ./ariashot-YYYYMMDD-HHMMSS.png when neither --clipboard nor --ocr is given.
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Copy the captured image to the system clipboard.
    #[arg(long)]
    pub clipboard: bool,

    /// Run OCR and print recognized text to stdout.
    #[arg(long)]
    pub ocr: bool,

    /// Redact detected PII in the image before output.
    /// Requires OCR; if OCR fails the image is NOT written (fail-closed).
    #[arg(long)]
    pub redact: bool,

    /// Style used for PII redaction overlays.
    #[arg(long, value_enum, default_value_t = RedactStyle::Pixelate)]
    pub redact_style: RedactStyle,
}

/// Redaction overlay style.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum RedactStyle {
    Pixelate,
    Blur,
    Fill,
}

impl RedactStyle {
    /// Map to the corresponding [`AnnotationTool`] variant.
    pub fn to_annotation_tool(self) -> AnnotationTool {
        match self {
            Self::Pixelate => AnnotationTool::Pixelate,
            Self::Blur => AnnotationTool::Blur,
            Self::Fill => AnnotationTool::FilledRectangle,
        }
    }
}

/// Parse a `--crop X,Y,W,H` value.
///
/// Returns `Err` if the format is invalid or W/H ≤ 0.
pub fn parse_crop(s: &str) -> Result<Rect, String> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 4 {
        return Err(format!(
            "expected 4 comma-separated values X,Y,W,H, got {}",
            parts.len()
        ));
    }
    let x: f64 = parts[0]
        .trim()
        .parse()
        .map_err(|_| format!("invalid X value: '{}'", parts[0].trim()))?;
    let y: f64 = parts[1]
        .trim()
        .parse()
        .map_err(|_| format!("invalid Y value: '{}'", parts[1].trim()))?;
    let w: f64 = parts[2]
        .trim()
        .parse()
        .map_err(|_| format!("invalid W value: '{}'", parts[2].trim()))?;
    let h: f64 = parts[3]
        .trim()
        .parse()
        .map_err(|_| format!("invalid H value: '{}'", parts[3].trim()))?;

    if w <= 0.0 {
        return Err(format!("width must be > 0, got {w}"));
    }
    if h <= 0.0 {
        return Err(format!("height must be > 0, got {h}"));
    }

    Ok(Rect::new(x, y, w, h))
}

/// Runtime error variants (not syntax errors — those are handled by clap).
#[derive(Debug)]
pub enum CliError {
    /// Generic runtime error (capture / permissions / I/O / clipboard). Exit code 1.
    Runtime(String),
    /// OCR unavailable or failed. Exit code 3.
    Ocr(String),
}

impl CliError {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Runtime(_) => EXIT_RUNTIME,
            Self::Ocr(_) => EXIT_OCR,
        }
    }
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Runtime(msg) => write!(f, "{msg}"),
            Self::Ocr(msg) => write!(f, "{msg}"),
        }
    }
}

/// Generate the default output filename: `./ariashot-YYYYMMDD-HHMMSS.png`.
pub fn default_output_path() -> PathBuf {
    let stamp = Local::now().format("%Y%m%d-%H%M%S");
    PathBuf::from(format!("ariashot-{stamp}.png"))
}

/// Convert a `FrameBuffer` into a `tiny_skia::Pixmap`.
fn framebuffer_to_pixmap(fb: &FrameBuffer) -> Result<Pixmap, CliError> {
    let rgba = fb.to_rgba8();
    let w = fb.width as u32;
    let h = fb.height as u32;
    Pixmap::from_vec(
        rgba,
        tiny_skia::IntSize::from_wh(w, h)
            .ok_or_else(|| CliError::Runtime(format!("invalid image dimensions: {w}x{h}")))?,
    )
    .ok_or_else(|| CliError::Runtime("failed to create Pixmap from RGBA data".to_string()))
}

/// Save a pixmap to a file, inferring format from the extension.
///
/// Supported: `.png`, `.jpg`/`.jpeg`, `.webp`. Defaults to PNG.
fn save_pixmap(pixmap: &Pixmap, path: &Path) -> Result<(), CliError> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("png")
        .to_ascii_lowercase();

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| {
                CliError::Runtime(format!("cannot create directory {:?}: {e}", parent))
            })?;
        }
    }

    let data = pixmap.data(); // premultiplied RGBA
    let w = pixmap.width();
    let h = pixmap.height();

    match ext.as_str() {
        "jpg" | "jpeg" => {
            // JPEG doesn't support alpha — convert RGBA to RGB.
            // For screenshots alpha is 255 everywhere so this is lossless.
            let rgba = image::RgbaImage::from_raw(w, h, data.to_vec())
                .ok_or_else(|| CliError::Runtime("failed to create image buffer".to_string()))?;
            let rgb = image::DynamicImage::ImageRgba8(rgba).to_rgb8();
            rgb.save(path)
                .map_err(|e| CliError::Runtime(format!("cannot write {:?}: {e}", path)))?;
        }
        "webp" => {
            let img = image::RgbaImage::from_raw(w, h, data.to_vec())
                .ok_or_else(|| CliError::Runtime("failed to create image buffer".to_string()))?;
            img.save(path)
                .map_err(|e| CliError::Runtime(format!("cannot write {:?}: {e}", path)))?;
        }
        _ => {
            // Default: PNG via tiny_skia (fast, lossless)
            pixmap
                .save_png(path)
                .map_err(|e| CliError::Runtime(format!("cannot write PNG {:?}: {e}", path)))?;
        }
    }

    Ok(())
}

/// Main CLI execution logic.
///
/// Returns the exit code (0 / 1 / 3). All status messages go to `err`; OCR
/// text goes to `out`.
pub fn run(
    cli: &Cli,
    backend: &dyn CaptureBackend,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    match run_inner(cli, backend, out, err) {
        Ok(()) => EXIT_OK,
        Err(e) => {
            let _ = writeln!(err, "error: {e}");
            e.exit_code()
        }
    }
}

fn run_inner(
    cli: &Cli,
    backend: &dyn CaptureBackend,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<(), CliError> {
    // 1. Capture
    let fb = capture(cli, backend)?;
    let mut pixmap = framebuffer_to_pixmap(&fb)?;
    let w = pixmap.width();
    let h = pixmap.height();

    // 2. OCR (run at most once, used by both --ocr and --redact)
    let ocr_blocks = if cli.ocr || cli.redact {
        let engine = OcrEngine::new();
        match engine.recognize_text(pixmap.data(), w, h) {
            Ok(blocks) => Some(blocks),
            Err(e) => return Err(CliError::Ocr(format!("OCR failed: {e}"))),
        }
    } else {
        None
    };

    // 3. Redact PII on the image (fail-closed: if we reach here OCR succeeded)
    let redaction_count = if cli.redact {
        let blocks = ocr_blocks
            .as_ref()
            .expect("OCR blocks must exist for redact");
        let redactor = PiiRedactor::new();
        let tool = cli.redact_style.to_annotation_tool();
        let (_batch_id, annotations) = redactions_from_blocks(&redactor, blocks, tool, 2.0);
        let count = annotations.len();
        if count > 0 {
            AnnotationRenderer::render_annotations(&mut pixmap.as_mut(), &annotations);
        }
        count
    } else {
        0
    };

    // 4. Output: file
    let needs_default_file = cli.output.is_none() && !cli.clipboard && !cli.ocr;
    let output_path = if let Some(ref p) = cli.output {
        Some(p.clone())
    } else if needs_default_file {
        Some(default_output_path())
    } else {
        None
    };

    if let Some(ref path) = output_path {
        save_pixmap(&pixmap, path)?;
        let _ = writeln!(err, "saved: {}", path.display());
    }

    // 5. Output: clipboard
    if cli.clipboard {
        clipboard::copy_rgba_image(w, h, pixmap.data())
            .map_err(|e| CliError::Runtime(format!("clipboard: {e}")))?;
        let _ = writeln!(err, "copied to clipboard");
    }

    // 6. Output: OCR text to stdout
    if cli.ocr {
        if let Some(ref blocks) = ocr_blocks {
            let text = blocks_to_text(blocks);
            let output_text = if cli.redact {
                let detector = PiiDetector::new();
                mask_pii_in_text(&detector, &text)
            } else {
                text
            };
            writeln!(out, "{output_text}")
                .map_err(|e| CliError::Runtime(format!("stdout write: {e}")))?;
        }
    }

    // 7. Status summary to stderr
    if cli.redact && redaction_count > 0 {
        let _ = writeln!(err, "redacted {redaction_count} PII region(s)");
    }

    Ok(())
}

/// Perform the screen capture step.
fn capture(cli: &Cli, backend: &dyn CaptureBackend) -> Result<FrameBuffer, CliError> {
    if let Some(rect) = cli.crop {
        backend
            .capture_rect(rect)
            .map_err(|e| CliError::Runtime(format!("capture_rect failed: {e}")))
    } else {
        // --full mode
        let display_id = if let Some(id) = cli.display {
            // Verify the display exists
            let displays = backend
                .enumerate_displays()
                .map_err(|e| CliError::Runtime(format!("enumerate_displays: {e}")))?;
            if !displays.iter().any(|d| d.id == id) {
                let ids: Vec<String> = displays
                    .iter()
                    .map(|d| format!("{} ({})", d.id, d.name))
                    .collect();
                return Err(CliError::Runtime(format!(
                    "display {id} not found. Available: {}",
                    ids.join(", ")
                )));
            }
            id
        } else {
            // Find primary display
            let displays = backend
                .enumerate_displays()
                .map_err(|e| CliError::Runtime(format!("enumerate_displays: {e}")))?;
            displays
                .iter()
                .find(|d| d.is_primary)
                .or_else(|| displays.first())
                .map(|d| d.id)
                .ok_or_else(|| CliError::Runtime("no displays found".to_string()))?
        };

        backend
            .capture_display(display_id)
            .map_err(|e| CliError::Runtime(format!("capture_display failed: {e}")))
    }
}
