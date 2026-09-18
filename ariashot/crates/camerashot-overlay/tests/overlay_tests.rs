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
    assert!(matches!(
        surface.controller.state,
        OverlayState::Idle { .. }
    ));

    // 2. Mouse down at (50, 50) initiates Selecting
    surface.on_mouse_down(Point::new(50.0, 50.0));
    assert!(matches!(
        surface.controller.state,
        OverlayState::Selecting { .. }
    ));

    // 3. Mouse drag to (350, 250)
    surface.on_mouse_move(Point::new(350.0, 250.0));
    let current_rect = surface.controller.current_selection_rect().unwrap();
    assert_eq!(current_rect, Rect::new(50.0, 50.0, 300.0, 200.0));

    // 4. Mouse up commits Selected state
    surface.on_mouse_up();
    assert!(matches!(
        surface.controller.state,
        OverlayState::Selected { .. }
    ));

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
    let cropped = surface
        .export_selection_pixmap()
        .expect("Cropped pixmap should be non-empty");
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
    let beautified = surface
        .export_selection_pixmap()
        .expect("Beautified pixmap should be created");
    // Expected width: 300 + 32 * 2 = 364
    // Expected height: 200 + 32 * 2 + 36 (title bar) = 300
    assert_eq!(beautified.width(), 364);
    assert_eq!(beautified.height(), 300);
}

// ============================================================
// Phase 2 tests
// ============================================================

#[test]
fn test_sample_magnified_checkered_4x4() {
    use camerashot_overlay::sample_magnified;

    // Build a 4×4 checkerboard image: alternating black/white rows
    let w = 4u32;
    let h = 4u32;
    let mut pixels = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let idx = ((y * w + x) * 4) as usize;
            let is_white = (x + y) % 2 == 0;
            let val = if is_white { 255 } else { 0 };
            pixels[idx] = val;
            pixels[idx + 1] = val;
            pixels[idx + 2] = val;
            pixels[idx + 3] = 255;
        }
    }

    // Copy pixels into the pixmap
    let mut bg_pixmap = Pixmap::new(w, h).unwrap();
    bg_pixmap.data_mut().copy_from_slice(&pixels);

    // Sample at center (2,2) with radius=4 → 8×8 output, mag=2
    let cursor = Point::new(2.0, 2.0);
    let result = sample_magnified(bg_pixmap.as_ref(), cursor, 4);

    assert_eq!(result.width(), 8);
    assert_eq!(result.height(), 8);

    // Verify: source pixel (2,2) is white ((2+2)%2==0), should appear at center block
    // Center of 8×8 is pixels (3,3),(3,4),(4,3),(4,4) — but actually with radius=4, mag=2:
    //   src_half = 4/2 = 2, so source range is [2-2, 2+2) = [0,4)
    //   For source (2,2): dx = (2-0+2)*2..  let's just verify the pixel at the center area
    let data = result.data();
    // Source pixel at cursor (2,2) maps to output block at dx=(2+0)*2=4, dy=(2+0)*2=4
    // Wait: sx_off goes from -src_half to src_half = -2 to 2
    // For sx_off=0, sy_off=0 → src=(2,2), dx=(0+2)*2=4, dy=(0+2)*2=4
    // So output pixel (4,4) should be white (255,255,255,255)
    let center_idx = (4 * 8 + 4) * 4;
    assert_eq!(data[center_idx], 255, "center R");
    assert_eq!(data[center_idx + 1], 255, "center G");
    assert_eq!(data[center_idx + 2], 255, "center B");
    assert_eq!(data[center_idx + 3], 255, "center A");

    // Source pixel (1,2) is black ((1+2)%2==1), maps to dx=((-1)+2)*2=2, dy=(0+2)*2=4
    let off_idx = (4 * 8 + 2) * 4;
    assert_eq!(data[off_idx], 0, "left-of-center R should be black");
    assert_eq!(data[off_idx + 1], 0, "left-of-center G");
    assert_eq!(data[off_idx + 2], 0, "left-of-center B");

    // Each source pixel should span a 2×2 block: check (4,5) is same as (4,4) (within block)
    let block_idx = (5 * 8 + 4) * 4;
    assert_eq!(data[block_idx], 255, "block neighbor R");
    assert_eq!(data[block_idx + 1], 255, "block neighbor G");
    assert_eq!(data[block_idx + 3], 255, "block neighbor A");
}

