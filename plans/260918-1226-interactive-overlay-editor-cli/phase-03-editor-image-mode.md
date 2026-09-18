---
phase: 3
title: "Slint Editor — Image Mode"
status: todo
priority: P1
effort: "1d"
dependencies: [1]
---

# Phase 3: Slint Editor — Image Mode

## Overview

Nối `ui/main_editor.slint` vào Rust lần đầu: thêm `slint` + `build.rs`, hiển thị ảnh thật trong viewport có zoom/pan, và nối mọi callback image mode (Copy, Save, OCR, Auto-Redact, Undo/Redo). Phase này cũng định nghĩa cấu trúc `src/app/` để Phase 4 cắm video mode vào.

## Key Insights

- `camerashot-editor/Cargo.toml` chưa có `slint`; không có `build.rs` → `.slint` chưa từng compile. `main.rs` chỉ in log timeline.
- `main_editor.slint` đã có: `in property <bool> is-video-mode`, `in property <float> zoom-level`, callbacks `copy-to-clipboard`, `save-image`, `auto-redact`, `perform-ocr`; viewport chỉ là Text placeholder; chưa có nút Save/Undo/Redo, chưa có property ảnh.
- `UndoStack` (core) có `push(UndoAction::Batch{..})`, `undo/redo(&mut Vec<Annotation>)` — dùng trực tiếp cho atomic undo của auto-redact.
- `AnnotationRenderer::render_annotations(&mut PixmapMut, &[Annotation])` render lên bản sao ảnh gốc → ảnh hiển thị.

## Requirements

Functional:
- CLI của binary editor: `camerashot-editor [IMAGE_PATH]`. Có path → load bằng crate `image` (PNG/JPEG/WebP). Không có → chụp display chính qua `create_default_backend`.
- Viewport: `Flickable` chứa `Image` với kích thước = kích thước ảnh × `zoom-level`; zoom bằng nút `−`, `+`, `Fit`, `100%` và ⌘/Ctrl + wheel (giới hạn 0.1–8.0). Pan bằng Flickable.
- Copy: ảnh đã composite (gốc + annotations) → `platform::clipboard::copy_rgba_image`.
- Save: ghi PNG. Có input path → `<stem>-edited.png` cạnh file gốc; chụp màn hình → `~/Pictures/AriaShot/ariashot-YYYYMMDD-HHMMSS.png` (tạo thư mục nếu thiếu). Hiển thị path trong status bar.
- OCR: chạy `OcrEngine` trên worker thread (`std::thread::spawn` + `slint::invoke_from_event_loop`), hiện panel bên phải với text (`ocr::pipeline::blocks_to_text`) + nút "Copy text" (`clipboard::copy_text`). Lỗi (vd. Linux unsupported) hiện ở status bar.
- Auto-Redact: `ocr::pipeline::detect_redactions(.., AnnotationTool::Pixelate, 4.0)` trên worker thread; nhận về → thêm annotations + `UndoAction::Batch{group_id, actions}` → re-render. Status: "Redacted N item(s)". 0 match → "No PII found".
- Undo/Redo: nút + phím ⌘Z / ⇧⌘Z (Ctrl trên Linux) qua `FocusScope`. Undo một lần gỡ toàn bộ batch redact.
- Trong lúc OCR/redact chạy: `busy` property khoá các nút hành động, hiện "Working…".

Non-functional: UI không đơ khi OCR; ảnh 5K render lại < 200ms ở release.

Non-goals: vẽ annotation thủ công bằng chuột trong editor (18 tool) — không nằm trong yêu cầu; hộp thoại Save As (dùng đường dẫn mặc định ở trên).

## Architecture

```
main.rs ── parse args ── ImageSession::load(path) | ImageSession::capture()
        └─ MainEditorWindow::new() ── app::bind_image_mode(&ui, Rc<RefCell<ImageSession>>) ── ui.run()

app/image_session.rs  (thuần Rust, không phụ thuộc Slint → test được)
  ImageSession { base: Pixmap, annotations: Vec<Annotation>, undo: UndoStack, source_path: Option<PathBuf> }
  fn composite(&self) -> Pixmap
  fn apply_redaction_batch(&mut self, group: Uuid, anns: Vec<Annotation>) -> usize
  fn undo(&mut self) -> bool; fn redo(&mut self) -> bool
  fn default_save_path(&self, now) -> PathBuf; fn save_png(&self, path) -> Result<..>

app/mod.rs  (keo Slint)
  fn pixmap_to_slint_image(&Pixmap) -> slint::Image   // SharedPixelBuffer<Rgba8Pixel>
  fn bind_image_mode(ui, session)                       // set canvas-image, đăng ký callbacks
```

