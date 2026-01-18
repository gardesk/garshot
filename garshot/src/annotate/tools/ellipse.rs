//! Ellipse tool - draw circles and ellipses.

use super::Tool;
use crate::annotate::state::ToolProperties;
use gartk_core::{InputEvent, MouseButton, Point};
use gartk_render::cairo::Context;
use gartk_render::set_color;
use gartk_x11::CursorShape;
use std::f64::consts::PI;

/// Ellipse drawing tool.
pub struct EllipseTool {
    /// Center or first corner of bounding box.
    start: Option<Point>,
    /// Edge point or opposite corner.
    end: Option<Point>,
    /// Whether we're currently drawing.
    drawing: bool,
}

impl EllipseTool {
    /// Create a new ellipse tool.
    pub fn new() -> Self {
        Self {
            start: None,
            end: None,
            drawing: false,
        }
    }

    /// Calculate ellipse parameters from start and end points.
    /// Returns (center_x, center_y, radius_x, radius_y).
    fn calculate_ellipse(&self) -> Option<(f64, f64, f64, f64)> {
        let (start, end) = (self.start?, self.end?);

        // Bounding box mode: start and end are opposite corners
        let cx = (start.x + end.x) as f64 / 2.0;
        let cy = (start.y + end.y) as f64 / 2.0;
        let rx = ((end.x - start.x) as f64 / 2.0).abs();
        let ry = ((end.y - start.y) as f64 / 2.0).abs();

        if rx > 0.0 && ry > 0.0 {
            Some((cx, cy, rx, ry))
        } else {
            None
        }
    }

    /// Draw an ellipse using Cairo.
    fn draw_ellipse(&self, ctx: &Context, props: &ToolProperties) {
        if let Some((cx, cy, rx, ry)) = self.calculate_ellipse() {
            // Use scaling to draw ellipse from circle
            ctx.save().ok();
            ctx.translate(cx, cy);
            ctx.scale(rx, ry);

            ctx.new_path();
            ctx.arc(0.0, 0.0, 1.0, 0.0, 2.0 * PI);

            ctx.restore().ok();

            set_color(ctx, props.color);

            if props.fill {
                let _ = ctx.fill();
            } else {
                // Scale line width to maintain consistent appearance
                ctx.set_line_width(props.line_width);
                let _ = ctx.stroke();
            }
        }
    }
}

impl Default for EllipseTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for EllipseTool {
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
        self.draw_ellipse(ctx, props);
    }

    fn commit(&self, ctx: &Context, props: &ToolProperties) {
        self.draw_ellipse(ctx, props);
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
        self.calculate_ellipse().is_some()
    }
}
