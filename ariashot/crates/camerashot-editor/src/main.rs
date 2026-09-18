//! Camerashot Editor binary — opens an image file or captures the primary display.
//!
//! Usage:
//!   camerashot-editor [IMAGE_PATH]
//!
//! If IMAGE_PATH is provided, loads it (PNG/JPEG/WebP).
//! Otherwise, captures the primary display via `create_default_backend`.

use camerashot_editor::app::image_session::ImageSession;
use camerashot_editor::app::{bind_image_mode, bind_video_mode, MainEditorWindow, VideoState};
use parking_lot::Mutex;
use slint::ComponentHandle;
use std::path::Path;
use std::sync::Arc;

fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("Starting Camerashot Editor…");

    let args: Vec<String> = std::env::args().collect();

    let session = if args.len() > 1 {
        // Load from file
        let path = Path::new(&args[1]);
        tracing::info!("Loading image: {}", path.display());
        match ImageSession::from_file(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
    } else {
        // Capture primary display
        tracing::info!("No image path provided — capturing primary display");
        match capture_primary_display() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Capture failed: {e}");
                std::process::exit(1);
            }
        }
    };

    let session = Arc::new(Mutex::new(session));
    #[allow(clippy::arc_with_non_send_sync)]
    let video_state = Arc::new(Mutex::new(VideoState::default()));
    let ui = MainEditorWindow::new().expect("Failed to create editor window");

    bind_image_mode(&ui, session.clone(), video_state.clone());
    bind_video_mode(&ui, video_state, session);

    ui.run().expect("Slint event loop failed");
}

/// Capture the primary display and create an ImageSession from it.
fn capture_primary_display() -> Result<ImageSession, String> {
    let backend =
        camerashot_platform::create_default_backend().map_err(|e| format!("Backend: {e}"))?;

    let displays = backend
        .enumerate_displays()
        .map_err(|e| format!("Enumerate displays: {e}"))?;

    let primary = displays
        .iter()
        .find(|d| d.is_primary)
        .or_else(|| displays.first())
        .ok_or("No displays found")?;

    let fb = backend
        .capture_display(primary.id)
        .map_err(|e| format!("Capture display: {e}"))?;

    let rgba = fb.to_rgba8();
    ImageSession::from_rgba(fb.width as u32, fb.height as u32, rgba)
}
