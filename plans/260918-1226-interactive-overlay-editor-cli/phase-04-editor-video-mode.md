---
phase: 4
title: "Slint Editor — Video Mode & Screen Recording"
status: done
priority: P1
effort: "1.5d"
dependencies: [3]
---

# Phase 4: Slint Editor — Video Mode & Screen Recording

## Overview

Cho editor khả năng quay màn hình (nút Record/Stop), chuyển sang video mode, phát lại có seek, thêm cut/zoom/censor qua `VideoTimelineView`, xem kết quả đã composite theo thời gian thực và export GIF. Dùng engine có sẵn: `ScreenRecorder`, `RecordingSession`, `VideoTimeline`, `TimelineCompositor`, `GifExporter`.

## Key Insights

- `ScreenRecorder::push_video_frame(rgba, w, h)` chỉ nhận frame; **không có vòng capture** — phải tự viết thread capture.
- Frame lưu RGBA thô trong RAM (`VideoEncoder` giữ `Vec<RecordedVideoFrame>`); 5K@30fps sẽ nổ RAM → bắt buộc downscale + giới hạn fps/thời lượng.
- `TimelineCompositor::composite_frames(&[frame], &timeline)` xử lý cut/zoom/censor cho 1 frame → dùng cho preview; `composite_session` + `GifExporter::export_gif(frames, fps)` cho export.
- `video_timeline.slint` có properties `current-time, total-duration, is-playing, cut-count, zoom-count, censor-count` và callbacks `toggle-playback, seek(float), add-cut, add-zoom, add-censor`, nhưng `MainEditorWindow` chưa chuyển tiếp chúng ra ngoài.
- Audio backend (`coreaudio.rs`, `pipewire.rs`) chưa bắt audio thật; GIF không có audio → audio là non-goal.

## Requirements

Functional:
- Nút `● Record` / `■ Stop` trên toolbar (ẩn khi `busy`). Record: quay display chính, 10 fps, downscale về rộng tối đa 960 px (giữ tỉ lệ, `image::imageops::resize` Triangle), tự dừng ở 30 s. Status hiện thời gian đã quay.
- Stop (hoặc tự dừng) → `RecordingSession` → `VideoSession` → `is-video-mode = true`, `total-duration = session.duration`.
- Preview: `canvas-image` hiển thị frame đã composite tại `current-time` (frame có `pts` lớn nhất ≤ t, tìm nhị phân).
- `toggle-playback`: `slint::Timer` bước 1/fps; nếu t rơi vào cut → nhảy tới cuối cut (dựa `kept_ranges()`); hết video → dừng.
- `seek(float)`: đặt `current-time` (clamp [0, duration]), render lại.
- Focus point: click lên video trong video mode đặt `focus` (toạ độ frame). Mặc định = tâm frame.
- `add-cut`: cut `[t, min(t+1.0, duration)]`. `add-zoom`: zoom 2.0x tại focus trong `[t, min(t+2.0, duration)]`. `add-censor`: Pixelate hộp 20%×12% kích thước frame, tâm tại focus, trong `[t, min(t+2.0, duration)]`. Cập nhật `cut-count/zoom-count/censor-count`.
- Save ở video mode = export GIF đã composite (`composite_session` → `GifExporter::export_gif(.., 10)`) ra `~/Movies/AriaShot/ariashot-YYYYMMDD-HHMMSS.gif` trên worker thread, status hiện path.
- Copy ở video mode = copy frame preview hiện tại.
- OCR/Auto-Redact ẩn ở video mode. Nút `Exit video` quay lại image mode (giải phóng session).

Non-functional: tổng RAM frame ≤ ~650 MB ở cấu hình mặc định (960×540×4 B × 300 frame ≈ 622 MB); hằng số cấu hình đặt ở một chỗ (`RecordingConfig`).

Non-goals: audio, export MP4 (encoder phần cứng chưa thật), speed ramp UI, chọn vùng quay, ẩn editor khi quay.

## Architecture

```
Record ─► RecordingHandle::start(backend, display, RecordingConfig)
            thread: loop { capture_display → to_rgba8 → downscale → recorder.push_video_frame; pace to fps;
                            break if stop_flag || elapsed ≥ max }  → recorder.stop() → RecordingSession
          UI Timer (250ms) cập nhật elapsed; khi thread xong → invoke_from_event_loop(enter_video_mode)

VideoSession { session: RecordingSession, timeline: VideoTimeline, current_time, focus: Point, fps }
  frame_index_at(t) -> Option<usize>
  render_at(t) -> Option<RecordedVideoFrame>          // composite_frames(&[frame], &timeline)
  next_play_time(t, dt) -> Option<f64>                 // bỏ qua cut, None khi hết
  add_cut/add_zoom/add_censor(t) ; counts()
  export_gif() -> Result<Vec<u8>, MediaError>
```

