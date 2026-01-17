//! Screen capture functionality.

pub mod cursor;
pub mod region;
pub mod screen;
pub mod window;

pub use cursor::{blend_cursor, get_cursor_image, CursorImage};
pub use region::{capture_region, Region, RegionCaptureResult};
pub use screen::capture_full_screen;
pub use window::{
    capture_active_window, capture_window, get_active_window, get_frame_window,
    get_window_geometry, WindowGeometry,
};
