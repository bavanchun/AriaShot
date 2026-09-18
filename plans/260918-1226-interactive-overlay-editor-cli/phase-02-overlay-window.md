---
phase: 2
title: "Interactive Overlay Window"
status: todo
priority: P1
effort: "1d"
dependencies: [1]
---

# Phase 2: Interactive Overlay Window

## Overview

Biến `camerashot-overlay` từ demo giả lập drag thành cửa sổ fullscreen thật: chụp màn hình chính → hiển thị ảnh đóng băng phủ toàn màn hình → chuột thật chọn vùng (có snap) → Loupe phóng 2x theo con trỏ → Enter/⌘C/nút Copy chép vùng chọn vào clipboard và thoát; Esc huỷ.

## Key Insights

- `OverlaySurface` (`src/surface.rs`) + `SelectionController` đã có logic chọn/snap/render; chỉ thiếu window + event loop + present.
- `Loupe::render` hiện chỉ vẽ một ô màu 16×16 ở tâm — chưa phải phóng đại.
- `FloatingToolbar` chỉ vẽ nền capsule, không có nút, không hit-test. `ToolbarAction` enum đã có `CopyToClipboard`, `Close`.
- `main.rs` đang có thay đổi chưa commit (lưu `test_capture.png`) — sẽ bị thay thế hoàn toàn; hành vi lưu file debug đó bỏ đi.
- Capture macOS trả pixel vật lý (Retina 2x); winit `PhysicalPosition` cùng hệ toạ độ khi cửa sổ phủ đúng monitor → map 1:1, không nhân scale.

## Requirements

Functional:
- Chụp display chính (`create_default_backend` + `capture_display`), mở cửa sổ không viền phủ đúng monitor đó, luôn trên cùng. macOS: dùng `WindowExtMacOS::set_simple_fullscreen(true)` (ẩn menu bar/dock, không tạo Space mới); Linux: `Fullscreen::Borderless(Some(monitor))`.
- Sự kiện: `CursorMoved` → `on_mouse_move`; `MouseInput Left Pressed/Released` → `on_mouse_down/up` (hoặc hit-test toolbar trước khi chuyển xuống controller khi đang `Selected`).
- Phím: `Escape` thoát không copy; `Enter` hoặc `⌘C`/`Ctrl+C` khi `Selected` → copy + thoát.
- Loupe 2x: lấy vùng vuông `(2R/2)×(2R/2)` pixel quanh con trỏ từ `base_image`, vẽ phóng 2x nearest-neighbor, clip trong hình tròn bán kính R=48, lưới pixel mờ, ô tâm viền nổi bật, giữ crosshair + viền, giữ logic lật vị trí khi sát mép. Xử lý biên ảnh (pixel ngoài ảnh vẽ trong suốt/đen).
- Loupe cũng hiển thị trong khi `Selecting` (theo con trỏ hiện tại) để căn mép chính xác — thay đổi nhỏ ở `render_frame`.
- Toolbar: vẽ 2 nút có icon vector (Copy, Close) và `FloatingToolbar::hit_test(selection, canvas_w, canvas_h, pt) -> Option<ToolbarAction>` dùng chung hàm tính layout với `render` (DRY).
- Copy: `export_selection_pixmap()` → `camerashot_platform::clipboard::copy_rgba_image`; log kích thước; lỗi clipboard in ra stderr và thoát mã ≠0.

Non-functional:
- Chỉ redraw khi state đổi (`request_redraw` sau event), không vòng lặp liên tục (`ControlFlow::Wait`).
- Present qua `softbuffer`: chuyển RGBA → `0x00RRGGBB` u32. Chạy thử bằng `--release` (debug quá chậm với 5K).

Non-goals: multi-monitor, chọn tool vẽ annotation trong overlay, text HUD (resolution số/HEX), mở editor từ overlay.

## Architecture

```
main.rs ── capture primary display ── OverlaySurface::new
   └─ EventLoop::run_app(OverlayApp)
        OverlayApp (window_app.rs, impl winit::ApplicationHandler)
          resumed: tạo Window + softbuffer Surface theo monitor
          window_event:
            CursorMoved ─► surface.on_mouse_move ─► request_redraw
            MouseInput  ─► toolbar hit_test? ─► action | surface.on_mouse_down/up
            KeyboardInput ─► Esc exit | Enter/⌘C copy+exit
            RedrawRequested ─► surface.render_frame(scratch Pixmap) ─► RGBA→u32 ─► buffer.present()
```

## Related Code Files

