use camerashot_overlay::{OverlayApp, OverlaySurface};
use camerashot_platform::create_default_backend;
use std::process::ExitCode;
use tracing::{error, info};
use winit::event_loop::EventLoop;

fn main() -> ExitCode {
    tracing_subscriber::fmt::init();

    info!("AriaShot Overlay starting…");

    // 1. Capture the primary display
    let backend = match create_default_backend() {
        Ok(b) => b,
        Err(e) => {
            error!("Failed to create capture backend: {}", e);
            return ExitCode::from(2);
        }
    };

    let displays = match backend.enumerate_displays() {
        Ok(d) => d,
        Err(e) => {
            error!("Failed to enumerate displays: {}", e);
            return ExitCode::from(2);
        }
    };

    let primary = match displays.iter().find(|d| d.is_primary).or(displays.first()) {
        Some(d) => d,
        None => {
            error!("No display found");
            return ExitCode::from(2);
        }
    };

    info!(
        "Capturing display: {} ({}×{})",
        primary.name,
        primary.bounds.width(),
        primary.bounds.height()
    );

    let frame = match backend.capture_display(primary.id) {
        Ok(f) => f,
        Err(e) => {
            error!("Capture failed: {}", e);
            return ExitCode::from(2);
        }
    };

    let rgba = frame.to_rgba8();
    let surface = match OverlaySurface::new(frame.width as u32, frame.height as u32, &rgba) {
        Some(s) => s,
        None => {
            error!("Failed to create OverlaySurface");
            return ExitCode::from(2);
        }
    };

    info!("Overlay surface: {}×{}", surface.width, surface.height);

    // 2. Open the overlay window and run the event loop
    let event_loop = match EventLoop::new() {
        Ok(el) => el,
        Err(e) => {
            error!("Failed to create event loop: {}", e);
            return ExitCode::from(2);
        }
    };

    let mut app = OverlayApp::new(surface);

    if let Err(e) = event_loop.run_app(&mut app) {
        error!("Event loop error: {}", e);
        return ExitCode::from(2);
    }

    ExitCode::from(app.exit_code as u8)
}