`MainEditorWindow` thêm: `in-out property <float> current-time`, `in property <float> total-duration`, `in property <bool> is-playing`, `in property <int> cut-count/zoom-count/censor-count`, `in property <bool> is-recording`, callbacks `toggle-record()`, `exit-video()`, `video-clicked(length, length)`, và chuyển tiếp `toggle-playback, seek, add-cut, add-zoom, add-censor` từ `VideoTimelineView`.

## Related Code Files

- Create: `ariashot/crates/camerashot-editor/src/app/recording.rs`, `src/app/video_session.rs`
- Create: `ariashot/crates/camerashot-editor/tests/video_session_tests.rs`
- Modify (sau Phase 3): `ui/main_editor.slint`, `src/app/mod.rs` (`bind_video_mode`, routing Copy/Save theo mode), `src/main.rs` (gọi `bind_video_mode`)
- Modify nếu cần: `ui/video_timeline.slint` (hiển thị marker cut/zoom/censor, scrubber click → `seek`), `src/timeline_ctrl.rs` (helper `add_*` nếu field không đủ tiện — giữ API cũ)
- Dùng, không sửa: `camerashot-media` (`ScreenRecorder`, `GifExporter`), `camerashot-platform`

## Implementation Steps

1. `recording.rs`: `RecordingConfig { fps: 10, max_width: 960, max_secs: 30 }`, hàm thuần `downscale_rgba(rgba, w, h, max_w) -> (Vec<u8>, u32, u32)`, `RecordingHandle { stop: Arc<AtomicBool>, started: Instant, join: JoinHandle<Result<RecordingSession, String>> }` với `start`, `elapsed`, `stop_and_join`, `is_finished`.
2. `video_session.rs` theo Architecture; hàm thuần, không phụ thuộc Slint.
3. Tests (`video_session_tests.rs`, dựng `RecordingSession` tổng hợp 30 frame 64×36 @10fps):
   - `frame_index_at` đúng biên (t=0, giữa, cuối, >duration).
   - `add_cut(1.0)` → `next_play_time` từ 0.95 nhảy ≥ 2.0.
   - `add_zoom`/`add_censor` clamp end ≤ duration; counts tăng; `render_at` trong vùng censor khác frame gốc ở hộp censor.
   - `export_gif` trả bytes bắt đầu `GIF8`.
   - `downscale_rgba(1920×1080 → 960)` ra 960×540, len khớp.
4. `main_editor.slint`: property/callback mới; nút Record/Stop, Exit video; `TouchArea` trên viewport phát `video-clicked` khi video mode; chuyển tiếp properties/callbacks vào `VideoTimelineView`.
5. `video_timeline.slint` (nếu cần): scrubber click → `seek(x / width * total-duration)`.
6. `app/mod.rs`: `bind_video_mode(ui, image_session, video_state)`; Copy/Save rẽ nhánh theo `is-video-mode`; playback `slint::Timer` giữ trong state.
7. `cargo test -p camerashot-editor`; chạy thủ công.

## Todo

- [x] RecordingConfig + downscale + RecordingHandle (thread capture)
- [x] VideoSession (frame lookup, playback skip cut, add cut/zoom/censor, export GIF)
- [x] Tests video_session_tests
- [x] UI: Record/Stop, Exit video, forward timeline props/callbacks, click focus
- [x] Playback timer + seek
- [x] Export GIF + copy frame ở video mode
- [ ] Manual E2E

## Success Criteria

- `cargo test -p camerashot-editor` xanh (cũ + image + video).
- Thủ công (release, macOS): Record ~10 s → Stop → video mode hiện frame đầu; Play chạy và dừng ở cuối; seek bằng scrubber; Add cut bỏ đúng 1 s khi phát; click điểm rồi Add zoom → phóng 2x mượt quanh điểm; Add censor → hộp pixelate; Save → file `.gif` mở được trong Preview/Quick Look và phản ánh các chỉnh sửa; RAM tiến trình < 1 GB khi quay 30 s.

## Risk Assessment

| Rủi ro | Giảm thiểu |
|---|---|
| `capture_display` (CGDisplay::image) chậm ở 5K → không đạt 10 fps | Đo thời gian mỗi frame; pacing theo PTS thật (recorder tự gắn PTS) nên video vẫn đúng tốc độ dù ít frame hơn; ghi fps thực đạt được vào status |
| RAM | Giới hạn ở `RecordingConfig`; tự dừng ở `max_secs` |
| `ScreenRecorder` không `Send` | Tạo recorder bên trong thread capture, chỉ trả `RecordingSession` (Vec dữ liệu thuần) qua `JoinHandle`; nếu `RecordingSession` không `Send`, trả struct frames/duration tự định nghĩa rồi dựng lại |
| Export GIF lâu với 300 frame | Chạy worker thread, `busy` khoá UI hành động |
| Quay cả cửa sổ editor | Chấp nhận (non-goal); ghi trong README |

## Security Considerations

Frame chỉ nằm trong RAM tới khi user bấm Save; không ghi file tạm.

## Next Steps

Phase 6 kiểm thử tích hợp + README.
