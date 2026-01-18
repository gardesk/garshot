//! Color picker dialog for annotation tool selection.

mod hsv_tab;
mod palette_tab;
mod rgb_tab;
mod tabs;

pub mod eyedropper;

use gartk_core::{Color, InputEvent, Point, Rect};
use gartk_render::Surface;

pub use tabs::ColorPickerTab;

/// Color picker dialog dimensions.
pub const PICKER_WIDTH: u32 = 320;
pub const PICKER_HEIGHT: u32 = 380;

/// Tab bar height.
const TAB_HEIGHT: u32 = 36;

/// Button height.
const BUTTON_HEIGHT: u32 = 32;

/// Padding.
const PADDING: u32 = 12;

/// Result of color picker interaction.
#[derive(Debug, Clone)]
pub enum ColorPickerResult {
    /// User confirmed color selection.
    Confirm(Color),
    /// User cancelled.
    Cancel,
    /// Start eyedropper mode.
    StartEyedropper,
    /// Color changed, needs redraw.
    Changed,
    /// No change, no redraw needed.
    None,
}

/// What the user is currently dragging.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DragTarget {
    HueRing,
    SvSquare,
    RedSlider,
    GreenSlider,
    BlueSlider,
    AlphaSlider,
}

/// Color picker dialog.
pub struct ColorPicker {
    /// Dialog bounds (position and size).
    rect: Rect,
    /// Current tab.
    tab: ColorPickerTab,
    /// HSV values (source of truth): hue 0-360, saturation 0-1, value 0-1.
    hsv: (f64, f64, f64),
    /// Alpha value 0-1.
    alpha: f64,
    /// Currently dragging target.
    dragging: Option<DragTarget>,
    /// Recent colors (max 8).
    recent: Vec<Color>,
    /// Original color (for cancel).
    original: Color,
    /// OK button rect.
    ok_rect: Rect,
    /// Cancel button rect.
    cancel_rect: Rect,
    /// Eyedropper button rect.
    eyedropper_rect: Rect,
}

impl ColorPicker {
    /// Create a new color picker dialog.
    ///
    /// # Arguments
    /// * `initial_color` - The initial color to display
    /// * `screen_width` - Screen width for centering
    /// * `screen_height` - Screen height for centering
    pub fn new(initial_color: Color, screen_width: u32, screen_height: u32) -> Self {
        // Center the dialog on screen
        let x = (screen_width as i32 - PICKER_WIDTH as i32) / 2;
        let y = (screen_height as i32 - PICKER_HEIGHT as i32) / 2;

        let rect = Rect::new(x, y, PICKER_WIDTH, PICKER_HEIGHT);

        // Convert initial color to HSV
        let hsv = initial_color.to_hsv();

        // Calculate button positions (at bottom of dialog)
        let button_y = PICKER_HEIGHT as i32 - BUTTON_HEIGHT as i32 - PADDING as i32;
        let button_width = 80u32;
        let cancel_x = PADDING as i32;
        let ok_x = PICKER_WIDTH as i32 - button_width as i32 - PADDING as i32;

        Self {
            rect,
            tab: ColorPickerTab::default(),
            hsv,
            alpha: initial_color.a,
            dragging: None,
            recent: Vec::new(),
            original: initial_color,
            ok_rect: Rect::new(ok_x, button_y, button_width, BUTTON_HEIGHT),
            cancel_rect: Rect::new(cancel_x, button_y, button_width, BUTTON_HEIGHT),
            eyedropper_rect: Rect::new(
                (PICKER_WIDTH as i32 - 32) / 2,
                button_y - 40,
                32,
                32,
            ),
        }
    }

    /// Get the current color.
    pub fn current_color(&self) -> Color {
        Color::from_hsva(self.hsv.0, self.hsv.1, self.hsv.2, self.alpha)
    }

    /// Set color from RGB.
    pub fn set_color(&mut self, color: Color) {
        self.hsv = color.to_hsv();
        self.alpha = color.a;
    }

