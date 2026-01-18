//! Color palette tab.

use gartk_core::{Color, Point, Rect};
use gartk_render::{fill_rounded_rect, set_color, stroke_rounded_rect};

use super::{PADDING, PICKER_WIDTH};

/// Color swatch size.
const SWATCH_SIZE: u32 = 28;

/// Swatch spacing.
const SWATCH_SPACING: u32 = 4;

/// Number of columns in the palette grid.
const COLUMNS: usize = 9;

/// Preset colors (matching the 1-9 keyboard shortcuts).
const PRESET_COLORS: [Color; 9] = [
    Color::new(1.0, 0.4, 0.0, 1.0),   // 1: Orange (default)
    Color::new(1.0, 0.0, 0.0, 1.0),   // 2: Red
    Color::new(0.0, 1.0, 0.0, 1.0),   // 3: Green
    Color::new(0.0, 0.5, 1.0, 1.0),   // 4: Blue
    Color::new(1.0, 1.0, 0.0, 1.0),   // 5: Yellow
    Color::new(1.0, 0.0, 1.0, 1.0),   // 6: Magenta
    Color::new(0.0, 1.0, 1.0, 1.0),   // 7: Cyan
    Color::new(1.0, 1.0, 1.0, 1.0),   // 8: White
    Color::new(0.0, 0.0, 0.0, 1.0),   // 9: Black
];

/// Extended palette colors.
const EXTENDED_PALETTE: [Color; 36] = [
    // Row 1: Reds
    Color::new(1.0, 0.8, 0.8, 1.0),
    Color::new(1.0, 0.6, 0.6, 1.0),
    Color::new(1.0, 0.4, 0.4, 1.0),
    Color::new(1.0, 0.2, 0.2, 1.0),
    Color::new(0.8, 0.0, 0.0, 1.0),
    Color::new(0.6, 0.0, 0.0, 1.0),
    Color::new(0.4, 0.0, 0.0, 1.0),
    Color::new(0.3, 0.0, 0.0, 1.0),
    Color::new(0.2, 0.0, 0.0, 1.0),
    // Row 2: Oranges/Yellows
    Color::new(1.0, 0.9, 0.7, 1.0),
    Color::new(1.0, 0.8, 0.4, 1.0),
    Color::new(1.0, 0.6, 0.2, 1.0),
    Color::new(1.0, 0.5, 0.0, 1.0),
    Color::new(0.9, 0.7, 0.0, 1.0),
    Color::new(0.8, 0.8, 0.0, 1.0),
    Color::new(0.6, 0.6, 0.0, 1.0),
    Color::new(0.4, 0.4, 0.0, 1.0),
    Color::new(0.3, 0.3, 0.0, 1.0),
    // Row 3: Greens
    Color::new(0.8, 1.0, 0.8, 1.0),
    Color::new(0.6, 1.0, 0.6, 1.0),
    Color::new(0.4, 1.0, 0.4, 1.0),
    Color::new(0.0, 0.9, 0.0, 1.0),
    Color::new(0.0, 0.7, 0.0, 1.0),
    Color::new(0.0, 0.5, 0.0, 1.0),
    Color::new(0.0, 0.4, 0.0, 1.0),
    Color::new(0.0, 0.3, 0.0, 1.0),
    Color::new(0.0, 0.2, 0.0, 1.0),
    // Row 4: Blues/Purples
    Color::new(0.8, 0.8, 1.0, 1.0),
    Color::new(0.6, 0.6, 1.0, 1.0),
    Color::new(0.4, 0.4, 1.0, 1.0),
    Color::new(0.2, 0.2, 1.0, 1.0),
    Color::new(0.0, 0.0, 0.8, 1.0),
    Color::new(0.4, 0.0, 0.8, 1.0),
    Color::new(0.6, 0.0, 0.8, 1.0),
    Color::new(0.8, 0.0, 0.8, 1.0),
    Color::new(0.4, 0.0, 0.4, 1.0),
];

/// Get the content rect for the palette tab.
fn content_rect() -> Rect {
    Rect::new(
        PADDING as i32,
        60, // After tab bar
        PICKER_WIDTH - PADDING * 2,
        250,
    )
}

/// Get rect for a swatch at given row and column.
fn swatch_rect(row: usize, col: usize, content: Rect) -> Rect {
    Rect::new(
        content.x + (col as i32 * (SWATCH_SIZE as i32 + SWATCH_SPACING as i32)),
        content.y + (row as i32 * (SWATCH_SIZE as i32 + SWATCH_SPACING as i32)),
        SWATCH_SIZE,
        SWATCH_SIZE,
    )
}

