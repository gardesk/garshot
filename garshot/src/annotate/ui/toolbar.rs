//! Annotation toolbar UI.

use crate::annotate::state::{ToolProperties, ToolType};
use gartk_core::{Color, Point, Rect};
use gartk_render::{
    fill_rect, fill_rounded_rect, set_color, stroke_rounded_rect, Surface,
};

/// Toolbar height in pixels.
pub const TOOLBAR_HEIGHT: u32 = 48;

/// Toolbar for selecting annotation tools.
pub struct Toolbar {
    /// Toolbar bounds.
    rect: Rect,
    /// Currently selected tool.
    selected_tool: ToolType,
    /// Current color.
    current_color: Color,
    /// Current line width.
    line_width: f64,
}

impl Toolbar {
    /// Tool button width.
    const BUTTON_WIDTH: u32 = 40;
    /// Button padding.
    const PADDING: u32 = 4;

    /// Create a new toolbar.
    pub fn new(screen_width: u32) -> Self {
        Self {
            rect: Rect::new(0, 0, screen_width, TOOLBAR_HEIGHT),
            selected_tool: ToolType::Arrow,
            current_color: Color::new(1.0, 0.4, 0.0, 1.0),
            line_width: 3.0,
        }
    }

    /// Update toolbar state from annotation state.
    pub fn update(&mut self, tool: ToolType, props: &ToolProperties) {
        self.selected_tool = tool;
        self.current_color = props.color;
        self.line_width = props.line_width;
    }

    /// Get the toolbar bounds.
    pub fn rect(&self) -> Rect {
        self.rect
    }

    /// Get the height.
    pub fn height(&self) -> u32 {
        TOOLBAR_HEIGHT
    }

    /// Calculate button rect for a tool at index.
    fn button_rect(&self, index: usize) -> Rect {
        let x = Self::PADDING as i32 + (index as i32 * (Self::BUTTON_WIDTH as i32 + Self::PADDING as i32));
        let y = Self::PADDING as i32;
        Rect::new(x, y, Self::BUTTON_WIDTH, TOOLBAR_HEIGHT - Self::PADDING * 2)
    }

    /// Draw the toolbar onto a surface.
    pub fn draw(&self, surface: &Surface) -> anyhow::Result<()> {
        let ctx = surface.context()?;

        // Draw background (fully opaque to avoid artifacts on reused surface)
        fill_rect(
            &ctx,
            self.rect,
            Color::new(0.15, 0.15, 0.15, 1.0),
        );

        // Draw tool buttons
        for (i, tool) in ToolType::all().iter().enumerate() {
            let btn_rect = self.button_rect(i);
            let is_selected = *tool == self.selected_tool;

            // Button background
            if is_selected {
                fill_rounded_rect(
                    &ctx,
                    btn_rect,
                    4.0,
                    Color::new(1.0, 1.0, 1.0, 0.2),
                );
            }

            // Button label (shortcut key)
            let label = tool.shortcut().to_ascii_uppercase().to_string();
            set_color(&ctx, Color::WHITE);
            ctx.select_font_face("monospace", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
            ctx.set_font_size(16.0);

            let extents = ctx.text_extents(&label)?;
            let text_x = btn_rect.x as f64 + (btn_rect.width as f64 - extents.width()) / 2.0;
            let text_y = btn_rect.y as f64 + (btn_rect.height as f64 + extents.height()) / 2.0;

            ctx.move_to(text_x, text_y);
            ctx.show_text(&label)?;
        }

        // Draw color preview
        let color_rect = Rect::new(
            (ToolType::all().len() as i32 + 1) * (Self::BUTTON_WIDTH as i32 + Self::PADDING as i32),
            Self::PADDING as i32 + 4,
            32,
            32,
        );
        fill_rounded_rect(&ctx, color_rect, 4.0, self.current_color);
        stroke_rounded_rect(
            &ctx,
            color_rect,
            4.0,
            Color::WHITE,
            1.0,
        );

        // Draw line width indicator
        let width_x = color_rect.x + color_rect.width as i32 + Self::PADDING as i32 * 2;
        set_color(&ctx, Color::WHITE);
        ctx.set_font_size(12.0);
        ctx.move_to(width_x as f64, (TOOLBAR_HEIGHT / 2 + 4) as f64);
        ctx.show_text(&format!("{}px", self.line_width as i32))?;

        // Draw hint text on right side
        let hint = "Ctrl+Enter: Save | Escape: Cancel";
        let hint_extents = ctx.text_extents(hint)?;
        let hint_x = self.rect.width as f64 - hint_extents.width() - 16.0;
        set_color(&ctx, Color::new(0.7, 0.7, 0.7, 1.0));
        ctx.set_font_size(12.0);
        ctx.move_to(hint_x, (TOOLBAR_HEIGHT / 2 + 4) as f64);
        ctx.show_text(hint)?;

        Ok(())
    }

    /// Handle click on toolbar, returns selected tool if a button was clicked.
    pub fn handle_click(&self, pos: Point) -> Option<ToolType> {
        if !self.rect.contains_point(pos) {
            return None;
        }

        for (i, tool) in ToolType::all().iter().enumerate() {
            let btn_rect = self.button_rect(i);
            if btn_rect.contains_point(pos) {
                return Some(*tool);
            }
        }

        None
    }
}
