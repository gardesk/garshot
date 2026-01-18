//! RGB sliders tab.

use gartk_core::{Color, Point, Rect};
use gartk_render::set_color;

use super::{DragTarget, PADDING, PICKER_WIDTH};

/// Slider height.
const SLIDER_HEIGHT: u32 = 24;

/// Slider spacing.
const SLIDER_SPACING: u32 = 40;

/// Label width.
const LABEL_WIDTH: u32 = 30;

/// Get the rect for a slider at given index.
fn slider_rect(index: usize, content_rect: Rect) -> Rect {
    let y = content_rect.y + (index as i32 * SLIDER_SPACING as i32);
    Rect::new(
        content_rect.x + LABEL_WIDTH as i32,
        y,
        content_rect.width - LABEL_WIDTH - 50, // Leave room for value text
        SLIDER_HEIGHT,
    )
}

/// Get the content rect for the RGB tab.
fn content_rect() -> Rect {
    Rect::new(
        PADDING as i32,
        60, // After tab bar
        PICKER_WIDTH - PADDING * 2,
        200,
    )
}

/// Handle mouse press in RGB tab.
pub fn handle_press(
    local_pos: Point,
    hsv: &mut (f64, f64, f64),
    alpha: &mut f64,
) -> Option<DragTarget> {
    let content = content_rect();

    // Check each slider
    let sliders = [
        DragTarget::RedSlider,
        DragTarget::GreenSlider,
        DragTarget::BlueSlider,
        DragTarget::AlphaSlider,
    ];

    for (i, target) in sliders.iter().enumerate() {
        let rect = slider_rect(i, content);
        if rect.contains_point(local_pos) {
            update_from_slider(local_pos, *target, rect, hsv, alpha);
            return Some(*target);
        }
    }

    None
}

/// Handle mouse drag in RGB tab.
pub fn handle_drag(
    local_pos: Point,
    target: DragTarget,
    hsv: &mut (f64, f64, f64),
    alpha: &mut f64,
) {
    let content = content_rect();
    let index = match target {
        DragTarget::RedSlider => 0,
        DragTarget::GreenSlider => 1,
        DragTarget::BlueSlider => 2,
        DragTarget::AlphaSlider => 3,
        _ => return,
    };
    let rect = slider_rect(index, content);
    update_from_slider(local_pos, target, rect, hsv, alpha);
}

/// Update color from slider position.
fn update_from_slider(
    local_pos: Point,
    target: DragTarget,
    rect: Rect,
    hsv: &mut (f64, f64, f64),
    alpha: &mut f64,
) {
    let value = ((local_pos.x - rect.x) as f64 / rect.width as f64).clamp(0.0, 1.0);

    // Convert current HSV to RGB
    let current = Color::from_hsva(hsv.0, hsv.1, hsv.2, *alpha);
    let mut r = current.r;
    let mut g = current.g;
    let mut b = current.b;

    match target {
        DragTarget::RedSlider => r = value,
        DragTarget::GreenSlider => g = value,
        DragTarget::BlueSlider => b = value,
        DragTarget::AlphaSlider => {
            *alpha = value;
            return;
        }
        _ => return,
    }

    // Convert back to HSV
    let new_color = Color::new(r, g, b, *alpha);
    *hsv = new_color.to_hsv();
}

