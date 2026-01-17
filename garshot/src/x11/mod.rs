//! X11 connection and utilities for garshot.

pub mod connection;
pub mod shm;

pub use connection::Connection;
pub use shm::ShmCapture;
