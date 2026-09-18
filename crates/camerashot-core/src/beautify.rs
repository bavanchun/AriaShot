use serde::{Deserialize, Serialize};
use tiny_skia::{
    Color, FillRule, GradientStop, LinearGradient, Paint, PathBuilder, Pixmap, PixmapPaint, Point,
    Rect as SkRect, SpreadMode, Transform,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BeautifyMode {
    #[default]
    Window = 0,  // macOS window chrome with traffic lights
    Rounded = 1, // Rounded corners only, no title bar
}

#[derive(Debug, Clone, Copy)]
pub struct GradientPreset {
    pub name: &'static str,
    pub colors: &'static [[f32; 4]],
}

pub const BEAUTIFY_PRESETS: &[GradientPreset] = &[
    GradientPreset {
        name: "Ultraviolet",
        colors: &[
            [0.55, 0.10, 0.95, 1.0],
            [0.85, 0.20, 0.85, 1.0],
            [0.15, 0.20, 0.95, 1.0],
        ],
    },
    GradientPreset {
        name: "Inferno",
        colors: &[
            [1.00, 0.25, 0.40, 1.0],
            [1.00, 0.50, 0.20, 1.0],
            [1.00, 0.85, 0.25, 1.0],
        ],
    },
    GradientPreset {
        name: "Emerald",
        colors: &[
            [0.05, 0.65, 0.45, 1.0],
            [0.15, 0.85, 0.60, 1.0],
            [0.05, 0.40, 0.30, 1.0],
        ],
    },
    GradientPreset {
        name: "Cyberpunk",
        colors: &[
            [0.00, 0.80, 0.95, 1.0],
            [0.50, 0.10, 0.85, 1.0],
            [0.95, 0.10, 0.55, 1.0],
        ],
    },
    GradientPreset {
        name: "Sunset",
        colors: &[
            [0.98, 0.45, 0.30, 1.0],
            [0.90, 0.20, 0.50, 1.0],
            [0.45, 0.10, 0.60, 1.0],
        ],
    },
    GradientPreset {
        name: "Midnight",
        colors: &[
            [0.08, 0.10, 0.18, 1.0],
            [0.15, 0.20, 0.35, 1.0],
            [0.05, 0.05, 0.10, 1.0],
        ],
    },
    GradientPreset {
        name: "Pastel Dream",
        colors: &[
            [0.95, 0.80, 0.85, 1.0],
            [0.85, 0.80, 0.95, 1.0],
            [0.80, 0.90, 0.95, 1.0],
        ],
    },
    GradientPreset {
        name: "Deep Ocean",
        colors: &[
            [0.05, 0.20, 0.45, 1.0],
            [0.10, 0.45, 0.70, 1.0],
            [0.02, 0.10, 0.25, 1.0],
        ],
    },
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeautifyConfig {
    pub mode: BeautifyMode,
    pub style_index: usize,
    pub padding: f32,
    pub corner_radius: f32,
    pub shadow_radius: f32,
    pub bg_radius: f32,
}

impl Default for BeautifyConfig {
    fn default() -> Self {
        Self {
            mode: BeautifyMode::Window,
            style_index: 0,
            padding: 48.0,
            corner_radius: 10.0,
            shadow_radius: 20.0,
            bg_radius: 12.0,
        }
    }
}

/// Beautify Renderer producing synthetic macOS window framing and gradient backdrops.
pub struct BeautifyRenderer;

impl BeautifyRenderer {
    pub fn render(source: &Pixmap, config: &BeautifyConfig) -> Option<Pixmap> {
        let title_bar_height = if config.mode == BeautifyMode::Window { 36.0f32 } else { 0.0f32 };
        let content_w = source.width() as f32;
        let content_h = source.height() as f32;

        let total_w = (content_w + config.padding * 2.0).round() as u32;
        let total_h = (content_h + config.padding * 2.0 + title_bar_height).round() as u32;

        let mut canvas = Pixmap::new(total_w, total_h)?;

        // 1. Draw gradient background
        let preset = &BEAUTIFY_PRESETS[config.style_index % BEAUTIFY_PRESETS.len()];
        let num_stops = preset.colors.len();
        let mut stops = Vec::with_capacity(num_stops);
        for (i, c) in preset.colors.iter().enumerate() {
            let pos = (i as f32) / ((num_stops - 1) as f32);
            stops.push(GradientStop::new(
                pos,
                Color::from_rgba(c[0], c[1], c[2], c[3])?,
            ));
        }

        let bg_shader = LinearGradient::new(
            Point::from_xy(0.0, 0.0),
            Point::from_xy(total_w as f32, total_h as f32),
            stops,
            SpreadMode::Pad,
            Transform::identity(),
        )?;

        let mut bg_paint = Paint::default();
        bg_paint.shader = bg_shader;
        bg_paint.anti_alias = true;

        let bg_path = Self::rounded_rect_path(0.0, 0.0, total_w as f32, total_h as f32, config.bg_radius);
        canvas.fill_path(&bg_path, &bg_paint, FillRule::Winding, Transform::identity(), None);

        // 2. Window position
        let win_x = config.padding;
        let win_y = config.padding;
        let win_w = content_w;
        let win_h = content_h + title_bar_height;

        // 3. Draw soft shadow under the window
        if config.shadow_radius > 0.0 {
            let mut shadow_paint = Paint::default();
            shadow_paint.set_color_rgba8(0, 0, 0, 45);
            shadow_paint.anti_alias = true;
            let shadow_path = Self::rounded_rect_path(
                win_x - 2.0,
                win_y + 4.0,
                win_w + 4.0,
                win_h + 4.0,
                config.corner_radius + 2.0,
            );
            canvas.fill_path(&shadow_path, &shadow_paint, FillRule::Winding, Transform::identity(), None);
        }

        // 4. Draw window base / title bar
        if config.mode == BeautifyMode::Window {
            // Title bar background
            let mut title_paint = Paint::default();
            title_paint.set_color_rgba8(35, 35, 38, 255);
            title_paint.anti_alias = true;
            let title_path = Self::rounded_rect_path(win_x, win_y, win_w, win_h, config.corner_radius);
            canvas.fill_path(&title_path, &title_paint, FillRule::Winding, Transform::identity(), None);

            // Traffic light buttons
            let button_y = win_y + title_bar_height / 2.0;
            let button_radius = 5.5f32;
            let start_x = win_x + 16.0;
            let spacing = 18.0;

            // Close (Red #FF5F56)
            Self::draw_circle(&mut canvas, start_x, button_y, button_radius, Color::from_rgba8(255, 95, 86, 255));
            // Minimize (Yellow #FFBD2E)
            Self::draw_circle(&mut canvas, start_x + spacing, button_y, button_radius, Color::from_rgba8(255, 189, 46, 255));
            // Maximize (Green #27C93F)
            Self::draw_circle(&mut canvas, start_x + spacing * 2.0, button_y, button_radius, Color::from_rgba8(39, 201, 63, 255));
        }

        // 5. Blit screenshot source image into window content area
        let blit_x = win_x.round() as i32;
        let blit_y = (win_y + title_bar_height).round() as i32;
        canvas.draw_pixmap(
            blit_x,
            blit_y,
            source.as_ref(),
            &PixmapPaint::default(),
            Transform::identity(),
            None,
        );

        Some(canvas)
    }

    fn rounded_rect_path(x: f32, y: f32, w: f32, h: f32, r: f32) -> tiny_skia::Path {
        let mut pb = PathBuilder::new();
        let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
        pb.move_to(x + r, y);
        pb.line_to(x + w - r, y);
        pb.quad_to(x + w, y, x + w, y + r);
        pb.line_to(x + w, y + h - r);
        pb.quad_to(x + w, y + h, x + w - r, y + h);
        pb.line_to(x + r, y + h);
        pb.quad_to(x, y + h, x, y + h - r);
        pb.line_to(x, y + r);
        pb.quad_to(x, y, x + r, y);
        pb.close();
        pb.finish().unwrap_or_else(|| {
            let mut fallback = PathBuilder::new();
            if let Some(r) = SkRect::from_xywh(x, y, w, h) {
                fallback.push_rect(r);
            }
            fallback.finish().unwrap()
        })
    }

    fn draw_circle(canvas: &mut Pixmap, cx: f32, cy: f32, radius: f32, color: Color) {
        let mut pb = PathBuilder::new();
        pb.push_circle(cx, cy, radius);
        if let Some(path) = pb.finish() {
            let mut paint = Paint::default();
            paint.set_color(color);
            paint.anti_alias = true;
            canvas.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
        }
    }
}
