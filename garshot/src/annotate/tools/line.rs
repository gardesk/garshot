//! Line tool - draw straight lines.

use super::Tool;
use crate::annotate::state::ToolProperties;
use gartk_core::{InputEvent, MouseButton, Point};
use gartk_render::cairo::Context;
use gartk_render::line as draw_line;
use gartk_x11::CursorShape;

/// Line drawing tool.
pub struct LineTool {
    /// Starting point of the line.
    start: Option<Point>,
    /// Ending point of the line.
    end: Option<Point>,
    /// Whether we're currently drawing.
    drawing: bool,
}

impl LineTool {
    /// Create a new line tool.
    pub fn new() -> Self {
        Self {
            start: None,
            end: None,
            drawing: false,
        }
    }
}

impl Default for LineTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for LineTool {
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
        if let (Some(start), Some(end)) = (self.start, self.end) {
            draw_line(
                ctx,
                start.x as f64,
                start.y as f64,
                end.x as f64,
                end.y as f64,
                props.color,
                props.line_width,
            );
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
        if let (Some(start), Some(end)) = (self.start, self.end) {
            // Line is valid if it has some length
            start.x != end.x || start.y != end.y
        } else {
            false
        }
    }
}
