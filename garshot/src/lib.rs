//! garshot - Screenshot utility for the gar desktop suite.
//!
//! garshot provides fast, reliable screenshot capture for X11 using the
//! MIT-SHM extension for performance. It supports full screen, region
//! selection, and window capture modes.
//!
//! ## Features
//!
//! - **Fast capture**: Uses MIT-SHM for ~50ms full-screen captures
//! - **Region selection**: Interactive selection with blur overlay
//! - **Window capture**: Capture specific windows with/without decorations
//! - **Multiple formats**: PNG, JPEG, WebP, PPM/PAM
//! - **Daemon mode**: Background service for instant captures via IPC
//!
//! ## Architecture
//!
//! garshot consists of:
//! - `garshot` (daemon): Background service managing X11 connection and captures
//! - `garshotctl` (CLI): Control tool for sending commands to the daemon
//! - `garshot-ipc`: Shared IPC protocol types

pub mod capture;
pub mod encode;
pub mod error;
pub mod x11;

pub use error::{GarshotError, Result};
pub use x11::{Connection, ShmCapture};
