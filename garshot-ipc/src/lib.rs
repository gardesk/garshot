//! IPC protocol types for garshot daemon communication.
//!
//! This crate defines the request/response types used for communication
//! between garshotctl (and other clients) and the garshot daemon.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Request from client to daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum Request {
    /// Capture full screen.
    Screen {
        /// Specific monitor name (None = all monitors combined).
        #[serde(default)]
        monitor: Option<String>,
        /// Include cursor in screenshot.
        #[serde(default)]
        cursor: bool,
        /// Output mode.
        #[serde(default)]
        output: OutputMode,
    },

    /// Interactive region selection with blur overlay.
    Region {
        /// Include cursor in screenshot.
        #[serde(default)]
        cursor: bool,
        /// Output mode.
        #[serde(default)]
        output: OutputMode,
    },

    /// Capture specific window.
    Window {
        /// Window ID (None = active window).
        #[serde(default)]
        id: Option<u32>,
        /// Include window decorations.
        #[serde(default)]
        decorations: bool,
        /// Include cursor in screenshot.
        #[serde(default)]
        cursor: bool,
        /// Output mode.
        #[serde(default)]
        output: OutputMode,
    },

    /// Capture currently active/focused window.
    ActiveWindow {
        /// Include window decorations.
        #[serde(default)]
        decorations: bool,
        /// Include cursor in screenshot.
        #[serde(default)]
        cursor: bool,
        /// Output mode.
        #[serde(default)]
        output: OutputMode,
    },

    /// Get daemon status.
    Status,

    /// List available monitors.
    ListMonitors,

    /// Update configuration value.
    SetConfig {
        /// Configuration key.
        key: String,
        /// New value.
        value: serde_json::Value,
    },

    /// Reload configuration from disk.
    ReloadConfig,

    /// Graceful shutdown.
    Shutdown,
}

/// Output mode for screenshots.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutputMode {
    /// Save to file (default behavior).
    #[default]
    File,
    /// Output raw image data to stdout.
    Stdout,
    /// Copy to clipboard.
    Clipboard,
    /// Both save to file and copy to clipboard.
    FileAndClipboard,
}

/// Response from daemon to client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Path to saved file (if saved to file).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    /// Base64-encoded image data (if stdout mode).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    /// Error message (if success = false).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Additional info (for status, list commands).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub info: Option<serde_json::Value>,
}

impl Response {
    /// Create a successful response with no data.
    pub fn ok() -> Self {
        Self {
            success: true,
            path: None,
            data: None,
            error: None,
            info: None,
        }
    }

    /// Create a successful response with a file path.
    pub fn ok_with_path(path: PathBuf) -> Self {
        Self {
            success: true,
            path: Some(path),
            data: None,
            error: None,
            info: None,
        }
    }

    /// Create a successful response with additional info.
    pub fn ok_with_info(info: serde_json::Value) -> Self {
        Self {
            success: true,
            path: None,
            data: None,
            error: None,
            info: Some(info),
        }
    }

    /// Create an error response.
    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            path: None,
            data: None,
            error: Some(msg.into()),
            info: None,
        }
    }
}

/// Event from daemon (for subscriptions, future use).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum Event {
    /// Screenshot capture completed.
    CaptureComplete {
        /// Path to saved file.
        path: PathBuf,
    },
    /// Interactive selection started.
    SelectionStarted,
    /// Interactive selection was cancelled.
    SelectionCancelled,
}

/// Get the socket path for garshot daemon.
pub fn socket_path() -> PathBuf {
    std::env::var("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
        .join("garshot.sock")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_serialization() {
        let req = Request::Screen {
            monitor: None,
            cursor: true,
            output: OutputMode::File,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"command\":\"screen\""));
        assert!(json.contains("\"cursor\":true"));
    }

    #[test]
    fn test_response_ok() {
        let resp = Response::ok();
        assert!(resp.success);
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_response_error() {
        let resp = Response::error("test error");
        assert!(!resp.success);
        assert_eq!(resp.error.as_deref(), Some("test error"));
    }

    #[test]
    fn test_socket_path() {
        let path = socket_path();
        assert!(path.to_string_lossy().contains("garshot.sock"));
    }
}
