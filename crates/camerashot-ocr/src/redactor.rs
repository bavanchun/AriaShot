use camerashot_core::annotation::{Annotation, AnnotationTool, RectFillStyle};
use camerashot_core::geometry::{Point, Rect};
use regex::Regex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PiiCategory {
    Email,
    Phone,
    Ssn,
    CreditCard,
    Cvv,
    Expiry,
    Ipv4,
    AwsKey,
    SecretAssignment,
    HexKey,
    BearerToken,
}

impl PiiCategory {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Email => "Email",
            Self::Phone => "Phone Number",
            Self::Ssn => "SSN",
            Self::CreditCard => "Credit Card",
            Self::Cvv => "CVV Code",
            Self::Expiry => "Expiry Date",
            Self::Ipv4 => "IP Address",
            Self::AwsKey => "AWS Key",
            Self::SecretAssignment => "Secret / Token",
            Self::HexKey => "Hex Key / Hash",
            Self::BearerToken => "Bearer Token",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PiiMatch {
    pub category: PiiCategory,
    pub text: String,
    pub start: usize,
    pub end: usize,
}

/// Fast multi-pattern regular expression matcher for 11 sensitive data categories.
pub struct PiiDetector {
    patterns: Vec<(PiiCategory, Regex)>,
}

impl Default for PiiDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl PiiDetector {
    pub fn new() -> Self {
        let defs: Vec<(PiiCategory, &str)> = vec![
            (PiiCategory::Email, r"(?i)[a-zA-Z0-9._%+\-]+@[a-zA-Z0-9.\-]+\.[a-zA-Z]{2,}"),
            (PiiCategory::Phone, r"(?:\+?1[-.\s]?)?(?:\(?\d{3}\)?[-.\s]?)\d{3}[-.\s]?\d{4}"),
            (PiiCategory::Ssn, r"\b\d{3}[-\s]\d{2}[-\s]\d{4}\b"),
            (PiiCategory::CreditCard, r"\b(?:\d{4}[-\s]*){3}\d{1,7}\b"),
            (PiiCategory::CreditCard, r"\b\d{4}[-\s]*\d{6}[-\s]*\d{5}\b"),
            (PiiCategory::Cvv, r"(?i)(?:CVV|CVC|CSC|CCV)\s*:?\s*\d{3,4}"),
            (PiiCategory::Expiry, r"\b(?:\d{2}[/\-]\d{2,4}|\d{4}[/\-]\d{2})\b"),
            (PiiCategory::Ipv4, r"\b(?:(?:25[0-5]|2[0-4]\d|[01]?\d\d?)\.){3}(?:25[0-5]|2[0-4]\d|[01]?\d\d?)\b"),
            (PiiCategory::AwsKey, r"\b(?:AKIA|ABIA|ACCA|ASIA)[0-9A-Z]{16}\b"),
            (PiiCategory::SecretAssignment, r"(?i)(?:password|passwd|secret|token|api[_-]?key|access[_-]?key|private[_-]?key)\s*[:=]\s*\S+"),
            (PiiCategory::HexKey, r"\b[0-9a-fA-F]{32,64}\b"),
            (PiiCategory::BearerToken, r"Bearer\s+[A-Za-z0-9\-._~+/]+=*"),
        ];

        let mut patterns = Vec::with_capacity(defs.len());
        for (category, pattern_str) in defs {
            if let Ok(re) = Regex::new(pattern_str) {
                patterns.push((category, re));
            }
        }

        Self { patterns }
    }

    /// Scan text and return all detected sensitive matches.
    pub fn scan_text(&self, text: &str) -> Vec<PiiMatch> {
        let mut matches = Vec::new();
        for (category, re) in &self.patterns {
            for m in re.find_iter(text) {
                matches.push(PiiMatch {
                    category: *category,
                    text: m.as_str().to_string(),
                    start: m.start(),
                    end: m.end(),
                });
            }
        }
        matches
    }
}

/// Generates redaction annotations (pixelate, blur, or filled rectangle)
/// with a shared batch UUID so they can be atomically undone with a single Ctrl+Z.
pub struct PiiRedactor {
    detector: PiiDetector,
}

impl Default for PiiRedactor {
    fn default() -> Self {
        Self::new()
    }
}

impl PiiRedactor {
    pub fn new() -> Self {
        Self {
            detector: PiiDetector::new(),
        }
    }

    pub fn detector(&self) -> &PiiDetector {
        &self.detector
    }

    /// Build redaction annotations for a set of text occurrences and their corresponding screen bounding boxes.
    /// `tool`: AnnotationTool::Pixelate, AnnotationTool::Blur, or AnnotationTool::FilledRectangle.
    pub fn build_redactions_for_blocks(
        &self,
        blocks: &[(&str, Rect)],
        tool: AnnotationTool,
        padding: f64,
    ) -> (Uuid, Vec<Annotation>) {
        let batch_id = Uuid::new_v4();
        let mut annotations = Vec::new();

        for (text, block_rect) in blocks {
            let mut matches = self.detector.scan_text(text);
            if matches.is_empty() {
                continue;
            }

            // Sort matches by start ascending, then length descending
            matches.sort_by(|a, b| a.start.cmp(&b.start).then_with(|| b.end.cmp(&a.end)));

            // Merge overlapping intervals [start, end]
            let mut merged_ranges: Vec<(usize, usize)> = Vec::new();
            for m in matches {
                if let Some(last) = merged_ranges.last_mut() {
                    if m.start <= last.1 {
                        // Overlap detected: extend end boundary if larger
                        last.1 = last.1.max(m.end);
                        continue;
                    }
                }
                merged_ranges.push((m.start, m.end));
            }

            let text_len = (text.chars().count().max(1)) as f64;

            for (start_idx, end_idx) in merged_ranges {
                // Calculate sub-rect proportional to character position within the block
                let char_start = text[..start_idx].chars().count() as f64;
                let char_len = text[start_idx..end_idx].chars().count() as f64;

                let frac_x = char_start / text_len;
                let frac_w = char_len / text_len;

                let sub_x = (block_rect.origin.x + frac_x * block_rect.size.width - padding).max(0.0);
                let sub_y = (block_rect.origin.y - padding).max(0.0);
                let sub_w = frac_w * block_rect.size.width + padding * 2.0;
                let sub_h = block_rect.size.height + padding * 2.0;

                let mut ann = Annotation::new(
                    tool,
                    Point::new(sub_x, sub_y),
                    Point::new(sub_x + sub_w, sub_y + sub_h),
                    [0, 0, 0, 255],
                    0.0,
                );

                ann.group_id = Some(batch_id);
                if tool == AnnotationTool::FilledRectangle {
                    ann.fill_style = Some(RectFillStyle::Fill);
                    ann.fill_color = Some([0, 0, 0, 255]);
                }

                annotations.push(ann);
            }
        }

        (batch_id, annotations)
    }
}
