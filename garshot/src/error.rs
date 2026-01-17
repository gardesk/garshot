//! Error types for garshot.

use thiserror::Error;

/// Garshot error type.
#[derive(Debug, Error)]
pub enum GarshotError {
    /// Failed to connect to X11 display.
    #[error("Failed to connect to X11 display: {0}")]
    X11Connect(#[from] x11rb::errors::ConnectError),

    /// X11 protocol error.
    #[error("X11 protocol error: {0}")]
    X11Protocol(#[from] x11rb::errors::ReplyError),

    /// X11 connection error.
    #[error("X11 connection error: {0}")]
    X11Connection(#[from] x11rb::errors::ConnectionError),

    /// X11 reply or ID error.
    #[error("X11 reply or ID error: {0}")]
    X11ReplyOrId(#[from] x11rb::errors::ReplyOrIdError),

    /// MIT-SHM extension not available.
    #[error("MIT-SHM extension not available")]
    ShmNotAvailable,

    /// Failed to create shared memory segment.
    #[error("Failed to create shared memory: {0}")]
    ShmCreate(String),

    /// XFixes extension not available.
    #[error("XFixes extension not available")]
    XFixesNotAvailable,

    /// XRandR extension not available.
    #[error("XRandR extension not available")]
    RandrNotAvailable,

    /// Monitor not found.
    #[error("Monitor '{0}' not found")]
    MonitorNotFound(String),

    /// Window not found.
    #[error("Window 0x{0:x} not found")]
    WindowNotFound(u32),

    /// No active window.
    #[error("No active window")]
    NoActiveWindow,

    /// Region selection cancelled.
    #[error("Region selection cancelled")]
    SelectionCancelled,

    /// Invalid region.
    #[error("Invalid region: {0}")]
    InvalidRegion(String),

    /// Failed to encode image.
    #[error("Failed to encode image: {0}")]
    EncodeError(String),

    /// Failed to save file.
    #[error("Failed to save file: {0}")]
    IoError(#[from] std::io::Error),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// IPC error.
    #[error("IPC error: {0}")]
    IpcError(String),

    /// Invalid image dimensions.
    #[error("Invalid image dimensions: {width}x{height}")]
    InvalidDimensions { width: u32, height: u32 },
}

/// Result type alias for garshot operations.
pub type Result<T> = std::result::Result<T, GarshotError>;
