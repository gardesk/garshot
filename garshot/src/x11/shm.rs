//! MIT-SHM extension for fast screen capture.
//!
//! The MIT-SHM (Shared Memory) extension allows the X server to read/write
//! image data directly from shared memory, avoiding the overhead of copying
//! data over the X protocol.
//!
//! This is critical for screenshot performance - without MIT-SHM, capturing
//! a full desktop can take 1-2 seconds. With MIT-SHM, it takes ~50ms.

use std::os::unix::io::{FromRawFd, OwnedFd};
use std::ptr;
use std::slice;

use x11rb::connection::Connection as X11Connection;
use x11rb::protocol::shm::{self, ConnectionExt as ShmExt};
use x11rb::protocol::xproto::*;

use crate::error::{GarshotError, Result};
use crate::x11::Connection;

/// Shared memory segment for fast X11 image capture.
pub struct ShmCapture {
    /// SHM segment ID.
    seg: shm::Seg,
    /// Pointer to shared memory.
    data: *mut u8,
    /// Size of shared memory in bytes.
    size: usize,
    /// File descriptor for the shared memory.
    #[allow(dead_code)]
    fd: i32,
}

// SAFETY: The shared memory is only accessed through &self or &mut self,
// and we ensure proper synchronization with the X server.
unsafe impl Send for ShmCapture {}
unsafe impl Sync for ShmCapture {}

impl ShmCapture {
    /// Create a new shared memory capture buffer.
    ///
    /// # Arguments
    /// * `conn` - X11 connection
    /// * `size` - Size of the buffer in bytes (should be width * height * 4)
    pub fn new(conn: &Connection, size: usize) -> Result<Self> {
        // Query SHM extension
        let shm_version = conn
            .conn
            .shm_query_version()?
            .reply()
            .map_err(|_| GarshotError::ShmNotAvailable)?;

        tracing::debug!(
            "MIT-SHM version {}.{}, shared_pixmaps: {}",
            shm_version.major_version,
            shm_version.minor_version,
            shm_version.shared_pixmaps
        );

        // Create shared memory using memfd_create (Linux 3.17+)
        // This is cleaner than the old shmget/shmat approach
        let fd = unsafe {
            libc::memfd_create(
                b"garshot-shm\0".as_ptr() as *const libc::c_char,
                libc::MFD_CLOEXEC,
            )
        };

        if fd < 0 {
            return Err(GarshotError::ShmCreate(
                "memfd_create failed".to_string(),
            ));
        }

        // Set the size
        if unsafe { libc::ftruncate(fd, size as libc::off_t) } < 0 {
            unsafe { libc::close(fd) };
            return Err(GarshotError::ShmCreate(
                "ftruncate failed".to_string(),
            ));
        }

        // Map the memory
        let data = unsafe {
            libc::mmap(
                ptr::null_mut(),
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                0,
            )
        };

        if data == libc::MAP_FAILED {
            unsafe { libc::close(fd) };
            return Err(GarshotError::ShmCreate("mmap failed".to_string()));
        }

        // Attach to X server
        let seg = conn.conn.generate_id()?;

        // Use shm_attach_fd (modern method with file descriptors)
        // We need to dup the fd because shm_attach_fd takes ownership
        let fd_for_x = unsafe { libc::dup(fd) };
        if fd_for_x < 0 {
            unsafe { libc::close(fd) };
            return Err(GarshotError::ShmCreate("dup failed".to_string()));
        }
        let owned_fd = unsafe { OwnedFd::from_raw_fd(fd_for_x) };
        conn.conn.shm_attach_fd(seg, owned_fd, false)?;
        conn.conn.flush()?;

        tracing::debug!("Created SHM segment {} with {} bytes", seg, size);

        Ok(Self {
            seg,
            data: data as *mut u8,
            size,
            fd,
        })
    }

