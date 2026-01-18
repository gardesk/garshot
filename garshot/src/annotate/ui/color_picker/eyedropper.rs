//! Eyedropper for sampling colors from the screen.

use anyhow::{Context, Result};
use gartk_core::{Color, Point};
use gartk_x11::{Connection, CursorManager, CursorShape, Window, WindowConfig};
use x11rb::connection::Connection as X11Connection;
use x11rb::protocol::xproto::{self, ConnectionExt, ImageFormat};
use x11rb::wrapper::ConnectionExt as WrapperConnectionExt;

/// Eyedropper result.
#[derive(Debug)]
pub enum EyedropperResult {
    /// User selected a color.
    Color(Color),
    /// User cancelled.
    Cancel,
}

/// Eyedropper state for color sampling.
pub struct Eyedropper {
    /// X11 connection.
    conn: Connection,
    /// Overlay window (transparent, for capturing input).
    window: Window,
    /// Cursor manager.
    cursor_manager: CursorManager,
    /// Root window ID.
    root: u32,
    /// Current mouse position.
    mouse_pos: Point,
    /// Current sampled color.
    current_color: Color,
}

impl Eyedropper {
    /// Create a new eyedropper.
    pub fn new() -> Result<Self> {
        let conn = Connection::connect(None).context("Failed to connect to X11")?;

        let screen_width = conn.screen_width() as u32;
        let screen_height = conn.screen_height() as u32;
        let root = conn.inner().setup().roots[conn.screen_num()].root;

        // Create fullscreen transparent overlay to grab input
        let config = WindowConfig::new()
            .override_redirect(true)
            .size(screen_width, screen_height)
            .position(0, 0)
            .map_on_create(false);

        let window = Window::create(conn.clone(), config).context("Failed to create window")?;

        // Make window transparent (input only)
        // We'll set _NET_WM_WINDOW_OPACITY to 0
        let opacity_atom = conn.inner()
            .intern_atom(false, b"_NET_WM_WINDOW_OPACITY")?
            .reply()
            .context("Failed to intern opacity atom")?
            .atom;

        conn.inner().change_property32(
            xproto::PropMode::REPLACE,
            window.id(),
            opacity_atom,
            xproto::AtomEnum::CARDINAL,
            &[0], // Fully transparent
        )?;

        let cursor_manager = CursorManager::new(conn.clone())
            .context("Failed to create cursor manager")?;

        Ok(Self {
            conn,
            window,
            cursor_manager,
            root,
            mouse_pos: Point::new(0, 0),
            current_color: Color::BLACK,
        })
    }

    /// Run the eyedropper and return the selected color.
    pub fn run(mut self) -> Result<EyedropperResult> {
        // Map window and grab pointer
        self.window.map()?;
        self.conn.inner().flush()?;

        // Set crosshair cursor
        self.cursor_manager.set_window_cursor(self.window.id(), CursorShape::Crosshair)?;

        // Grab pointer
        self.window.grab_pointer()?;
        self.window.grab_keyboard_with_retry(10, 50)?;

        // Event loop
        loop {
            let event = self.conn.inner().wait_for_event()?;

            match event {
                x11rb::protocol::Event::ButtonPress(e) => {
                    if e.detail == 1 {
                        // Left click - sample color and confirm
                        let color = self.sample_color(e.root_x as i32, e.root_y as i32)?;
                        self.cleanup()?;
                        return Ok(EyedropperResult::Color(color));
                    } else if e.detail == 3 {
                        // Right click - cancel
                        self.cleanup()?;
                        return Ok(EyedropperResult::Cancel);
                    }
                }
                x11rb::protocol::Event::MotionNotify(e) => {
                    self.mouse_pos = Point::new(e.root_x as i32, e.root_y as i32);
                    // Sample color at current position (for preview if we add one)
                    self.current_color = self.sample_color(e.root_x as i32, e.root_y as i32)?;
                }
                x11rb::protocol::Event::KeyPress(e) => {
                    // Escape to cancel
                    if e.detail == 9 {
                        self.cleanup()?;
                        return Ok(EyedropperResult::Cancel);
                    }
                    // Enter to confirm current color
                    if e.detail == 36 || e.detail == 104 {
                        self.cleanup()?;
                        return Ok(EyedropperResult::Color(self.current_color));
                    }
                }
                _ => {}
            }
        }
    }

    /// Sample the color at the given screen coordinates.
    fn sample_color(&self, x: i32, y: i32) -> Result<Color> {
        // Get 1x1 pixel from root window
        let reply = self.conn.inner()
            .get_image(
                ImageFormat::Z_PIXMAP,
                self.root,
                x as i16,
                y as i16,
                1,
                1,
                !0, // All planes
            )?
            .reply()
            .context("Failed to get image")?;

        // Parse the pixel data (format depends on depth, assume 24/32 bit)
        let data = reply.data;
        if data.len() >= 3 {
            // Assume BGRA or BGR format (X11 native)
            let b = data[0];
            let g = data[1];
            let r = data[2];
            let a = if data.len() >= 4 { data[3] } else { 255 };

            Ok(Color::from_u8(r, g, b, a))
        } else {
            Ok(Color::BLACK)
        }
    }

    /// Cleanup resources.
    fn cleanup(&mut self) -> Result<()> {
        self.window.ungrab_keyboard()?;
        self.window.ungrab_pointer()?;
        Ok(())
    }
}
