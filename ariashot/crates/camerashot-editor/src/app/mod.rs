//! Slint UI bindings — connects sessions to `MainEditorWindow` callbacks.

pub mod image_session;
pub mod recording;
pub mod video_session;

slint::include_modules!();

use crate::app::image_session::ImageSession;
use crate::app::recording::{RecordingConfig, RecordingHandle};
use crate::app::video_session::VideoSession;
use camerashot_core::geometry::Point;
use camerashot_core::AnnotationTool;
use camerashot_media::RecordedVideoFrame;
use camerashot_ocr::{blocks_to_text, detect_redactions, OcrEngine};
use camerashot_platform::clipboard;
use chrono::Local;
use parking_lot::Mutex;
use std::path::PathBuf;
use std::sync::Arc;
use tiny_skia::Pixmap;

/// Convert a tiny-skia `Pixmap` to a `slint::Image` via `SharedPixelBuffer<Rgba8Pixel>`.
pub fn pixmap_to_slint_image(pixmap: &Pixmap) -> slint::Image {
    let w = pixmap.width();
    let h = pixmap.height();
    let mut buf = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(w, h);
    buf.make_mut_bytes().copy_from_slice(pixmap.data());
    slint::Image::from_rgba8(buf)
}

/// Refresh the canvas image displayed in the UI from the current session state.
fn refresh_canvas(ui: &MainEditorWindow, session: &ImageSession) {
    let composited = session.composite();
    ui.set_canvas_image(pixmap_to_slint_image(&composited));
    ui.set_image_width(session.width() as f32);
    ui.set_image_height(session.height() as f32);
    ui.set_can_undo(session.undo.can_undo());
    ui.set_can_redo(session.undo.can_redo());
}

