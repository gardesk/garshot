//! Daemon state machine.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use garshot_ipc::{OutputMode, Request, Response};

use crate::capture::{
    blend_cursor, capture_active_window, capture_full_screen, capture_region, capture_window,
    get_cursor_image, Region,
};
use crate::encode::encode_png;
use crate::selection::overlay::{interactive_selection, SelectionConfig};
use crate::x11::{capture_monitor, get_monitors, Connection, ShmCapture};

/// Daemon state holding X11 connection and capture resources.
pub struct DaemonState {
    conn: Connection,
    shm: ShmCapture,
    save_dir: PathBuf,
    format: String,
    shutdown: Arc<AtomicBool>,
}

impl DaemonState {
    /// Create a new daemon state.
    pub fn new() -> crate::error::Result<Self> {
        let conn = Connection::new()?;
        let buffer_size = conn.width as usize * conn.height as usize * 4;
        let shm = ShmCapture::new(&conn, buffer_size)?;

        let save_dir = dirs::picture_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Screenshots");

        Ok(Self {
            conn,
            shm,
            save_dir,
            format: "png".to_string(),
            shutdown: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Get a shutdown flag handle.
    pub fn shutdown_flag(&self) -> Arc<AtomicBool> {
        self.shutdown.clone()
    }

    /// Check if shutdown was requested.
    pub fn should_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst)
    }

    /// Handle an incoming request.
    pub fn handle_request(&self, request: Request) -> Response {
        match request {
            Request::Screen {
                monitor,
                cursor,
                output,
            } => self.handle_screen(monitor, cursor, output),

            Request::Region { cursor, output } => self.handle_region(cursor, output),

            Request::Window {
                id,
                decorations,
                cursor,
                output,
            } => self.handle_window(id, decorations, cursor, output),

            Request::ActiveWindow {
                decorations,
                cursor,
                output,
            } => self.handle_window(None, decorations, cursor, output),

            Request::Status => self.handle_status(),

            Request::ListMonitors => self.handle_list_monitors(),

            Request::SetConfig { key, value } => self.handle_set_config(&key, value),

            Request::ReloadConfig => self.handle_reload_config(),

            Request::Shutdown => {
                self.shutdown.store(true, Ordering::SeqCst);
                Response::ok()
            }
        }
    }

    fn handle_screen(
        &self,
        monitor: Option<String>,
        include_cursor: bool,
        output: OutputMode,
    ) -> Response {
        let result = if let Some(monitor_name) = &monitor {
            capture_monitor(&self.conn, &self.shm, monitor_name)
        } else {
            capture_full_screen(&self.conn, &self.shm).map(|r| crate::capture::RegionCaptureResult {
                data: r.data,
                width: r.width,
                height: r.height,
                region: Region::new(0, 0, self.conn.width, self.conn.height),
            })
        };

        match result {
            Ok(mut capture) => {
                if include_cursor {
                    if let Ok(cursor) = get_cursor_image(&self.conn) {
                        blend_cursor(
                            &mut capture.data,
                            capture.width,
                            capture.height,
                            &capture.region,
                            &cursor,
                        );
                    }
                }
                self.save_capture(&capture.data, capture.width, capture.height, output)
            }
            Err(e) => Response::error(e.to_string()),
        }
    }

    fn handle_region(&self, include_cursor: bool, output: OutputMode) -> Response {
        let config = SelectionConfig::default();

        match interactive_selection(&self.conn, &self.shm, &config) {
            Ok(Some(region)) => {
                match capture_region(&self.conn, &self.shm, &region) {
                    Ok(mut capture) => {
                        if include_cursor {
                            if let Ok(cursor) = get_cursor_image(&self.conn) {
                                blend_cursor(
                                    &mut capture.data,
                                    capture.width,
                                    capture.height,
                                    &capture.region,
                                    &cursor,
                                );
                            }
                        }
                        self.save_capture(&capture.data, capture.width, capture.height, output)
                    }
                    Err(e) => Response::error(e.to_string()),
                }
            }
            Ok(None) => Response::error("Selection cancelled"),
            Err(e) => Response::error(e.to_string()),
        }
    }

    fn handle_window(
        &self,
        id: Option<u32>,
        decorations: bool,
        include_cursor: bool,
        output: OutputMode,
    ) -> Response {
        let result = if let Some(window_id) = id {
            capture_window(&self.conn, &self.shm, window_id, decorations)
        } else {
            capture_active_window(&self.conn, &self.shm, decorations)
        };

        match result {
            Ok(mut capture) => {
                if include_cursor {
                    if let Ok(cursor) = get_cursor_image(&self.conn) {
                        blend_cursor(
                            &mut capture.data,
                            capture.width,
                            capture.height,
                            &capture.region,
                            &cursor,
                        );
                    }
                }
                self.save_capture(&capture.data, capture.width, capture.height, output)
            }
            Err(e) => Response::error(e.to_string()),
        }
    }

    fn handle_status(&self) -> Response {
        let info = serde_json::json!({
            "running": true,
            "screen_width": self.conn.width,
            "screen_height": self.conn.height,
            "save_dir": self.save_dir.display().to_string(),
            "format": self.format,
        });
        Response::ok_with_info(info)
    }

    fn handle_list_monitors(&self) -> Response {
        match get_monitors(&self.conn) {
            Ok(monitors) => {
                let info: Vec<_> = monitors
                    .iter()
                    .map(|m| {
                        serde_json::json!({
                            "name": m.name,
                            "width": m.width,
                            "height": m.height,
                            "x": m.x,
                            "y": m.y,
                            "primary": m.primary,
                        })
                    })
                    .collect();
                Response::ok_with_info(serde_json::json!(info))
            }
            Err(e) => Response::error(e.to_string()),
        }
    }

    fn handle_set_config(&self, _key: &str, _value: serde_json::Value) -> Response {
        // TODO: Implement config updates
        Response::error("Config updates not yet implemented")
    }

    fn handle_reload_config(&self) -> Response {
        // TODO: Implement config reload
        Response::error("Config reload not yet implemented")
    }

    fn save_capture(&self, data: &[u8], width: u32, height: u32, output: OutputMode) -> Response {
        match output {
            OutputMode::File | OutputMode::FileAndClipboard => {
                let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
                let filename = format!("screenshot-{}.{}", timestamp, self.format);
                let path = self.save_dir.join(&filename);

                // Ensure save directory exists
                if let Err(e) = std::fs::create_dir_all(&self.save_dir) {
                    return Response::error(format!("Failed to create save directory: {}", e));
                }

                match encode_png(data, width, height, &path) {
                    Ok(()) => {
                        tracing::info!("Saved {}x{} to {}", width, height, path.display());
                        Response::ok_with_path(path)
                    }
                    Err(e) => Response::error(format!("Failed to encode PNG: {}", e)),
                }
            }
            OutputMode::Stdout => {
                // Return base64-encoded PNG data
                match crate::encode::encode_png_to_vec(data, width, height) {
                    Ok(png_data) => {
                        use base64::Engine;
                        let encoded = base64::engine::general_purpose::STANDARD.encode(&png_data);
                        Response {
                            success: true,
                            path: None,
                            data: Some(encoded),
                            error: None,
                            info: None,
                        }
                    }
                    Err(e) => Response::error(format!("Failed to encode PNG: {}", e)),
                }
            }
            OutputMode::Clipboard => {
                // TODO: Implement clipboard
                Response::error("Clipboard not yet implemented")
            }
        }
    }
}
