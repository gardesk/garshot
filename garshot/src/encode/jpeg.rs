//! JPEG encoding for garshot.

use std::io::Cursor;
use std::path::Path;

use image::{ImageBuffer, RgbaImage};

use crate::error::Result;

/// Encode RGBA image data to JPEG file.
pub fn encode_jpeg(data: &[u8], width: u32, height: u32, path: &Path, quality: u8) -> Result<()> {
    let jpeg_data = encode_jpeg_to_vec(data, width, height, quality)?;
    std::fs::write(path, jpeg_data)?;
    Ok(())
}

/// Encode RGBA image data to JPEG bytes.
pub fn encode_jpeg_to_vec(data: &[u8], width: u32, height: u32, quality: u8) -> Result<Vec<u8>> {
    // Create image buffer from RGBA data
    let img: RgbaImage = ImageBuffer::from_raw(width, height, data.to_vec())
        .ok_or_else(|| crate::error::GarshotError::EncodeError("Invalid image dimensions".into()))?;

    // Convert to RGB (JPEG doesn't support alpha)
    let rgb_img = image::DynamicImage::ImageRgba8(img).to_rgb8();

    // Encode to JPEG
    let mut buffer = Cursor::new(Vec::new());
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, quality);
    encoder
        .encode(
            rgb_img.as_raw(),
            width,
            height,
            image::ExtendedColorType::Rgb8,
        )
        .map_err(|e| crate::error::GarshotError::EncodeError(e.to_string()))?;

    Ok(buffer.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_jpeg_to_vec() {
        // Create a 2x2 red image
        let data = vec![
            255, 0, 0, 255, // Red pixel
            255, 0, 0, 255, // Red pixel
            255, 0, 0, 255, // Red pixel
            255, 0, 0, 255, // Red pixel
        ];

        let jpeg_data = encode_jpeg_to_vec(&data, 2, 2, 90).unwrap();

        // JPEG files start with FF D8 FF
        assert!(jpeg_data.len() > 10);
        assert_eq!(jpeg_data[0], 0xFF);
        assert_eq!(jpeg_data[1], 0xD8);
    }
}
