---
phase: 6
title: "Integration, Verification & Docs"
status: todo
priority: P1
effort: "0.5d"
dependencies: [2, 4, 5]
---

# Phase 6: Integration, Verification & Docs

## Overview

Gộp kết quả Phase 2/4/5, chạy cổng chất lượng toàn workspace, kiểm thử thủ công cả 3 sản phẩm trên macOS, và cập nhật README cho hành vi/command mới.

## Requirements

- Cổng tự động (từ `ariashot/`): `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo build --workspace --release`.
- Kiểm thử thủ công theo checklist Success Criteria của Phase 2, 3, 4, 5; ghi kết quả (pass/fail + ghi chú) vào report.
- README root (`README.md`) cập nhật: cách chạy `camerashot-overlay`, `camerashot-editor [IMAGE]`, `ariashot` (bảng cờ + ví dụ + exit code); quyền macOS Screen Recording; ghi chú toạ độ crop là point; hạn chế đã biết (OCR chỉ macOS, chỉ display chính, video mode không audio và giới hạn 30 s/10 fps/960 px, export GIF; Linux clipboard block); dependency hệ thống Linux cho winit/slint (nếu phát hiện khi build).
- Sửa mục Features trong README cho khớp thực tế (hiện ghi ONNX RapidOCR Linux, NSPanel level 257, wlr-layer-shell — những thứ chưa có); không phóng đại.

## Related Code Files

- Modify: `README.md` (root repo)
- Create: `plans/reports/integration-260918-<HHMM>-interactive-overlay-editor-cli.md`
- Sửa code chỉ khi cổng tự động fail — sửa trong file của phase gây lỗi, ghi lý do vào report.

## Implementation Steps

1. Pull/merge các nhánh/worktree Phase 2, 4, 5 (nếu chạy song song bằng worktree); giải quyết xung đột (không nên có nếu ownership đúng).
2. Chạy 4 lệnh cổng tự động; sửa lỗi phát sinh.
3. Chạy checklist thủ công (release build) trên macOS; chụp kết quả vào report.
4. Cập nhật README; kiểm tra lại mọi lệnh trong README chạy được đúng như viết.
5. `ak plan` cập nhật trạng thái phase.

## Todo

- [ ] Merge phase outputs
- [ ] fmt / clippy / test / release build xanh
- [ ] Manual E2E overlay, editor image, editor video, CLI
- [ ] README cập nhật và đối chiếu thực tế
- [ ] Integration report

## Success Criteria

- 4 lệnh cổng tự động exit 0.
- Mọi mục checklist thủ công pass hoặc có issue ghi rõ trong report.
- README không còn claim sai về tính năng; mọi ví dụ lệnh chạy được.

## Risk Assessment

| Rủi ro | Giảm thiểu |
|---|---|
| Clippy `-D warnings` bắt lỗi cũ không liên quan | Chỉ sửa cảnh báo trong file plan này chạm; cảnh báo có sẵn ở crate khác → ghi vào report, chạy clippy riêng từng crate mới/đã sửa |
| Test thủ công phụ thuộc quyền Screen Recording | Cấp quyền cho terminal trước khi test; ghi vào README |

## Security Considerations

Report không chứa screenshot có dữ liệu thật nhạy cảm; dùng fixture PII giả.
