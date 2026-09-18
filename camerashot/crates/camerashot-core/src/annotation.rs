use crate::geometry::{Point, Rect};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 18 vector and raster annotation tools matching macshot's tool suite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AnnotationTool {
    Pencil,
    Line,
    Arrow,
    Rectangle,
    FilledRectangle,
    Ellipse,
    Marker,
    Text,
    Number,
    Pixelate,
    Blur,
    Measure,
    Loupe,
    Select,
    TranslateOverlay,
    Crop,
    ColorSampler,
    Stamp,
    Highlight,
}

/// Line stroke styles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LineStyle {
    #[default]
    Solid = 0,
    Dashed = 1,
    Dotted = 2,
}

/// Rectangle and shape fill options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum RectFillStyle {
    #[default]
    Stroke = 0,
    StrokeAndFill = 1,
    Fill = 2,
}

/// Formatting style for numbered counter badges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum NumberFormat {
    #[default]
    Decimal = 0,
    Roman = 1,
    Alpha = 2,
    AlphaLower = 3,
}

impl NumberFormat {
    pub fn format(&self, number: usize) -> String {
        match self {
            Self::Decimal => number.to_string(),
            Self::Roman => Self::to_roman(number),
            Self::Alpha => Self::to_alpha(number, true),
            Self::AlphaLower => Self::to_alpha(number, false),
        }
    }

    fn to_roman(mut n: usize) -> String {
        if n == 0 {
            return "0".to_string();
        }
        let table = [
            (1000, "M"),
            (900, "CM"),
            (500, "D"),
            (400, "CD"),
            (100, "C"),
            (90, "XC"),
            (50, "L"),
            (40, "XL"),
            (10, "X"),
            (9, "IX"),
            (5, "V"),
            (4, "IV"),
            (1, "I"),
        ];
        let mut result = String::new();
        n = n.min(3999);
        for (val, symbol) in table {
            while n >= val {
                result.push_str(symbol);
                n -= val;
            }
        }
        result
    }

    fn to_alpha(mut n: usize, uppercase: bool) -> String {
        if n == 0 {
            return if uppercase { "A".to_string() } else { "a".to_string() };
        }
        let base_char = if uppercase { b'A' } else { b'a' };
        let mut chars = Vec::new();
        while n > 0 {
            n -= 1;
            let rem = (n % 26) as u8;
            chars.push((base_char + rem) as char);
            n /= 26;
        }
        chars.into_iter().rev().collect()
    }
}

/// Core domain model for a single vector annotation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Annotation {
    pub id: Uuid,
    /// Batch group identifier for atomic actions (e.g. auto-redact batch or multi-selection).
    pub group_id: Option<Uuid>,
    pub tool: AnnotationTool,
    pub start_point: Point,
    pub end_point: Point,
    pub points: Option<Vec<Point>>,
    pub pressures: Option<Vec<f64>>,
    pub color: [u8; 4],
    pub stroke_width: f64,
    pub line_style: LineStyle,
    pub fill_style: Option<RectFillStyle>,
    pub fill_color: Option<[u8; 4]>,
    pub number_value: Option<usize>,
    pub number_format: Option<NumberFormat>,
    pub text: Option<String>,
    pub font_size: Option<f64>,
    pub font_family: Option<String>,
    pub blur_radius: Option<f64>,
    pub loupe_magnification: Option<f64>,
    pub stamp_text: Option<String>,
    pub is_locked: bool,
    pub z_index: usize,
}

impl Annotation {
    pub fn new(tool: AnnotationTool, start_point: Point, end_point: Point, color: [u8; 4], stroke_width: f64) -> Self {
        Self {
            id: Uuid::new_v4(),
            group_id: None,
            tool,
            start_point,
            end_point,
            points: None,
            pressures: None,
            color,
            stroke_width,
            line_style: LineStyle::Solid,
            fill_style: None,
            fill_color: None,
            number_value: None,
            number_format: None,
            text: None,
            font_size: None,
            font_family: None,
            blur_radius: None,
            loupe_magnification: None,
            stamp_text: None,
            is_locked: false,
            z_index: 0,
        }
    }

    /// Calculate bounding box covering the annotation's geometry.
    pub fn bounding_box(&self) -> Rect {
        if let Some(ref pts) = self.points {
            if !pts.is_empty() {
                let mut min_x = pts[0].x;
                let mut max_x = pts[0].x;
                let mut min_y = pts[0].y;
                let mut max_y = pts[0].y;
                for p in pts.iter().skip(1) {
                    min_x = min_x.min(p.x);
                    max_x = max_x.max(p.x);
                    min_y = min_y.min(p.y);
                    max_y = max_y.max(p.y);
                }
                let half_stroke = self.stroke_width / 2.0;
                return Rect::new(
                    min_x - half_stroke,
                    min_y - half_stroke,
                    (max_x - min_x) + self.stroke_width,
                    (max_y - min_y) + self.stroke_width,
                );
            }
        }

        let base = Rect::from_points(self.start_point, self.end_point);
        let half_stroke = self.stroke_width / 2.0;
        base.inset(-half_stroke, -half_stroke)
    }
}