Contract Slint mới (Phase 3 thêm vào `MainEditorWindow`): `in property <image> canvas-image`, `in property <length> image-width/height`, `in-out property <float> zoom-level`, `in property <string> status-text`, `in property <bool> busy`, `in property <string> ocr-text`, `in property <bool> ocr-panel-visible`, callbacks `save-image()`, `undo()`, `redo()`, `copy-ocr-text()`, `zoom-in()`, `zoom-out()`, `zoom-fit()`, `zoom-actual()`. Giữ nguyên 4 callback và 2 property hiện có.

## Related Code Files

- Modify: `ariashot/crates/camerashot-editor/Cargo.toml` (+`slint`, `[build-dependencies] slint-build`)
- Create: `ariashot/crates/camerashot-editor/build.rs` (`slint_build::compile("ui/main_editor.slint")`)
- Modify: `ariashot/crates/camerashot-editor/ui/main_editor.slint`
- Rewrite: `ariashot/crates/camerashot-editor/src/main.rs`
- Modify: `ariashot/crates/camerashot-editor/src/lib.rs` (`pub mod app;`)
- Create: `ariashot/crates/camerashot-editor/src/app/mod.rs`, `src/app/image_session.rs`
- Create: `ariashot/crates/camerashot-editor/tests/image_session_tests.rs`
- Không sửa: `ui/video_timeline.slint`, `compositor.rs`, `timeline_ctrl.rs`, `tests/editor_tests.rs` (Phase 4 / hiện có)

Lưu ý: `slint::include_modules!()` đặt trong `src/app/mod.rs` (lib) để cả binary và Phase 4 dùng chung kiểu `MainEditorWindow`.

## Implementation Steps

1. Deps + `build.rs`; `cargo build -p camerashot-editor` phải compile được `.slint` hiện có trước khi sửa UI (bắt lỗi cú pháp sớm).
2. `image_session.rs` + tests trước (TDD-ish): composite kích thước đúng; `apply_redaction_batch` rồi `undo()` → annotations rỗng, `redo()` → trả lại đủ N; `default_save_path` đúng hai nhánh.
3. Sửa `main_editor.slint`: thêm property/callback contract ở trên; viewport = `Flickable { Image { source: canvas-image; width: image-width * zoom-level; height: ...; image-rendering: pixelated khi zoom ≥ 2 } }`; thanh toolbar thêm Save, Undo, Redo, nhóm zoom; panel OCR bên phải (`if ocr-panel-visible`); status bar dưới; `FocusScope` bắt phím.
4. `app/mod.rs`: `bind_image_mode` đăng ký callbacks; worker thread cho OCR/redact dùng `ui.as_weak()` + `slint::invoke_from_event_loop`; mọi lỗi → `status-text`.
5. `main.rs`: parse `std::env::args` (1 tham số tuỳ chọn, không cần clap), init tracing, dựng session, bind, `run()`.
6. `cargo test -p camerashot-editor`; chạy thủ công với ảnh fixture `crates/camerashot-ocr/tests/fixtures/pii-sample.png`.

## Todo

- [ ] slint deps + build.rs, compile UI hiện có
- [ ] ImageSession + tests
- [ ] Mở rộng main_editor.slint (viewport, zoom, save/undo/redo, OCR panel, status, phím tắt)
- [ ] bind_image_mode + worker thread OCR/redact
- [ ] main.rs mới (path | capture)
- [ ] Manual E2E

## Success Criteria

- `cargo test -p camerashot-editor` xanh (test timeline cũ + `image_session_tests`).
- Thủ công: mở fixture → Auto-Redact che email + số thẻ (status "Redacted 2 item(s)" hoặc hơn) → ⌘Z gỡ toàn bộ trong 1 lần → ⇧⌘Z trả lại → Copy dán được vào Preview → Save tạo `pii-sample-edited.png` → OCR hiện text chứa `jane.doe@example.com` và Copy text hoạt động; zoom/pan mượt; UI không đơ khi OCR.

## Risk Assessment

| Rủi ro | Giảm thiểu |
|---|---|
| Slint backend (winit/femtovg vs skia) build nặng hoặc thiếu lib hệ thống trên Linux | Dùng feature mặc định của `slint`; ghi yêu cầu hệ thống vào README (Phase 6) |
| `Rc<RefCell<ImageSession>>` bị borrow lồng trong callback | Mỗi callback borrow ngắn, clone dữ liệu cần gửi sang worker trước khi spawn |
| Ảnh lớn → `SharedPixelBuffer` copy tốn thời gian | Chỉ rebuild image khi composite thay đổi |
| Xung đột với Phase 4 trên `main_editor.slint`/`app/mod.rs` | Phase 4 chỉ bắt đầu sau khi Phase 3 merge |

## Security Considerations

OCR/redact local. Save chỉ ghi vào cạnh file gốc hoặc `~/Pictures/AriaShot`, không ghi đè file gốc.

## Next Steps

Phase 4 thêm video mode vào cùng window.
