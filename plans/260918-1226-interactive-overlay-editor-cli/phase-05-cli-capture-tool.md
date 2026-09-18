---
phase: 5
title: "CLI Capture Tool (ariashot)"
status: todo
priority: P1
effort: "0.75d"
dependencies: [1]
---

# Phase 5: CLI Capture Tool

## Overview

Crate `camerashot-cli`, binary `ariashot`: chụp màn hình không cần GUI, hỗ trợ `--full`, `--crop`, `--clipboard`, `--ocr`, `--redact`. Dùng lại platform backend, `platform::clipboard`, `ocr::pipeline`, `AnnotationRenderer`.

## Key Insights

- `CaptureBackend::capture_display(id)` và `capture_rect(Rect)` đã có; trên macOS `capture_rect` nhận rect theo **point logic** (hệ toạ độ desktop toàn cục của CoreGraphics) và trả pixel vật lý.
- `FrameBuffer::to_rgba8()` cho RGBA chặt → tạo `tiny_skia::Pixmap` (alpha=255 nên premultiplied = straight).
- Phase 1 cung cấp `redactions_from_blocks` để OCR đúng một lần rồi dùng kết quả cho cả in text và che PII.

## Requirements

Giao diện:

```
ariashot (--full | --crop X,Y,W,H) [--display ID] [-o, --output PATH]
         [--clipboard] [--ocr] [--redact] [--redact-style pixelate|blur|fill]
```

- `--full` và `--crop` loại trừ nhau, bắt buộc một (clap `ArgGroup` required). `--display` chỉ đi với `--full` (mặc định display chính).
- `--crop X,Y,W,H`: số thực, W,H > 0, parse bằng `value_parser` tự viết; sai → clap lỗi, exit 2.
- Thứ tự xử lý: capture → OCR (nếu `--ocr` hoặc `--redact`, chạy **một lần**) → che PII lên ảnh (nếu `--redact`) → xuất.
- Xuất:
  - Ghi file nếu có `-o`, hoặc nếu không có `--clipboard` lẫn `--ocr` (mặc định `./ariashot-YYYYMMDD-HHMMSS.png`). Tạo thư mục cha nếu thiếu; định dạng theo đuôi file (png/jpg/webp) qua crate `image`, mặc định PNG.
  - `--clipboard`: copy ảnh (đã che nếu `--redact`).
  - `--ocr`: in text ra **stdout** (`blocks_to_text`); nếu kèm `--redact` thì PII trong text được che bằng `mask_pii_in_text`.
  - Mọi thông báo trạng thái (path đã lưu, số mục đã che) ra **stderr** để pipe stdout sạch.
- Fail-closed: `--redact` mà OCR lỗi (vd. Linux) → **không** ghi file/clipboard ảnh chưa che, exit 3.
- Exit code: 0 thành công; 1 lỗi runtime (capture/quyền/IO/clipboard); 2 lỗi cú pháp (clap); 3 OCR không khả dụng/thất bại.
- `--help` ghi chú: toạ độ crop là point logic; trên Linux `--clipboard` block tới khi app khác lấy clipboard; OCR chỉ hỗ trợ macOS.

## Architecture

```
main.rs: let cli = Cli::parse(); std::process::exit(camerashot_cli::run(cli, &*create_default_backend()?, &mut stdout, &mut stderr))

lib.rs
  #[derive(Parser)] Cli { target: CaptureTarget(group), display, output, clipboard, ocr, redact, redact_style }
  parse_crop(&str) -> Result<Rect, String>
  RedactStyle -> AnnotationTool { Pixelate | Blur | FilledRectangle }
  run(cli, backend: &dyn CaptureBackend, out: &mut impl Write, err: &mut impl Write) -> i32
     capture() -> Pixmap
     if ocr||redact: blocks = OcrEngine::new().recognize_text(..) (Err → exit 3)
     if redact: (_, anns) = redactions_from_blocks(..); AnnotationRenderer::render_annotations(pixmap)
     outputs...
  CliError { Runtime(String)=1, Ocr(String)=3 }
```

Tách `run` nhận `&dyn CaptureBackend` + writer để test được với backend test trả ảnh fixture (đây là test double trong code test, không phải dữ liệu giả trong sản phẩm).

## Related Code Files

- Modify: `ariashot/crates/camerashot-cli/Cargo.toml` (đã tạo ở Phase 1; thêm dev-deps nếu cần: `tempfile`)
- Rewrite: `ariashot/crates/camerashot-cli/src/main.rs`
- Create: `ariashot/crates/camerashot-cli/src/lib.rs`
- Create: `ariashot/crates/camerashot-cli/tests/cli_tests.rs`
- Đọc (không sửa): `crates/camerashot-ocr/tests/fixtures/pii-sample.png`

## Implementation Steps

1. `lib.rs`: `Cli` + `parse_crop` + `RedactStyle`; tests parse trước.
2. `capture()`: `--full` → `enumerate_displays` tìm display (id hoặc primary, không có → exit 1 kèm danh sách id) → `capture_display`; `--crop` → `capture_rect`.
3. OCR/redact/output theo Requirements; `default_output_path(now)` dùng `chrono`.
4. `main.rs` mỏng: init tracing (mức `warn` mặc định, `RUST_LOG` để tăng), gọi `run`, `process::exit`.
5. Tests `cli_tests.rs`:
   - Parse: thiếu target → lỗi; `--full --crop ..` → lỗi; `--crop 10,20,300,200` → Rect đúng; `--crop 1,2,0,5` và `--crop a,b,c,d` → lỗi; `--display` với `--crop` → lỗi.
   - `run` với backend test trả `pii-sample.png`: `--full -o <tmp>/a.png` → file tồn tại, kích thước đúng, exit 0, stdout rỗng.
   - macOS: `--full --ocr` → stdout chứa `jane.doe@example.com`; `--full --ocr --redact -o r.png` → stdout không chứa email, có `█`; pixel trong bbox email của `r.png` khác ảnh gốc.
   - Linux: `--full --redact -o r.png` → exit 3 và `r.png` không tồn tại.
6. Thủ công: `cargo run -p camerashot-cli --release -- --full -o /tmp/a.png`, `-- --crop 0,0,800,600 --clipboard`, `-- --full --ocr | head`, `-- --full --redact -o /tmp/r.png`.

## Todo

- [ ] Cli (clap derive) + parse_crop + RedactStyle
- [ ] Capture full/display/crop
- [ ] OCR một lần, redact fail-closed, render annotations
- [ ] Xuất file (định dạng theo đuôi) / clipboard / stdout text
- [ ] Exit codes + stderr status
- [ ] Tests cli_tests
- [ ] Manual E2E

## Success Criteria

- `cargo test -p camerashot-cli` xanh.
- `ariashot --help` liệt kê đủ cờ + ghi chú; 4 lệnh thủ công ở bước 6 cho kết quả đúng; `ariashot --full --ocr > out.txt` chỉ có text trong file.

## Risk Assessment

| Rủi ro | Giảm thiểu |
|---|---|
| Toạ độ crop point vs pixel gây nhầm | Ghi rõ trong `--help` + README; test parse |
| Rò rỉ PII khi OCR lỗi | Fail-closed, test Linux xác nhận không ghi file |
| Tên binary `ariashot` trùng binary khác trong workspace | Grep `[[bin]]` các crate khác; hiện chỉ có `camerashot-overlay`, `camerashot-editor` |

## Security Considerations

`--redact` fail-closed; OCR on-device; không ghi file tạm ngoài output user chọn.

## Next Steps

Phase 6 kiểm thử tích hợp + README.
