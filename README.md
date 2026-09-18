# AriaShot 📸

[![Rust](https://img.shields.io/badge/rust-2021-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20(Wayland%2FX11)-lightgrey.svg)]()

> **AriaShot** is a high-performance, local-first screen capture, recording, scroll stitching, and vector annotation suite built with **Rust** for **macOS (13+)** and **Linux (Wayland & X11)**.

Part of the **Ariadnev Ecosystem**.

---

## ✨ Features

- **⚡ Instant Overlay (<15ms):** Lightweight Tier 1 presentation surface powered by `tiny-skia` on native protocols (`NSPanel` Level 257 on macOS, `wlr-layer-shell` on Wayland, borderless on X11).
- **🧲 Smart Magnetic Snapping:** `BoundarySnapIndex` Euclidean color-gradient edge detector ($\Delta C \ge 28$, span support $\ge 55\%$) for instant window and element border snapping.
- **✍️ 18 Vector Annotation Tools:** Fully featured drawing canvas with Chaikin curve subdivision ($N=8$, 2 passes) for silky smooth pencil strokes.
- **📜 Deterministic Scroll Capture:** Automated step-and-capture scroll engine using 2D FFT Phase Correlation vertical displacement, SAD margin and sticky header exclusion, and strict **-1px seam bias** overlap eliminating subpixel tearing.
- **🎥 High-FPS Recording & Audio Engine:** 60/120 FPS capture, hardware-accelerated video encoding (VideoToolbox, VA-API, NVENC), `AudioMixer` with 48kHz stereo float PCM soft-clipping, `PtsAccumulator` preserving A/V sync across pause/resume, and native `gifski` animated GIF export.
- **🎬 Tier 2 Desktop Editor & Video Timeline:** Slint-based standalone editor with infinite pan & zoom canvas, multi-track video timeline featuring Hermite cubic smoothstep zoom punch ($S(x) = x^2(3-2x)$), cuts, censor blurs, and speed ramps (0.25x–4.0x).
- **🔒 Local-First OCR & Auto-Redactor:** Offline Apple Vision (macOS) and in-process ONNX RapidOCR INT8 (Linux), plus automated 11-category PII detection (emails, phones, credit cards, SSNs, API keys) with 1-click atomic undo via `CompositeGroup`.

---

## 🏗️ Architecture

The workspace is organized into 6 modular Rust crates:

```
ariashot/crates/
├── camerashot-core       # 2D geometry, Chaikin smoothing, BoundarySnapIndex, undo stack, scroll engine
├── camerashot-platform   # ScreenCaptureKit, MIT-SHM, PipeWire, synthetic input (libei/XTest/CGEvent)
├── camerashot-overlay    # Fast Tier 1 overlay application, Loupe 2x, floating HUDs
├── camerashot-media      # Screen recorder, AudioMixer, PtsAccumulator, hw encoders, gifski
├── camerashot-editor     # Tier 2 Slint desktop editor, multi-track video timeline & compositor
└── camerashot-ocr        # Apple Vision FFI, ONNX RapidOCR, 11-category PII AutoRedactor
```

---

## 🛠️ Build & Test

Ensure you have Rust (stable 2021 edition) installed:

```bash
cd ariashot

# Run unit and integration tests across the workspace
cargo test --workspace

# Run release build
cargo build --release --workspace
```

---

## 📜 License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
