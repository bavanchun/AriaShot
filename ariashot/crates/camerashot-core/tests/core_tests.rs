use camerashot_core::{
    chaikin_smooth, interpolate_to_count, moving_average_smooth, smooth_pencil_stroke, Annotation,
    AnnotationTool, BoundarySnapIndex, NumberFormat, PencilSmoothMode, Point, Rect, UndoAction,
    UndoStack,
};
use uuid::Uuid;

#[test]
fn test_geometry_primitives() {
    let p1 = Point::new(10.0, 20.0);
    let p2 = Point::new(40.0, 60.0);
    assert_eq!(p1.distance_to(p2), 50.0);

    let lerped = p1.lerp(p2, 0.5);
    assert_eq!(lerped, Point::new(25.0, 40.0));

    let rect1 = Rect::new(10.0, 10.0, 100.0, 100.0);
    let rect2 = Rect::new(50.0, 50.0, 100.0, 100.0);

    assert!(rect1.contains(Point::new(50.0, 50.0)));
    assert!(!rect1.contains(Point::new(150.0, 150.0)));

    assert!(rect1.intersects(&rect2));
    let inter = rect1.intersection(&rect2).unwrap();
    assert_eq!(inter, Rect::new(50.0, 50.0, 60.0, 60.0));

    let union_rect = rect1.union(&rect2);
    assert_eq!(union_rect, Rect::new(10.0, 10.0, 140.0, 140.0));
}

#[test]
fn test_boundary_snap_index_vertical_and_horizontal() {
    // 100x100 synthetic image
    let width = 100;
    let height = 100;
    let mut pixels = vec![255u8; width * height * 4]; // white background

    // Create a high-contrast black vertical strip at x = 40..=60 for all y
    for y in 0..height {
        for x in 40..=60 {
            let idx = (y * width + x) * 4;
            pixels[idx] = 0; // R
            pixels[idx + 1] = 0; // G
            pixels[idx + 2] = 0; // B
            pixels[idx + 3] = 255;
        }
    }

    let draw_rect = Rect::new(0.0, 0.0, 100.0, 100.0);
    let index =
        BoundarySnapIndex::build(width, height, &pixels, draw_rect).expect("Failed to build index");

    // Boundary 40 is between pixel 39 (white, 255) and pixel 40 (black, 0). Contrast = ~441.67 > 28.0.
    // Query near view_x = 42.0 with radius = 5.0 points along y in [10.0, 90.0].
    let hit_v = index.nearest_vertical(42.0, 10.0, 90.0, 5.0);
    assert!(hit_v.is_some(), "Expected snap hit at boundary 40");
    let hit = hit_v.unwrap();
    assert_eq!(hit.pixel_boundary, 40);
    assert!((hit.view_position - 40.0).abs() < 1e-4);
    assert!(hit.strength >= BoundarySnapIndex::MIN_MEAN_DIFF);

    // Query outside search radius -> None
    let hit_miss = index.nearest_vertical(20.0, 10.0, 90.0, 5.0);
    assert!(hit_miss.is_none());

    // Create a horizontal boundary in another image: row 0..49 white, row 50..99 black
    let mut h_pixels = vec![255u8; width * height * 4];
    for y in 50..height {
        for x in 0..width {
            let idx = (y * width + x) * 4;
            h_pixels[idx] = 0;
            h_pixels[idx + 1] = 0;
            h_pixels[idx + 2] = 0;
            h_pixels[idx + 3] = 255;
        }
    }
    let h_index = BoundarySnapIndex::build(width, height, &h_pixels, draw_rect)
        .expect("Failed to build index");
    let hit_h = h_index.nearest_horizontal(48.0, 10.0, 90.0, 5.0);
    assert!(
        hit_h.is_some(),
        "Expected horizontal snap hit at boundary 50"
    );
    assert_eq!(hit_h.unwrap().pixel_boundary, 50);
}

