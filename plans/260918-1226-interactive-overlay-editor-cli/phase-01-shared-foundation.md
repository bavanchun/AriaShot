---
phase: 1
title: "Shared Foundation: deps, clipboard, Vision OCR, redact pipeline"
status: done
priority: P1
effort: "1d"
dependencies: []
---

# Phase 1: Shared Foundation

## Overview

Tạo mọi thứ mà 3 bề mặt (overlay, editor, CLI) cùng dùng, để Phase 2/3/5 chạy song song mà không đụng file của nhau. Đây là phase duy nhất sửa root `Cargo.toml`.

## Key Insights (từ scout)

- `crates/camerashot-ocr/src/vision_macos.rs` và `onnx_linux.rs` là stub trả `Ok(Vec::new())` → `--ocr`, `--redact`, nút OCR/Auto-Redact đều vô dụng nếu không sửa.
- `PiiRedactor::build_redactions_for_blocks(&[(&str, Rect)], tool, padding) -> (Uuid, Vec<Annotation>)` đã có; `AnnotationRenderer::render_annotations` đã render Pixelate/Blur/FilledRectangle. Chỉ thiếu keo nối OCR → redactor → renderer.
- Chưa có clipboard ở đâu cả. Pixmap tiny-skia là premultiplied RGBA; ảnh capture có alpha=255 nên bytes trùng straight RGBA.
- `.gitignore` đang có thay đổi chưa commit thêm `*.png` → fixture test PNG sẽ bị ignore nếu không thêm ngoại lệ.

## Requirements

- Workspace deps mới trong `[workspace.dependencies]`: `winit = "0.30"`, `softbuffer = "0.4"`, `arboard = "3"`, `slint = "1"`, `slint-build = "1"`, `clap = { version = "4", features = ["derive"] }`, `chrono = { version = "0.4", default-features = false, features = ["clock"] }` (tên file theo timestamp cho editor + CLI), `objc2`, `objc2-foundation`, `objc2-vision` (phiên bản cùng họ, kiểm tra trên crates.io lúc làm; bật feature cần cho `VNRecognizeTextRequest`, `VNImageRequestHandler`, `VNRecognizedTextObservation`).
- Thêm member `crates/camerashot-cli`.
- `camerashot-platform::clipboard`: `copy_rgba_image(width: u32, height: u32, rgba: &[u8]) -> Result<(), PlatformError>` và `copy_text(text: &str) -> Result<(), PlatformError>`.
- `AppleVisionOcr::recognize_text` thật: trả `OcrBlock { text, confidence, bounds }` với `bounds` tính bằng pixel, gốc trên-trái của ảnh đầu vào.
- `RapidOnnxOcr::recognize_text` trả `Err("OCR is not supported on Linux yet ...")` thay vì rỗng.
- `camerashot-ocr::pipeline`: 
  - `redactions_from_blocks(redactor: &PiiRedactor, blocks: &[OcrBlock], tool: AnnotationTool, padding: f64) -> (Uuid, Vec<Annotation>)` (thuần, cho phép CLI OCR một lần rồi dùng blocks cho cả `--ocr` và `--redact`)
  - `detect_redactions(engine: &OcrEngine, rgba: &[u8], w: u32, h: u32, tool: AnnotationTool, padding: f64) -> Result<(Uuid, Vec<Annotation>), String>` = `recognize_text` + `redactions_from_blocks`
  - `blocks_to_text(blocks: &[OcrBlock]) -> String` (sắp theo y rồi x, mỗi block một dòng)
  - `mask_pii_in_text(detector: &PiiDetector, text: &str) -> String` (thay đoạn match bằng `█`, dùng cho CLI `--ocr --redact`).
- Giữ nguyên chữ ký public hiện có của `OcrEngine`, `PiiRedactor`, `CaptureBackend`.

## Architecture

```
OcrEngine (Vision on macOS | Err on Linux)
      │ Vec<OcrBlock{text, bounds(px, top-left)}>
      ▼
pipeline::detect_redactions ──► PiiRedactor::build_redactions_for_blocks ──► (batch Uuid, Vec<Annotation>)
                                                                                   │
                        editor: UndoAction::Batch + re-render  ◄───────────────────┤
                        cli:    AnnotationRenderer::render_annotations(pixmap)  ◄──┘
platform::clipboard ◄── overlay (Enter/⌘C), editor (Copy), cli (--clipboard)
```

Vision: encode RGBA → PNG bằng crate `image` → `NSData` → `VNImageRequestHandler::initWithData_options` → `VNRecognizeTextRequest` (recognitionLevel = Accurate, usesLanguageCorrection = true) → mỗi `VNRecognizedTextObservation.topCandidates(1)` lấy string + confidence; `boundingBox` chuẩn hoá gốc dưới-trái → pixel: `x = bx*w`, `y = (1 - by - bh)*h`, `w = bw*w`, `h = bh*h`. Tránh interop `CGImage` giữa `core-graphics` và `objc2`.

## Related Code Files

