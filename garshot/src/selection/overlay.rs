//! Fullscreen overlay window for interactive selection.

use x11rb::connection::Connection as X11Connection;
use x11rb::protocol::xproto::*;
use x11rb::COPY_DEPTH_FROM_PARENT;

use crate::capture::{capture_full_screen, Region};
use crate::error::Result;
use crate::selection::blur::blur_rgba;
use crate::selection::events::{SelectionHandler, SelectionResult, SelectionState};
use crate::x11::{Connection, ShmCapture};

/// Configuration for the selection overlay.
#[derive(Debug, Clone)]
pub struct SelectionConfig {
    /// Blur radius in pixels.
    pub blur_radius: usize,
    /// Selection line color (RGB).
    pub line_color: u32,
    /// Selection line width.
    pub line_width: u32,
}

impl Default for SelectionConfig {
    fn default() -> Self {
        Self {
            blur_radius: 15,
            line_color: 0xFF6600, // Orange
            line_width: 2,
        }
    }
}

/// Perform interactive region selection.
///
/// Creates a fullscreen overlay with blurred background, allowing the user
/// to select a region by clicking and dragging.
///
/// Returns the selected region, or None if cancelled.
pub fn interactive_selection(
    conn: &Connection,
    shm: &ShmCapture,
    config: &SelectionConfig,
) -> Result<Option<Region>> {
    tracing::debug!("Starting interactive selection");

    // 1. Capture current screen
    let capture = capture_full_screen(conn, shm)?;
    let width = capture.width as usize;
    let height = capture.height as usize;

    // Keep original for the clear region
    let original_data = capture.data.clone();

    // 2. Apply blur for overlay background
    let mut blurred_data = capture.data;
    tracing::debug!("Applying blur with radius {}", config.blur_radius);
    blur_rgba(&mut blurred_data, width, height, config.blur_radius);

    // 3. Create overlay window
    let overlay = Overlay::new(conn, &blurred_data, width as u16, height as u16)?;
    overlay.show(conn)?;

    // 4. Event loop
    let result = run_event_loop(conn, &overlay, &original_data, &blurred_data, config)?;

    // 5. Cleanup
    overlay.hide(conn)?;

    match result {
        SelectionResult::Selected(region) => {
            tracing::debug!("Selection complete: {:?}", region);
            Ok(Some(region))
        }
        SelectionResult::Cancelled => {
            tracing::debug!("Selection cancelled");
            Ok(None)
        }
    }
}

/// Overlay window state.
struct Overlay {
    window: Window,
    gc: Gcontext,
    pixmap: Pixmap,
    width: u16,
    height: u16,
}

impl Overlay {
    fn new(conn: &Connection, blurred_data: &[u8], width: u16, height: u16) -> Result<Self> {
        let window = conn.conn.generate_id()?;
        let gc = conn.conn.generate_id()?;
        let pixmap = conn.conn.generate_id()?;

        // Create pixmap for blurred background
        conn.conn.create_pixmap(conn.depth, pixmap, conn.root, width, height)?;

        // Put blurred image data into pixmap
        // Convert RGBA to native format (BGRA for X11)
        let bgra_data = rgba_to_bgra(blurred_data);
        conn.conn.put_image(
            ImageFormat::Z_PIXMAP,
            pixmap,
            gc,
            width,
            height,
            0,
            0,
            0,
            conn.depth,
            &bgra_data,
        )?;

        // Create fullscreen overlay window
        conn.conn.create_window(
            COPY_DEPTH_FROM_PARENT,
            window,
            conn.root,
            0,
            0,
            width,
            height,
            0,
            WindowClass::INPUT_OUTPUT,
            0,
            &CreateWindowAux::new()
                .override_redirect(1)
                .background_pixmap(pixmap)
                .event_mask(
                    EventMask::EXPOSURE
                        | EventMask::BUTTON_PRESS
                        | EventMask::BUTTON_RELEASE
                        | EventMask::POINTER_MOTION
                        | EventMask::KEY_PRESS,
                ),
        )?;

        // Create GC for drawing
        conn.conn.create_gc(gc, window, &CreateGCAux::new())?;

        conn.conn.flush()?;

        Ok(Self {
            window,
            gc,
            pixmap,
            width,
            height,
        })
    }

    fn show(&self, conn: &Connection) -> Result<()> {
        conn.conn.map_window(self.window)?;

        // Grab pointer
        let grab_result = conn
            .conn
            .grab_pointer(
                true,
                self.window,
                EventMask::BUTTON_PRESS
                    | EventMask::BUTTON_RELEASE
                    | EventMask::POINTER_MOTION,
                GrabMode::ASYNC,
                GrabMode::ASYNC,
                self.window,
                x11rb::NONE,
                x11rb::CURRENT_TIME,
            )?
            .reply()?;

        if grab_result.status != GrabStatus::SUCCESS {
            tracing::warn!("Failed to grab pointer: {:?}", grab_result.status);
        }

        // Grab keyboard
        let grab_result = conn
            .conn
            .grab_keyboard(
                true,
                self.window,
                x11rb::CURRENT_TIME,
                GrabMode::ASYNC,
                GrabMode::ASYNC,
            )?
            .reply()?;

        if grab_result.status != GrabStatus::SUCCESS {
            tracing::warn!("Failed to grab keyboard: {:?}", grab_result.status);
        }

        conn.conn.flush()?;
        Ok(())
    }