#[test]
fn test_chaikin_and_moving_average_smoothing() {
    let raw_points = vec![
        Point::new(0.0, 0.0),
        Point::new(10.0, 25.0),
        Point::new(20.0, 5.0),
        Point::new(30.0, 30.0),
        Point::new(40.0, 10.0),
    ];

    // Chaikin smoothing
    let smoothed = chaikin_smooth(&raw_points, 2);
    // Endpoints must remain exact
    assert_eq!(smoothed.first().copied(), Some(Point::new(0.0, 0.0)));
    assert_eq!(smoothed.last().copied(), Some(Point::new(40.0, 10.0)));
    assert!(smoothed.len() > raw_points.len());

    // Moving average
    let ma = moving_average_smooth(&raw_points, 3);
    assert_eq!(ma.len(), raw_points.len());
    assert_eq!(ma[0], Point::new(0.0, 0.0));

    // Full pencil pipeline with Refined mode
    let (final_pts, _) = smooth_pencil_stroke(&raw_points, None, PencilSmoothMode::Refined);
    assert!(!final_pts.is_empty());
    assert_eq!(final_pts.first().copied(), Some(Point::new(0.0, 0.0)));

    // Pressure interpolation
    let pressures = vec![0.2, 0.5, 0.8];
    let interpolated = interpolate_to_count(&pressures, 5);
    assert_eq!(interpolated.len(), 5);
    assert!((interpolated[0] - 0.2).abs() < 1e-6);
    assert!((interpolated[4] - 0.8).abs() < 1e-6);
    assert!((interpolated[2] - 0.5).abs() < 1e-6);
}

#[test]
fn test_undo_stack_operations() {
    let mut stack = UndoStack::new(50);
    let mut annotations = Vec::new();

    let ann1 = Annotation::new(
        AnnotationTool::Rectangle,
        Point::new(10.0, 10.0),
        Point::new(100.0, 100.0),
        [255, 0, 0, 255],
        2.0,
    );
    let ann2 = Annotation::new(
        AnnotationTool::Arrow,
        Point::new(20.0, 20.0),
        Point::new(80.0, 80.0),
        [0, 255, 0, 255],
        3.0,
    );

    // 1. Add ann1
    annotations.push(ann1.clone());
    stack.push(UndoAction::Add {
        annotation: ann1.clone(),
    });
    assert_eq!(annotations.len(), 1);

    // 2. Add ann2
    annotations.push(ann2.clone());
    stack.push(UndoAction::Add {
        annotation: ann2.clone(),
    });
    assert_eq!(annotations.len(), 2);

    // 3. Undo add ann2
    assert!(stack.can_undo());
    assert!(stack.undo(&mut annotations));
    assert_eq!(annotations.len(), 1);
    assert_eq!(annotations[0].id, ann1.id);

    // 4. Redo add ann2
    assert!(stack.can_redo());
    assert!(stack.redo(&mut annotations));
    assert_eq!(annotations.len(), 2);

    // 5. Batch action (like auto-redact PII)
    let pii1 = Annotation::new(
        AnnotationTool::FilledRectangle,
        Point::new(0.0, 0.0),
        Point::new(50.0, 20.0),
        [0, 0, 0, 255],
        1.0,
    );
    let pii2 = Annotation::new(
        AnnotationTool::FilledRectangle,
        Point::new(0.0, 30.0),
        Point::new(50.0, 50.0),
        [0, 0, 0, 255],
        1.0,
    );
    let group_id = Uuid::new_v4();

    annotations.push(pii1.clone());
    annotations.push(pii2.clone());
    stack.push(UndoAction::Batch {
        group_id: Some(group_id),
        actions: vec![
            UndoAction::Add {
                annotation: pii1.clone(),
            },
            UndoAction::Add {
                annotation: pii2.clone(),
            },
        ],
    });
    assert_eq!(annotations.len(), 4);

    // Batch undo removes both atomically
    assert!(stack.undo(&mut annotations));
    assert_eq!(annotations.len(), 2);

    // Batch redo restores both atomically
    assert!(stack.redo(&mut annotations));
    assert_eq!(annotations.len(), 4);
}

#[test]
fn test_number_formatting() {
    assert_eq!(NumberFormat::Decimal.format(1), "1");
    assert_eq!(NumberFormat::Decimal.format(42), "42");
    assert_eq!(NumberFormat::Roman.format(1), "I");
    assert_eq!(NumberFormat::Roman.format(4), "IV");
    assert_eq!(NumberFormat::Roman.format(9), "IX");
    assert_eq!(NumberFormat::Roman.format(2026), "MMXXVI");
    assert_eq!(NumberFormat::Alpha.format(1), "A");
    assert_eq!(NumberFormat::Alpha.format(26), "Z");
    assert_eq!(NumberFormat::Alpha.format(27), "AA");
    assert_eq!(NumberFormat::AlphaLower.format(1), "a");
}

#[test]
fn test_boundary_snap_index_4k_performance() {
    let width = 3840;
    let height = 2160;
    let pixels = vec![240u8; width * height * 4];
    let draw_rect = Rect::new(0.0, 0.0, 1920.0, 1080.0);

    let start = std::time::Instant::now();
    let index = BoundarySnapIndex::build(width, height, &pixels, draw_rect);
    let duration = start.elapsed();

    assert!(index.is_some());
    println!(
        "4K UHD (3840x2160) BoundarySnapIndex build time: {:?}",
        duration
    );
}
