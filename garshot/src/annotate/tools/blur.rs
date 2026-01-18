//! Blur tool - pixelate regions for redaction.

use super::Tool;
use crate::annotate::state::ToolProperties;
use gartk_core::{InputEvent, MouseButton, Point, Rect};
use gartk_render::cairo::Context;
use gartk_render::stroke_rect;
use gartk_x11::CursorShape;

/// Blur/pixelate tool for redacting sensitive information.
pub struct BlurTool {
    /// Starting corner of the region.
    start: Option<Point>,
    /// Opposite corner of the region.
    end: Option<Point>,
    /// Whether we're currently drawing.
    drawing: bool,
}

impl BlurTool {
    /// Create a new blur tool.
    pub fn new() -> Self {
        Self {
            start: None,
            end: None,
            drawing: false,
        }
    }

    /// Calculate normalized rectangle from start and end points.
    pub fn calculate_rect(&self) -> Option<Rect> {
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

    /// Draw the selection rectangle outline.
    fn draw_selection(&self, ctx: &Context, props: &ToolProperties) {
        if let Some(rect) = self.calculate_rect() {
            // Draw dashed outline to indicate blur region
            ctx.set_dash(&[5.0, 5.0], 0.0);
            stroke_rect(ctx, rect, props.color, 2.0);
            ctx.set_dash(&[], 0.0);
        }
    }
}

impl Default for BlurTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for BlurTool {
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
        self.draw_selection(ctx, props);
    }

    fn commit(&self, ctx: &Context, props: &ToolProperties) {
        // For the preview/commit, we draw a pixelated pattern
        // The actual blur is applied by the overlay when committing
        if let Some(rect) = self.calculate_rect() {
            // Draw a checkerboard pattern to indicate blur
            let block_size = props.blur_radius.max(8) as f64;

            ctx.save().ok();
            ctx.rectangle(
                rect.x as f64,
                rect.y as f64,
                rect.width as f64,
                rect.height as f64,
            );
            ctx.clip();

            // Draw checkerboard pattern
            let mut dark = true;
            let mut y = rect.y as f64;
            while y < (rect.y + rect.height as i32) as f64 {
                let mut x = rect.x as f64;
                let row_start_dark = dark;
                while x < (rect.x + rect.width as i32) as f64 {
                    if dark {
                        ctx.set_source_rgba(0.2, 0.2, 0.2, 0.8);
                    } else {
                        ctx.set_source_rgba(0.3, 0.3, 0.3, 0.8);
                    }
                    ctx.rectangle(x, y, block_size, block_size);
                    let _ = ctx.fill();
                    dark = !dark;
                    x += block_size;
                }
                dark = !row_start_dark;
                y += block_size;
            }

            ctx.restore().ok();
        }
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
