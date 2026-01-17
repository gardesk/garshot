//! Full screen capture functionality.

use crate::error::Result;
use crate::x11::{shm::bgra_to_rgba, Connection, ShmCapture};

/// Capture the full screen.
///
/// Returns RGBA pixel data for the entire screen.
pub fn capture_full_screen(conn: &Connection, shm: &ShmCapture) -> Result<CaptureResult> {
    let data = shm.capture(conn, 0, 0, conn.width, conn.height)?;

    // Convert BGRA (X11 format) to RGBA (standard format)
    let rgba = bgra_to_rgba(data);

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
