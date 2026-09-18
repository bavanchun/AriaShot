use camerashot_core::geometry::Point;
use camerashot_overlay::OverlaySurface;
use camerashot_platform::create_default_backend;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Camerashot Tier 1 Overlay Engine ===");

    let backend = create_default_backend()?;
    let displays = backend.enumerate_displays()?;
    println!("Found {} active display(s)", displays.len());

    for (i, d) in displays.iter().enumerate() {
        println!(
            "  [{}] {} (bounds: {:?}, scale: {:.1}, primary: {})",
            i, d.name, d.bounds, d.scale_factor, d.is_primary
        );
    }

    if let Some(primary) = displays.iter().find(|d| d.is_primary).or_else(|| displays.first()) {
        println!("Capturing display {}...", primary.name);
        let start = std::time::Instant::now();
        let frame = backend.capture_display(primary.id)?;
        let capture_duration = start.elapsed();
        println!(
            "Captured {}x{} frame in {:?}",
            frame.width, frame.height, capture_duration
        );

        let rgba = frame.to_rgba8();
        let mut surface = OverlaySurface::new(frame.width as u32, frame.height as u32, &rgba)
            .ok_or("Failed to create OverlaySurface")?;

        println!("Overlay surface initialized with BoundarySnapIndex successfully.");

        // Simulate interactive selection drag
        surface.on_mouse_down(Point::new(100.0, 100.0));
        surface.on_mouse_move(Point::new(600.0, 400.0));
        surface.on_mouse_up();

        if let Some(sel) = surface.controller.current_selection_rect() {
            println!("Selected region: {:?}", sel);
            if let Some(cropped) = surface.export_selection_pixmap() {
                println!("Exported selection pixmap: {}x{}", cropped.width(), cropped.height());
            }
        }
    }

    Ok(())
}
