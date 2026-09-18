use camerashot_core::scroll::{
    SadFilter, ScrollStitcher, SettlementDetector, VerticalShiftEstimator,
};
use tiny_skia::{Paint, Pixmap, PixmapPaint, Transform};

#[test]
fn test_settlement_detector() {
    let mut detector = SettlementDetector::new(2);
    let frame_a = vec![128u8; 1000];
    let frame_b = vec![128u8; 1000];
    let frame_c = vec![255u8; 1000];

    assert!(!detector.update(&frame_a)); // First frame, not settled yet
    assert!(detector.update(&frame_b)); // Second identical frame, settled!
    assert!(!detector.update(&frame_c)); // Changed content, unsettled
    assert!(detector.update(&frame_c)); // Consecutive identical frame, becomes settled
}

#[test]
fn test_scrollbar_and_sticky_header_detection() {
    let w = 400;
    let h = 300;
    let mut prev = Pixmap::new(w, h).unwrap();
    let mut curr = Pixmap::new(w, h).unwrap();

    // Fill content with base document pattern
    for y in 0..h {
        let mut p = Paint::default();
        let c = (y % 180) as u8;
        p.set_color_rgba8(c, c, c, 255);
        prev.fill_rect(
            tiny_skia::Rect::from_xywh(0.0, y as f32, w as f32, 1.0).unwrap(),
            &p,
            Transform::identity(),
            None,
        );
        curr.fill_rect(
            tiny_skia::Rect::from_xywh(0.0, y as f32, w as f32, 1.0).unwrap(),
            &p,
            Transform::identity(),
            None,
        );
    }

    // 1. Scrollbar test: mutate right 12 columns in curr (simulating scrollbar thumb)
    let scrollbar_w = 12u32;
    for y in 60..240 {
        for x in (w - scrollbar_w)..w {
            curr.fill_rect(
                tiny_skia::Rect::from_xywh(x as f32, y as f32, 1.0, 1.0).unwrap(),
                &Paint::default(),
                Transform::identity(),
                None,
            );
        }
    }

    let detected_margin = SadFilter::detect_scrollbar_margin(&prev, &curr);
    // Detected margin = scrollbar_width + 4 padding pixels = 12 + 4 = 16
    assert_eq!(detected_margin, 16);

    // 2. Sticky header test:
    // Create new prev and curr where top 40 rows are frozen header (identical),
    // and content below row 40 is scrolled down by 60px.
    let mut h_prev = Pixmap::new(w, h).unwrap();
    let mut h_curr = Pixmap::new(w, h).unwrap();

    // Sticky header (rows 0..40 identical)
    for y in 0..40 {
        let mut p = Paint::default();
        p.set_color_rgba8(40, 40, 45, 255);
        h_prev.fill_rect(
            tiny_skia::Rect::from_xywh(0.0, y as f32, w as f32, 1.0).unwrap(),
            &p,
            Transform::identity(),
            None,
        );
        h_curr.fill_rect(
            tiny_skia::Rect::from_xywh(0.0, y as f32, w as f32, 1.0).unwrap(),
            &p,
            Transform::identity(),
            None,
        );
    }

    // Scrolled body (rows 40..h differ by 60px offset)
    for y in 40..h {
        let mut p_prev = Paint::default();
        let c1 = ((y * 3) % 250) as u8;
        p_prev.set_color_rgba8(c1, c1, c1, 255);
        h_prev.fill_rect(
            tiny_skia::Rect::from_xywh(0.0, y as f32, w as f32, 1.0).unwrap(),
            &p_prev,
            Transform::identity(),
            None,
        );

        let mut p_curr = Paint::default();
        let c2 = (((y + 60) * 3) % 250) as u8;
        p_curr.set_color_rgba8(c2, c2, c2, 255);
        h_curr.fill_rect(
            tiny_skia::Rect::from_xywh(0.0, y as f32, w as f32, 1.0).unwrap(),
            &p_curr,
            Transform::identity(),
            None,
        );
    }

    let header_h = SadFilter::detect_sticky_header(&h_prev, &h_curr, 60);
    assert_eq!(header_h, 40);
}

#[test]
fn test_continuous_scrolling_stitch_10000px() {
    let doc_w = 400u32;
    let doc_h = 10_000u32;
    let mut doc = Pixmap::new(doc_w, doc_h).unwrap();

    // Paint a distinct visual pattern every 80 pixels
    for y in (0..doc_h).step_by(80) {
        let mut p = Paint::default();
        let r = ((y * 7) % 255) as u8;
        let g = ((y * 13) % 255) as u8;
        let b = ((y * 19) % 255) as u8;
        p.set_color_rgba8(r, g, b, 255);
        doc.fill_rect(
            tiny_skia::Rect::from_xywh(0.0, y as f32, doc_w as f32, 20.0).unwrap(),
            &p,
            Transform::identity(),
            None,
        );
    }

    let viewport_h = 600u32;
    let scroll_step = 200usize; // user scrolls down 200px each step

    let mut stitcher = ScrollStitcher::new(30_000);

    // Initial viewport frame at y = 0
    let mut frame0 = Pixmap::new(doc_w, viewport_h).unwrap();
    frame0.draw_pixmap(
        0,
        0,
        doc.as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        None,
    );
    stitcher.set_initial_frame(frame0.clone());

    let mut prev_frame = frame0;
    let num_steps = 15; // 15 steps * 200px = 3000px scrolled

    for step in 1..=num_steps {
        let y_offset = (step * scroll_step) as i32;
        let mut curr_frame = Pixmap::new(doc_w, viewport_h).unwrap();
        curr_frame.draw_pixmap(
            0,
            -y_offset,
            doc.as_ref(),
            &PixmapPaint::default(),
            Transform::identity(),
            None,
        );

        // Estimate displacement
        let estimated_shift = VerticalShiftEstimator::estimate_shift(&prev_frame, &curr_frame, 0)
            .expect("Should reliably estimate shift");

        assert_eq!(estimated_shift, scroll_step);

        // Append to stitcher
        let ok = stitcher.append_frame(&curr_frame, estimated_shift);
        assert!(ok);

        prev_frame = curr_frame;
    }

    let stitched = stitcher
        .stitched_image()
        .expect("Stitched image should exist");
    assert_eq!(stitched.width(), doc_w);

    // Verified height: initial (600) + 15 * (200 - 1) = 600 + 15 * 199 = 3585
    let expected_height = (viewport_h as usize) + num_steps * (scroll_step - 1);
    assert_eq!(stitched.height() as usize, expected_height);
    println!(
        "Successfully stitched 15 continuous strips into {}x{} document (seam bias overlap applied)",
        stitched.width(),
        stitched.height()
    );
}
