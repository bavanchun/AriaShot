use serde::{Deserialize, Serialize};

/// A 2D point with f64 precision.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    #[inline]
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    #[inline]
    pub fn distance_to(&self, other: Point) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }

    #[inline]
    pub fn lerp(&self, other: Point, t: f64) -> Self {
        Self {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
        }
    }
}

/// A 2D size with f64 precision.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

impl Size {
    pub const ZERO: Self = Self {
        width: 0.0,
        height: 0.0,
    };

    #[inline]
    pub fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }
}

/// A 2D rectangle in top-left coordinate space.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub origin: Point,
    pub size: Size,
}

impl Rect {
    pub const ZERO: Self = Self {
        origin: Point::ZERO,
        size: Size::ZERO,
    };

    #[inline]
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            origin: Point::new(x, y),
            size: Size::new(width, height),
        }
    }

    #[inline]
    pub fn from_points(p1: Point, p2: Point) -> Self {
        let min_x = p1.x.min(p2.x);
        let min_y = p1.y.min(p2.y);
        let max_x = p1.x.max(p2.x);
        let max_y = p1.y.max(p2.y);
        Self {
            origin: Point::new(min_x, min_y),
            size: Size::new(max_x - min_x, max_y - min_y),
        }
    }

    #[inline]
    pub fn min_x(&self) -> f64 {
        self.origin.x
    }

    #[inline]
    pub fn min_y(&self) -> f64 {
        self.origin.y
    }

    #[inline]
    pub fn max_x(&self) -> f64 {
        self.origin.x + self.size.width
    }

    #[inline]
    pub fn max_y(&self) -> f64 {
        self.origin.y + self.size.height
    }

    #[inline]
    pub fn width(&self) -> f64 {
        self.size.width
    }

    #[inline]
    pub fn height(&self) -> f64 {
        self.size.height
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.size.is_empty()
    }

    #[inline]
    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.min_x() && p.x <= self.max_x() && p.y >= self.min_y() && p.y <= self.max_y()
    }

    #[inline]
    pub fn intersects(&self, other: &Rect) -> bool {
        self.min_x() <= other.max_x()
            && self.max_x() >= other.min_x()
            && self.min_y() <= other.max_y()
            && self.max_y() >= other.min_y()
    }

    #[inline]
    pub fn intersection(&self, other: &Rect) -> Option<Rect> {
        if !self.intersects(other) {
            return None;
        }
        let x0 = self.min_x().max(other.min_x());
        let y0 = self.min_y().max(other.min_y());
        let x1 = self.max_x().min(other.max_x());
        let y1 = self.max_y().min(other.max_y());
        Some(Rect::new(x0, y0, (x1 - x0).max(0.0), (y1 - y0).max(0.0)))
    }

    #[inline]
    pub fn union(&self, other: &Rect) -> Rect {
        if self.is_empty() {
            return *other;
        }
        if other.is_empty() {
            return *self;
        }
        let x0 = self.min_x().min(other.min_x());
        let y0 = self.min_y().min(other.min_y());
        let x1 = self.max_x().max(other.max_x());
        let y1 = self.max_y().max(other.max_y());
        Rect::new(x0, y0, x1 - x0, y1 - y0)
    }

    #[inline]
    pub fn inset(&self, dx: f64, dy: f64) -> Rect {
        Rect::new(
            self.origin.x + dx,
            self.origin.y + dy,
            (self.size.width - 2.0 * dx).max(0.0),
            (self.size.height - 2.0 * dy).max(0.0),
        )
    }

    #[inline]
    pub fn round(&self) -> Rect {
        let x = self.origin.x.round();
        let y = self.origin.y.round();
        let w = self.size.width.round();
        let h = self.size.height.round();
        Rect::new(x, y, w, h)
    }
}
