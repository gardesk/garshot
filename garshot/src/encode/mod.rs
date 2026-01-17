//! Image encoding modules for garshot.

pub mod png;

pub use self::png::encode_png;

use std::path::Path;

use crate::error::{GarshotError, Result};

/// Encode image data to a file based on format.
pub fn encode(
    data: &[u8],
    width: u32,
    height: u32,
    path: &Path,
    format: &str,
    _quality: u8,
) -> Result<()> {
    match format.to_lowercase().as_str() {
        "png" => encode_png(data, width, height, path),
        // TODO: Sprint 6 will add jpeg, webp, ppm, pam
        _ => Err(GarshotError::EncodeError(format!(
            "Unsupported format: {}",
            format
        ))),
    }
}