/// Bind all image-mode callbacks on `MainEditorWindow` to an `ImageSession`.
///
/// Uses `Arc<Mutex<ImageSession>>` so worker threads can send results back via
/// `slint::invoke_from_event_loop` which requires `Send` closures.
///
/// `video_state` is passed so Copy/Save can branch on video mode.
pub fn bind_image_mode(
    ui: &MainEditorWindow,
    session: Arc<Mutex<ImageSession>>,
    video_state: Arc<Mutex<VideoState>>,
) {
    // Initial canvas render
    {
        let s = session.lock();
        refresh_canvas(ui, &s);
        ui.set_status_text(slint::SharedString::from("Ready"));
    }

    // ── Copy to clipboard ────────────────────────────────────────────
    {
        let session = session.clone();
        let vs = video_state.clone();
        let ui_weak = ui.as_weak();
        ui.on_copy_to_clipboard(move || {
            let ui = match ui_weak.upgrade() {
                Some(u) => u,
                None => return,
            };
            let (w, h, data) = if ui.get_is_video_mode() {
                // Video mode: copy current preview frame
                let state = vs.lock();
                if let Some(ref video) = state.video_session {
                    if let Some(frame) = video.render_at(video.current_time) {
                        (frame.width, frame.height, frame.rgba_data)
                    } else {
                        return;
                    }
                } else {
                    return;
                }
            } else {
                // Image mode
                let s = session.lock();
                let composited = s.composite();
                (
                    composited.width(),
                    composited.height(),
                    composited.data().to_vec(),
                )
            };
            match clipboard::copy_rgba_image(w, h, &data) {
                Ok(_) => {
                    ui.set_status_text(slint::SharedString::from("Copied to clipboard"));
                }
                Err(e) => {
                    tracing::error!("Clipboard copy failed: {e}");
                    ui.set_status_text(slint::SharedString::from(format!("Copy failed: {e}")));
                }
            }
        });
    }

    // ── Save image / Export GIF ───────────────────────────────────────
    {
        let session = session.clone();
        let vs = video_state.clone();
        let ui_weak = ui.as_weak();
        ui.on_save_image(move || {
            let ui = match ui_weak.upgrade() {
                Some(u) => u,
                None => return,
            };
            if ui.get_is_video_mode() {
                // Video mode: export GIF on worker thread
                let state = vs.lock();
                if let Some(ref video) = state.video_session {
                    // Gather data synchronously for the worker
                    let gif_result = video.export_gif();
                    let path = default_gif_path();
                    let ui_weak2 = ui.as_weak();
                    ui.set_busy(true);
                    ui.set_status_text(slint::SharedString::from("Exporting GIF…"));
                    drop(state);

                    std::thread::spawn(move || {
                        let result = gif_result.and_then(|bytes| {
                            std::fs::write(&path, &bytes)
                                .map_err(camerashot_media::MediaError::Io)?;
                            Ok(path)
                        });
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_weak2.upgrade() {
                                match result {
                                    Ok(saved) => {
                                        let msg = format!("Saved: {}", saved.display());
                                        tracing::info!("{msg}");
                                        ui.set_status_text(slint::SharedString::from(msg));
                                    }
                                    Err(e) => {
                                        tracing::error!("GIF export failed: {e}");
                                        ui.set_status_text(slint::SharedString::from(format!(
                                            "Export failed: {e}"
                                        )));
                                    }
                                }
                                ui.set_busy(false);
                            }
                        });
                    });
                }
            } else {
                // Image mode: save PNG
                let s = session.lock();
                let path = s.default_save_path();
                match s.save_png(&path) {
                    Ok(saved) => {
                        let msg = format!("Saved: {}", saved.display());
                        tracing::info!("{msg}");
                        ui.set_status_text(slint::SharedString::from(msg));
                    }
                    Err(e) => {
                        tracing::error!("Save failed: {e}");
                        ui.set_status_text(slint::SharedString::from(format!("Save failed: {e}")));
                    }
                }
            }
        });
    }

    // ── Undo ─────────────────────────────────────────────────────────
    {
        let session = session.clone();
        let ui_weak = ui.as_weak();
        ui.on_undo(move || {
            let mut s = session.lock();
            if s.undo() {
                if let Some(ui) = ui_weak.upgrade() {
                    refresh_canvas(&ui, &s);
                    ui.set_status_text(slint::SharedString::from("Undo"));
                }
            }
        });
    }

    // ── Redo ─────────────────────────────────────────────────────────
    {
        let session = session.clone();
        let ui_weak = ui.as_weak();
        ui.on_redo(move || {
            let mut s = session.lock();
            if s.redo() {
                if let Some(ui) = ui_weak.upgrade() {
                    refresh_canvas(&ui, &s);
                    ui.set_status_text(slint::SharedString::from("Redo"));
                }
            }
        });
    }

    // ── Zoom controls ────────────────────────────────────────────────
    {
        let ui_weak = ui.as_weak();
        ui.on_zoom_in(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let z = (ui.get_zoom_level() * 1.25).min(8.0);
                ui.set_zoom_level(z);
            }
        });
    }
    {
        let ui_weak = ui.as_weak();
        ui.on_zoom_out(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let z = (ui.get_zoom_level() / 1.25).max(0.1);
                ui.set_zoom_level(z);
            }
        });
    }
    {
        let ui_weak = ui.as_weak();
        ui.on_zoom_actual(move || {
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_zoom_level(1.0);
            }
        });
    }
    {
        let session = session.clone();
        let ui_weak = ui.as_weak();
        ui.on_zoom_fit(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let s = session.lock();
                // Approximate fit: use logical window size minus toolbar/status
                let avail_w = 1200.0_f32;
                let avail_h = 800.0 - 44.0 - 24.0; // toolbar + status bar
                let iw = s.width() as f32;
                let ih = s.height() as f32;
                if iw > 0.0 && ih > 0.0 {
                    let z = (avail_w / iw).min(avail_h / ih).clamp(0.1, 8.0);
                    ui.set_zoom_level(z);
                }
            }
        });
    }

    // ── OCR (worker thread) ──────────────────────────────────────────
    {
        let session = session.clone();
        let ui_weak = ui.as_weak();
        ui.on_perform_ocr(move || {
            let (w, h, rgba) = {
                let s = session.lock();
                (s.width(), s.height(), s.base_rgba().to_vec())
            };

            let ui_weak = ui_weak.clone();

            // Set busy
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_busy(true);
                ui.set_status_text(slint::SharedString::from("Running OCR…"));
            }

            std::thread::spawn(move || {
                let engine = OcrEngine::new();
                let result = engine.recognize_text(&rgba, w, h);

                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_weak.upgrade() {
                        match result {
                            Ok(blocks) => {
                                let text = blocks_to_text(&blocks);
                                ui.set_ocr_text(slint::SharedString::from(&text));
                                ui.set_ocr_panel_visible(true);
                                ui.set_status_text(slint::SharedString::from(format!(
                                    "OCR: {} block(s) recognized",
                                    blocks.len()
                                )));
                            }
                            Err(e) => {
                                tracing::error!("OCR failed: {e}");
                                ui.set_status_text(slint::SharedString::from(format!(
                                    "OCR failed: {e}"
                                )));
                            }
                        }
                        ui.set_busy(false);
                    }
                });
            });
        });
    }

    // ── Copy OCR text ────────────────────────────────────────────────
    {
        let ui_weak = ui.as_weak();
        ui.on_copy_ocr_text(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let text = ui.get_ocr_text();
                match clipboard::copy_text(&text) {
                    Ok(_) => {
                        ui.set_status_text(slint::SharedString::from("OCR text copied"));
                    }
                    Err(e) => {
                        ui.set_status_text(slint::SharedString::from(format!("Copy failed: {e}")));
                    }
                }
            }
        });
    }

    // ── Auto-Redact (worker thread) ──────────────────────────────────
    {
        let session = session.clone();
        let ui_weak = ui.as_weak();
        ui.on_auto_redact(move || {
            let (w, h, rgba) = {
                let s = session.lock();
                (s.width(), s.height(), s.base_rgba().to_vec())
            };

            let session = session.clone();
            let ui_weak = ui_weak.clone();

            // Set busy
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_busy(true);
                ui.set_status_text(slint::SharedString::from("Running Auto-Redact…"));
            }

            std::thread::spawn(move || {
                let result = detect_redactions(
                    &OcrEngine::new(),
                    &rgba,
                    w,
                    h,
                    AnnotationTool::Pixelate,
                    4.0,
                );

                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_weak.upgrade() {
                        match result {
                            Ok((group_id, annotations)) => {
                                let count = {
                                    let mut s = session.lock();
                                    let n = s.apply_redaction_batch(group_id, annotations);
                                    refresh_canvas(&ui, &s);
                                    n
                                };
                                if count > 0 {
                                    ui.set_status_text(slint::SharedString::from(format!(
                                        "Redacted {} item(s)",
                                        count
                                    )));
                                } else {
                                    ui.set_status_text(slint::SharedString::from("No PII found"));
                                }
                            }
                            Err(e) => {
                                tracing::error!("Auto-redact failed: {e}");
                                ui.set_status_text(slint::SharedString::from(format!(
                                    "Auto-redact failed: {e}"
                                )));
                            }
                        }
                        ui.set_busy(false);
                    }
                });
            });
        });
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Video mode helpers + binding
// ═══════════════════════════════════════════════════════════════════════

