---
phase: 4
title: "Screen Recording, Hardware Encoding & Audio Engine"
description: "Implement high-FPS screen recording, PipeWire and CoreAudio system loopback + mic capture, VA-API/NVENC/VideoToolbox encoders, and native gifski GIF export."
status: "pending"
owner: "engine-media"
---

# Phase 4: Screen Recording, Hardware Encoding & Audio Engine

## 1. Objectives & Scope
- High-performance screen capture loop up to 60/120 FPS:
  - Linux Wayland: PipeWire DMA-BUF stream.
  - macOS: `SCStream` with `SCStreamConfiguration` (forcing 32BGRA sRGB).
  - Linux X11: High-speed MIT-SHM polling ring-buffer.
- Audio capture and mixing:
  - Linux: Binds PipeWire loopback monitor source (desktop audio) and microphone input.
  - macOS: `SCStream` loopback audio and `AVCaptureSession` microphone input.
  - Dynamically resampled to 48kHz float PCM stereo.
- Hardware-accelerated video encoding via `libavcodec` / `ffmpeg-next`:
  - Linux: VA-API (Intel/AMD) and NVENC (Nvidia) H.264/HEVC.
  - macOS: VideoToolbox H.264 hardware encoding.
  - Presentation Timestamp (PTS) accumulator handling seamless pause/resume.
- Animated GIF export pipeline:
  - Direct integration with native Rust `gifski` library for multi-threaded palette quantization without disk bottlenecks or OOM crashes.
- Live Recording HUD overlay:
  - Floating recording controls (pause, resume, stop, elapsed time).
  - Click ripple highlight and keystroke visualization badges.

## 2. File Ownership & Boundaries
- `crates/camerashot-media/Cargo.toml`
- `crates/camerashot-media/src/lib.rs`
- `crates/camerashot-media/src/recorder.rs`
- `crates/camerashot-media/src/audio/pipewire.rs`
- `crates/camerashot-media/src/audio/coreaudio.rs`
- `crates/camerashot-media/src/video/hw_encoder.rs`
- `crates/camerashot-media/src/video/pts.rs`
- `crates/camerashot-media/src/gif/gifski.rs`

## 3. Verification Criteria
- Sustained 60 FPS recording of high-motion video with $< 1\%$ dropped frames.
- Exported MP4 maintains audio/video synchronization drift $\le 15\text{ms}$ over a 10-minute capture.
- Exporting a 5-second 1080p clip to GIF finishes in $< 3$ seconds with 256-color palette fidelity.
