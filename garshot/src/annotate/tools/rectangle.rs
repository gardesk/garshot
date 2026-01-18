//! Rectangle tool - draw rectangles.

use super::Tool;
use crate::annotate::state::ToolProperties;
use gartk_core::{InputEvent, MouseButton, Point, Rect};
use gartk_render::cairo::Context;
use gartk_render::{fill_rect, stroke_rect};
use gartk_x11::CursorShape;

/// Rectangle drawing tool.
pub struct RectangleTool {
    /// Starting corner of the rectangle.
    start: Option<Point>,
    /// Opposite corner of the rectangle.
    end: Option<Point>,
    /// Whether we're currently drawing.
    drawing: bool,
}

impl RectangleTool {
    /// Create a new rectangle tool.
    pub fn new() -> Self {
        Self {
            start: None,
            end: None,
            drawing: false,
        }
    }

    /// Calculate normalized rectangle from start and end points.
    fn calculate_rect(&self) -> Option<Rect> {
        let (start, end) = (self.start?, self.end?);

        let x = start.x.min(end.x);
        let y = start.y.min(end.y);
        let width = (end.x - start.x).unsigned_abs();
        let height = (end.y - start.y).unsigned_abs();

        if width > 0 && height > 0 {
            Some(Rect::new(x, y, width, height))
        } else {
            None
        }
    }
}

impl Default for RectangleTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for RectangleTool {
    fn handle_event(&mut self, event: &InputEvent, _props: &ToolProperties) -> bool {
        match event {
            InputEvent::MousePress(e) if e.button == Some(MouseButton::Left) => {
                self.start = Some(e.position);
                self.end = Some(e.position);
                self.drawing = true;
                true
            }
            InputEvent::MouseMove(e) if self.drawing => {
                self.end = Some(e.position);
                true
            }
            InputEvent::MouseRelease(e) if e.button == Some(MouseButton::Left) && self.drawing => {
                self.end = Some(e.position);
                self.drawing = false;
                true
            }
            _ => false,
        }
    }

    fn draw_preview(&self, ctx: &Context, props: &ToolProperties) {
        if let Some(rect) = self.calculate_rect() {
            if props.fill {
                fill_rect(ctx, rect, props.color);
            } else {
                stroke_rect(ctx, rect, props.color, props.line_width);
            }
        }
    }

    fn commit(&self, ctx: &Context, props: &ToolProperties) {
        self.draw_preview(ctx, props);
    }

    fn reset(&mut self) {
        self.start = None;
        self.end = None;
        self.drawing = false;
    }

    fn cursor(&self) -> CursorShape {
        CursorShape::Crosshair
    }

    fn is_drawing(&self) -> bool {
        self.drawing
    }

    fn can_commit(&self) -> bool {
        self.calculate_rect().is_some()
    }
}
