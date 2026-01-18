//! Brush tool - freehand drawing.

use super::Tool;
use crate::annotate::state::ToolProperties;
use gartk_core::{InputEvent, MouseButton, Point};
use gartk_render::cairo::Context;
use gartk_render::set_color;
use gartk_x11::CursorShape;

/// Freehand brush drawing tool.
pub struct BrushTool {
    /// Points in the current stroke.
    points: Vec<Point>,
    /// Whether we're currently drawing.
    drawing: bool,
}

impl BrushTool {
    /// Create a new brush tool.
    pub fn new() -> Self {
        Self {
            points: Vec::new(),
            drawing: false,
        }
    }

    /// Draw the stroke path.
    fn draw_stroke(&self, ctx: &Context, props: &ToolProperties) {
        if self.points.len() < 2 {
            return;
        }

        set_color(ctx, props.color);
        ctx.set_line_width(props.line_width);
        ctx.set_line_cap(cairo::LineCap::Round);
        ctx.set_line_join(cairo::LineJoin::Round);

        ctx.new_path();
        ctx.move_to(self.points[0].x as f64, self.points[0].y as f64);

        for point in &self.points[1..] {
            ctx.line_to(point.x as f64, point.y as f64);
        }

        let _ = ctx.stroke();
    }
}

impl Default for BrushTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for BrushTool {
    fn handle_event(&mut self, event: &InputEvent, _props: &ToolProperties) -> bool {
        match event {
            InputEvent::MousePress(e) if e.button == Some(MouseButton::Left) => {
                self.points.clear();
                self.points.push(e.position);
                self.drawing = true;
                true
            }
            InputEvent::MouseMove(e) if self.drawing => {
                self.points.push(e.position);
                true
            }
            InputEvent::MouseRelease(e) if e.button == Some(MouseButton::Left) && self.drawing => {
                self.points.push(e.position);
                self.drawing = false;
                true
            }
            _ => false,
        }
    }

    fn draw_preview(&self, ctx: &Context, props: &ToolProperties) {
        self.draw_stroke(ctx, props);
    }

    fn commit(&self, ctx: &Context, props: &ToolProperties) {
        self.draw_stroke(ctx, props);
    }

    fn reset(&mut self) {
        self.points.clear();
        self.drawing = false;
    }

    fn cursor(&self) -> CursorShape {
        CursorShape::Crosshair
    }

    fn is_drawing(&self) -> bool {
        self.drawing
    }

    fn can_commit(&self) -> bool {
        self.points.len() >= 2
    }
}