    /// Capture a region of the screen into the shared memory buffer.
    ///
    /// Returns a slice of the captured pixel data in BGRA format.
    ///
    /// # Arguments
    /// * `conn` - X11 connection
    /// * `x` - X coordinate of the capture region
    /// * `y` - Y coordinate of the capture region
    /// * `width` - Width of the capture region
    /// * `height` - Height of the capture region
    pub fn capture(
        &self,
        conn: &Connection,
        x: i16,
        y: i16,
        width: u16,
        height: u16,
    ) -> Result<&[u8]> {
        let pixel_count = width as usize * height as usize;
        let byte_count = pixel_count * 4; // BGRA = 4 bytes per pixel

        if byte_count > self.size {
            return Err(GarshotError::InvalidRegion(format!(
                "Region {}x{} requires {} bytes, but buffer is only {} bytes",
                width, height, byte_count, self.size
            )));
        }

        // Use overlay window if compositor is active, otherwise use root
        let drawable = conn.overlay_window.unwrap_or(conn.root);

        tracing::debug!(
            "Capturing region {}x{}+{}+{} ({} bytes) from drawable 0x{:x}{}",
            width, height, x, y, byte_count, drawable,
            if conn.compositor_active { " (compositor active)" } else { "" }
        );

        // Use shm_get_image to capture directly to shared memory
        let reply = conn
            .conn
            .shm_get_image(
                drawable,
                x,
                y,
                width,
                height,
                0xFFFFFFFF, // plane_mask: all planes
                ImageFormat::Z_PIXMAP.into(),
                self.seg,
                0, // offset in shm segment
            )?
            .reply()?;

        tracing::debug!(
            "Captured {} bytes, depth {}, visual 0x{:x}",
            reply.size,
            reply.depth,
            reply.visual
        );

        // SAFETY: The X server has written to the shared memory, and we
        // have exclusive access to this buffer through &self.
        let data = unsafe { slice::from_raw_parts(self.data, byte_count) };

        Ok(data)
    }

    /// Get the raw data pointer (for advanced use).
    pub fn data_ptr(&self) -> *mut u8 {
        self.data
    }

    /// Get the size of the shared memory buffer.
    pub fn size(&self) -> usize {
        self.size
    }
}

impl Drop for ShmCapture {
    fn drop(&mut self) {
        // Note: We don't detach the segment here because the Connection
        // might already be dropped. The X server will clean up when the
        // connection closes.

        // Unmap the memory
        unsafe {
            libc::munmap(self.data as *mut libc::c_void, self.size);
        }

        // Close the file descriptor
        // Note: The X server now owns a dup of the fd, so this is safe
        unsafe {
            libc::close(self.fd);
        }

        tracing::debug!("Dropped SHM segment {}", self.seg);
    }
}

/// Convert BGRA pixel data (X11 format) to RGBA (standard image format).
///
/// This is necessary because X11 returns pixels in BGRA format, but most
/// image formats expect RGBA.
pub fn bgra_to_rgba(data: &[u8]) -> Vec<u8> {
    bgra_to_rgba_with_alpha(data, false)
}

/// Convert BGRA pixel data (X11 format) to RGBA, optionally forcing opaque alpha.
///
/// When `force_opaque` is true, all alpha values are set to 255. This is needed
/// when capturing from the compositor overlay window, which returns valid RGB
/// data but with alpha=0 (transparent).
pub fn bgra_to_rgba_with_alpha(data: &[u8], force_opaque: bool) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(data.len());

    for chunk in data.chunks_exact(4) {
        rgba.push(chunk[2]); // R (was B)
        rgba.push(chunk[1]); // G
        rgba.push(chunk[0]); // B (was R)
        rgba.push(if force_opaque { 255 } else { chunk[3] }); // A
    }

    rgba
}

/// Convert BGRA pixel data to RGBA in place.
pub fn bgra_to_rgba_inplace(data: &mut [u8]) {
    bgra_to_rgba_inplace_with_alpha(data, false)
}

/// Convert BGRA pixel data to RGBA in place, optionally forcing opaque alpha.
pub fn bgra_to_rgba_inplace_with_alpha(data: &mut [u8], force_opaque: bool) {
    for chunk in data.chunks_exact_mut(4) {
        chunk.swap(0, 2); // Swap B and R
        if force_opaque {
            chunk[3] = 255;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bgra_to_rgba() {
        let bgra = vec![0, 128, 255, 255]; // B=0, G=128, R=255, A=255
        let rgba = bgra_to_rgba(&bgra);
        assert_eq!(rgba, vec![255, 128, 0, 255]); // R=255, G=128, B=0, A=255
    }

    #[test]
    fn test_bgra_to_rgba_inplace() {
        let mut data = vec![0, 128, 255, 255];
        bgra_to_rgba_inplace(&mut data);
        assert_eq!(data, vec![255, 128, 0, 255]);
    }
}