- Modify: `ariashot/Cargo.toml` (members + workspace deps)
- Modify: `.gitignore` (thêm `!ariashot/crates/**/tests/fixtures/*.png`)
- Modify: `ariashot/crates/camerashot-platform/Cargo.toml` (+`arboard`), `src/lib.rs` (`pub mod clipboard;`)
- Create: `ariashot/crates/camerashot-platform/src/clipboard.rs`
- Modify: `ariashot/crates/camerashot-ocr/Cargo.toml` (+`tiny-skia` nếu pipeline cần, target macOS: `objc2*`), `src/lib.rs`, `src/vision_macos.rs`, `src/onnx_linux.rs`
- Create: `ariashot/crates/camerashot-ocr/src/pipeline.rs`, `tests/fixtures/pii-sample.png`, `tests/pipeline_tests.rs`
- Create: `ariashot/crates/camerashot-cli/Cargo.toml` (deps: core, platform, ocr, clap, tiny-skia, image, tracing, tracing-subscriber; `[[bin]] name = "ariashot"`), `ariashot/crates/camerashot-cli/src/main.rs` chỉ gồm `fn main() {}` để workspace build được. Đây là điểm bàn giao: sau Phase 1, toàn bộ `crates/camerashot-cli/**` (kể cả `Cargo.toml`) thuộc Phase 5.

## Implementation Steps

1. Root `Cargo.toml`: thêm member + deps. `cargo metadata` để xác nhận phân giải phiên bản.
2. `.gitignore`: thêm dòng ngoại lệ fixture (sau `*.png`).
3. `clipboard.rs`: dùng `arboard::Clipboard::new()?.set_image(ImageData { width, height, bytes: Cow::Borrowed(rgba) })`; map lỗi sang `PlatformError::CaptureFailed(format!("clipboard: {e}"))` (hoặc thêm variant `Clipboard(String)` — thêm variant là thay đổi public enum, chấp nhận vì enum không `#[non_exhaustive]` và mọi match hiện tại dùng `?`; grep xác nhận không có match exhaustive trước khi thêm). Linux: dùng `arboard::SetExtLinux::wait()` để giữ quyền sở hữu clipboard tới khi app khác lấy — ghi rõ trong doc comment rằng hàm block trên Linux.
4. `vision_macos.rs`: toàn bộ FFI sau `#[cfg(target_os = "macos")]`; bọc trong `objc2::rc::autoreleasepool`. Trả `Err(String)` khi request fail. Giữ struct `OcrBlock` ở đây (các module khác đang import từ đây).
5. `onnx_linux.rs`: trả `Err("OCR is not supported on Linux yet; use macOS".into())`.
6. `pipeline.rs` + export trong `lib.rs`.
7. Fixture: tạo `tests/fixtures/pii-sample.png` (nền trắng, chữ đen, chứa `contact: jane.doe@example.com` và `4111 1111 1111 1111`). Cách tạo: `magick -size 900x160 xc:white -font Helvetica -pointsize 36 -fill black -annotate +20+60 '...' -annotate +20+120 '...' pii-sample.png` (nếu thiếu ImageMagick, dùng `sips`/Preview hoặc screenshot thật; ghi lại lệnh đã dùng trong commit message body).
8. Tests `pipeline_tests.rs`:
   - `blocks_to_text` sắp xếp đúng thứ tự dòng (thuần Rust, mọi OS).
   - `mask_pii_in_text` che email/thẻ (mọi OS).
   - `#[cfg(target_os = "macos")]` Vision nhận ra `jane.doe@example.com` trong fixture và `detect_redactions` trả ≥1 annotation với `group_id` chung, bounds nằm trong ảnh.
   - `#[cfg(target_os = "linux")]` `OcrEngine::recognize_text` trả `Err`.
9. `cargo build --workspace && cargo test -p camerashot-ocr -p camerashot-platform`.

## Todo

- [x] Root Cargo.toml: member cli + workspace deps
- [x] .gitignore ngoại lệ fixture PNG
- [x] platform::clipboard (image + text)
- [x] Apple Vision OCR thật
- [x] Linux OCR trả lỗi rõ ràng
- [x] ocr::pipeline (detect_redactions, blocks_to_text, mask_pii_in_text)
- [x] Skeleton crate camerashot-cli build được
- [x] Fixture + pipeline tests xanh

## Success Criteria

- `cargo build --workspace` xanh; `cargo test -p camerashot-ocr -p camerashot-platform` xanh trên macOS, gồm test Vision thật trên fixture.
- Test cũ `crates/camerashot-ocr/tests/ocr_tests.rs` vẫn xanh (không đổi chữ ký).
- API công bố ở mục Requirements tồn tại đúng tên — Phase 2/3/5 code theo đúng chữ ký này.

## Risk Assessment

| Rủi ro | Giảm thiểu |
|---|---|
| Binding `objc2-vision` khác tên method giữa phiên bản | Đọc docs.rs của phiên bản khoá; nếu thiếu binding, dùng `objc2::msg_send!` trực tiếp cho vài selector cần thiết |
| Vision trả toạ độ gốc dưới-trái | Công thức lật trục ở Architecture + test assert bounds nằm trong ảnh và email ở nửa trên fixture |
| Thêm variant vào `PlatformError` phá match exhaustive downstream | `grep -rn "PlatformError::" ariashot/crates` trước; nếu có match exhaustive, dùng `CaptureFailed` thay vì thêm variant |
| Linux clipboard block | Doc comment + Phase 5 ghi hành vi vào `--help` |

## Security Considerations

- OCR hoàn toàn local (Vision on-device); không gửi ảnh ra mạng.
- Fixture chỉ chứa PII giả (`example.com`, số thẻ test Visa).

## Next Steps

Mở khoá Phase 2, 3, 5 chạy song song.
