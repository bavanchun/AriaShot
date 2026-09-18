---
phase: 5
title: "Tier 2 Slint Editor, Video Timeline & Local OCR"
description: "Implement the detached Slint desktop editor, the multi-track video timeline editor, embedded ONNX RapidOCR, Apple Vision FFI, and 11 PII regexes with atomic undo."
status: "pending"
owner: "ui-editor"
---

# Phase 5: Tier 2 Slint Editor, Video Timeline & Local OCR

## 1. Objectives & Scope
- Implement Tier 2 standalone desktop application (`camerashot-editor`) in Slint:
  - Detached canvas viewport with infinite pan and zoom (`CenteringClipView` equivalent).
  - Comprehensive tool inspector, font picker, and color wheel popovers.
- Post-Capture Video Timeline Editor:
  - Interactive playback scrubber with audio waveform visualization.
  - Multi-track non-destructive segments:
    - Trim / Cut segments (`VideoCutSegment`).
    - Camera Zoom Punch (`VideoZoomSegment`) with Hermite cubic smoothstep easing ($S(x) = x^2(3-2x)$) and clamped center point preventing border bleed.
    - Censor blur/pixelate blocks (`VideoCensorSegment`).
    - Speed ramps from $0.25\times$ to $4.0\times$ (`VideoSpeedSegment`) with audio pitch compensation.
    - Timed text callouts (`VideoTextSegment`).
- Offline Local OCR & Scanning (`camerashot-ocr`):
  - macOS: Apple Vision (`VNRecognizeTextRequest`, `VNDetectBarcodesRequest`).
  - Linux: Embedded in-process ONNX Runtime (`ort`) with quantized RapidOCR INT8 models (DBNet text detection + SVTR text recognition) + `zxing-cpp` QR reader.
- Automated PII Redaction:
  - Regular expression detection across 11 sensitive data categories (email, phone, SSN, credit cards, CVV, expiry, IPv4, AWS keys, secrets, hex hashes, bearer tokens) + lightweight face detection.
  - Generates composite `UndoAction::CompositeGroup` with shared UUID for 1-click atomic rollback.

## 2. File Ownership & Boundaries
- `crates/camerashot-editor/Cargo.toml`
- `crates/camerashot-editor/ui/main_editor.slint`
- `crates/camerashot-editor/ui/video_timeline.slint`
- `crates/camerashot-editor/src/main.rs`
- `crates/camerashot-editor/src/timeline_ctrl.rs`
- `crates/camerashot-editor/src/compositor.rs`
- `crates/camerashot-ocr/Cargo.toml`
- `crates/camerashot-ocr/src/lib.rs`
- `crates/camerashot-ocr/src/vision_macos.rs`
- `crates/camerashot-ocr/src/onnx_linux.rs`
- `crates/camerashot-ocr/src/barcode.rs`
- `crates/camerashot-ocr/src/redactor.rs`

## 3. Verification Criteria
- Non-destructive video export applies trimmed time bounds, zoom punches with smoothstep curves, and censor blur boxes with bit-for-bit frame accuracy.
- OCR text recognition on 4K screenshots completes in $\le 80\text{ms}$ on CPU with $\ge 98\%$ accuracy.
- 1-click Auto-Redact obscures sensitive data; single `Ctrl+Z` / `Cmd+Z` reverts all created redaction blocks atomically.
