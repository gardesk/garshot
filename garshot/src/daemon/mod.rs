//! Daemon module for garshot background service.
//!
//! The daemon maintains an X11 connection and handles screenshot requests
//! from clients via Unix socket IPC.

mod server;
mod state;

pub use server::run_server;
pub use state::DaemonState;
