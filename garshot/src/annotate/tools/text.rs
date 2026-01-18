//! Text tool - add text annotations.

use super::Tool;
use crate::annotate::state::ToolProperties;
use gartk_core::{InputEvent, Key, MouseButton, Point};
use gartk_render::cairo::Context;
use gartk_render::set_color;
use gartk_x11::CursorShape;

/// Text annotation tool.
pub struct TextTool {
    /// Position of text anchor.
    position: Option<Point>,
    /// Current text content.
    text: String,
    /// Whether we're in text editing mode.
    editing: bool,
    /// Cursor blink state (for visual feedback).
    cursor_visible: bool,
}

impl TextTool {
    /// Create a new text tool.
    pub fn new() -> Self {
        Self {
            position: None,
            text: String::new(),
            editing: false,
            cursor_visible: true,
        }
    }

    /// Draw the text with optional cursor.
    fn draw_text(&self, ctx: &Context, props: &ToolProperties, show_cursor: bool) {
        if let Some(pos) = self.position {
            if self.text.is_empty() && !show_cursor {
                return;
            }

            set_color(ctx, props.color);

            // Set font
            ctx.select_font_face("monospace", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
            ctx.set_font_size(props.font_size);

            // Draw text
            ctx.move_to(pos.x as f64, pos.y as f64 + props.font_size);

            if self.text.is_empty() {
                // Show placeholder cursor
                if show_cursor && self.cursor_visible {
                    let _ = ctx.show_text("|");
                }
            } else {
                let _ = ctx.show_text(&self.text);

                // Draw cursor at end
                if show_cursor && self.cursor_visible && self.editing {
                    let extents = match ctx.text_extents(&self.text) {
                        Ok(e) => e,
                        Err(_) => return,
                    };
                    ctx.move_to(
                        pos.x as f64 + extents.width(),
                        pos.y as f64 + props.font_size,
                    );
                    let _ = ctx.show_text("|");
                }
            }
        }
    }
}

impl Default for TextTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for TextTool {
    fn handle_event(&mut self, event: &InputEvent, _props: &ToolProperties) -> bool {
        match event {
            InputEvent::MousePress(e) if e.button == Some(MouseButton::Left) => {
                if self.editing {
                    // Clicking while editing commits the text at new position
                    self.editing = false;
                    // Don't reset - keep the text for commit
                    return true;
                }
                // Start new text at click position
                self.position = Some(e.position);
                self.text.clear();
                self.editing = true;
                true
            }
            InputEvent::Key(e) if self.editing && e.pressed => {
                match e.key {
                    Key::Return => {
                        // Finish editing
                        self.editing = false;
                        true
                    }
                    Key::Escape => {
                        // Cancel text
                        self.reset();
                        true
                    }
                    Key::Backspace => {
                        self.text.pop();
                        true
                    }
                    Key::Char(c) => {
                        // Handle character input
                        if !c.is_control() {
                            self.text.push(c);
                            return true;
                        }
                        false
                    }
                    Key::Space => {
                        self.text.push(' ');
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    fn draw_preview(&self, ctx: &Context, props: &ToolProperties) {
        self.draw_text(ctx, props, true);
    }

    fn commit(&self, ctx: &Context, props: &ToolProperties) {
        self.draw_text(ctx, props, false);
    }

    fn reset(&mut self) {
        self.position = None;
        self.text.clear();
        self.editing = false;
    }

    fn cursor(&self) -> CursorShape {
        CursorShape::Text
    }

    fn is_drawing(&self) -> bool {
        self.editing
    }

    fn can_commit(&self) -> bool {
        self.position.is_some() && !self.text.is_empty() && !self.editing
    }
}
