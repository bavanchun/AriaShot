//! High-level OCR pipeline functions bridging `OcrEngine`, `PiiRedactor`,
//! and `AnnotationRenderer` for CLI / editor / overlay consumers.

use camerashot_core::annotation::{Annotation, AnnotationTool};
use camerashot_core::geometry::Rect;
use uuid::Uuid;

use crate::redactor::{PiiDetector, PiiRedactor};
use crate::vision_macos::OcrBlock;
use crate::OcrEngine;

/// Build redaction annotations from pre-computed OCR blocks.
///
/// Converts `OcrBlock`s into the `(&str, Rect)` tuples expected by
/// `PiiRedactor::build_redactions_for_blocks` and returns the batch UUID
/// plus any generated annotations.
pub fn redactions_from_blocks(
    redactor: &PiiRedactor,
    blocks: &[OcrBlock],
    tool: AnnotationTool,
    padding: f64,
) -> (Uuid, Vec<Annotation>) {
    let pairs: Vec<(&str, Rect)> = blocks.iter().map(|b| (b.text.as_str(), b.bounds)).collect();
    redactor.build_redactions_for_blocks(&pairs, tool, padding)
}

/// Run OCR then build redaction annotations in one call.
///
/// Equivalent to `engine.recognize_text(…)` followed by `redactions_from_blocks`.
pub fn detect_redactions(
    engine: &OcrEngine,
    rgba: &[u8],
    w: u32,
    h: u32,
    tool: AnnotationTool,
    padding: f64,
) -> Result<(Uuid, Vec<Annotation>), String> {
    let blocks = engine.recognize_text(rgba, w, h)?;
    let redactor = PiiRedactor::new();
    Ok(redactions_from_blocks(&redactor, &blocks, tool, padding))
}

/// Convert OCR blocks to a single text string.
///
/// Blocks are sorted top-to-bottom (by Y), then left-to-right (by X).
/// Each block occupies one line in the output.
pub fn blocks_to_text(blocks: &[OcrBlock]) -> String {
    let mut sorted: Vec<&OcrBlock> = blocks.iter().collect();
    sorted.sort_by(|a, b| {
        let y_cmp = a
            .bounds
            .origin
            .y
            .partial_cmp(&b.bounds.origin.y)
            .unwrap_or(std::cmp::Ordering::Equal);
        if y_cmp != std::cmp::Ordering::Equal {
            return y_cmp;
        }
        a.bounds
            .origin
            .x
            .partial_cmp(&b.bounds.origin.x)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    sorted
        .iter()
        .map(|b| b.text.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Replace PII occurrences in `text` with the `█` character.
///
/// Uses `PiiDetector` to find matches and replaces each matched character
/// with `█`.
pub fn mask_pii_in_text(detector: &PiiDetector, text: &str) -> String {
    let matches = detector.scan_text(text);
    if matches.is_empty() {
        return text.to_string();
    }

    // Sort matches by start position, then merge overlapping ranges
    let mut ranges: Vec<(usize, usize)> = matches.iter().map(|m| (m.start, m.end)).collect();
    ranges.sort_by_key(|r| r.0);

    let mut merged: Vec<(usize, usize)> = Vec::new();
    for (s, e) in ranges {
        if let Some(last) = merged.last_mut() {
            if s <= last.1 {
                last.1 = last.1.max(e);
                continue;
            }
        }
        merged.push((s, e));
    }

    let mut result = String::with_capacity(text.len());
    let mut cursor = 0;

    for (start, end) in merged {
        // Copy text before this match
        result.push_str(&text[cursor..start]);
        // Replace each character in the match with █
        let matched_chars = text[start..end].chars().count();
        for _ in 0..matched_chars {
            result.push('█');
        }
        cursor = end;
    }
    // Copy remaining text after last match
    result.push_str(&text[cursor..]);

    result
}