    /// Add a color to the recent colors list.
    pub fn add_recent(&mut self, color: Color) {
        // Remove if already present
        self.recent.retain(|c| c != &color);
        // Add to front
        self.recent.insert(0, color);
        // Keep only 8 recent colors
        self.recent.truncate(8);
    }

    /// Get the dialog bounds.
    pub fn rect(&self) -> Rect {
        self.rect
    }

    /// Handle input event.
    ///
    /// Returns a result if the interaction is complete.
    pub fn handle_event(&mut self, event: &InputEvent) -> ColorPickerResult {
        match event {
            InputEvent::MousePress(e) => {
                let local = self.to_local(e.position);

                // Check tab bar
                if let Some(new_tab) = tabs::handle_click(local) {
                    if new_tab != self.tab {
                        self.tab = new_tab;
                        return ColorPickerResult::Changed;
                    }
                    return ColorPickerResult::None;
                }

                // Check OK button
                if self.ok_rect.contains_point(local) {
                    let color = self.current_color();
                    self.add_recent(color);
                    return ColorPickerResult::Confirm(color);
                }

                // Check Cancel button
                if self.cancel_rect.contains_point(local) {
                    return ColorPickerResult::Cancel;
                }

                // Check eyedropper button
                if self.eyedropper_rect.contains_point(local) {
                    return ColorPickerResult::StartEyedropper;
                }

                // Handle tab-specific interactions
                match self.tab {
                    ColorPickerTab::Hsv => {
                        if let Some(target) = hsv_tab::handle_press(local, &mut self.hsv) {
                            self.dragging = Some(target);
                            return ColorPickerResult::Changed;
                        }
                    }
                    ColorPickerTab::Rgb => {
                        if let Some(target) = rgb_tab::handle_press(local, &mut self.hsv, &mut self.alpha) {
                            self.dragging = Some(target);
                            return ColorPickerResult::Changed;
                        }
                    }
                    ColorPickerTab::Palette => {
                        if let Some(color) = palette_tab::handle_click(local, &self.recent) {
                            self.set_color(color);
                            return ColorPickerResult::Changed;
                        }
                    }
                }
            }

            InputEvent::MouseRelease(_) => {
                if self.dragging.is_some() {
                    self.dragging = None;
                    // No redraw needed on release
                }
            }

            InputEvent::MouseMove(e) => {
                if let Some(target) = self.dragging {
                    let local = self.to_local(e.position);
                    match target {
                        DragTarget::HueRing | DragTarget::SvSquare => {
                            hsv_tab::handle_drag(local, target, &mut self.hsv);
                            return ColorPickerResult::Changed;
                        }
                        DragTarget::RedSlider
                        | DragTarget::GreenSlider
                        | DragTarget::BlueSlider
                        | DragTarget::AlphaSlider => {
                            rgb_tab::handle_drag(local, target, &mut self.hsv, &mut self.alpha);
                            return ColorPickerResult::Changed;
                        }
                    }
                }
            }

            InputEvent::Key(e) if e.pressed => {
                use gartk_core::Key;
                match e.key {
                    Key::Return => {
                        let color = self.current_color();
                        self.add_recent(color);
                        return ColorPickerResult::Confirm(color);
                    }
                    Key::Escape => {
                        return ColorPickerResult::Cancel;
                    }
                    _ => {}
                }
            }

            _ => {}
        }

        ColorPickerResult::None
    }

