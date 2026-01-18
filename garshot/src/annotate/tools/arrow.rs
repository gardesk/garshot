//! Arrow tool - draw lines with arrowheads.

use super::Tool;
use crate::annotate::state::ToolProperties;
use gartk_core::{Color, InputEvent, MouseButton, Point};
use gartk_render::cairo::Context;
use gartk_render::{line as draw_line, set_color};
use gartk_x11::CursorShape;
use std::f64::consts::PI;

/// Arrow drawing tool.
pub struct ArrowTool {
    /// Starting point of the arrow (tail).
    start: Option<Point>,
    /// Ending point of the arrow (head).
    end: Option<Point>,
    /// Whether we're currently drawing.
    drawing: bool,
}

impl ArrowTool {
    /// Create a new arrow tool.
    pub fn new() -> Self {
        Self {
            start: None,
            end: None,
            drawing: false,
        }
    }

    /// Draw an arrowhead at the given point.
    fn draw_arrowhead(ctx: &Context, tip: Point, angle: f64, size: f64, color: Color) {
        let wing_angle = PI / 6.0; // 30 degrees

        // Calculate wing points
        let wing1_angle = angle + PI - wing_angle;
        let wing2_angle = angle + PI + wing_angle;

        let wing1 = Point::new(
            tip.x + (size * wing1_angle.cos()) as i32,
            tip.y + (size * wing1_angle.sin()) as i32,
        );
        let wing2 = Point::new(
            tip.x + (size * wing2_angle.cos()) as i32,
            tip.y + (size * wing2_angle.sin()) as i32,
        );

        // Draw filled triangle
        set_color(ctx, color);
        ctx.new_path();
        ctx.move_to(tip.x as f64, tip.y as f64);
        ctx.line_to(wing1.x as f64, wing1.y as f64);
        ctx.line_to(wing2.x as f64, wing2.y as f64);
        ctx.close_path();
        let _ = ctx.fill();
    }
}

impl Default for ArrowTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for ArrowTool {
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
            // Draw the line
            draw_line(
                ctx,
                start.x as f64,
                start.y as f64,
                end.x as f64,
                end.y as f64,
                props.color,
                props.line_width,
            );

            // Calculate arrow angle
            let dx = (end.x - start.x) as f64;
            let dy = (end.y - start.y) as f64;
            let angle = dy.atan2(dx);

            // Draw arrowhead (size proportional to line width)
            let head_size = props.line_width * 4.0;
            Self::draw_arrowhead(ctx, end, angle, head_size, props.color);
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
            start.x != end.x || start.y != end.y
        } else {
            false
        }
    }
}