/// Convert a `RecordedVideoFrame` to a `slint::Image`.
fn frame_to_slint_image(frame: &RecordedVideoFrame) -> slint::Image {
    let mut buf = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(frame.width, frame.height);
    buf.make_mut_bytes().copy_from_slice(&frame.rgba_data);
    slint::Image::from_rgba8(buf)
}

/// Refresh the canvas with a composited frame at `t` from the video session.
fn refresh_video_frame(ui: &MainEditorWindow, vs: &VideoSession) {
    if let Some(frame) = vs.render_at(vs.current_time) {
        ui.set_canvas_image(frame_to_slint_image(&frame));
        ui.set_image_width(frame.width as f32);
        ui.set_image_height(frame.height as f32);
    }
    let (cuts, zooms, censors) = vs.counts();
    ui.set_cut_count(cuts as i32);
    ui.set_zoom_count(zooms as i32);
    ui.set_censor_count(censors as i32);
    ui.set_current_time(vs.current_time as f32);
}

/// Enter video mode: set timeline properties, render first frame.
fn enter_video_mode(ui: &MainEditorWindow, vs: &VideoSession) {
    ui.set_is_video_mode(true);
    ui.set_total_duration(vs.duration() as f32);
    ui.set_current_time(0.0);
    ui.set_is_playing(false);
    ui.set_ocr_panel_visible(false);
    refresh_video_frame(ui, vs);
    ui.set_status_text(slint::SharedString::from(format!(
        "Video: {} frames, {:.1}s",
        vs.session.frames.len(),
        vs.duration()
    )));
}