- Modify: `ariashot/crates/camerashot-overlay/Cargo.toml` (+`winit`, `softbuffer` workspace deps)
- Rewrite: `ariashot/crates/camerashot-overlay/src/main.rs`
- Create: `ariashot/crates/camerashot-overlay/src/window_app.rs`
- Modify: `src/lib.rs` (`pub mod window_app;`), `src/surface.rs` (Loupe khi Selecting), `src/hud/loupe.rs`, `src/hud/toolbar.rs`
- Modify: `ariashot/crates/camerashot-overlay/tests/overlay_tests.rs` (thêm test, không sửa test cũ)

## Implementation Steps

1. Cargo deps; `cargo build -p camerashot-overlay`.
2. `loupe.rs`: viết lại phần thân — giữ chữ ký `render(pixmap, bg_frame, cursor)`. Sinh `Pixmap` nhỏ kích thước 2R, fill từng ô 2×2 theo pixel nguồn, rồi `draw_pixmap` với `mask` tròn (`tiny_skia::Mask` từ path circle). Tách hàm thuần `sample_magnified(bg, cursor, radius) -> Pixmap` để test.
3. `toolbar.rs`: tách `fn layout(selection, canvas_w, canvas_h) -> ToolbarLayout { bar: Rect, buttons: [(ToolbarAction, Rect); 2] }`; `render` và `hit_test` cùng dùng `layout`. Icon: Copy = 2 hình chữ nhật chồng; Close = dấu X.
4. `surface.rs`: trong `render_frame`, khi `Selecting` vẽ Loupe tại `current`. Không đổi API public khác.
5. `window_app.rs`: `OverlayApp { surface, window: Option<Rc<Window>>, sb_surface, scratch: Pixmap, modifiers, exit_code }`. Tạo window trong `resumed` (winit 0.30 API). Chuyển đổi toạ độ: dùng trực tiếp `PhysicalPosition` (clamp vào kích thước ảnh).
6. Copy flow: `export_selection_pixmap` → `clipboard::copy_rgba_image(w, h, pixmap.data())` → `event_loop.exit()`.
7. `main.rs`: init `tracing_subscriber` (thêm dep nếu cần), capture, dựng surface, chạy app, trả exit code.
8. Tests (không cần display): 
   - `sample_magnified` với ảnh caro 4×4: pixel (i,j) nguồn xuất hiện ở 2×2 ô tương ứng.
   - Loupe gần mép (cursor (0,0)) không panic.
   - `hit_test` trả `CopyToClipboard` ở tâm nút Copy, `None` ngoài bar; nhất quán với `layout`.
9. Chạy thủ công: `cargo run -p camerashot-overlay --release`, kiểm tra bằng checklist bên dưới; dán vào Preview (⌘N from clipboard) để xác nhận ảnh.

## Todo

- [ ] Thêm deps winit/softbuffer
- [ ] Loupe 2x thật + test
- [ ] Toolbar layout/hit-test + icon + test
- [ ] Loupe khi Selecting
- [ ] OverlayApp (window, events, present)
- [ ] Copy to clipboard + exit; Esc huỷ
- [ ] main.rs mới
- [ ] Manual E2E trên macOS

## Success Criteria

- `cargo test -p camerashot-overlay` xanh (test cũ + mới).
- Thủ công (release, macOS): overlay hiện <300ms sau lệnh; menu bar/dock bị che; drag vẽ vùng chọn có dim + snap; Loupe phóng 2x bám con trỏ, lật khi sát mép; Enter/⌘C/nút Copy → thoát và Preview "New from Clipboard" ra đúng vùng chọn ở độ phân giải vật lý; Esc thoát không đổi clipboard.

## Risk Assessment

| Rủi ro | Giảm thiểu |
|---|---|
| Full-frame redraw 5K bằng CPU chậm (>30ms) | Chỉ redraw khi có event; cache sẵn bản u32 của ảnh nền và chỉ chuyển đổi khi cần; nếu vẫn chậm, tính dirty rect (vùng Loupe cũ ∪ mới ∪ vùng chọn) và chỉ chuyển đổi/present vùng đó (`present_with_damage`) |
| `set_simple_fullscreen` không phủ notch/menu bar đúng | Fallback `Fullscreen::Borderless`; ghi nhận hành vi trong báo cáo |
| Thiếu quyền Screen Recording → ảnh chỉ có wallpaper | Không đoán bằng heuristic; Phase 6 ghi hướng dẫn cấp quyền System Settings → Privacy → Screen Recording vào README |
| Premultiplied vs straight alpha | Ảnh capture alpha=255; ghi assert debug |

## Security Considerations

Ảnh chụp chỉ nằm trong RAM và clipboard; không ghi file tạm (bỏ `test_capture.png`).

## Next Steps

Phase 6 kiểm thử tích hợp + README.
