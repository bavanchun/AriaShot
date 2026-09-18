pub mod annotation;
pub mod beautify;
pub mod chaikin;
pub mod geometry;
pub mod renderer;
pub mod scroll;
pub mod snap_index;
pub mod undo;

pub use annotation::{Annotation, AnnotationTool, LineStyle, NumberFormat, RectFillStyle};
pub use beautify::{BeautifyConfig, BeautifyMode, BeautifyRenderer, BEAUTIFY_PRESETS};
pub use chaikin::{
    chaikin_smooth, interpolate_to_count, moving_average_smooth, moving_average_smooth_values,
    smooth_pencil_stroke, PencilSmoothMode, DEFAULT_SMOOTH_WINDOW_SIZE,
};
pub use geometry::{Point, Rect, Size};
pub use renderer::AnnotationRenderer;
pub use scroll::{SadFilter, ScrollStitcher, SettlementDetector, VerticalShiftEstimator};
pub use snap_index::{BoundarySnapIndex, Hit};
pub use undo::{UndoAction, UndoStack};
