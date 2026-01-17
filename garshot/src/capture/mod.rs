//! Screen capture functionality.

pub mod region;
pub mod screen;

pub use region::{capture_region, Region, RegionCaptureResult};
pub use screen::capture_full_screen;
