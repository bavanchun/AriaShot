use camerashot_media::{
    AudioConfig, AudioMixer, GifExporter, PtsAccumulator, RecordedVideoFrame, ScreenRecorder,
    VideoEncoderConfig,
};
use std::thread::sleep;
use std::time::Duration;

#[test]
fn test_pts_accumulator_monotonicity_and_pause_resume() {
    let mut pts = PtsAccumulator::new(60);
    assert_eq!(pts.recorded_duration(), Duration::ZERO);
    assert_eq!(pts.current_frame_index(), 0);
    assert!(!pts.is_paused());

    pts.start();
    sleep(Duration::from_millis(30));
    let t1 = pts.recorded_duration();
    assert!(t1 >= Duration::from_millis(25));

    // Pause recording
    pts.pause();
    assert!(pts.is_paused());
    sleep(Duration::from_millis(40));
    let t_paused = pts.recorded_duration();

    // While paused, recorded duration should not grow
    assert!(t_paused >= t1);
    assert!(t_paused < t1 + Duration::from_millis(15));

    // Resume recording
    pts.resume();
    assert!(!pts.is_paused());
    sleep(Duration::from_millis(30));
    let t2 = pts.recorded_duration();

    assert!(t2 > t1);
    // Frame index should be proportional to duration * fps
    let frame_idx = pts.current_frame_index();
    let expected_idx = (t2.as_secs_f64() * 60.0).floor() as u64;
    assert_eq!(frame_idx, expected_idx);
}

#[test]
fn test_audio_mixer_soft_clipping_and_gains() {
    let mixer = AudioMixer::new(AudioConfig::default());
    let mut out = Vec::new();

    // Normal mixing within range
    let sys = vec![0.3, 0.4, -0.2];
    let mic = vec![0.2, -0.1, 0.3];
    mixer.mix(&sys, &mic, &mut out);
    assert_eq!(out.len(), 3);
    for sample in &out {
        assert!(*sample >= -1.0 && *sample <= 1.0);
    }

    // Extreme mixing: sum > 1.0 should trigger smooth soft-clipping without harsh clipping or NaN
    let mut out_extreme = Vec::new();
    let loud_sys = vec![1.5, -2.0, 3.0];
    let loud_mic = vec![1.2, -1.8, 2.5];
    mixer.mix(&loud_sys, &loud_mic, &mut out_extreme);

    for sample in &out_extreme {
        assert!(!sample.is_nan());
        assert!(!sample.is_infinite());
        // Soft clipper bounds output
        assert!(*sample >= -1.0 && *sample <= 1.0);
    }

    // Test mute flags
    let mut mute_mixer = AudioMixer::new(AudioConfig::default());
    mute_mixer.mute_mic = true;
    let mut out_muted = Vec::new();
    mute_mixer.mix(&[0.5], &[0.9], &mut out_muted);
    // Should only have sys (0.5 passed through soft-clip)
    assert!(out_muted[0] < 0.6 && out_muted[0] > 0.4);
}

#[test]
fn test_screen_recorder_lifecycle_and_frame_capture() {
    let video_config = VideoEncoderConfig {
        width: 128,
        height: 128,
        fps: 30,
        bitrate_kbps: 1000,
        hardware_accelerated: false,
    };
    let audio_config = AudioConfig::default();
    let mut recorder = ScreenRecorder::new(video_config, audio_config);

    assert_eq!(recorder.state(), camerashot_media::RecordingState::Idle);
    assert!(!recorder.is_recording());

    recorder.start().expect("recorder should start");
    assert!(recorder.is_recording());

    // Generate dummy frame
    let frame_bytes = vec![255u8; 128 * 128 * 4];
    recorder
        .push_video_frame(frame_bytes.clone(), 128, 128)
        .expect("push frame should succeed");

    // Invalid frame dimension should fail cleanly
    let invalid_bytes = vec![0u8; 100];
    let err = recorder.push_video_frame(invalid_bytes, 128, 128);
    assert!(err.is_err());
    assert_eq!(recorder.dropped_frames(), 1);

    // Audio chunk push
    let sys_chunk = vec![0.1f32; 100];
    let mic_chunk = vec![0.05f32; 100];
    recorder
        .push_audio_chunk(Some(&sys_chunk), Some(&mic_chunk))
        .expect("audio push should succeed");

    // Stop recording
    let session = recorder.stop().expect("recorder should stop");
    assert_eq!(session.frame_count(), 1);
    assert_eq!(session.mixed_audio.len(), 100);
    assert_eq!(session.frames[0].width, 128);
    assert_eq!(session.frames[0].height, 128);
}

#[test]
fn test_gif_exporter_synthetic_animation() {
    let width = 32;
    let height = 32;
    let num_frames = 4;
    let mut frames = Vec::new();

    for i in 0..num_frames {
        let r = (i * 60) as u8;
        let g = 100_u8;
        let b = (255 - i * 60) as u8;

        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
        for _ in 0..(width * height) {
            rgba.extend_from_slice(&[r, g, b, 255]);
        }

        frames.push(RecordedVideoFrame {
            pts: Duration::from_millis((i * 100) as u64),
            width,
            height,
            rgba_data: rgba,
        });
    }

    let gif_data = GifExporter::export_gif(&frames, 10).expect("GIF export should succeed");
    assert!(!gif_data.is_empty());
    // Verify standard GIF magic bytes: "GIF89a" or "GIF87a"
    assert!(gif_data.starts_with(b"GIF89a") || gif_data.starts_with(b"GIF87a"));
}