/// Handle click in palette tab.
pub fn handle_click(local_pos: Point, recent: &[Color]) -> Option<Color> {
    let content = content_rect();

    // Check preset colors (row 0)
    for (col, color) in PRESET_COLORS.iter().enumerate() {
        let rect = swatch_rect(0, col, content);
        if rect.contains_point(local_pos) {
            return Some(*color);
        }
    }

    // Check extended palette (rows 1-4)
    for (i, color) in EXTENDED_PALETTE.iter().enumerate() {
        let row = 2 + (i / COLUMNS);
        let col = i % COLUMNS;
        let rect = swatch_rect(row, col, content);
        if rect.contains_point(local_pos) {
            return Some(*color);
        }
    }

    // Check recent colors (last row)
    let recent_row = 7;
    for (col, color) in recent.iter().enumerate().take(8) {
        let rect = swatch_rect(recent_row, col, content);
        if rect.contains_point(local_pos) {
            return Some(*color);
        }
    }

    None
}

/// Draw the palette tab content.
pub fn draw(ctx: &cairo::Context, _content_rect: Rect, recent: &[Color]) -> anyhow::Result<()> {
    let content = content_rect();

    // Draw section label for presets
    set_color(ctx, Color::new(0.7, 0.7, 0.7, 1.0));
    ctx.select_font_face("sans-serif", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
    ctx.set_font_size(11.0);
    ctx.move_to(content.x as f64, (content.y - 4) as f64);
    ctx.show_text("Presets (1-9)")?;

    // Draw preset colors
    for (col, color) in PRESET_COLORS.iter().enumerate() {
        let rect = swatch_rect(0, col, content);
        draw_swatch(ctx, rect, *color)?;

        // Draw key number
        set_color(ctx, Color::WHITE);
        ctx.set_font_size(10.0);
        ctx.move_to((rect.x + 2) as f64, (rect.y + rect.height as i32 - 3) as f64);
        ctx.show_text(&format!("{}", col + 1))?;
    }

    // Draw section label for extended palette
    let extended_y = content.y + (SWATCH_SIZE as i32 + SWATCH_SPACING as i32) + 8;
    set_color(ctx, Color::new(0.7, 0.7, 0.7, 1.0));
    ctx.set_font_size(11.0);
    ctx.move_to(content.x as f64, extended_y as f64);
    ctx.show_text("Extended")?;

    // Draw extended palette
    for (i, color) in EXTENDED_PALETTE.iter().enumerate() {
        let row = 2 + (i / COLUMNS);
        let col = i % COLUMNS;
        let rect = swatch_rect(row, col, content);
        draw_swatch(ctx, rect, *color)?;
    }

    // Draw section label for recent colors
    let recent_y = swatch_rect(6, 0, content).y + SWATCH_SIZE as i32 + 8;
    set_color(ctx, Color::new(0.7, 0.7, 0.7, 1.0));
    ctx.set_font_size(11.0);
    ctx.move_to(content.x as f64, recent_y as f64);
    ctx.show_text("Recent")?;

    // Draw recent colors
    let recent_row = 7;
    for col in 0..8 {
        let rect = swatch_rect(recent_row, col, content);
        if col < recent.len() {
            draw_swatch(ctx, rect, recent[col])?;
        } else {
            // Draw empty swatch placeholder
            fill_rounded_rect(ctx, rect, 4.0, Color::new(0.2, 0.2, 0.2, 1.0));
            stroke_rounded_rect(ctx, rect, 4.0, Color::new(0.3, 0.3, 0.3, 1.0), 1.0);
        }
    }

    Ok(())
}

/// Draw a color swatch.
fn draw_swatch(ctx: &cairo::Context, rect: Rect, color: Color) -> anyhow::Result<()> {
    // Draw checkerboard for alpha
    if color.a < 1.0 {
        let check_size = 4.0;
        for y in 0..((rect.height as f64 / check_size) as i32) {
            for x in 0..((rect.width as f64 / check_size) as i32) {
                let is_light = (x + y) % 2 == 0;
                set_color(
                    ctx,
                    if is_light {
                        Color::new(0.6, 0.6, 0.6, 1.0)
                    } else {
                        Color::new(0.4, 0.4, 0.4, 1.0)
                    },
                );
                ctx.rectangle(
                    rect.x as f64 + x as f64 * check_size,
                    rect.y as f64 + y as f64 * check_size,
                    check_size,
                    check_size,
                );
                ctx.fill()?;
            }
        }
    }

    fill_rounded_rect(ctx, rect, 4.0, color);
    stroke_rounded_rect(ctx, rect, 4.0, Color::new(0.4, 0.4, 0.4, 1.0), 1.0);

    Ok(())
}
