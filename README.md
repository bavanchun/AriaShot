# AriaShot 📸

[![Rust](https://img.shields.io/badge/rust-2021-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Linux-lightgrey.svg)]()

> **AriaShot** is a high-performance, local-first screen capture, recording, and vector annotation suite built with **Rust** for **macOS (13+)** and **Linux**.

Part of the **Ariadnev Ecosystem**.

---

## ✨ Implemented Features

- **⚡ Fast Interactive Overlay:** Built with `winit 0.30` and `softbuffer 0.4`. Features a floating 2x Loupe with pixel grid, RGB hex readout, crosshair, magnetic snapping, and floating action toolbar. Keyboard shortcuts: `Enter` or `Cmd+C` / `Ctrl+C` to copy selection, `Esc` to dismiss.
- **🧲 Smart Magnetic Snapping:** `BoundarySnapIndex` Euclidean color-gradient edge detector ($\Delta C \ge 28$, span support $\ge 55\%$) for instant window and element border snapping.
- **✍️ Vector Annotation Canvas:** Rich drawing primitives with Chaikin curve subdivision ($N=8$, 2 passes) for smoothed pen strokes, arrows, rectangles, ellipses, number badges, and measurement ticks.
- **📜 Deterministic Scroll Capture:** Automated step-and-capture scroll engine using 2D FFT Phase Correlation vertical displacement, SAD margin and sticky header exclusion, and strict **-1px seam bias** overlap eliminating subpixel tearing.
- **🎬 Desktop Editor (Slint UI):** Standalone desktop GUI supporting both Image Mode and Video Mode:
  - **Image Mode:** Infinite pan & zoom canvas (Zoom In/Out/100%/Fit), 1-click Apple Vision OCR text panel, 11-pattern PII Auto-Redaction with multi-step undo/redo, and direct Save / Clipboard copy.
  - **Video Mode:** In-memory RAM-safe screen recording (10 FPS, max 960px width, 30s auto-stop buffer), interactive timeline scrubber, skip-cut segments, zoom-punch, censor overlays, and animated GIF export.
- **🔍 CLI Capture Tool (`ariashot`):** Full-featured command-line utility for scriptable full-screen or crop captures, direct clipboard copies, Apple Vision OCR text extraction to stdout, and fail-closed PII auto-redaction.
- **🔒 Local-First OCR & Auto-Redaction:** Native Apple Vision framework integration (macOS) providing offline text recognition, bounding boxes, and automated 11-category PII detection (emails, phone numbers, credit cards, SSNs, API keys).

---

## 🏗️ Architecture

The workspace is organized into 7 modular Rust crates:

```
ariashot/crates/
├── camerashot-core       # 2D geometry, Chaikin smoothing, BoundarySnapIndex, undo stack, scroll engine
├── camerashot-platform   # macOS CGDisplay/ScreenCaptureKit backend, clipboard integration, synthetic input
├── camerashot-overlay    # Interactive fullscreen overlay window, 2x Loupe, toolbar HUD (winit + softbuffer)
├── camerashot-editor     # Slint desktop editor (Image pan/zoom, OCR, Auto-Redact, Video timeline & GIF)
├── camerashot-cli        # CLI tool binary (`ariashot`) for headless capture, OCR, and redaction
├── camerashot-media      # Screen recording buffer, PTS accumulator, audio mixer, and GIF encoder
└── camerashot-ocr        # Apple Vision OCR engine and 11-category PII detector & redactor
```

---

## 🚀 Usage

### 1. Interactive Overlay Window (`camerashot-overlay`)

Captures the current display and opens an interactive fullscreen selection overlay:

```bash
cd ariashot
cargo run --release -p camerashot-overlay
```

- **Drag:** Select a capture region.
- **Loupe (2x):** Follows cursor displaying zoomed pixels, RGB hex value, and coordinate dimensions.
- **Copy:** Click the Copy button on the floating toolbar, or press `Enter` / `Cmd+C` (`Ctrl+C` on Linux).
- **Cancel:** Click the Close button on the toolbar or press `Esc`.

---

### 2. Slint Desktop Editor (`camerashot-editor`)

Launch the editor with an existing image file, or without arguments to capture the primary screen:

```bash
# Open an existing image file (PNG, JPG, WebP)
cargo run --release -p camerashot-editor -- /path/to/screenshot.png

# Capture the primary display and open in editor
cargo run --release -p camerashot-editor
```

- **Pan & Zoom:** Zoom in (`+`), Zoom out (`-`), Actual size (`1:1`), or Fit to window (`Fit`).
- **OCR:** Click **OCR** to scan text and view results in the right drawer panel.
- **Auto-Redact:** Click **Auto-Redact** to detect sensitive PII (emails, cards, keys) and blur them.
- **Undo / Redo:** Full history tracking for all annotation and redaction actions.
- **Screen Recording:** Click **Record** in the top bar to record up to 30s. Click **Stop** to preview and edit on the timeline, add cuts/zooms/censors, and click **Export GIF**.

---

### 3. CLI Capture Tool (`ariashot`)

The binary `ariashot` provides fast, scriptable captures directly from your shell:

```bash
# Build the release binary
cargo build --release -p camerashot-cli
# Path: target/release/ariashot
```

#### Command Options & Flags

| Flag | Description |
|---|---|
| `--full` | Capture the entire display (defaults to primary display). |
| `--crop <X,Y,W,H>` | Capture a rectangular region in point-logic coordinates (e.g. `100,100,800,600`). |
| `--display <ID>` | Target a specific display ID (only valid with `--full`). |
| `-o, --output <PATH>` | Output file path (`.png`, `.jpg`, `.webp`). Defaults to `./ariashot-YYYYMMDD-HHMMSS.png` if neither `--clipboard` nor `--ocr` is given. |
| `--clipboard` | Copy the captured image to the system clipboard. |
| `--ocr` | Run local OCR and print recognized text to `stdout`. |
| `--redact` | Redact detected PII before saving. Requires OCR (fail-closed). |
| `--redact-style <STYLE>` | Redaction style: `pixelate` (default), `blur`, or `fill`. |

#### Examples

```bash
# Capture full screen to default timestamped PNG
./target/release/ariashot --full

# Capture full screen directly to clipboard
./target/release/ariashot --full --clipboard

# Capture specific crop region to WebP
./target/release/ariashot --crop 100,100,800,600 -o capture.webp

# Extract text from a screen region via OCR to stdout
./target/release/ariashot --crop 200,300,500,200 --ocr

# Capture full screen, auto-redact sensitive PII, and save to file
./target/release/ariashot --full --redact -o sanitized.png
```

#### Exit Codes

- `0`: Success.
- `1`: Argument validation error (e.g. invalid crop format, non-positive dimensions, invalid extension).
- `2`: Capture, processing, or file writing error.
- `3`: Platform unsupported feature (e.g. OCR requested on non-macOS platform).

---

## ⚙️ Requirements & Permissions

### macOS
- **macOS 13.0+ (Ventura or newer)**
- **Screen Recording Permission:** Your terminal emulator (e.g. Terminal, iTerm2, Alacritty, Ghostty, or VSCode) must have permission granted in:
  `System Settings > Privacy & Security > Screen Recording`.
- **Display Coordinates:** All `--crop` coordinates are in macOS point-logic units. On Retina / HiDPI displays, output image dimensions will scale with the backing display scale factor (e.g. 2x).

### Linux
- System dependencies for Slint and Winit:
  ```bash
  # Debian / Ubuntu:
  sudo apt-get install -y libxkbcommon-dev libfontconfig1-dev libwayland-dev
  ```
- **Clipboard:** On Linux, `--clipboard` uses cooperative clipboard ownership and blocks until another application requests the clipboard content.

---

## ⚠️ Known Limitations & Current Constraints

- **OCR Engine:** OCR is implemented natively using the Apple Vision framework on macOS. Linux ONNX RapidOCR runtime is currently stubbed and planned for a future release.
- **Display Selection:** Multi-display capture targets the primary display by default.
- **Video Recording:** In-editor video recording is RAM-safe and capped at 10 FPS, 960px max width, 30s max duration, and exports as GIF (no audio stream).
- **Redaction Policy:** `--redact` operates in **fail-closed** mode; if OCR fails or cannot run, no output image is written to prevent leaking sensitive unredacted data.

---

## 🛠️ Build & Test

```bash
cd ariashot

# Format check
cargo fmt --all -- --check

# Clippy linter
cargo clippy --workspace --all-targets -- -D warnings

# Run all 86 unit and integration tests across the workspace
cargo test --workspace

# Build all release binaries
cargo build --release --workspace
```

---

## 📜 License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
