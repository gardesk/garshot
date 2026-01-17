//! WebP encoding for garshot.

use std::io::Cursor;
use std::path::Path;

use image::{ImageBuffer, RgbaImage};

use crate::error::Result;

/// Encode RGBA image data to WebP file.
pub fn encode_webp(data: &[u8], width: u32, height: u32, path: &Path, quality: u8) -> Result<()> {
    let webp_data = encode_webp_to_vec(data, width, height, quality)?;
    std::fs::write(path, webp_data)?;
    Ok(())
}

/// Encode RGBA image data to WebP bytes.
pub fn encode_webp_to_vec(data: &[u8], width: u32, height: u32, _quality: u8) -> Result<Vec<u8>> {
    // Create image buffer from RGBA data
    let img: RgbaImage = ImageBuffer::from_raw(width, height, data.to_vec())
        .ok_or_else(|| crate::error::GarshotError::EncodeError("Invalid image dimensions".into()))?;

    // Encode to WebP (lossless for screenshots)
    let mut buffer = Cursor::new(Vec::new());
    img.write_to(&mut buffer, image::ImageFormat::WebP)
        .map_err(|e| crate::error::GarshotError::EncodeError(e.to_string()))?;

    Ok(buffer.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_webp_to_vec() {
        // Create a 2x2 red image
        let data = vec![
            255, 0, 0, 255, // Red pixel
            255, 0, 0, 255, // Red pixel
            255, 0, 0, 255, // Red pixel
            255, 0, 0, 255, // Red pixel
        ];

        let webp_data = encode_webp_to_vec(&data, 2, 2, 90).unwrap();

        // WebP files start with RIFF header
        assert!(webp_data.len() > 10);
        assert_eq!(&webp_data[0..4], b"RIFF");
        assert_eq!(&webp_data[8..12], b"WEBP");
    }
}
