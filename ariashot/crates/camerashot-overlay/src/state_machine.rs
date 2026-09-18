use camerashot_core::geometry::{Point, Rect};
use camerashot_core::snap_index::BoundarySnapIndex;
use serde::{Deserialize, Serialize};

/// Interactive selection states during overlay lifetime.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OverlayState {
    /// Idle: mouse hovering across screen, loupe active, no selection initiated.
    Idle { cursor: Point },
    /// Selecting: mouse dragged to form rubber-band rectangle.
    Selecting {
        start: Point,
        current: Point,
        snapped_current: Point,
    },
    /// Selected: rectangle locked, ready for tool annotations, resize, or export.
    Selected { rect: Rect },
    /// Resizing: dragging one of the 8 selection boundary handles.
    Resizing {
        original_rect: Rect,
        handle: ResizeHandle,
        current: Point,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResizeHandle {
    TopLeft,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
}

pub struct SelectionController {
    pub state: OverlayState,
    pub snap_radius_points: f64,
}

impl Default for SelectionController {
    fn default() -> Self {
        Self {
            state: OverlayState::Idle {
                cursor: Point::ZERO,
            },
            snap_radius_points: 8.0,
        }
    }
}

impl SelectionController {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn on_mouse_move(&mut self, pt: Point, snap_index: Option<&BoundarySnapIndex>) {
        match &mut self.state {
            OverlayState::Idle { cursor } => {
                *cursor = pt;
            }
            OverlayState::Selecting {
                start,
                current,
                snapped_current,
            } => {
                *current = pt;
                let mut snapped = pt;

                if let Some(index) = snap_index {
                    // Test vertical boundary snapping along the current selection Y span
                    let y_min = start.y.min(pt.y);
                    let y_max = start.y.max(pt.y);
                    if let Some(hit_x) =
                        index.nearest_vertical(pt.x, y_min, y_max, self.snap_radius_points)
                    {
                        snapped.x = hit_x.view_position;
                    }

                    // Test horizontal boundary snapping along the current selection X span
                    let x_min = start.x.min(pt.x);
                    let x_max = start.x.max(pt.x);
                    if let Some(hit_y) =
                        index.nearest_horizontal(pt.y, x_min, x_max, self.snap_radius_points)
                    {
                        snapped.y = hit_y.view_position;
                    }
                }

                *snapped_current = snapped;
            }
            _ => {}
        }
    }

    pub fn on_mouse_down(&mut self, pt: Point) {
        match self.state {
            OverlayState::Idle { .. } => {
                self.state = OverlayState::Selecting {
                    start: pt,
                    current: pt,
                    snapped_current: pt,
                };
            }
            OverlayState::Selected { rect } if !rect.contains(pt) => {
                // Click outside selection clears and restarts
                self.state = OverlayState::Selecting {
                    start: pt,
                    current: pt,
                    snapped_current: pt,
                };
            }
            _ => {}
        }
    }

    pub fn on_mouse_up(&mut self) {
        if let OverlayState::Selecting {
            start,
            snapped_current,
            ..
        } = self.state
        {
            let rect = Rect::from_points(start, snapped_current);
            if rect.width() >= 4.0 && rect.height() >= 4.0 {
                self.state = OverlayState::Selected { rect };
            } else {
                self.state = OverlayState::Idle {
                    cursor: snapped_current,
                };
            }
        }
    }

    pub fn current_selection_rect(&self) -> Option<Rect> {
        match self.state {
            OverlayState::Selecting {
                start,
                snapped_current,
                ..
            } => Some(Rect::from_points(start, snapped_current)),
            OverlayState::Selected { rect } => Some(rect),
            _ => None,
        }
    }
}