#[test]
fn test_sample_magnified_at_origin_no_panic() {
    use camerashot_overlay::sample_magnified;

    let w = 8u32;
    let h = 8u32;
    let pixels = vec![128u8; (w * h * 4) as usize];
    let mut bg = Pixmap::new(w, h).unwrap();
    bg.data_mut().copy_from_slice(&pixels);

    // Cursor at (0,0) — should handle out-of-bounds source pixels gracefully
    let result = sample_magnified(bg.as_ref(), Point::new(0.0, 0.0), 4);
    assert_eq!(result.width(), 8);
    assert_eq!(result.height(), 8);

    // Top-left quadrant should be transparent (out of bounds)
    // sx_off=-2, sy_off=-2 → src=(-2,-2) → transparent
    let idx = 0;
    assert_eq!(
        result.data()[idx + 3],
        0,
        "out-of-bounds pixel should be transparent"
    );
}

#[test]
fn test_loupe_render_edge_no_panic() {
    use camerashot_overlay::Loupe;

    let w = 200u32;
    let h = 200u32;
    let pixels = vec![100u8; (w * h * 4) as usize];
    let mut bg = Pixmap::new(w, h).unwrap();
    bg.data_mut().copy_from_slice(&pixels);

    let mut target = Pixmap::new(w, h).unwrap();

    // Render at corners — should not panic
    Loupe::render(&mut target.as_mut(), bg.as_ref(), Point::new(0.0, 0.0));
    Loupe::render(&mut target.as_mut(), bg.as_ref(), Point::new(199.0, 199.0));
    Loupe::render(&mut target.as_mut(), bg.as_ref(), Point::new(0.0, 199.0));
    Loupe::render(&mut target.as_mut(), bg.as_ref(), Point::new(199.0, 0.0));
}

#[test]
fn test_toolbar_hit_test_copy_button() {
    use camerashot_overlay::{FloatingToolbar, ToolbarAction, ToolbarLayout};

    let selection = Rect::new(200.0, 100.0, 400.0, 300.0);
    let canvas_w = 1920u32;
    let canvas_h = 1080u32;

    let layout = ToolbarLayout::compute(selection, canvas_w, canvas_h);

    // Hit the center of the Copy button
    let (copy_action, [cx, cy, cw, ch]) = layout.buttons[0];
    assert_eq!(copy_action, ToolbarAction::CopyToClipboard);
    let center = Point::new((cx + cw / 2.0) as f64, (cy + ch / 2.0) as f64);
    let result = FloatingToolbar::hit_test(selection, canvas_w, canvas_h, center);
    assert_eq!(result, Some(ToolbarAction::CopyToClipboard));

    // Hit the center of the Close button
    let (close_action, [cx2, cy2, cw2, ch2]) = layout.buttons[1];
    assert_eq!(close_action, ToolbarAction::Close);
    let center2 = Point::new((cx2 + cw2 / 2.0) as f64, (cy2 + ch2 / 2.0) as f64);
    let result2 = FloatingToolbar::hit_test(selection, canvas_w, canvas_h, center2);
    assert_eq!(result2, Some(ToolbarAction::Close));
}

#[test]
fn test_toolbar_hit_test_outside_returns_none() {
    use camerashot_overlay::FloatingToolbar;

    let selection = Rect::new(200.0, 100.0, 400.0, 300.0);
    let canvas_w = 1920u32;
    let canvas_h = 1080u32;

    // Point far away from the toolbar
    let result = FloatingToolbar::hit_test(selection, canvas_w, canvas_h, Point::new(10.0, 10.0));
    assert_eq!(result, None);
}

#[test]
fn test_loupe_shown_during_selecting() {
    // Verify that render_frame doesn't panic when in Selecting state
    let width = 200u32;
    let height = 200u32;
    let pixels = vec![150u8; (width * height * 4) as usize];

    let mut surface = OverlaySurface::new(width, height, &pixels).expect("surface");

    // Transition to Selecting state
    surface.on_mouse_down(Point::new(50.0, 50.0));
    surface.on_mouse_move(Point::new(100.0, 100.0));

    assert!(matches!(
        surface.controller.state,
        OverlayState::Selecting { .. }
    ));

    // Render should succeed (Loupe visible during Selecting)
    let mut target = Pixmap::new(width, height).unwrap();
    surface.render_frame(&mut target);
}
