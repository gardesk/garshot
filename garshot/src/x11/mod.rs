//! X11 connection and utilities for garshot.

pub mod connection;
pub mod monitors;
pub mod shm;

pub use connection::Connection;
pub use monitors::{
    capture_monitor, find_monitor, get_monitors, get_primary_monitor, list_monitor_names, Monitor,
};
pub use shm::ShmCapture;
