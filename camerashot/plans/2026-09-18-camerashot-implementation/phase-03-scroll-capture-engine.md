---
phase: 3
title: "Deterministic Scroll Capture Engine"
description: "Implement step-and-stitch scroll capture with xxHash64 settlement, 2D FFT phase correlation displacement, SAD scrollbar/header exclusion, and -1px seam bias."
status: "completed"
owner: "engine-scroll"
---

# Phase 3: Deterministic Scroll Capture Engine

## 1. Objectives & Scope
- Implement automated step-and-capture scroll engine (`camerashot-core::scroll`):
  - Synthetic scroll wheel event dispatching: `libei` / `org.freedesktop.portal.RemoteDesktop` (Wayland), `XTest` (X11), `CGEventCreateScrollWheelEvent2` (macOS).
  - Consecutive frame settlement detector using fast non-cryptographic 64-bit hashing (`xxHash64`).
  - Vertical displacement estimation using 2D FFT Phase Correlation (`rustfft`) on central frequency bands.
  - Sum of Absolute Differences (SAD) margin filtering: exclusion of rightmost scrollbar thumb ($\text{avgSAD} > 8$).
  - SAD sticky header detection: scans top rows; rows with $\text{avgSAD} \le 8$ for $\ge 10$ rows are flagged as sticky headers and cropped from incremental strips.
  - Invariant: Appending strips strictly with $-1\text{px}$ seam bias overlap ($\text{safeOffset} = \max(1, \Delta y - 1)$) to prevent horizontal subpixel tears.

## 2. File Ownership & Boundaries
- `crates/camerashot-core/src/scroll/mod.rs`
- `crates/camerashot-core/src/scroll/settlement.rs`
- `crates/camerashot-core/src/scroll/fft_phase.rs`
- `crates/camerashot-core/src/scroll/sad_filter.rs`
- `crates/camerashot-core/src/scroll/stitcher.rs`
- `crates/camerashot-platform/src/input/mod.rs`
- `crates/camerashot-platform/src/input/synthetic_scroll.rs`

## 3. Verification Criteria
- Automated test capturing a 10,000px synthetic scrolling document.
- Stitched result contains 0 duplicate rows, 0 horizontal line artifacts, and perfectly matches continuous source document height within $\pm 2\text{px}$.