/// Default GIF save path: `~/Movies/AriaShot/ariashot-YYYYMMDD-HHMMSS.gif`.
fn default_gif_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let dir = PathBuf::from(home).join("Movies").join("AriaShot");
    let _ = std::fs::create_dir_all(&dir);
    let ts = Local::now().format("%Y%m%d-%H%M%S");
    dir.join(format!("ariashot-{ts}.gif"))
}

/// Shared video state — wrapped in `Arc<Mutex<_>>` for cross-callback access.
#[derive(Default)]
pub struct VideoState {
    pub video_session: Option<VideoSession>,
    pub recording: Option<RecordingHandle>,
    pub playback_timer: Option<slint::Timer>,
}

/// Bind video-mode callbacks: Record/Stop, playback, seek, edit, export, exit.
pub fn bind_video_mode(
    ui: &MainEditorWindow,
    video_state: Arc<Mutex<VideoState>>,
    image_session: Arc<Mutex<ImageSession>>,
) {
    // ── Toggle Record / Stop ─────────────────────────────────────────
    {
        let vs = video_state.clone();
        let ui_weak = ui.as_weak();
        ui.on_toggle_record(move || {
            let mut state = vs.lock();
            if state.recording.is_some() {
                // Stop recording
                let handle = state.recording.take().unwrap();
                match handle.stop_and_join() {
                    Ok(rec_session) => {
                        let fps = 10u32;
                        let video = VideoSession::from_recording(rec_session, fps);
                        if let Some(ui) = ui_weak.upgrade() {
                            enter_video_mode(&ui, &video);
                            ui.set_is_recording(false);
                        }
                        state.video_session = Some(video);
                    }
                    Err(e) => {
                        tracing::error!("Recording failed: {e}");
                        if let Some(ui) = ui_weak.upgrade() {
                            ui.set_is_recording(false);
                            ui.set_status_text(slint::SharedString::from(format!(
                                "Recording failed: {e}"
                            )));
                        }
                    }
                }
            } else {
                // Start recording
                let config = RecordingConfig::default();
                match camerashot_platform::create_default_backend() {
                    Ok(backend) => match backend.enumerate_displays() {
                        Ok(displays) => {
                            let primary = displays
                                .iter()
                                .find(|d| d.is_primary)
                                .or_else(|| displays.first());
                            if let Some(disp) = primary {
                                let handle = RecordingHandle::start(backend, disp.id, config);
                                state.recording = Some(handle);
                                if let Some(ui) = ui_weak.upgrade() {
                                    ui.set_is_recording(true);
                                    ui.set_status_text(slint::SharedString::from("Recording…"));
                                }
                            }
                        }
                        Err(e) => {
                            if let Some(ui) = ui_weak.upgrade() {
                                ui.set_status_text(slint::SharedString::from(format!(
                                    "Display error: {e}"
                                )));
                            }
                        }
                    },
                    Err(e) => {
                        if let Some(ui) = ui_weak.upgrade() {
                            ui.set_status_text(slint::SharedString::from(format!(
                                "Backend error: {e}"
                            )));
                        }
                    }
                }
            }
        });
    }

    // ── Recording elapsed timer (updates status every 250ms) ─────────
    {
        let vs = video_state.clone();
        let ui_weak = ui.as_weak();
        let rec_timer = slint::Timer::default();
        rec_timer.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_millis(250),
            move || {
                let state = vs.lock();
                if let Some(ref handle) = state.recording {
                    let elapsed = handle.elapsed().as_secs_f64();
                    if let Some(ui) = ui_weak.upgrade() {
                        ui.set_status_text(slint::SharedString::from(format!(
                            "Recording… {:.1}s",
                            elapsed
                        )));
                    }
                    // Check if auto-stopped
                    if handle.is_finished() {
                        drop(state);
                        // The actual stop is handled by toggle-record
                    }
                }
            },
        );
        // Store timer so it stays alive
        video_state.lock().playback_timer = Some(rec_timer);
    }

    // ── Toggle Playback ──────────────────────────────────────────────
    {
        let vs = video_state.clone();
        let ui_weak = ui.as_weak();
        ui.on_toggle_playback(move || {
            let mut state = vs.lock();
            if let Some(ref ui) = ui_weak.upgrade() {
                let playing = ui.get_is_playing();
                if playing {
                    // Pause
                    ui.set_is_playing(false);
                } else if state.video_session.is_some() {
                    // Play
                    ui.set_is_playing(true);
                    let vs_clone = vs.clone();
                    let ui_w = ui.as_weak();
                    let fps = state.video_session.as_ref().map(|v| v.fps).unwrap_or(10);
                    let dt = 1.0 / fps as f64;

                    let timer = slint::Timer::default();
                    timer.start(
                        slint::TimerMode::Repeated,
                        std::time::Duration::from_secs_f64(dt),
                        move || {
                            let mut st = vs_clone.lock();
                            if let Some(ref mut video) = st.video_session {
                                match video.next_play_time(video.current_time, dt) {
                                    Some(next_t) => {
                                        video.current_time = next_t;
                                        if let Some(ui) = ui_w.upgrade() {
                                            refresh_video_frame(&ui, video);
                                        }
                                    }
                                    None => {
                                        // Reached end
                                        if let Some(ui) = ui_w.upgrade() {
                                            ui.set_is_playing(false);
                                        }
                                    }
                                }
                            }
                        },
                    );
                    state.playback_timer = Some(timer);
                }
            }
        });
    }

    // ── Seek ─────────────────────────────────────────────────────────
    {
        let vs = video_state.clone();
        let ui_weak = ui.as_weak();
        ui.on_seek(move |t| {
            let mut state = vs.lock();
            if let Some(ref mut video) = state.video_session {
                video.current_time = (t as f64).clamp(0.0, video.duration());
                if let Some(ui) = ui_weak.upgrade() {
                    refresh_video_frame(&ui, video);
                }
            }
        });
    }

    // ── Add Cut ──────────────────────────────────────────────────────
    {
        let vs = video_state.clone();
        let ui_weak = ui.as_weak();
        ui.on_add_cut(move || {
            let mut state = vs.lock();
            if let Some(ref mut video) = state.video_session {
                video.add_cut(video.current_time);
                if let Some(ui) = ui_weak.upgrade() {
                    refresh_video_frame(&ui, video);
                    ui.set_status_text(slint::SharedString::from("Added cut"));
                }
            }
        });
    }

    // ── Add Zoom ─────────────────────────────────────────────────────
    {
        let vs = video_state.clone();
        let ui_weak = ui.as_weak();
        ui.on_add_zoom(move || {
            let mut state = vs.lock();
            if let Some(ref mut video) = state.video_session {
                video.add_zoom(video.current_time);
                if let Some(ui) = ui_weak.upgrade() {
                    refresh_video_frame(&ui, video);
                    ui.set_status_text(slint::SharedString::from("Added zoom"));
                }
            }
        });
    }

    // ── Add Censor ───────────────────────────────────────────────────
    {
        let vs = video_state.clone();
        let ui_weak = ui.as_weak();
        ui.on_add_censor(move || {
            let mut state = vs.lock();
            if let Some(ref mut video) = state.video_session {
                video.add_censor(video.current_time);
                if let Some(ui) = ui_weak.upgrade() {
                    refresh_video_frame(&ui, video);
                    ui.set_status_text(slint::SharedString::from("Added censor"));
                }
            }
        });
    }

    // ── Video Click (set focus point) ────────────────────────────────
    {
        let vs = video_state.clone();
        ui.on_video_clicked(move |x, y| {
            let mut state = vs.lock();
            if let Some(ref mut video) = state.video_session {
                // x, y are logical pixels within the viewport image
                let iw = video
                    .session
                    .frames
                    .first()
                    .map(|f| f.width as f64)
                    .unwrap_or(1.0);
                let ih = video
                    .session
                    .frames
                    .first()
                    .map(|f| f.height as f64)
                    .unwrap_or(1.0);
                let nx = (x as f64 / iw).clamp(0.0, 1.0);
                let ny = (y as f64 / ih).clamp(0.0, 1.0);
                video.focus = Point::new(nx, ny);
            }
        });
    }

    // ── Exit Video Mode ──────────────────────────────────────────────
    {
        let vs = video_state.clone();
        let is = image_session.clone();
        let ui_weak = ui.as_weak();
        ui.on_exit_video(move || {
            let mut state = vs.lock();
            state.video_session = None;
            state.playback_timer = None;
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_is_video_mode(false);
                ui.set_is_playing(false);
                ui.set_is_recording(false);
                let s = is.lock();
                refresh_canvas(&ui, &s);
                ui.set_status_text(slint::SharedString::from("Returned to image mode"));
            }
        });
    }
}