    /// Draw the color picker to a surface.
    pub fn draw(&self, surface: &Surface) -> anyhow::Result<()> {
        use gartk_render::{fill_rounded_rect, set_color, stroke_rounded_rect};

        let ctx = surface.context()?;

        // Draw background
        let bg = Color::new(0.15, 0.15, 0.15, 1.0);
        fill_rounded_rect(&ctx, Rect::new(0, 0, PICKER_WIDTH, PICKER_HEIGHT), 8.0, bg);

        // Draw tabs
        tabs::draw(&ctx, self.tab)?;

        // Draw tab content
        let content_rect = Rect::new(
            PADDING as i32,
            (TAB_HEIGHT + PADDING) as i32,
            PICKER_WIDTH - PADDING * 2,
            PICKER_HEIGHT - TAB_HEIGHT - BUTTON_HEIGHT - PADDING * 4 - 40,
        );

        match self.tab {
            ColorPickerTab::Hsv => hsv_tab::draw(&ctx, content_rect, self.hsv)?,
            ColorPickerTab::Rgb => rgb_tab::draw(&ctx, content_rect, self.hsv, self.alpha)?,
            ColorPickerTab::Palette => palette_tab::draw(&ctx, content_rect, &self.recent)?,
        }

        // Draw color preview
        let preview_rect = Rect::new(
            PADDING as i32,
            self.eyedropper_rect.y,
            80,
            32,
        );
        fill_rounded_rect(&ctx, preview_rect, 4.0, self.current_color());
        stroke_rounded_rect(&ctx, preview_rect, 4.0, Color::WHITE, 1.0);

        // Draw hex value
        let hex = self.current_color().to_hex();
        set_color(&ctx, Color::WHITE);
        ctx.select_font_face("monospace", cairo::FontSlant::Normal, cairo::FontWeight::Normal);
        ctx.set_font_size(12.0);
        ctx.move_to(
            (preview_rect.x + preview_rect.width as i32 + 8) as f64,
            (preview_rect.y + 20) as f64,
        );
        ctx.show_text(&hex)?;

        // Draw eyedropper button
        fill_rounded_rect(&ctx, self.eyedropper_rect, 4.0, Color::new(0.25, 0.25, 0.25, 1.0));
        stroke_rounded_rect(&ctx, self.eyedropper_rect, 4.0, Color::new(0.4, 0.4, 0.4, 1.0), 1.0);
        // Draw eyedropper icon (simple crosshair)
        set_color(&ctx, Color::WHITE);
        let cx = self.eyedropper_rect.x as f64 + self.eyedropper_rect.width as f64 / 2.0;
        let cy = self.eyedropper_rect.y as f64 + self.eyedropper_rect.height as f64 / 2.0;
        ctx.set_line_width(2.0);
        ctx.move_to(cx - 6.0, cy);
        ctx.line_to(cx + 6.0, cy);
        ctx.move_to(cx, cy - 6.0);
        ctx.line_to(cx, cy + 6.0);
        ctx.stroke()?;

        // Draw buttons
        self.draw_button(&ctx, self.cancel_rect, "Cancel", false)?;
        self.draw_button(&ctx, self.ok_rect, "OK", true)?;

        Ok(())
    }

    /// Draw a button.
    fn draw_button(
        &self,
        ctx: &cairo::Context,
        rect: Rect,
        label: &str,
        primary: bool,
    ) -> anyhow::Result<()> {
        use gartk_render::{fill_rounded_rect, set_color, stroke_rounded_rect};

        let bg = if primary {
            Color::new(0.2, 0.5, 0.9, 1.0)
        } else {
            Color::new(0.3, 0.3, 0.3, 1.0)
        };

        fill_rounded_rect(ctx, rect, 4.0, bg);
        stroke_rounded_rect(ctx, rect, 4.0, bg.lighten(0.2), 1.0);

        set_color(ctx, Color::WHITE);
        ctx.select_font_face("sans-serif", cairo::FontSlant::Normal, cairo::FontWeight::Bold);
        ctx.set_font_size(13.0);

        let extents = ctx.text_extents(label)?;
        let text_x = rect.x as f64 + (rect.width as f64 - extents.width()) / 2.0;
        let text_y = rect.y as f64 + (rect.height as f64 + extents.height()) / 2.0;

        ctx.move_to(text_x, text_y);
        ctx.show_text(label)?;

        Ok(())
    }

    /// Convert screen position to local position within dialog.
    fn to_local(&self, pos: Point) -> Point {
        Point::new(pos.x - self.rect.x, pos.y - self.rect.y)
    }
}
