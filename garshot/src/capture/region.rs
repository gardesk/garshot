//! Region capture functionality.

use crate::error::{GarshotError, Result};
use crate::x11::{shm::bgra_to_rgba_with_alpha, Connection, ShmCapture};

/// A rectangular region on the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
}

impl Region {
    /// Create a new region.
    pub fn new(x: i16, y: i16, width: u16, height: u16) -> Self {
        Self { x, y, width, height }
    }

    /// Parse a geometry string in the format WxH+X+Y or WxH-X-Y.
    ///
    /// Examples: "800x600+100+50", "1920x1080+0+0", "640x480-10-10"
    pub fn from_geometry(s: &str) -> Result<Self> {
        let s = s.trim();

        // Find the 'x' separator between width and height
        let x_pos = s
            .find('x')
            .ok_or_else(|| GarshotError::InvalidRegion("missing 'x' separator".into()))?;

        let width: u16 = s[..x_pos]
            .parse()
            .map_err(|_| GarshotError::InvalidRegion("invalid width".into()))?;

        let rest = &s[x_pos + 1..];

        // Find the +/- separator for x coordinate
        let sign_pos = rest
            .find(|c| c == '+' || c == '-')
            .ok_or_else(|| GarshotError::InvalidRegion("missing +/- for x coordinate".into()))?;

        let height: u16 = rest[..sign_pos]
            .parse()
            .map_err(|_| GarshotError::InvalidRegion("invalid height".into()))?;

        let x_negative = rest.chars().nth(sign_pos) == Some('-');
        let rest = &rest[sign_pos + 1..];

        // Find the +/- separator for y coordinate
        let sign_pos = rest
            .find(|c| c == '+' || c == '-')
            .ok_or_else(|| GarshotError::InvalidRegion("missing +/- for y coordinate".into()))?;

        let x_val: i16 = rest[..sign_pos]
            .parse()
            .map_err(|_| GarshotError::InvalidRegion("invalid x coordinate".into()))?;

        let y_negative = rest.chars().nth(sign_pos) == Some('-');
        let y_val: i16 = rest[sign_pos + 1..]
            .parse()
            .map_err(|_| GarshotError::InvalidRegion("invalid y coordinate".into()))?;

        let x = if x_negative { -x_val } else { x_val };
        let y = if y_negative { -y_val } else { y_val };

        Ok(Self { x, y, width, height })
    }

    /// Clip region to screen bounds.
    pub fn clip_to_screen(&self, screen_width: u16, screen_height: u16) -> Self {
        let x = self.x.max(0);
        let y = self.y.max(0);

        // Account for pixels clipped off the left/top
        let x_shift = (x - self.x) as u16;
        let y_shift = (y - self.y) as u16;

        let adjusted_width = self.width.saturating_sub(x_shift);
        let adjusted_height = self.height.saturating_sub(y_shift);

        let max_width = (screen_width as i16 - x).max(0) as u16;
        let max_height = (screen_height as i16 - y).max(0) as u16;

        let width = adjusted_width.min(max_width);
        let height = adjusted_height.min(max_height);

        Self { x, y, width, height }
    }

    /// Check if the region is valid (non-zero dimensions).
    pub fn is_valid(&self) -> bool {
        self.width > 0 && self.height > 0
    }
}

/// Result of a region capture operation.
pub struct RegionCaptureResult {
    /// RGBA pixel data.
    pub data: Vec<u8>,
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
    /// The actual region that was captured (may differ from requested due to clipping).
    pub region: Region,
}

/// Capture a specific region of the screen.
pub fn capture_region(
    conn: &Connection,
    shm: &ShmCapture,
    region: &Region,
) -> Result<RegionCaptureResult> {
    // Clip to screen bounds
    let clipped = region.clip_to_screen(conn.width, conn.height);

    if !clipped.is_valid() {
        return Err(GarshotError::InvalidRegion(format!(
            "region {}x{}+{}+{} is outside screen bounds",
            region.width, region.height, region.x, region.y
        )));
    }

    tracing::debug!(
        "Capturing region {}x{}+{}+{} (clipped from {}x{}+{}+{})",
        clipped.width,
        clipped.height,
        clipped.x,
        clipped.y,
        region.width,
        region.height,
        region.x,
        region.y
    );

    let data = shm.capture(conn, clipped.x, clipped.y, clipped.width, clipped.height)?;
    // Force opaque alpha when capturing from compositor overlay (which has alpha=0)
    let rgba = bgra_to_rgba_with_alpha(data, conn.compositor_active);

    Ok(RegionCaptureResult {
        data: rgba,
        width: clipped.width as u32,
        height: clipped.height as u32,
        region: clipped,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_from_geometry() {
        let r = Region::from_geometry("800x600+100+50").unwrap();
        assert_eq!(r.width, 800);
        assert_eq!(r.height, 600);
        assert_eq!(r.x, 100);
        assert_eq!(r.y, 50);
    }

    #[test]
    fn test_region_from_geometry_negative() {
        let r = Region::from_geometry("640x480-10-20").unwrap();
        assert_eq!(r.width, 640);
        assert_eq!(r.height, 480);
        assert_eq!(r.x, -10);
        assert_eq!(r.y, -20);
    }

    #[test]
    fn test_region_clip() {
        let r = Region::new(-10, -10, 100, 100);
        let clipped = r.clip_to_screen(1920, 1080);
        assert_eq!(clipped.x, 0);
        assert_eq!(clipped.y, 0);
        assert_eq!(clipped.width, 90);
        assert_eq!(clipped.height, 90);
    }

    #[test]
    fn test_region_clip_overflow() {
        let r = Region::new(1900, 1000, 100, 200);
        let clipped = r.clip_to_screen(1920, 1080);
        assert_eq!(clipped.x, 1900);
        assert_eq!(clipped.y, 1000);
        assert_eq!(clipped.width, 20);
        assert_eq!(clipped.height, 80);
    }
}
