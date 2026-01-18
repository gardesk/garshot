//! Selection overlay for interactive region capture.
//!
//! Creates a fullscreen overlay with a blurred background. The user can drag
//! to select a region, which shows the unblurred original image.

use x11rb::connection::Connection as X11Connection;
use x11rb::protocol::xproto::{
    AtomEnum, ChangeGCAux, ConnectionExt, CreateGCAux, CreateWindowAux, Cursor,
    EventMask, Font, Gcontext, GrabMode, GrabStatus, ImageFormat, Pixmap, PropMode, Rectangle,
    Window, WindowClass,
};
use x11rb::wrapper::ConnectionExt as WrapperConnectionExt;

use super::blur::blur_rgba;
use super::events::{SelectionHandler, SelectionResult};
use crate::capture::Region;
use crate::error::Result;
use crate::x11::{Connection, ShmCapture};

/// X11 constant for copying depth from parent.
const COPY_DEPTH_FROM_PARENT: u8 = 0;

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

/// Run interactive selection and return the selected region.
pub fn interactive_selection(
    conn: &Connection,
    shm: &ShmCapture,
    config: &SelectionConfig,
) -> Result<Option<Region>> {
    tracing::debug!("Starting interactive selection");

    let width = conn.width as usize;
    let height = conn.height as usize;

    // 1. Capture current screen
    let capture_data = shm.capture(conn, 0, 0, conn.width, conn.height)?;
    // Convert BGRA to RGBA and copy to owned buffer
    // Force opaque alpha when capturing from compositor overlay (which has alpha=0)
    let original_data = crate::x11::shm::bgra_to_rgba_with_alpha(capture_data, conn.compositor_active);

    // 2. Apply blur for overlay background
    let mut blurred_data = original_data.clone();
    tracing::debug!("Applying blur with radius {}", config.blur_radius);
    blur_rgba(&mut blurred_data, width, height, config.blur_radius);

    // 3. Create overlay with server-side pixmaps (fast!)
    let overlay = Overlay::new(conn, &original_data, &blurred_data, conn.width, conn.height)?;
    overlay.show(conn)?;

    // 4. Event loop
    let result = run_event_loop(conn, &overlay, config)?;

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

/// Convert RGBA to BGRA (X11 native format).
fn rgba_to_bgra(data: &[u8]) -> Vec<u8> {
    let mut bgra = data.to_vec();
    for pixel in bgra.chunks_exact_mut(4) {
        pixel.swap(0, 2); // Swap R and B
    }
    bgra
}

/// Put image data in chunks to avoid exceeding X11 max request size.
fn put_image_chunked(
    conn: &Connection,
    drawable: u32,
    gc: u32,
    width: u16,
    height: u16,
    dst_x: i16,
    dst_y: i16,
    data: &[u8],
) -> Result<()> {
    let bytes_per_row = width as usize * 4;
    let max_chunk_bytes = 65536;
    let rows_per_chunk = (max_chunk_bytes / bytes_per_row).max(1);

    let mut y = 0u16;
    while (y as usize) < height as usize {
        let chunk_height = ((height as usize - y as usize).min(rows_per_chunk)) as u16;
        let start = y as usize * bytes_per_row;
        let end = start + chunk_height as usize * bytes_per_row;
        let chunk_data = &data[start..end];

        conn.conn.put_image(
            ImageFormat::Z_PIXMAP,
            drawable,
            gc,
            width,
            chunk_height,
            dst_x,
            dst_y + y as i16,
            0,
            conn.depth,
            chunk_data,
        )?;

        y += chunk_height;
    }

    Ok(())
}

/// Overlay window state with server-side pixmaps for fast drawing.
struct Overlay {
    window: Window,
    gc: Gcontext,
    original_pixmap: Pixmap,  // Unblurred screen
    blurred_pixmap: Pixmap,   // Blurred screen
    cursor: Cursor,
    width: u16,
    height: u16,
}

impl Overlay {
    fn new(
        conn: &Connection,
        original_data: &[u8],
        blurred_data: &[u8],
        width: u16,
        height: u16,
    ) -> Result<Self> {
        let window = conn.conn.generate_id()?;
        let gc = conn.conn.generate_id()?;
        let original_pixmap = conn.conn.generate_id()?;
        let blurred_pixmap = conn.conn.generate_id()?;

        // Create pixmaps
        conn.conn.create_pixmap(conn.depth, original_pixmap, conn.root, width, height)?;
        conn.conn.create_pixmap(conn.depth, blurred_pixmap, conn.root, width, height)?;

        // Create GC
        conn.conn.create_gc(gc, original_pixmap, &CreateGCAux::new())?;

        // Upload original image to pixmap (one-time cost)
        let bgra_original = rgba_to_bgra(original_data);
        put_image_chunked(conn, original_pixmap, gc, width, height, 0, 0, &bgra_original)?;

        // Upload blurred image to pixmap (one-time cost)
        let bgra_blurred = rgba_to_bgra(blurred_data);
        put_image_chunked(conn, blurred_pixmap, gc, width, height, 0, 0, &bgra_blurred)?;

        // Create crosshair cursor
        let cursor = create_crosshair_cursor(conn)?;

        // Create fullscreen overlay window with blurred background
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
                .background_pixmap(blurred_pixmap)
                .event_mask(
                    EventMask::EXPOSURE
                        | EventMask::BUTTON_PRESS
                        | EventMask::BUTTON_RELEASE
                        | EventMask::POINTER_MOTION
                        | EventMask::KEY_PRESS,
                )
                .cursor(cursor),
        )?;

        // Tell compositor to bypass this window (no blur/effects from picom)
        let bypass_atom = conn.conn.intern_atom(false, b"_NET_WM_BYPASS_COMPOSITOR")?.reply()?.atom;
        WrapperConnectionExt::change_property32(
            &conn.conn,
            PropMode::REPLACE,
            window,
            bypass_atom,
            AtomEnum::CARDINAL,
            &[1], // 1 = bypass compositor
        )?;

        // Set WM_CLASS for identification
        let wm_class = b"garshot\0garshot\0";
        WrapperConnectionExt::change_property8(
            &conn.conn,
            PropMode::REPLACE,
            window,
            AtomEnum::WM_CLASS,
            AtomEnum::STRING,
            wm_class,
        )?;

        conn.conn.flush()?;

        Ok(Self {
            window,
            gc,
            original_pixmap,
            blurred_pixmap,
            cursor,
            width,
            height,
        })
    }

    fn show(&self, conn: &Connection) -> Result<()> {
        conn.conn.map_window(self.window)?;

        // Grab pointer with crosshair cursor
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
                self.cursor,
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
        conn.conn.free_pixmap(self.original_pixmap)?;
        conn.conn.free_pixmap(self.blurred_pixmap)?;
        conn.conn.free_gc(self.gc)?;
        conn.conn.free_cursor(self.cursor)?;
        conn.conn.destroy_window(self.window)?;
        // Sync to ensure window is fully destroyed before continuing
        // This prevents the selection overlay from lingering when annotation opens
        conn.conn.sync()?;
        Ok(())
    }

    /// Fast redraw using server-side CopyArea (no CPU work, no data transfer).
    fn draw(&self, conn: &Connection, region: Option<&Region>, config: &SelectionConfig) -> Result<()> {
        // Copy entire blurred pixmap to window (single X11 request!)
        conn.conn.copy_area(
            self.blurred_pixmap,
            self.window,
            self.gc,
            0, 0,
            0, 0,
            self.width,
            self.height,
        )?;

        if let Some(region) = region {
            if region.width > 0 && region.height > 0 {
                // Copy selection region from original pixmap
                conn.conn.copy_area(
                    self.original_pixmap,
                    self.window,
                    self.gc,
                    region.x,
                    region.y,
                    region.x,
                    region.y,
                    region.width,
                    region.height,
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

                // Draw text background
                conn.conn.change_gc(
                    self.gc,
                    &ChangeGCAux::new().foreground(0x000000),
                )?;
                conn.conn.image_text8(self.window, self.gc, text_x + 1, text_y + 1, text.as_bytes())?;

                // Draw text foreground
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

/// Create a crosshair cursor.
fn create_crosshair_cursor(conn: &Connection) -> Result<Cursor> {
    // Use the standard cursor font
    let font: Font = conn.conn.generate_id()?;
    conn.conn.open_font(font, b"cursor")?;

    let cursor: Cursor = conn.conn.generate_id()?;
    // 34 is the crosshair glyph in the cursor font
    conn.conn.create_glyph_cursor(
        cursor,
        font,
        font,
        34,     // Source char (crosshair)
        35,     // Mask char
        0xFFFF, 0xFFFF, 0xFFFF, // Foreground RGB (white)
        0, 0, 0,               // Background RGB (black)
    )?;

    conn.conn.close_font(font)?;

    Ok(cursor)
}

/// Run the event loop for selection.
fn run_event_loop(
    conn: &Connection,
    overlay: &Overlay,
    config: &SelectionConfig,
) -> Result<SelectionResult> {
    let mut handler = SelectionHandler::new();
    let mut last_region: Option<Region> = None;

    loop {
        let event = conn.conn.wait_for_event()?;

        if let Some(result) = handler.handle_event(&event) {
            return Ok(result);
        }

        // Only redraw if the region changed (reduces unnecessary draws)
        let current_region = handler.current_region();
        if current_region != last_region {
            overlay.draw(conn, current_region.as_ref(), config)?;
            last_region = current_region;
        }
    }
}
