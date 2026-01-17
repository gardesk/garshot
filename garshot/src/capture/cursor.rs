//! Cursor capture and blending via XFixes.

use x11rb::protocol::xfixes::ConnectionExt as XfixesExt;

use crate::capture::Region;
use crate::error::{GarshotError, Result};
use crate::x11::Connection;

/// Cursor image data from XFixes.
#[derive(Debug)]
pub struct CursorImage {
    /// Cursor X position on screen.
    pub x: i16,
    /// Cursor Y position on screen.
    pub y: i16,
    /// Cursor image width.
    pub width: u16,
    /// Cursor image height.
    pub height: u16,
    /// Hotspot X offset.
    pub xhot: u16,
    /// Hotspot Y offset.
    pub yhot: u16,
    /// ARGB pixel data (premultiplied alpha).
    pub pixels: Vec<u32>,
}

/// Get the current cursor image.
pub fn get_cursor_image(conn: &Connection) -> Result<CursorImage> {
    // Query XFixes version
    let version = conn
        .conn
        .xfixes_query_version(5, 0)?
        .reply()
        .map_err(|_| GarshotError::XFixesNotAvailable)?;

    tracing::debug!(
        "XFixes version {}.{}",
        version.major_version,
        version.minor_version
    );

    // Get cursor image
    let cursor = conn.conn.xfixes_get_cursor_image()?.reply()?;

    Ok(CursorImage {
        x: cursor.x,
        y: cursor.y,
        width: cursor.width,
        height: cursor.height,
        xhot: cursor.xhot,
        yhot: cursor.yhot,
        pixels: cursor.cursor_image,
    })
}

/// Blend cursor onto RGBA image data.
///
/// The cursor is blended at its current screen position, adjusted for the
/// capture region offset.
///
/// # Arguments
/// * `image` - RGBA pixel data (modified in place)
/// * `image_width` - Image width in pixels
/// * `image_height` - Image height in pixels
/// * `region` - The screen region that was captured
/// * `cursor` - Cursor image from XFixes
pub fn blend_cursor(
    image: &mut [u8],
    image_width: u32,
    image_height: u32,
    region: &Region,
    cursor: &CursorImage,
) {
    // Calculate cursor position relative to captured region
    let cursor_x = cursor.x as i32 - cursor.xhot as i32 - region.x as i32;
    let cursor_y = cursor.y as i32 - cursor.yhot as i32 - region.y as i32;

    tracing::debug!(
        "Blending cursor at screen ({}, {}), region offset ({}, {}), relative ({}, {})",
        cursor.x,
        cursor.y,
        region.x,
        region.y,
        cursor_x,
        cursor_y
    );

    for cy in 0..cursor.height as i32 {
        for cx in 0..cursor.width as i32 {
            let img_x = cursor_x + cx;
            let img_y = cursor_y + cy;

            // Skip pixels outside image bounds
            if img_x < 0
                || img_y < 0
                || img_x >= image_width as i32
                || img_y >= image_height as i32
            {
                continue;
            }

            let cursor_idx = (cy * cursor.width as i32 + cx) as usize;
            let cursor_pixel = cursor.pixels[cursor_idx];

            // Extract ARGB components (XFixes returns premultiplied alpha)
            let src_a = ((cursor_pixel >> 24) & 0xFF) as u8;

            if src_a == 0 {
                continue; // Fully transparent
            }

            let src_r = ((cursor_pixel >> 16) & 0xFF) as u8;
            let src_g = ((cursor_pixel >> 8) & 0xFF) as u8;
            let src_b = (cursor_pixel & 0xFF) as u8;

            let img_idx = ((img_y * image_width as i32 + img_x) * 4) as usize;

            if src_a == 255 {
                // Fully opaque - just overwrite
                image[img_idx] = src_r;
                image[img_idx + 1] = src_g;
                image[img_idx + 2] = src_b;
                // Keep original alpha
            } else {
                // Alpha blend (source is premultiplied)
                let dst_r = image[img_idx] as u16;
                let dst_g = image[img_idx + 1] as u16;
                let dst_b = image[img_idx + 2] as u16;

                let inv_alpha = 255 - src_a as u16;

                image[img_idx] = (src_r as u16 + (dst_r * inv_alpha) / 255) as u8;
                image[img_idx + 1] = (src_g as u16 + (dst_g * inv_alpha) / 255) as u8;
                image[img_idx + 2] = (src_b as u16 + (dst_b * inv_alpha) / 255) as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blend_cursor_outside_bounds() {
        // Cursor completely outside the region
        let mut image = vec![255u8; 4 * 4 * 4]; // 4x4 white image
        let region = Region::new(0, 0, 4, 4);
        let cursor = CursorImage {
            x: 100, // Far outside
            y: 100,
            width: 2,
            height: 2,
            xhot: 0,
            yhot: 0,
            pixels: vec![0xFF000000; 4], // Black cursor
        };

        blend_cursor(&mut image, 4, 4, &region, &cursor);

        // Image should be unchanged
        assert!(image.iter().all(|&p| p == 255));
    }

    #[test]
    fn test_blend_cursor_opaque() {
        // 2x2 white image
        let mut image = vec![255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255];
        let region = Region::new(0, 0, 2, 2);
        let cursor = CursorImage {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
            xhot: 0,
            yhot: 0,
            pixels: vec![0xFF0000FF], // Fully opaque blue (ARGB)
        };

        blend_cursor(&mut image, 2, 2, &region, &cursor);

        // First pixel should be blue (RGB)
        assert_eq!(image[0], 0);   // R
        assert_eq!(image[1], 0);   // G
        assert_eq!(image[2], 255); // B
    }
}
