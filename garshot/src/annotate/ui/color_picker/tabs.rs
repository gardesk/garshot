//! Tab bar for color picker dialog.

use gartk_core::{Color, Point, Rect};
use gartk_render::{fill_rounded_rect, set_color};

use super::PADDING;

/// Tab height.
const TAB_HEIGHT: u32 = 28;

/// Tab width.
const TAB_WIDTH: u32 = 70;

/// Color picker tab selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorPickerTab {
    /// HSV color wheel tab.
    #[default]
    Hsv,
    /// RGB sliders tab.
    Rgb,
    /// Color palette tab.
    Palette,
}

impl ColorPickerTab {
    /// Get all tabs in order.
    pub const fn all() -> [Self; 3] {
        [Self::Hsv, Self::Rgb, Self::Palette]
    }

    /// Get tab label.
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Hsv => "HSV",
            Self::Rgb => "RGB",
            Self::Palette => "Palette",
        }
    }
}

/// Get the rect for a tab at given index.
fn tab_rect(index: usize) -> Rect {
    let x = PADDING as i32 + (index as i32 * (TAB_WIDTH as i32 + 4));
    Rect::new(x, PADDING as i32, TAB_WIDTH, TAB_HEIGHT)
}

/// Handle click on tab bar, returns new tab if one was clicked.
pub fn handle_click(local_pos: Point) -> Option<ColorPickerTab> {
    for (i, tab) in ColorPickerTab::all().iter().enumerate() {
        if tab_rect(i).contains_point(local_pos) {
            return Some(*tab);
        }
    }
    None
}

/// Draw the tab bar.
pub fn draw(ctx: &cairo::Context, selected: ColorPickerTab) -> anyhow::Result<()> {
    for (i, tab) in ColorPickerTab::all().iter().enumerate() {
        let rect = tab_rect(i);
        let is_selected = *tab == selected;

        // Tab background
        let bg = if is_selected {
            Color::new(0.3, 0.3, 0.3, 1.0)
        } else {
            Color::new(0.2, 0.2, 0.2, 1.0)
        };
        fill_rounded_rect(ctx, rect, 4.0, bg);

        // Tab label
        set_color(
            ctx,
            if is_selected {
                Color::WHITE
            } else {
                Color::new(0.7, 0.7, 0.7, 1.0)
            },
        );
        ctx.select_font_face("sans-serif", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
        ctx.set_font_size(12.0);

        let label = tab.label();
        let extents = ctx.text_extents(label)?;
        let text_x = rect.x as f64 + (rect.width as f64 - extents.width()) / 2.0;
        let text_y = rect.y as f64 + (rect.height as f64 + extents.height()) / 2.0;

        ctx.move_to(text_x, text_y);
        ctx.show_text(label)?;
    }

    Ok(())
}
