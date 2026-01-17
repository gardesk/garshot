//! X11 connection management for garshot.

use x11rb::connection::Connection as X11Connection;
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

        Ok(Self {
            root: screen.root,
            width: screen.width_in_pixels,
            height: screen.height_in_pixels,
            depth: screen.root_depth,
            visual: screen.root_visual,
            conn,
            screen_num,
        })
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
            .finish()
    }
}
