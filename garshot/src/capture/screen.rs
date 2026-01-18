//! Full screen capture functionality.

use crate::error::Result;
use crate::x11::{shm::bgra_to_rgba_with_alpha, Connection, ShmCapture};

/// Capture the full screen.
///
/// Returns RGBA pixel data for the entire screen.
pub fn capture_full_screen(conn: &Connection, shm: &ShmCapture) -> Result<CaptureResult> {
    let data = shm.capture(conn, 0, 0, conn.width, conn.height)?;

    // Convert BGRA (X11 format) to RGBA (standard format)
    // Force opaque alpha when capturing from compositor overlay (which has alpha=0)
    let rgba = bgra_to_rgba_with_alpha(data, conn.compositor_active);

    Ok(CaptureResult {
        data: rgba,
        width: conn.width as u32,
        height: conn.height as u32,
    })
}

/// Result of a screen capture operation.
pub struct CaptureResult {
    /// RGBA pixel data.
    pub data: Vec<u8>,
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
}
