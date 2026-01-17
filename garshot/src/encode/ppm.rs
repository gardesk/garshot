//! PPM/PAM encoding for garshot.
//!
//! PPM (Portable Pixel Map) and PAM (Portable Arbitrary Map) are simple
//! uncompressed formats, ideal for piping to other tools.

use std::io::Write;
use std::path::Path;

use crate::error::Result;

/// Encode RGBA image data to PPM file (RGB, no alpha).
pub fn encode_ppm(data: &[u8], width: u32, height: u32, path: &Path) -> Result<()> {
    let ppm_data = encode_ppm_to_vec(data, width, height)?;
    std::fs::write(path, ppm_data)?;
    Ok(())
}

/// Encode RGBA image data to PPM bytes (RGB, no alpha).
pub fn encode_ppm_to_vec(data: &[u8], width: u32, height: u32) -> Result<Vec<u8>> {
    let mut buffer = Vec::with_capacity(width as usize * height as usize * 3 + 50);

    // PPM header: P6 (binary RGB)
    writeln!(buffer, "P6")?;
    writeln!(buffer, "{} {}", width, height)?;
    writeln!(buffer, "255")?;

    // Convert RGBA to RGB
    for pixel in data.chunks_exact(4) {
        buffer.push(pixel[0]); // R
        buffer.push(pixel[1]); // G
        buffer.push(pixel[2]); // B
    }

    Ok(buffer)
}

/// Encode RGBA image data to PAM file (with alpha).
pub fn encode_pam(data: &[u8], width: u32, height: u32, path: &Path) -> Result<()> {
    let pam_data = encode_pam_to_vec(data, width, height)?;
    std::fs::write(path, pam_data)?;
    Ok(())
}

/// Encode RGBA image data to PAM bytes (with alpha).
pub fn encode_pam_to_vec(data: &[u8], width: u32, height: u32) -> Result<Vec<u8>> {
    let mut buffer = Vec::with_capacity(data.len() + 100);

    // PAM header
    writeln!(buffer, "P7")?;
    writeln!(buffer, "WIDTH {}", width)?;
    writeln!(buffer, "HEIGHT {}", height)?;
    writeln!(buffer, "DEPTH 4")?;
    writeln!(buffer, "MAXVAL 255")?;
    writeln!(buffer, "TUPLTYPE RGB_ALPHA")?;
    writeln!(buffer, "ENDHDR")?;

    // Raw RGBA data
    buffer.extend_from_slice(data);

    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_ppm_to_vec() {
        // Create a 2x2 red image
        let data = vec![
            255, 0, 0, 255, // Red pixel
            0, 255, 0, 255, // Green pixel
            0, 0, 255, 255, // Blue pixel
            255, 255, 255, 255, // White pixel
        ];

        let ppm_data = encode_ppm_to_vec(&data, 2, 2).unwrap();

        // Check header starts correctly
        assert!(ppm_data.starts_with(b"P6\n2 2\n255\n"));

        // Header is "P6\n2 2\n255\n" = 11 bytes, then 12 bytes of pixel data
        assert_eq!(ppm_data.len(), 11 + 12);
    }

    #[test]
    fn test_encode_pam_to_vec() {
        let data = vec![
            255, 0, 0, 128, // Semi-transparent red
            0, 255, 0, 255, // Opaque green
        ];

        let pam_data = encode_pam_to_vec(&data, 2, 1).unwrap();

        // Check header contains expected elements
        let header_str = String::from_utf8_lossy(&pam_data);
        assert!(header_str.contains("P7"));
        assert!(header_str.contains("WIDTH 2"));
        assert!(header_str.contains("HEIGHT 1"));
        assert!(header_str.contains("DEPTH 4"));
        assert!(header_str.contains("TUPLTYPE RGB_ALPHA"));
        assert!(header_str.contains("ENDHDR"));
    }
}
