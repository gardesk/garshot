//! Image encoding modules for garshot.

pub mod jpeg;
pub mod png;
pub mod ppm;
pub mod webp;

pub use self::jpeg::{encode_jpeg, encode_jpeg_to_vec};
pub use self::png::{encode_png, encode_png_to_vec};
pub use self::ppm::{encode_pam, encode_pam_to_vec, encode_ppm, encode_ppm_to_vec};
pub use self::webp::{encode_webp, encode_webp_to_vec};

use std::path::Path;

use crate::error::{GarshotError, Result};

/// Encode image data to a file based on format.
pub fn encode(
    data: &[u8],
    width: u32,
    height: u32,
    path: &Path,
    format: &str,
    quality: u8,
) -> Result<()> {
    match format.to_lowercase().as_str() {
        "png" => encode_png(data, width, height, path),
        "jpg" | "jpeg" => encode_jpeg(data, width, height, path, quality),
        "webp" => encode_webp(data, width, height, path, quality),
        "ppm" => encode_ppm(data, width, height, path),
        "pam" => encode_pam(data, width, height, path),
        _ => Err(GarshotError::EncodeError(format!(
            "Unsupported format: {}",
            format
        ))),
    }
}

/// Encode image data to bytes based on format.
pub fn encode_to_vec(
    data: &[u8],
    width: u32,
    height: u32,
    format: &str,
    quality: u8,
) -> Result<Vec<u8>> {
    match format.to_lowercase().as_str() {
        "png" => encode_png_to_vec(data, width, height),
        "jpg" | "jpeg" => encode_jpeg_to_vec(data, width, height, quality),
        "webp" => encode_webp_to_vec(data, width, height, quality),
        "ppm" => encode_ppm_to_vec(data, width, height),
        "pam" => encode_pam_to_vec(data, width, height),
        _ => Err(GarshotError::EncodeError(format!(
            "Unsupported format: {}",
            format
        ))),
    }
}
