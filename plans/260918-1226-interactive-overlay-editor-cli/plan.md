---
title: "AriaShot: Interactive Overlay, Slint Editor & CLI Capture"
description: "Hoàn thiện 3 bề mặt tương tác người dùng: overlay fullscreen thật, Slint editor (image + video mode), CLI capture tool."
status: pending
priority: P1
effort: "5d"
tags: [overlay, editor, cli, ocr, clipboard, parallel]
blockedBy: []
blocks: []
created: 2026-09-18
---

# AriaShot: Interactive Overlay, Slint Editor & CLI Capture

## Overview

Scout (2026-09-18) cho thấy các bề mặt người dùng chưa chạy thật: overlay `main.rs` chỉ giả lập drag, Loupe chỉ vẽ ô màu, editor crate chưa có dependency `slint` (UI `.slint` chưa từng compile), OCR macOS/Linux đều là stub trả `Vec::new()`. Plan này nối engine đã có (core/platform/media/ocr) thành 3 sản phẩm dùng được. Workspace: `ariashot/` (cargo build hiện xanh).

## Quyết định đã chốt (user, 2026-09-18)

- OCR: implement Apple Vision thật trên macOS; Linux trả lỗi rõ ràng "unsupported", không trả rỗng im lặng.
- Editor: image mode đầy đủ + **video mode có quay màn hình** (Record → timeline → cut/zoom/censor → export GIF).
- CLI: crate mới `camerashot-cli`, binary `ariashot`, dùng `clap`.

## Goals

| # | Goal | Priority |
|---|------|----------|
| 1 | Overlay fullscreen với chuột/phím thật, Loupe 2x thật, copy vùng chọn vào clipboard | P1 |
| 2 | Slint editor chạy thật, mọi callback của `main_editor.slint` + `video_timeline.slint` được nối | P1 |
| 3 | CLI `ariashot --full/--crop/--clipboard/--ocr/--redact` | P1 |

## Phases

| # | Phase | Status |
|---|-------|--------|
| 1 | [Shared Foundation (deps, clipboard, Vision OCR, redact pipeline)](./phase-01-shared-foundation.md) | Pending |
| 2 | [Interactive Overlay Window](./phase-02-overlay-window.md) | Pending |
| 3 | [Slint Editor — Image Mode](./phase-03-editor-image-mode.md) | Pending |
| 4 | [Slint Editor — Video Mode & Recording](./phase-04-editor-video-mode.md) | Pending |
| 5 | [CLI Capture Tool](./phase-05-cli-capture-tool.md) | Pending |
| 6 | [Integration, Verification & Docs](./phase-06-integration-and-docs.md) | Pending |

## Execution Graph (parallel)

```
Phase 1 ──┬── Phase 2 (overlay crate) ─────────────┐
          ├── Phase 3 (editor) ── Phase 4 (editor) ├── Phase 6
          └── Phase 5 (cli crate) ─────────────────┘
```

- Group A (tuần tự): Phase 1 — sở hữu root `Cargo.toml`, `.gitignore`, crate `platform` + `ocr`, skeleton crate `cli`.
- Group B (song song): Phase 2, Phase 3, Phase 5 — file ownership rời nhau theo crate.
- Phase 4 phụ thuộc Phase 3 (cùng crate editor, cùng `main_editor.slint`); chạy song song với phần đuôi của 2 và 5.
- Group C: Phase 6 sau khi 2, 4, 5 xong.

## File Ownership Matrix

| Phase | Sở hữu độc quyền |
|---|---|
| 1 | `ariashot/Cargo.toml`, `.gitignore`, `crates/camerashot-platform/**`, `crates/camerashot-ocr/**`, `crates/camerashot-cli/Cargo.toml` (tạo) |
| 2 | `crates/camerashot-overlay/**` |
| 3 | `crates/camerashot-editor/{Cargo.toml,build.rs,src/main.rs,src/lib.rs,src/app/mod.rs,src/app/image_session.rs,ui/main_editor.slint,tests/image_session_tests.rs}` |
| 4 | `crates/camerashot-editor/{src/app/recording.rs,src/app/video_session.rs,ui/video_timeline.slint,tests/video_session_tests.rs}` + chỉnh sau Phase 3: `ui/main_editor.slint`, `src/main.rs`, `src/app/mod.rs`; tuỳ chọn `src/timeline_ctrl.rs` (chỉ thêm, không đổi API) |
| 5 | `crates/camerashot-cli/**` (sau bàn giao skeleton từ Phase 1) |
| 6 | `README.md` (root repo), `docs/**` |

Không phase nào sửa `camerashot-core` hoặc `camerashot-media` (chỉ tiêu thụ API public). Nếu cần sửa, dừng và escalate.

## Success Criteria

- [ ] `cargo build --workspace` và `cargo test --workspace` xanh; `cargo clippy --workspace -- -D warnings` không lỗi mới.
- [ ] `cargo run -p camerashot-overlay --release`: overlay phủ màn hình, drag chọn vùng có snap, Loupe phóng 2x theo con trỏ, Enter/⌘C copy ảnh vào clipboard và thoát, Esc huỷ.
- [ ] `cargo run -p camerashot-editor --release [-- ảnh.png]`: hiển thị ảnh, zoom/pan, Copy/Save/OCR/Auto-Redact/Undo/Redo chạy; Record → video mode → play/seek/cut/zoom/censor → export GIF.
- [ ] `ariashot --full -o a.png`, `--crop 0,0,800,600 --clipboard`, `--full --ocr`, `--full --redact -o r.png` cho kết quả đúng.
- [ ] Không placeholder/mock; Linux OCR trả lỗi rõ ràng.

## Dependencies

- Không phụ thuộc plan khác. Plan trước (`ariashot/plans/2026-09-18-camerashot-implementation`) đã completed; plan này sửa các khoảng trống thực tế của nó.
- Cần quyền macOS Screen Recording cho terminal khi test thủ công.
