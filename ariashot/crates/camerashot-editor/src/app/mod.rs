//! Slint UI bindings — connects `ImageSession` state to `MainEditorWindow` callbacks.

pub mod image_session;

slint::include_modules!();

use crate::app::image_session::ImageSession;
use camerashot_core::AnnotationTool;
use camerashot_ocr::{blocks_to_text, detect_redactions, OcrEngine};
use camerashot_platform::clipboard;
use parking_lot::Mutex;
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
pub fn bind_image_mode(ui: &MainEditorWindow, session: Arc<Mutex<ImageSession>>) {
    // Initial canvas render
    {
        let s = session.lock();
        refresh_canvas(ui, &*s);
        ui.set_status_text(slint::SharedString::from("Ready"));
    }

    // ── Copy to clipboard ────────────────────────────────────────────
    {
        let session = session.clone();
        let ui_weak = ui.as_weak();
        ui.on_copy_to_clipboard(move || {
            let s = session.lock();
            let composited = s.composite();
            let w = composited.width();
            let h = composited.height();
            let data = composited.data().to_vec();
            drop(s);
            match clipboard::copy_rgba_image(w, h, &data) {
                Ok(_) => {
                    if let Some(ui) = ui_weak.upgrade() {
                        ui.set_status_text(slint::SharedString::from("Copied to clipboard"));
                    }
                }
                Err(e) => {
                    tracing::error!("Clipboard copy failed: {e}");
                    if let Some(ui) = ui_weak.upgrade() {
                        ui.set_status_text(slint::SharedString::from(format!("Copy failed: {e}")));
                    }
                }
            }
        });
    }

    // ── Save image ───────────────────────────────────────────────────
    {
        let session = session.clone();
        let ui_weak = ui.as_weak();
        ui.on_save_image(move || {
            let s = session.lock();
            let path = s.default_save_path();
            match s.save_png(&path) {
                Ok(saved) => {
                    let msg = format!("Saved: {}", saved.display());
                    tracing::info!("{msg}");
                    if let Some(ui) = ui_weak.upgrade() {
                        ui.set_status_text(slint::SharedString::from(msg));
                    }
                }
                Err(e) => {
                    tracing::error!("Save failed: {e}");
                    if let Some(ui) = ui_weak.upgrade() {
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
                    refresh_canvas(&ui, &*s);
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
                    refresh_canvas(&ui, &*s);
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
                    let z = (avail_w / iw).min(avail_h / ih).min(8.0).max(0.1);
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
                        ui.set_status_text(slint::SharedString::from(format!(
                            "Copy failed: {e}"
                        )));
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
                                    refresh_canvas(&ui, &*s);
                                    n
                                };
                                if count > 0 {
                                    ui.set_status_text(slint::SharedString::from(format!(
                                        "Redacted {} item(s)",
                                        count
                                    )));
                                } else {
                                    ui.set_status_text(slint::SharedString::from(
                                        "No PII found",
                                    ));
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
