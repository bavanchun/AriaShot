---
title: "Plan: interactive overlay, Slint editor, CLI capture"
date: 2026-09-18
summary: "Scouted real gaps behind 'completed' phases and planned 6-phase parallel delivery"
---

# Plan: interactive overlay, Slint editor, CLI capture

## What happened
Scouted ariashot/crates before planning. Previous plan marked all 5 phases completed, but user-facing surfaces are not wired:
- camerashot-overlay main.rs only simulates a drag; no window/event loop; Loupe draws a 16px color block, not 2x magnification; toolbar has no buttons/hit-test; no clipboard anywhere.
- camerashot-editor has no `slint` dependency and no build.rs, so ui/main_editor.slint never compiled.
- AppleVisionOcr and RapidOnnxOcr are stubs returning Ok(Vec::new()).
- ScreenRecorder has no capture loop and keeps raw RGBA frames in RAM.

## Decision
Plan at plans/260918-1226-interactive-overlay-editor-cli (6 phases, --parallel):
1 shared foundation (workspace deps, platform::clipboard, real Vision OCR, ocr::pipeline) -> 2 overlay / 3 editor image / 5 CLI in parallel -> 4 editor video (after 3) -> 6 integration + README.
User choices: real macOS Vision OCR, Linux returns explicit error; editor gets screen recording + video mode (10fps, 960px, 30s cap, GIF export); new crate camerashot-cli, binary `ariashot`. CLI `--redact` fails closed.

## Next steps
Run /ak:cook --parallel on the plan. Watch: objc2-vision binding names, CPU redraw cost of 5K overlay, recording RAM.

> Historical work record — not durable authority. Prefer docs/specs/ADRs for current decisions.
