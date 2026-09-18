---
phase: 1
title: "Foundations & Platform Core"
description: "Initialize Cargo workspace, implement core domain models, BoundarySnapIndex edge detector, Chaikin curve smoothing, and cross-platform screen grabbers."
status: "completed"
owner: "system-core"
---

# Phase 1: Foundations & Platform Core

## 1. Objectives & Scope
- Set up root Cargo workspace with 8 modular crates and strict acyclic dependencies.
- Implement `camerashot-core` primitives:
  - 2D Geometry (`Point`, `Rect`, `Size`).
  - Mathematical models: Chaikin smoothing ($N=8$ moving average, 2 subdivision passes, pressure lerp).
  - `BoundarySnapIndex` image-space Euclidean color-gradient edge detector ($\Delta C \ge 28$, span support $\ge 55\%$).
  - Event-sourced `UndoAction` and `CompositeGroup` for atomic auto-redaction rollback.
- Implement `camerashot-platform` screen capture abstraction:
  - `CaptureBackend` trait.
  - macOS backend: `ScreenCaptureKit` (concurrent TaskGroup) + `CGWindowListCreateImage` instant fallback.
  - Linux X11 backend: `x11rb` MIT-SHM shared memory pixmap capture.
  - Linux Wayland backend: `zwlr_screencopy_v1` fast path + `ashpd` / PipeWire MemFd fallback.

## 2. File Ownership & Boundaries
- `Cargo.toml` (root workspace)
- `crates/camerashot-core/Cargo.toml`
- `crates/camerashot-core/src/lib.rs`
- `crates/camerashot-core/src/geometry.rs`
- `crates/camerashot-core/src/chaikin.rs`
- `crates/camerashot-core/src/snap_index.rs`
- `crates/camerashot-core/src/undo.rs`
- `crates/camerashot-platform/Cargo.toml`
- `crates/camerashot-platform/src/lib.rs`
- `crates/camerashot-platform/src/traits.rs`
- `crates/camerashot-platform/src/macos/mod.rs`
- `crates/camerashot-platform/src/linux/mod.rs`
- `crates/camerashot-platform/src/linux/x11.rs`
- `crates/camerashot-platform/src/linux/wayland.rs`

## 3. Implementation Steps
1. **Initialize Cargo Workspace:** Define `members = ["crates/*"]` and shared workspace dependencies (`image`, `parking_lot`, `thiserror`, `tracing`, `serde`, `uuid`).
2. **Implement Core Primitives (`camerashot-core`):**
   - Implement `BoundarySnapIndex` in `snap_index.rs` with SIMD or optimized integer Euclidean color distance math.
   - Implement Chaikin smoothing in `chaikin.rs` matching `PencilToolHandler.swift`.
   - Write comprehensive unit tests for `snap_index` on synthetic 2D test patterns and `chaikin` on noisy stroke coordinates.
3. **Implement Platform Trait (`camerashot-platform`):**
   - Define `PlatformDisplay` and `FrameBuffer` (raw RGBA8/BGRA8 buffer, stride, pixel format, display bounds).
   - Wire platform conditional compilation (`#[cfg(target_os = "macos")]`, `#[cfg(target_os = "linux")]`).
4. **Verification:**
   - `cargo test --workspace` passes cleanly.
   - Run bench/unit test verifying `BoundarySnapIndex` processes a 4K frame in $\le 10\text{ms}$.
