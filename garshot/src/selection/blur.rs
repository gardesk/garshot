//! Gaussian blur implementation for selection overlay.
//!
//! Uses a box blur approximation (3 passes) which is faster than true Gaussian
//! and visually similar for the blur radii we use.

/// Apply a box blur approximation of Gaussian blur to RGBA image data.
///
/// Three passes of box blur closely approximate a Gaussian blur.
///
/// # Arguments
/// * `data` - RGBA pixel data (modified in place)
/// * `width` - Image width
/// * `height` - Image height
/// * `radius` - Blur radius in pixels
pub fn blur_rgba(data: &mut [u8], width: usize, height: usize, radius: usize) {
    if radius == 0 {
        return;
    }

    // Three passes approximate Gaussian
    box_blur(data, width, height, radius);
    box_blur(data, width, height, radius);
    box_blur(data, width, height, radius);
}

/// Single pass box blur (horizontal then vertical).
fn box_blur(data: &mut [u8], width: usize, height: usize, radius: usize) {
    let mut temp = vec![0u8; data.len()];

    // Horizontal pass
    box_blur_horizontal(data, &mut temp, width, height, radius);

    // Vertical pass
    box_blur_vertical(&temp, data, width, height, radius);
}

/// Horizontal box blur pass.
fn box_blur_horizontal(src: &[u8], dst: &mut [u8], width: usize, height: usize, radius: usize) {
    let diameter = radius * 2 + 1;

    for y in 0..height {
        let row_offset = y * width * 4;

        // Initialize running sums for first pixel
        let mut r_sum: u32 = 0;
        let mut g_sum: u32 = 0;
        let mut b_sum: u32 = 0;
        let mut a_sum: u32 = 0;

        // Sum pixels in initial window (extending left edge)
        for i in 0..=radius {
            let idx = row_offset + i.min(width - 1) * 4;
            r_sum += src[idx] as u32;
            g_sum += src[idx + 1] as u32;
            b_sum += src[idx + 2] as u32;
            a_sum += src[idx + 3] as u32;
        }

        // Mirror left edge
        for _ in 0..radius {
            let idx = row_offset;
            r_sum += src[idx] as u32;
            g_sum += src[idx + 1] as u32;
            b_sum += src[idx + 2] as u32;
            a_sum += src[idx + 3] as u32;
        }

        // Process each pixel
        for x in 0..width {
            let out_idx = row_offset + x * 4;
            dst[out_idx] = (r_sum / diameter as u32) as u8;
            dst[out_idx + 1] = (g_sum / diameter as u32) as u8;
            dst[out_idx + 2] = (b_sum / diameter as u32) as u8;
            dst[out_idx + 3] = (a_sum / diameter as u32) as u8;

            // Slide window: subtract left pixel, add right pixel
            let left_x = (x as isize - radius as isize).max(0) as usize;
            let right_x = (x + radius + 1).min(width - 1);

            let left_idx = row_offset + left_x * 4;
            let right_idx = row_offset + right_x * 4;

            r_sum = r_sum - src[left_idx] as u32 + src[right_idx] as u32;
            g_sum = g_sum - src[left_idx + 1] as u32 + src[right_idx + 1] as u32;
            b_sum = b_sum - src[left_idx + 2] as u32 + src[right_idx + 2] as u32;
            a_sum = a_sum - src[left_idx + 3] as u32 + src[right_idx + 3] as u32;
        }
    }
}

/// Vertical box blur pass.
fn box_blur_vertical(src: &[u8], dst: &mut [u8], width: usize, height: usize, radius: usize) {
    let diameter = radius * 2 + 1;

    for x in 0..width {
        // Initialize running sums for first pixel
        let mut r_sum: u32 = 0;
        let mut g_sum: u32 = 0;
        let mut b_sum: u32 = 0;
        let mut a_sum: u32 = 0;

        // Sum pixels in initial window (extending top edge)
        for i in 0..=radius {
            let idx = i.min(height - 1) * width * 4 + x * 4;
            r_sum += src[idx] as u32;
            g_sum += src[idx + 1] as u32;
            b_sum += src[idx + 2] as u32;
            a_sum += src[idx + 3] as u32;
        }

        // Mirror top edge
        for _ in 0..radius {
            let idx = x * 4;
            r_sum += src[idx] as u32;
            g_sum += src[idx + 1] as u32;
            b_sum += src[idx + 2] as u32;
            a_sum += src[idx + 3] as u32;
        }

        // Process each pixel
        for y in 0..height {
            let out_idx = y * width * 4 + x * 4;
            dst[out_idx] = (r_sum / diameter as u32) as u8;
            dst[out_idx + 1] = (g_sum / diameter as u32) as u8;
            dst[out_idx + 2] = (b_sum / diameter as u32) as u8;
            dst[out_idx + 3] = (a_sum / diameter as u32) as u8;

            // Slide window
            let top_y = (y as isize - radius as isize).max(0) as usize;
            let bottom_y = (y + radius + 1).min(height - 1);

            let top_idx = top_y * width * 4 + x * 4;
            let bottom_idx = bottom_y * width * 4 + x * 4;

            r_sum = r_sum - src[top_idx] as u32 + src[bottom_idx] as u32;
            g_sum = g_sum - src[top_idx + 1] as u32 + src[bottom_idx + 1] as u32;
            b_sum = b_sum - src[top_idx + 2] as u32 + src[bottom_idx + 2] as u32;
            a_sum = a_sum - src[top_idx + 3] as u32 + src[bottom_idx + 3] as u32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blur_doesnt_crash() {
        let mut data = vec![128u8; 100 * 100 * 4];
        blur_rgba(&mut data, 100, 100, 5);
        // Just verify it doesn't crash
    }

    #[test]
    fn test_blur_zero_radius() {
        let original = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let mut data = original.clone();
        blur_rgba(&mut data, 2, 1, 0);
        assert_eq!(data, original);
    }
}
