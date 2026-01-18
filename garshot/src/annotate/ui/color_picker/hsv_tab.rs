//! HSV color wheel tab.

use std::f64::consts::PI;

use gartk_core::{Color, Point, Rect};
use gartk_render::set_color;

use super::DragTarget;

/// Outer radius of the hue ring.
const HUE_RING_OUTER: f64 = 100.0;

/// Inner radius of the hue ring (SV square fits inside).
const HUE_RING_INNER: f64 = 70.0;

/// SV square size (fits inside the hue ring).
const SV_SQUARE_SIZE: f64 = 90.0;

/// Handle mouse press in HSV tab.
pub fn handle_press(local_pos: Point, hsv: &mut (f64, f64, f64)) -> Option<DragTarget> {
    let center = content_center();
    let dx = local_pos.x as f64 - center.0;
    let dy = local_pos.y as f64 - center.1;
    let dist = (dx * dx + dy * dy).sqrt();

    // Check if in hue ring
    if dist >= HUE_RING_INNER && dist <= HUE_RING_OUTER {
        // Update hue based on angle
        let angle = dy.atan2(dx);
        let hue = ((angle * 180.0 / PI) + 90.0 + 360.0) % 360.0;
        hsv.0 = hue;
        return Some(DragTarget::HueRing);
    }

    // Check if in SV square
    let sv_rect = sv_square_rect();
    if sv_rect.contains_point(local_pos) {
        let s = ((local_pos.x - sv_rect.x) as f64 / sv_rect.width as f64).clamp(0.0, 1.0);
        let v = 1.0 - ((local_pos.y - sv_rect.y) as f64 / sv_rect.height as f64).clamp(0.0, 1.0);
        hsv.1 = s;
        hsv.2 = v;
        return Some(DragTarget::SvSquare);
    }

    None
}

/// Handle mouse drag in HSV tab.
pub fn handle_drag(local_pos: Point, target: DragTarget, hsv: &mut (f64, f64, f64)) {
    match target {
        DragTarget::HueRing => {
            let center = content_center();
            let dx = local_pos.x as f64 - center.0;
            let dy = local_pos.y as f64 - center.1;
            let angle = dy.atan2(dx);
            let hue = ((angle * 180.0 / PI) + 90.0 + 360.0) % 360.0;
            hsv.0 = hue;
        }
        DragTarget::SvSquare => {
            let sv_rect = sv_square_rect();
            let s = ((local_pos.x - sv_rect.x) as f64 / sv_rect.width as f64).clamp(0.0, 1.0);
            let v = 1.0 - ((local_pos.y - sv_rect.y) as f64 / sv_rect.height as f64).clamp(0.0, 1.0);
            hsv.1 = s;
            hsv.2 = v;
        }
        _ => {}
    }
}

/// Get the center point for the color wheel.
fn content_center() -> (f64, f64) {
    // Content area starts at y=48 (TAB_HEIGHT + PADDING) and we center in available space
    let cx = super::PICKER_WIDTH as f64 / 2.0;
    let cy = 48.0 + 110.0; // Approximate center in content area
    (cx, cy)
}

/// Get the SV square rect.
fn sv_square_rect() -> Rect {
    let (cx, cy) = content_center();
    let half = SV_SQUARE_SIZE / 2.0;
    Rect::new(
        (cx - half) as i32,
        (cy - half) as i32,
        SV_SQUARE_SIZE as u32,
        SV_SQUARE_SIZE as u32,
    )
}

/// Draw the HSV tab content.
pub fn draw(ctx: &cairo::Context, _content_rect: Rect, hsv: (f64, f64, f64)) -> anyhow::Result<()> {
    let (cx, cy) = content_center();

    // Draw hue ring
    draw_hue_ring(ctx, cx, cy)?;

    // Draw SV square
    draw_sv_square(ctx, hsv.0)?;

    // Draw hue indicator (small circle on ring)
    let hue_angle = (hsv.0 - 90.0) * PI / 180.0;
    let ring_mid = (HUE_RING_OUTER + HUE_RING_INNER) / 2.0;
    let hue_x = cx + ring_mid * hue_angle.cos();
    let hue_y = cy + ring_mid * hue_angle.sin();

    ctx.set_line_width(2.0);
    set_color(ctx, Color::WHITE);
    ctx.arc(hue_x, hue_y, 6.0, 0.0, 2.0 * PI);
    ctx.stroke()?;

    set_color(ctx, Color::BLACK);
    ctx.arc(hue_x, hue_y, 4.0, 0.0, 2.0 * PI);
    ctx.stroke()?;

    // Draw SV indicator (small circle in square)
    let sv_rect = sv_square_rect();
    let sv_x = sv_rect.x as f64 + hsv.1 * sv_rect.width as f64;
    let sv_y = sv_rect.y as f64 + (1.0 - hsv.2) * sv_rect.height as f64;

    ctx.set_line_width(2.0);
    set_color(ctx, Color::WHITE);
    ctx.arc(sv_x, sv_y, 6.0, 0.0, 2.0 * PI);
    ctx.stroke()?;

    set_color(ctx, Color::BLACK);
    ctx.arc(sv_x, sv_y, 4.0, 0.0, 2.0 * PI);
    ctx.stroke()?;

    Ok(())
}

/// Draw the hue ring.
fn draw_hue_ring(ctx: &cairo::Context, cx: f64, cy: f64) -> anyhow::Result<()> {
    // Draw the hue ring using filled wedges (pie slices)
    let segments = 360;
    let segment_angle = 2.0 * PI / segments as f64;

    for i in 0..segments {
        let angle_start = (i as f64 * segment_angle) - PI / 2.0;
        let angle_end = angle_start + segment_angle + 0.005; // Tiny overlap to prevent gaps

        let hue = (i as f64 / segments as f64) * 360.0;
        let color = Color::from_hsv(hue, 1.0, 1.0);
        set_color(ctx, color);

        // Draw a filled wedge between inner and outer radius
        ctx.new_path();
        ctx.arc(cx, cy, HUE_RING_OUTER, angle_start, angle_end);
        ctx.arc_negative(cx, cy, HUE_RING_INNER, angle_end, angle_start);
        ctx.close_path();
        ctx.fill()?;
    }

    Ok(())
}

/// Draw the saturation/value square.
fn draw_sv_square(ctx: &cairo::Context, hue: f64) -> anyhow::Result<()> {
    let sv_rect = sv_square_rect();
    let size = sv_rect.width as i32;

    // Draw pixel by pixel for accurate gradient
    // This is slow but accurate - could be optimized with gradients
    for y in 0..size {
        for x in 0..size {
            let s = x as f64 / size as f64;
            let v = 1.0 - (y as f64 / size as f64);
            let color = Color::from_hsv(hue, s, v);
            set_color(ctx, color);
            ctx.rectangle(
                (sv_rect.x + x) as f64,
                (sv_rect.y + y) as f64,
                1.0,
                1.0,
            );
            ctx.fill()?;
        }
    }

    // Draw border
    set_color(ctx, Color::new(0.3, 0.3, 0.3, 1.0));
    ctx.set_line_width(1.0);
    ctx.rectangle(
        sv_rect.x as f64,
        sv_rect.y as f64,
        sv_rect.width as f64,
        sv_rect.height as f64,
    );
    ctx.stroke()?;

    Ok(())
}
