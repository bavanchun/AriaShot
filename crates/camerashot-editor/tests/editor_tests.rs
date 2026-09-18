use camerashot_core::geometry::{Point, Rect};
use camerashot_editor::{
    smoothstep, CensorStyle, TimelineCompositor, VideoCensorSegment, VideoCutSegment,
    VideoSpeedSegment, VideoTimeline, VideoZoomSegment,
};
use camerashot_media::RecordedVideoFrame;
use std::time::Duration;

#[test]
fn test_smoothstep_math_and_zoom_punch_easing() {
    assert_eq!(smoothstep(0.0), 0.0);
    assert_eq!(smoothstep(1.0), 1.0);
    assert_eq!(smoothstep(0.5), 0.5);

    // Monotonicity check
    let mut prev = 0.0;
    for i in 1..=100 {
        let x = i as f64 / 100.0;
        let s = smoothstep(x);
        assert!(s >= prev);
        prev = s;
    }

    // Zoom segment easing
    let zoom_seg = VideoZoomSegment::new(2.0, 6.0, 2.0, Point::new(0.5, 0.5));
    // Outside range
    assert_eq!(zoom_seg.zoom_level_at(1.0), 1.0);
    assert_eq!(zoom_seg.zoom_level_at(7.0), 1.0);

    // Plateau (midpoint)
    assert_eq!(zoom_seg.zoom_level_at(4.0), 2.0);

    // Fade in ramp
    let fade_in_mid = 2.0 + zoom_seg.effective_fade_in() / 2.0;
    let zoom_in_ramp = zoom_seg.zoom_level_at(fade_in_mid);
    assert!(zoom_in_ramp > 1.0 && zoom_in_ramp < 2.0);

    // Clamped center check on frame borders
    let clamped_edge = VideoZoomSegment::clamped_center(Point::new(0.01, 0.01), 2.0);
    assert!(clamped_edge.x >= 0.25);
    assert!(clamped_edge.y >= 0.25);

    let crop = zoom_seg.crop_rect(2.0, 1920.0, 1080.0);
    assert_eq!(crop.size.width, 960.0);
    assert_eq!(crop.size.height, 540.0);
    assert!(crop.origin.x >= 0.0 && crop.origin.x + crop.size.width <= 1920.0);
    assert!(crop.origin.y >= 0.0 && crop.origin.y + crop.size.height <= 1080.0);
}

#[test]
fn test_video_cut_segment_and_kept_ranges() {
    let mut timeline = VideoTimeline::new(10.0);
    timeline.cuts.push(VideoCutSegment::new(2.0, 4.0));
    timeline.cuts.push(VideoCutSegment::new(6.0, 7.5));

    assert!(!timeline.is_cut(1.0));
    assert!(timeline.is_cut(3.0));
    assert!(!timeline.is_cut(5.0));
    assert!(timeline.is_cut(6.8));
    assert!(!timeline.is_cut(8.0));

    let kept = timeline.kept_ranges();
    assert_eq!(kept.len(), 3);
    assert_eq!(kept[0], (0.0, 2.0));
    assert_eq!(kept[1], (4.0, 6.0));
    assert_eq!(kept[2], (7.5, 10.0));
}

#[test]
fn test_video_speed_segment_duration_scaling() {
    let mut timeline = VideoTimeline::new(10.0);
    // Add 2x speed for 4 seconds from t=2.0 to t=6.0
    timeline.speeds.push(VideoSpeedSegment::new(2.0, 6.0, 2.0));

    let comp_duration = timeline.composition_duration();
    // 2s (normal) + (4s / 2.0 = 2s) + 4s (normal) = 8.0s
    assert!((comp_duration - 8.0).abs() < 0.001);
}

#[test]
fn test_timeline_compositor_cuts_zooms_censors() {
    let mut timeline = VideoTimeline::new(10.0);
    timeline.cuts.push(VideoCutSegment::new(3.0, 5.0));

    let censor_rect = Rect::new(0.2, 0.2, 0.4, 0.4);
    timeline.censors.push(VideoCensorSegment::new(
        6.0,
        9.0,
        censor_rect,
        CensorStyle::Solid,
    ));

    // Generate 10 frames, 1 per second
    let mut frames = Vec::new();
    let frame_w = 64;
    let frame_h = 64;
    for i in 0..10 {
        let pts = Duration::from_secs(i as u64);
        let rgba = vec![200u8; (frame_w * frame_h * 4) as usize];
        frames.push(RecordedVideoFrame {
            pts,
            width: frame_w,
            height: frame_h,
            rgba_data: rgba,
        });
    }

    let composited = TimelineCompositor::composite_frames(&frames, &timeline);

    // Frame at t=3 and t=4 and t=5 should be cut
    let pts_list: Vec<u64> = composited.iter().map(|f| f.pts.as_secs()).collect();
    assert!(!pts_list.contains(&3));
    assert!(!pts_list.contains(&4));
    assert!(pts_list.contains(&1));
    assert!(pts_list.contains(&7));

    // Check that censor was applied on frame at t=7
    let frame_7 = composited.iter().find(|f| f.pts.as_secs() == 7).unwrap();
    // Pixel inside censor rect (normalized 0.3, 0.3 -> px 19, 19) should be black [0, 0, 0, 255]
    let px_idx = (19 * 64 + 19) * 4;
    assert_eq!(frame_7.rgba_data[px_idx], 0);
    assert_eq!(frame_7.rgba_data[px_idx + 1], 0);
    assert_eq!(frame_7.rgba_data[px_idx + 2], 0);

    // Pixel outside censor rect (px 2, 2) should remain original 200
    let outside_idx = (2 * 64 + 2) * 4;
    assert_eq!(frame_7.rgba_data[outside_idx], 200);
}
