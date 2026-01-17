//! Window capture functionality.

use x11rb::protocol::xproto::*;

use crate::capture::region::{capture_region, Region, RegionCaptureResult};
use crate::error::{GarshotError, Result};
use crate::x11::{Connection, ShmCapture};

/// Information about a window's geometry.
#[derive(Debug, Clone)]
pub struct WindowGeometry {
    /// Window ID.
    pub id: Window,
    /// X coordinate relative to root window.
    pub x: i16,
    /// Y coordinate relative to root window.
    pub y: i16,
    /// Window width.
    pub width: u16,
    /// Window height.
    pub height: u16,
    /// Border width.
    pub border_width: u16,
}

impl WindowGeometry {
    /// Convert to a capture region.
    pub fn to_region(&self) -> Region {
        Region::new(self.x, self.y, self.width, self.height)
    }

    /// Convert to a capture region including borders.
    pub fn to_region_with_borders(&self) -> Region {
        let border = self.border_width as i16;
        Region::new(
            self.x - border,
            self.y - border,
            self.width + self.border_width * 2,
            self.height + self.border_width * 2,
        )
    }
}

/// Get the geometry of a window in root coordinates.
pub fn get_window_geometry(conn: &Connection, window: Window) -> Result<WindowGeometry> {
    // Get window geometry
    let geom = conn
        .conn
        .get_geometry(window)?
        .reply()
        .map_err(|_| GarshotError::WindowNotFound(window))?;

    // Translate to root coordinates
    let translated = conn
        .conn
        .translate_coordinates(window, conn.root, 0, 0)?
        .reply()
        .map_err(|_| GarshotError::WindowNotFound(window))?;

    Ok(WindowGeometry {
        id: window,
        x: translated.dst_x,
        y: translated.dst_y,
        width: geom.width,
        height: geom.height,
        border_width: geom.border_width,
    })
}

/// Get the frame (decoration) window for a client window.
///
/// Walks up the window tree until finding a direct child of the root.
pub fn get_frame_window(conn: &Connection, window: Window) -> Result<Window> {
    let mut current = window;

    loop {
        let tree = conn
            .conn
            .query_tree(current)?
            .reply()
            .map_err(|_| GarshotError::WindowNotFound(window))?;

        if tree.parent == conn.root || tree.parent == 0 {
            return Ok(current);
        }

        current = tree.parent;
    }
}

/// Get the currently active (focused) window.
pub fn get_active_window(conn: &Connection) -> Result<Window> {
    // Try _NET_ACTIVE_WINDOW first (EWMH standard)
    let atom = conn
        .conn
        .intern_atom(false, b"_NET_ACTIVE_WINDOW")?
        .reply()?
        .atom;

    let reply = conn
        .conn
        .get_property(false, conn.root, atom, AtomEnum::WINDOW, 0, 1)?
        .reply()?;

    if reply.value_len == 1 && reply.format == 32 {
        let window = u32::from_ne_bytes(
            reply.value[0..4]
                .try_into()
                .map_err(|_| GarshotError::NoActiveWindow)?,
        );

        if window != 0 {
            return Ok(window);
        }
    }

    // Fallback: get input focus
    let focus = conn.conn.get_input_focus()?.reply()?;

    if focus.focus == 0 || focus.focus == conn.root {
        return Err(GarshotError::NoActiveWindow);
    }

    Ok(focus.focus)
}

/// Capture a window.
///
/// # Arguments
/// * `conn` - X11 connection
/// * `shm` - SHM capture buffer
/// * `window` - Window ID to capture
/// * `include_decorations` - Whether to include window manager decorations
pub fn capture_window(
    conn: &Connection,
    shm: &ShmCapture,
    window: Window,
    include_decorations: bool,
) -> Result<RegionCaptureResult> {
    let target_window = if include_decorations {
        get_frame_window(conn, window)?
    } else {
        window
    };

    let geom = get_window_geometry(conn, target_window)?;

    tracing::debug!(
        "Capturing window 0x{:x} at {}x{}+{}+{} (decorations: {})",
        window,
        geom.width,
        geom.height,
        geom.x,
        geom.y,
        include_decorations
    );

    let region = if include_decorations {
        geom.to_region_with_borders()
    } else {
        geom.to_region()
    };

    capture_region(conn, shm, &region)
}

/// Capture the currently active window.
pub fn capture_active_window(
    conn: &Connection,
    shm: &ShmCapture,
    include_decorations: bool,
) -> Result<RegionCaptureResult> {
    let window = get_active_window(conn)?;
    tracing::debug!("Active window: 0x{:x}", window);
    capture_window(conn, shm, window, include_decorations)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_geometry_to_region() {
        let geom = WindowGeometry {
            id: 0x12345,
            x: 100,
            y: 200,
            width: 800,
            height: 600,
            border_width: 2,
        };

        let region = geom.to_region();
        assert_eq!(region.x, 100);
        assert_eq!(region.y, 200);
        assert_eq!(region.width, 800);
        assert_eq!(region.height, 600);
    }

    #[test]
    fn test_window_geometry_to_region_with_borders() {
        let geom = WindowGeometry {
            id: 0x12345,
            x: 100,
            y: 200,
            width: 800,
            height: 600,
            border_width: 2,
        };

        let region = geom.to_region_with_borders();
        assert_eq!(region.x, 98);
        assert_eq!(region.y, 198);
        assert_eq!(region.width, 804);
        assert_eq!(region.height, 604);
    }
}