    fn hide(&self, conn: &Connection) -> Result<()> {
        conn.conn.ungrab_pointer(x11rb::CURRENT_TIME)?;
        conn.conn.ungrab_keyboard(x11rb::CURRENT_TIME)?;
        conn.conn.unmap_window(self.window)?;
        conn.conn.free_pixmap(self.pixmap)?;
        conn.conn.free_gc(self.gc)?;
        conn.conn.destroy_window(self.window)?;
        conn.conn.flush()?;
        Ok(())
    }

    fn draw(&self, conn: &Connection, region: Option<&Region>, original: &[u8], blurred: &[u8], config: &SelectionConfig) -> Result<()> {
        // Redraw blurred background
        let bgra_blurred = rgba_to_bgra(blurred);
        conn.conn.put_image(
            ImageFormat::Z_PIXMAP,
            self.window,
            self.gc,
            self.width,
            self.height,
            0,
            0,
            0,
            conn.depth,
            &bgra_blurred,
        )?;

        if let Some(region) = region {
            if region.width > 0 && region.height > 0 {
                // Draw clear (unblurred) region
                let clear_data = extract_region(original, self.width as usize, region);
                let bgra_clear = rgba_to_bgra(&clear_data);

                conn.conn.put_image(
                    ImageFormat::Z_PIXMAP,
                    self.window,
                    self.gc,
                    region.width,
                    region.height,
                    region.x,
                    region.y,
                    0,
                    conn.depth,
                    &bgra_clear,
                )?;

                // Draw selection border
                conn.conn.change_gc(
                    self.gc,
                    &ChangeGCAux::new()
                        .foreground(config.line_color)
                        .line_width(config.line_width),
                )?;

                conn.conn.poly_rectangle(
                    self.window,
                    self.gc,
                    &[Rectangle {
                        x: region.x,
                        y: region.y,
                        width: region.width,
                        height: region.height,
                    }],
                )?;

                // Draw dimensions text
                let text = format!("{}x{}", region.width, region.height);
                let text_x = region.x + 5;
                let text_y = region.y + region.height as i16 - 5;

                // Draw text background for readability
                conn.conn.change_gc(
                    self.gc,
                    &ChangeGCAux::new().foreground(0x000000),
                )?;

                conn.conn.image_text8(self.window, self.gc, text_x + 1, text_y + 1, text.as_bytes())?;

                conn.conn.change_gc(
                    self.gc,
                    &ChangeGCAux::new().foreground(0xFFFFFF),
                )?;

                conn.conn.image_text8(self.window, self.gc, text_x, text_y, text.as_bytes())?;
            }
        }

        conn.conn.flush()?;
        Ok(())
    }
}

/// Run the selection event loop.
fn run_event_loop(
    conn: &Connection,
    overlay: &Overlay,
    original: &[u8],
    blurred: &[u8],
    config: &SelectionConfig,
) -> Result<SelectionResult> {
    let mut handler = SelectionHandler::new();

    // Initial draw
    overlay.draw(conn, None, original, blurred, config)?;

    loop {
        let event = conn.conn.wait_for_event()?;

        let result = match event {
            x11rb::protocol::Event::ButtonPress(e) => handler.handle_button_press(&e),
            x11rb::protocol::Event::ButtonRelease(e) => handler.handle_button_release(&e),
            x11rb::protocol::Event::MotionNotify(e) => {
                handler.handle_motion(&e);
                None
            }
            x11rb::protocol::Event::KeyPress(e) => handler.handle_key_press(&e),
            x11rb::protocol::Event::Expose(_) => {
                overlay.draw(conn, handler.state().current_region().as_ref(), original, blurred, config)?;
                None
            }
            _ => None,
        };

        // Redraw on motion
        if matches!(handler.state(), SelectionState::Dragging { .. }) {
            overlay.draw(conn, handler.state().current_region().as_ref(), original, blurred, config)?;
        }

        if let Some(result) = result {
            return Ok(result);
        }
    }
}

/// Convert RGBA to BGRA (X11 native format).
fn rgba_to_bgra(data: &[u8]) -> Vec<u8> {
    let mut bgra = Vec::with_capacity(data.len());
    for chunk in data.chunks_exact(4) {
        bgra.push(chunk[2]); // B
        bgra.push(chunk[1]); // G
        bgra.push(chunk[0]); // R
        bgra.push(chunk[3]); // A
    }
    bgra
}

/// Extract a region from image data.
fn extract_region(data: &[u8], image_width: usize, region: &Region) -> Vec<u8> {
    let mut result = Vec::with_capacity(region.width as usize * region.height as usize * 4);

    for y in 0..region.height as usize {
        let src_y = region.y as usize + y;
        let src_offset = (src_y * image_width + region.x as usize) * 4;
        let src_end = src_offset + region.width as usize * 4;

        if src_end <= data.len() {
            result.extend_from_slice(&data[src_offset..src_end]);
        }
    }

    result
}
