---
phase: 2
title: "Tier 1 Fast Overlay & 18 Vector Tools"
description: "Implement the lightweight <15ms fullscreen interactive overlay surface, the tiny-skia 2D canvas, and all 18 vector annotation tools with Chaikin smoothing."
status: "completed"
owner: "ui-overlay"
---

# Phase 2: Tier 1 Fast Overlay & 18 Vector Tools

## 1. Objectives & Scope
- Implement the Tier 1 low-latency overlay application (`camerashot-overlay`):
  - Wayland: `wlr-layer-shell-unstable-v1` overlay layer (`ZWLR_LAYER_SHELL_V1_LAYER_OVERLAY`) with keyboard grab.
  - X11: Borderless `override_redirect` window with `_NET_WM_STATE_STAYS_ON_TOP`.
  - macOS: `NSPanel` at Level 257 (`canJoinAllSpaces`, `fullScreenAuxiliary`, `nonactivatingPanel`).
- Interactive selection state machine: `Idle` $\to$ `Selecting` $\to$ `Selected`.
- Integration of `BoundarySnapIndex` for magnetic edge snapping during rubber-band drag.
- High-DPI Loupe Magnifier ($2\times$ pixel grid with hex/RGB color readout).
- Floating action toolbar and floating resolution indicator HUD.
- Implementation of all 18 vector tools using `tiny-skia` with exact `captureDrawRect` coordinate anchoring.
- Beautify canvas renderer: synthetic window chrome (macOS traffic lights), configurable drop shadows, and 30 linear/mesh gradient presets.

## 2. File Ownership & Boundaries
- `crates/camerashot-overlay/Cargo.toml`
- `crates/camerashot-overlay/src/main.rs`
- `crates/camerashot-overlay/src/surface.rs`
- `crates/camerashot-overlay/src/state_machine.rs`
- `crates/camerashot-overlay/src/hud/resolution_box.rs`
- `crates/camerashot-overlay/src/hud/loupe.rs`
- `crates/camerashot-overlay/src/hud/toolbar.rs`
- `crates/camerashot-core/src/annotation/mod.rs`
- `crates/camerashot-core/src/annotation/tools/*.rs`
- `crates/camerashot-core/src/beautify/mod.rs`

## 3. Verification Criteria
- Launch benchmark: overlay visible on screen in $\le 15\text{ms}$ after hotkey trigger.
- Freehand pencil with Chaikin smoothing maintains smooth 60 FPS drawing without stutter.
- Loupe correctly displays magnified pixels matching underlying desktop colors.
