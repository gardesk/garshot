//! PNG encoding for garshot.

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use png::{BitDepth, ColorType, Compression, Encoder, FilterType};

use crate::error::{GarshotError, Result};

/// Encode RGBA image data to a PNG file.
///
/// Uses fast compression by default for better screenshot responsiveness.
///
/// # Arguments
/// * `data` - RGBA pixel data (4 bytes per pixel)
/// * `width` - Image width in pixels
/// * `height` - Image height in pixels
/// * `path` - Output file path
pub fn encode_png(data: &[u8], width: u32, height: u32, path: &Path) -> Result<()> {
    let expected_size = (width * height * 4) as usize;
    if data.len() != expected_size {
        return Err(GarshotError::EncodeError(format!(
            "Data size {} does not match expected size {} for {}x{} RGBA image",
            data.len(),
            expected_size,
            width,
            height
        )));
    }

    let file = File::create(path)?;
    let writer = BufWriter::new(file);

    let mut encoder = Encoder::new(writer, width, height);
    encoder.set_color(ColorType::Rgba);
    encoder.set_depth(BitDepth::Eight);

    // Use fast compression for responsiveness
    // Screenshots are typically viewed immediately, so speed > size
    encoder.set_compression(Compression::Fast);
    encoder.set_filter(FilterType::Sub); // Good for screenshots

    let mut png_writer = encoder
        .write_header()
        .map_err(|e| GarshotError::EncodeError(e.to_string()))?;

    png_writer
        .write_image_data(data)
        .map_err(|e| GarshotError::EncodeError(e.to_string()))?;

    tracing::debug!("Encoded PNG {}x{} to {}", width, height, path.display());

    Ok(())
}

/// Encode RGBA image data to PNG bytes in memory.
///
/// Useful for stdout output or clipboard operations.
pub fn encode_png_to_vec(data: &[u8], width: u32, height: u32) -> Result<Vec<u8>> {
    let expected_size = (width * height * 4) as usize;
    if data.len() != expected_size {
        return Err(GarshotError::EncodeError(format!(
            "Data size {} does not match expected size {} for {}x{} RGBA image",
            data.len(),
            expected_size,
            width,
            height
        )));
    }

    let mut buffer = Vec::new();

    {
        let mut encoder = Encoder::new(&mut buffer, width, height);
        encoder.set_color(ColorType::Rgba);
        encoder.set_depth(BitDepth::Eight);
        encoder.set_compression(Compression::Fast);
        encoder.set_filter(FilterType::Sub);

        let mut png_writer = encoder
            .write_header()
            .map_err(|e| GarshotError::EncodeError(e.to_string()))?;

        png_writer
            .write_image_data(data)
            .map_err(|e| GarshotError::EncodeError(e.to_string()))?;
    }

    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn test_encode_png_to_vec() {
        // Create a 2x2 red image
        let data = vec![
            255, 0, 0, 255, // Red pixel
            255, 0, 0, 255, // Red pixel
            255, 0, 0, 255, // Red pixel
            255, 0, 0, 255, // Red pixel
        ];

        let png_data = encode_png_to_vec(&data, 2, 2).unwrap();

        // Check PNG magic bytes
        assert_eq!(&png_data[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
    }

    #[test]
    fn test_encode_png_invalid_size() {
        let data = vec![0; 10]; // Wrong size
        let result = encode_png_to_vec(&data, 2, 2);
        assert!(result.is_err());
    }
}
