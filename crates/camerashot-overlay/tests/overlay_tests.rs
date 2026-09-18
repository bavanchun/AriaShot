use camerashot_core::annotation::{Annotation, AnnotationTool, LineStyle};
use camerashot_core::beautify::{BeautifyConfig, BeautifyMode};
use camerashot_core::geometry::{Point, Rect};
use camerashot_overlay::{OverlayState, OverlaySurface};
use tiny_skia::Pixmap;

#[test]
fn test_overlay_surface_selection_and_rendering() {
    let width = 800;
    let height = 600;
    let pixels = vec![200u8; width * height * 4];

    let mut surface = OverlaySurface::new(width as u32, height as u32, &pixels)
        .expect("Failed to initialize OverlaySurface");

    // 1. Initial state is Idle
    assert!(matches!(surface.controller.state, OverlayState::Idle { .. }));

    // 2. Mouse down at (50, 50) initiates Selecting
    surface.on_mouse_down(Point::new(50.0, 50.0));
    assert!(matches!(surface.controller.state, OverlayState::Selecting { .. }));

    // 3. Mouse drag to (350, 250)
    surface.on_mouse_move(Point::new(350.0, 250.0));
    let current_rect = surface.controller.current_selection_rect().unwrap();
    assert_eq!(current_rect, Rect::new(50.0, 50.0, 300.0, 200.0));

    // 4. Mouse up commits Selected state
    surface.on_mouse_up();
    assert!(matches!(surface.controller.state, OverlayState::Selected { .. }));

    // 5. Add vector annotations to canvas
    let mut arrow = Annotation::new(
        AnnotationTool::Arrow,
        Point::new(60.0, 60.0),
        Point::new(200.0, 150.0),
        [255, 50, 50, 255],
        3.0,
    );
    arrow.line_style = LineStyle::Solid;
    surface.annotations.push(arrow);

    let mut badge = Annotation::new(
        AnnotationTool::Number,
        Point::new(100.0, 100.0),
        Point::new(100.0, 100.0),
        [0, 120, 255, 255],
        3.0,
    );
    badge.number_value = Some(1);
    surface.annotations.push(badge);

    // 6. Render full frame
    let mut target = Pixmap::new(width as u32, height as u32).unwrap();
    surface.render_frame(&mut target);
    assert_eq!(target.width(), width as u32);
    assert_eq!(target.height(), height as u32);

    // 7. Export selection pixmap
    let cropped = surface.export_selection_pixmap().expect("Cropped pixmap should be non-empty");
    assert_eq!(cropped.width(), 300);
    assert_eq!(cropped.height(), 200);

    // 8. Test Beautify export with macOS window chrome
    surface.beautify_config = Some(BeautifyConfig {
        mode: BeautifyMode::Window,
        style_index: 0,
        padding: 32.0,
        corner_radius: 8.0,
        shadow_radius: 16.0,
        bg_radius: 10.0,
    });
    let beautified = surface.export_selection_pixmap().expect("Beautified pixmap should be created");
    // Expected width: 300 + 32 * 2 = 364
    // Expected height: 200 + 32 * 2 + 36 (title bar) = 300
    assert_eq!(beautified.width(), 364);
    assert_eq!(beautified.height(), 300);
}
