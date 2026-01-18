//! X11 connection management for garshot.

use x11rb::connection::Connection as X11Connection;
use x11rb::protocol::composite::ConnectionExt as CompositeExt;
use x11rb::protocol::xproto::*;
use x11rb::rust_connection::RustConnection;

use crate::error::{GarshotError, Result};

/// X11 connection wrapper with cached screen information.
pub struct Connection {
    /// The underlying X11 connection.
    pub conn: RustConnection,
    /// Screen number.
    pub screen_num: usize,
    /// Root window ID.
    pub root: Window,
    /// Screen width in pixels.
    pub width: u16,
    /// Screen height in pixels.
    pub height: u16,
    /// Root window depth.
    pub depth: u8,
    /// Root visual ID.
    pub visual: Visualid,
    /// Whether a compositor is active.
    pub compositor_active: bool,
    /// Composite overlay window (if compositor is active).
    pub overlay_window: Option<Window>,
}

impl Connection {
    /// Connect to the X11 display.
    ///
    /// Uses the `DISPLAY` environment variable if no display is specified.
    pub fn new() -> Result<Self> {
        Self::connect(None)
    }

    /// Connect to a specific X11 display.
    pub fn connect(display: Option<&str>) -> Result<Self> {
        let (conn, screen_num) = x11rb::connect(display)?;
        let screen = &conn.setup().roots[screen_num];

        tracing::info!(
            "Connected to X11 display, screen {}x{} depth {}",
            screen.width_in_pixels,
            screen.height_in_pixels,
            screen.root_depth
        );

        // Check if a compositor is running by looking for _NET_WM_CM_S{screen} selection owner
        let compositor_active = Self::detect_compositor(&conn, screen_num, screen.root)?;

        // Initialize composite extension and get overlay window if compositor is active
        let overlay_window = if compositor_active {
            // Query composite extension version
            if let Ok(reply) = conn.composite_query_version(0, 4)?.reply() {
                tracing::info!(
                    "Composite extension version {}.{}",
                    reply.major_version,
                    reply.minor_version
                );

                // Get the overlay window - this is where composited content is rendered
                match conn.composite_get_overlay_window(screen.root)?.reply() {
                    Ok(overlay) => {
                        tracing::info!("Got composite overlay window: 0x{:x}", overlay.overlay_win);
                        Some(overlay.overlay_win)
                    }
                    Err(e) => {
                        tracing::warn!("Failed to get overlay window: {}", e);
                        None
                    }
                }
            } else {
                tracing::warn!("Composite extension not available");
                None
            }
        } else {
            None
        };

        if compositor_active {
            tracing::info!("Compositor detected - will use overlay window for capture");
        }

        Ok(Self {
            root: screen.root,
            width: screen.width_in_pixels,
            height: screen.height_in_pixels,
            depth: screen.root_depth,
            visual: screen.root_visual,
            conn,
            screen_num,
            compositor_active,
            overlay_window,
        })
    }

    /// Detect if a compositor is running.
    fn detect_compositor(conn: &RustConnection, screen_num: usize, _root: Window) -> Result<bool> {
        // The compositor manager selection is _NET_WM_CM_S{screen}
        let atom_name = format!("_NET_WM_CM_S{}", screen_num);
        let atom = conn.intern_atom(false, atom_name.as_bytes())?.reply()?.atom;

        // Check if the selection has an owner
        let owner = conn.get_selection_owner(atom)?.reply()?.owner;

        if owner != x11rb::NONE {
            tracing::debug!("Compositor detected: _NET_WM_CM_S{} owner is 0x{:x}", screen_num, owner);
            Ok(true)
        } else {
            tracing::debug!("No compositor detected");
            Ok(false)
        }
    }

    /// Get the screen structure.
    pub fn screen(&self) -> &Screen {
        &self.conn.setup().roots[self.screen_num]
    }

    /// Generate a new X11 resource ID.
    pub fn generate_id(&self) -> Result<u32> {
        self.conn.generate_id().map_err(GarshotError::X11ReplyOrId)
    }

    /// Flush pending requests to the X server.
    pub fn flush(&self) -> Result<()> {
        self.conn.flush()?;
        Ok(())
    }

    /// Synchronize with the X server (flush and wait for reply).
    pub fn sync(&self) -> Result<()> {
        // GetInputFocus is a simple request that forces a round-trip
        self.conn.get_input_focus()?.reply()?;
        Ok(())
    }
}

impl std::fmt::Debug for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Connection")
            .field("screen_num", &self.screen_num)
            .field("root", &format_args!("0x{:x}", self.root))
            .field("size", &format_args!("{}x{}", self.width, self.height))
            .field("depth", &self.depth)
            .field("compositor_active", &self.compositor_active)
            .finish()
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        // Release the overlay window if we acquired it
        if let Some(overlay) = self.overlay_window {
            tracing::debug!("Releasing composite overlay window 0x{:x}", overlay);
            let _ = self.conn.composite_release_overlay_window(self.root);
            let _ = self.conn.flush();
        }
    }
}