/// Draw the RGB tab content.
pub fn draw(
    ctx: &cairo::Context,
    _content_rect: Rect,
    hsv: (f64, f64, f64),
    alpha: f64,
) -> anyhow::Result<()> {
    let content = content_rect();
    let current = Color::from_hsva(hsv.0, hsv.1, hsv.2, alpha);

    let labels = ["R", "G", "B", "A"];
    let values = [current.r, current.g, current.b, alpha];
    let colors = [
        (Color::RED, Color::new(0.2, 0.0, 0.0, 1.0)),
        (Color::GREEN, Color::new(0.0, 0.2, 0.0, 1.0)),
        (Color::BLUE, Color::new(0.0, 0.0, 0.2, 1.0)),
        (Color::WHITE, Color::new(0.2, 0.2, 0.2, 1.0)),
    ];

    for (i, (label, value)) in labels.iter().zip(values.iter()).enumerate() {
        let rect = slider_rect(i, content);
        let (high_color, low_color) = colors[i];

        // Draw label
        set_color(ctx, Color::WHITE);
        ctx.select_font_face("monospace", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
        ctx.set_font_size(14.0);
        ctx.move_to(
            (content.x + 4) as f64,
            (rect.y + rect.height as i32 - 6) as f64,
        );
        ctx.show_text(label)?;

        // Draw slider track with gradient
        draw_slider_gradient(ctx, rect, low_color, high_color)?;

        // Draw slider handle
        let handle_x = rect.x as f64 + value * rect.width as f64;
        draw_slider_handle(ctx, handle_x, rect.y as f64 + rect.height as f64 / 2.0)?;

        // Draw value text
        let value_text = format!("{}", (value * 255.0).round() as u8);
        set_color(ctx, Color::new(0.8, 0.8, 0.8, 1.0));
        ctx.set_font_size(12.0);
        ctx.move_to(
            (rect.x + rect.width as i32 + 8) as f64,
            (rect.y + rect.height as i32 - 6) as f64,
        );
        ctx.show_text(&value_text)?;
    }

    Ok(())
}

/// Draw a slider gradient background.
fn draw_slider_gradient(
    ctx: &cairo::Context,
    rect: Rect,
    from: Color,
    to: Color,
) -> anyhow::Result<()> {
    // Create horizontal linear gradient
    let gradient = cairo::LinearGradient::new(
        rect.x as f64,
        rect.y as f64,
        (rect.x + rect.width as i32) as f64,
        rect.y as f64,
    );
    gradient.add_color_stop_rgba(0.0, from.r, from.g, from.b, from.a);
    gradient.add_color_stop_rgba(1.0, to.r, to.g, to.b, to.a);

    // Draw rounded rect with gradient
    let radius = 4.0;
    ctx.new_sub_path();
    ctx.arc(
        rect.x as f64 + rect.width as f64 - radius,
        rect.y as f64 + radius,
        radius,
        -0.5 * std::f64::consts::PI,
        0.0,
    );
    ctx.arc(
        rect.x as f64 + rect.width as f64 - radius,
        rect.y as f64 + rect.height as f64 - radius,
        radius,
        0.0,
        0.5 * std::f64::consts::PI,
    );
    ctx.arc(
        rect.x as f64 + radius,
        rect.y as f64 + rect.height as f64 - radius,
        radius,
        0.5 * std::f64::consts::PI,
        std::f64::consts::PI,
    );
    ctx.arc(
        rect.x as f64 + radius,
        rect.y as f64 + radius,
        radius,
        std::f64::consts::PI,
        1.5 * std::f64::consts::PI,
    );
    ctx.close_path();

    ctx.set_source(&gradient)?;
    ctx.fill()?;

    // Draw border
    set_color(ctx, Color::new(0.4, 0.4, 0.4, 1.0));
    ctx.set_line_width(1.0);
    ctx.new_sub_path();
    ctx.arc(
        rect.x as f64 + rect.width as f64 - radius,
        rect.y as f64 + radius,
        radius,
        -0.5 * std::f64::consts::PI,
        0.0,
    );
    ctx.arc(
        rect.x as f64 + rect.width as f64 - radius,
        rect.y as f64 + rect.height as f64 - radius,
        radius,
        0.0,
        0.5 * std::f64::consts::PI,
    );
    ctx.arc(
        rect.x as f64 + radius,
        rect.y as f64 + rect.height as f64 - radius,
        radius,
        0.5 * std::f64::consts::PI,
        std::f64::consts::PI,
    );
    ctx.arc(
        rect.x as f64 + radius,
        rect.y as f64 + radius,
        radius,
        std::f64::consts::PI,
        1.5 * std::f64::consts::PI,
    );
    ctx.close_path();
    ctx.stroke()?;

    Ok(())
}

/// Draw a slider handle.
fn draw_slider_handle(ctx: &cairo::Context, x: f64, y: f64) -> anyhow::Result<()> {
    let radius = 8.0;

    // White fill
    set_color(ctx, Color::WHITE);
    ctx.arc(x, y, radius, 0.0, 2.0 * std::f64::consts::PI);
    ctx.fill()?;

    // Dark border
    set_color(ctx, Color::new(0.3, 0.3, 0.3, 1.0));
    ctx.set_line_width(2.0);
    ctx.arc(x, y, radius, 0.0, 2.0 * std::f64::consts::PI);
    ctx.stroke()?;

    Ok(())
}
