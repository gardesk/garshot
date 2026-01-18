//! Annotation overlay window using gartk-x11.

use anyhow::{Context, Result};

use crate::annotate::canvas::AnnotationCanvas;
use crate::annotate::history::{History, Snapshot};
use crate::annotate::state::{AnnotationResult, AnnotationState, ToolType};
use crate::annotate::tools::{self, Tool};
use crate::annotate::ui::{Toolbar, TOOLBAR_HEIGHT};

use gartk_core::{InputEvent, Key, KeyEvent, Modifiers, MouseButton, MouseEvent, Point};
use gartk_render::Surface;
use gartk_x11::{Connection, CursorManager, Window, WindowConfig};
use x11rb::connection::Connection as X11Connection;
use x11rb::protocol::xproto::{self, AtomEnum, ConnectionExt, ImageFormat, PropMode};
use x11rb::wrapper::ConnectionExt as WrapperConnectionExt;

/// Maximum bytes per put_image request (conservative, below typical 256KB limit).
const MAX_PUT_IMAGE_BYTES: usize = 65536;

/// Annotation overlay for editing screenshots.
pub struct AnnotationOverlay {
    /// X11 connection.
    conn: Connection,
    /// Overlay window.
    window: Window,
    /// Graphics context for blitting.
    gc: u32,
    /// Annotation canvas.
    canvas: AnnotationCanvas,
    /// Annotation state.
    state: AnnotationState,
    /// Current tool instance.
    tool: Box<dyn Tool>,
    /// Undo/redo history.
    history: History,
    /// Toolbar UI.
    toolbar: Toolbar,
    /// Toolbar surface (reused to avoid allocation each frame).
    toolbar_surface: Surface,
    /// Cursor manager.
    cursor_manager: CursorManager,
    /// Whether a redraw is needed.
    needs_redraw: bool,
    /// Image offset from top (for toolbar).
    image_offset_y: i32,
}

impl AnnotationOverlay {
    /// Create a new annotation overlay.
    ///
    /// # Arguments
    /// * `image_data` - RGBA pixel data of the screenshot
    /// * `width` - Image width
    /// * `height` - Image height
    pub fn new(image_data: &[u8], width: u32, height: u32) -> Result<Self> {
        tracing::debug!("Creating annotation overlay for image {}x{}", width, height);

        // Connect to X11
        let conn = Connection::connect(None).context("Failed to connect to X11")?;

        // Window size = image size + toolbar height
        let window_width = width;
        let window_height = height + TOOLBAR_HEIGHT;
        tracing::debug!("Window size will be {}x{} (including {}px toolbar)",
            window_width, window_height, TOOLBAR_HEIGHT);

        // Center window on screen
        let screen_width = conn.screen_width();
        let screen_height = conn.screen_height();
        let pos_x = (screen_width as i32 - window_width as i32) / 2;
        let pos_y = (screen_height as i32 - window_height as i32) / 2;

        // Create floating window (no override_redirect so gar can manage it)
        let config = WindowConfig::new()
            .title("garshot annotation")
            .class("garshot-annotate")
            .size(window_width, window_height)
            .position(pos_x.max(0), pos_y.max(0))
            .map_on_create(false);

        let window = Window::create(conn.clone(), config).context("Failed to create window")?;
        tracing::debug!("Created window {} at position ({}, {})",
            window.id(), pos_x.max(0), pos_y.max(0));

        // Set window type to DIALOG so gar floats it automatically
        let window_type_atom = conn.inner()
            .intern_atom(false, b"_NET_WM_WINDOW_TYPE")?
            .reply()
            .context("Failed to intern window type atom")?
            .atom;
        let dialog_type_atom = conn.inner()
            .intern_atom(false, b"_NET_WM_WINDOW_TYPE_DIALOG")?
            .reply()
            .context("Failed to intern dialog type atom")?
            .atom;
        conn.inner().change_property32(
            PropMode::REPLACE,
            window.id(),
            window_type_atom,
            AtomEnum::ATOM,
            &[dialog_type_atom],
        )?;

        // Create GC for blitting
        let gc = conn.generate_id()?;
        conn.inner()
            .create_gc(gc, window.id(), &Default::default())?;

        // Create canvas
        let canvas =
            AnnotationCanvas::new(image_data, width, height).context("Failed to create canvas")?;

        // Create toolbar (sized to image width)
        let toolbar = Toolbar::new(width);

        // Create cursor manager
        let cursor_manager =
            CursorManager::new(conn.clone()).context("Failed to create cursor manager")?;

        // Create initial tool
        let tool = tools::create_tool(ToolType::Arrow);

        // Create reusable toolbar surface
        let toolbar_surface = Surface::new(width, TOOLBAR_HEIGHT)
            .context("Failed to create toolbar surface")?;

        // Set WM_NORMAL_HINTS to lock window size (prevents WM from resizing)
        // Flags: PMinSize (16) | PMaxSize (32) | PSize (8) = 56
        let size_hints: [u32; 18] = [
            56,                      // flags: PSize | PMinSize | PMaxSize
            0, 0,                    // x, y (obsolete)
            window_width, window_height,  // width, height (PSize)
            window_width, window_height,  // min_width, min_height (PMinSize)
            window_width, window_height,  // max_width, max_height (PMaxSize)
            0, 0,                    // width_inc, height_inc
            0, 0,                    // min_aspect_num, min_aspect_den
            0, 0,                    // max_aspect_num, max_aspect_den
            0, 0,                    // base_width, base_height
            0,                       // win_gravity
        ];
        conn.inner().change_property32(
            PropMode::REPLACE,
            window.id(),
            AtomEnum::WM_NORMAL_HINTS,
            AtomEnum::WM_SIZE_HINTS,
            &size_hints,
        )?;
        tracing::debug!("Set WM_NORMAL_HINTS: min/max {}x{}", window_width, window_height);

        // Tell compositor to bypass this window (reduces effects/lag from picom)
        let bypass_atom = conn.inner()
            .intern_atom(false, b"_NET_WM_BYPASS_COMPOSITOR")?
            .reply()
            .context("Failed to intern bypass atom")?
            .atom;
        conn.inner().change_property32(
            PropMode::REPLACE,
            window.id(),
            bypass_atom,
            AtomEnum::CARDINAL,
            &[1], // 1 = bypass compositor
        )?;

        Ok(Self {
            conn,
            window,
            gc,
            canvas,
            state: AnnotationState::new(),
            tool,
            history: History::new(),
            toolbar,
            toolbar_surface,
            cursor_manager,
            needs_redraw: true,
            image_offset_y: TOOLBAR_HEIGHT as i32,
        })
    }

    /// Run the annotation overlay event loop.
    pub fn run(mut self) -> Result<AnnotationResult> {
        // Set full opacity to prevent picom from making window transparent
        self.set_full_opacity()?;

        // Map window (gar will manage it as a floating dialog)
        self.window.map()?;

        // Grab keyboard for our shortcuts (Escape, Ctrl+Enter, tool keys, etc.)
        // Don't grab pointer - let gar handle mod+drag for window movement
        self.window.grab_keyboard_with_retry(10, 50)?;

        // Set initial cursor
        self.update_cursor()?;

        // Initial draw
        self.redraw()?;

        // Event loop
        loop {
            // Wait for first event
            let event = self.conn.inner().wait_for_event()?;
            let mut needs_redraw = false;

            // Translate and handle the first event
            if let Some(input_event) = self.translate_event(&event) {
                needs_redraw |= self.handle_event(input_event)?;
            }

            // Check if finished after first event
            if self.state.is_finished() {
                break;
            }

            // Process all pending events before redrawing (batching)
            while let Some(event) = self.conn.inner().poll_for_event()? {
                if let Some(input_event) = self.translate_event(&event) {
                    needs_redraw |= self.handle_event(input_event)?;
                }
                if self.state.is_finished() {
                    break;
                }
            }

            // Single redraw for all batched events
            if needs_redraw {
                self.redraw()?;
            }

            // Check if finished
            if self.state.is_finished() {
                break;
            }
        }

        // Cleanup
        self.window.ungrab_keyboard()?;

        // Return result
        Ok(self.state.take_result().unwrap_or(AnnotationResult::Cancel))
    }

    /// Translate X11 event to InputEvent.
    fn translate_event(&self, event: &x11rb::protocol::Event) -> Option<InputEvent> {
        use x11rb::protocol::Event;

        match event {
            Event::ButtonPress(e) => Some(InputEvent::MousePress(MouseEvent {
                position: Point::new(e.event_x as i32, e.event_y as i32 - self.image_offset_y),
                button: match e.detail {
                    1 => Some(MouseButton::Left),
                    2 => Some(MouseButton::Middle),
                    3 => Some(MouseButton::Right),
                    _ => None,
                },
                modifiers: self.translate_modifiers(e.state),
            })),
            Event::ButtonRelease(e) => Some(InputEvent::MouseRelease(MouseEvent {
                position: Point::new(e.event_x as i32, e.event_y as i32 - self.image_offset_y),
                button: match e.detail {
                    1 => Some(MouseButton::Left),
                    2 => Some(MouseButton::Middle),
                    3 => Some(MouseButton::Right),
                    _ => None,
                },
                modifiers: self.translate_modifiers(e.state),
            })),
            Event::MotionNotify(e) => Some(InputEvent::MouseMove(MouseEvent {
                position: Point::new(e.event_x as i32, e.event_y as i32 - self.image_offset_y),
                button: None,
                modifiers: self.translate_modifiers(e.state),
            })),
            Event::KeyPress(e) => {
                let key = self.translate_keycode(e.detail);
                Some(InputEvent::Key(KeyEvent {
                    key,
                    keycode: e.detail,
                    pressed: true,
                    modifiers: self.translate_modifiers(e.state),
                }))
            }
            Event::KeyRelease(e) => {
                let key = self.translate_keycode(e.detail);
                Some(InputEvent::Key(KeyEvent {
                    key,
                    keycode: e.detail,
                    pressed: false,
                    modifiers: self.translate_modifiers(e.state),
                }))
            }
            Event::Expose(_) => Some(InputEvent::Expose),
            _ => None,
        }
    }

    /// Translate X11 key modifiers.
    fn translate_modifiers(&self, state: xproto::KeyButMask) -> Modifiers {
        Modifiers {
            shift: state.contains(xproto::KeyButMask::SHIFT),
            ctrl: state.contains(xproto::KeyButMask::CONTROL),
            alt: state.contains(xproto::KeyButMask::MOD1),
            super_key: state.contains(xproto::KeyButMask::MOD4),
            caps_lock: state.contains(xproto::KeyButMask::LOCK),
            num_lock: state.contains(xproto::KeyButMask::MOD2),
        }
    }

    /// Translate X11 keycode to Key.
    fn translate_keycode(&self, keycode: u8) -> Key {
        // Common keycodes (evdev-based)
        match keycode {
            9 => Key::Escape,
            36 => Key::Return,
            22 => Key::Backspace,
            65 => Key::Space,
            104 => Key::Return, // Numpad enter

            // Letters (a-z: keycodes 38-58 approximately)
            38 => Key::Char('a'),
            56 => Key::Char('b'),
            54 => Key::Char('c'),
            40 => Key::Char('d'),
            26 => Key::Char('e'),
            41 => Key::Char('f'),
            42 => Key::Char('g'),
            43 => Key::Char('h'),
            31 => Key::Char('i'),
            44 => Key::Char('j'),
            45 => Key::Char('k'),
            46 => Key::Char('l'),
            58 => Key::Char('m'),
            57 => Key::Char('n'),
            32 => Key::Char('o'),
            33 => Key::Char('p'),
            24 => Key::Char('q'),
            27 => Key::Char('r'),
            39 => Key::Char('s'),
            28 => Key::Char('t'),
            30 => Key::Char('u'),
            55 => Key::Char('v'),
            25 => Key::Char('w'),
            53 => Key::Char('x'),
            29 => Key::Char('y'),
            52 => Key::Char('z'),

            // Numbers
            10 => Key::Char('1'),
            11 => Key::Char('2'),
            12 => Key::Char('3'),
            13 => Key::Char('4'),
            14 => Key::Char('5'),
            15 => Key::Char('6'),
            16 => Key::Char('7'),
            17 => Key::Char('8'),
            18 => Key::Char('9'),
            19 => Key::Char('0'),

            // Special characters
            20 => Key::Char('-'),
            21 => Key::Char('='),

            _ => Key::Unknown(keycode),
        }
    }

    /// Handle an input event.
    fn handle_event(&mut self, event: InputEvent) -> Result<bool> {
        let mut needs_redraw = false;

        match &event {
            // Handle toolbar clicks
            InputEvent::MousePress(e) if e.position.y < 0 => {
                // Click is in toolbar area (y < 0 because of offset)
                let toolbar_pos = Point::new(e.position.x, e.position.y + self.image_offset_y);
                if let Some(tool_type) = self.toolbar.handle_click(toolbar_pos) {
                    self.select_tool(tool_type)?;
                    needs_redraw = true;
                }
                return Ok(needs_redraw);
            }

            // Handle keyboard shortcuts
            InputEvent::Key(e) if e.pressed => {
                // Ctrl+Enter: Save
                if e.modifiers.ctrl && e.key == Key::Return {
                    self.save()?;
                    return Ok(false);
                }

                // Escape: Cancel (if not in text editing mode)
                if e.key == Key::Escape && !self.tool.is_drawing() {
                    self.state.finish_cancel();
                    return Ok(false);
                }

                // Ctrl+Z: Undo
                if e.modifiers.ctrl && e.key == Key::Char('z') {
                    if e.modifiers.shift {
                        self.redo()?;
                    } else {
                        self.undo()?;
                    }
                    needs_redraw = true;
                    return Ok(needs_redraw);
                }

                // Ctrl+Y: Redo
                if e.modifiers.ctrl && e.key == Key::Char('y') {
                    self.redo()?;
                    needs_redraw = true;
                    return Ok(needs_redraw);
                }

                // Tool shortcuts (only when not editing text)
                if !self.tool.is_drawing() {
                    if let Key::Char(c) = e.key {
                        // Number keys for color presets
                        if let Some(digit) = c.to_digit(10) {
                            if digit >= 1 && digit <= 9 {
                                self.state.set_color_preset((digit - 1) as usize);
                                needs_redraw = true;
                                return Ok(needs_redraw);
                            }
                        }

                        // Tool shortcuts
                        if let Some(tool_type) = ToolType::from_shortcut(c) {
                            self.select_tool(tool_type)?;
                            needs_redraw = true;
                            return Ok(needs_redraw);
                        }

                        // Line width adjustment
                        if c == '+' || c == '=' {
                            self.state.properties.increase_line_width();
                            needs_redraw = true;
                            return Ok(needs_redraw);
                        }
                        if c == '-' {
                            self.state.properties.decrease_line_width();
                            needs_redraw = true;
                            return Ok(needs_redraw);
                        }
                    }
                }
            }

            _ => {}
        }

        // Pass event to current tool
        let tool_needs_redraw = self.tool.handle_event(&event, &self.state.properties);
        needs_redraw |= tool_needs_redraw;

        // Check if tool completed a drawing
        if let InputEvent::MouseRelease(_) = &event {
            if self.tool.can_commit() && !self.tool.is_drawing() {
                self.commit_tool()?;
                needs_redraw = true;
            }
        }

        Ok(needs_redraw)
    }

    /// Select a tool.
    fn select_tool(&mut self, tool_type: ToolType) -> Result<()> {
        // Commit current tool if it has a pending drawing
        if self.tool.can_commit() {
            self.commit_tool()?;
        }

        self.state.select_tool(tool_type);
        self.tool = tools::create_tool(tool_type);
        self.update_cursor()?;

        Ok(())
    }

    /// Commit the current tool's drawing to the canvas.
    fn commit_tool(&mut self) -> Result<()> {
        // Save snapshot for undo
        let snapshot = Snapshot::new(self.canvas.snapshot_annotations()?);
        self.history.push(snapshot);

        // Commit tool drawing to annotations layer
        let ctx = self.canvas.annotations_surface().context()?;
        self.tool.commit(&ctx, &self.state.properties);

        // Reset tool
        self.tool.reset();

        Ok(())
    }

    /// Undo last action.
    fn undo(&mut self) -> Result<()> {
        let current = Snapshot::new(self.canvas.snapshot_annotations()?);
        if let Some(previous) = self.history.undo(current) {
            self.canvas.restore_annotations(&previous.data)?;
        }
        Ok(())
    }

    /// Redo last undone action.
    fn redo(&mut self) -> Result<()> {
        let current = Snapshot::new(self.canvas.snapshot_annotations()?);
        if let Some(next) = self.history.redo(current) {
            self.canvas.restore_annotations(&next.data)?;
        }
        Ok(())
    }

    /// Save the annotated image.
    fn save(&mut self) -> Result<()> {
        // Commit any pending tool drawing
        if self.tool.can_commit() {
            self.commit_tool()?;
        }

        // Export final image
        let data = self.canvas.export()?;
        let width = self.canvas.width();
        let height = self.canvas.height();

        self.state.finish_save(data, width, height);

        Ok(())
    }

    /// Update cursor for current tool.
    fn update_cursor(&mut self) -> Result<()> {
        let shape = self.tool.cursor();
        self.cursor_manager.set_window_cursor(self.window.id(), shape)?;
        Ok(())
    }

    /// Redraw the overlay.
    fn redraw(&mut self) -> Result<()> {
        // Draw current tool preview
        self.canvas.clear_preview()?;
        let preview_ctx = self.canvas.preview_surface().context()?;
        self.tool.draw_preview(&preview_ctx, &self.state.properties);

        // Render canvas layers
        self.canvas.render()?;

        // Update toolbar state
        self.toolbar
            .update(self.state.current_tool, &self.state.properties);

        // Blit to window (toolbar + image)
        self.blit_to_window()?;

        self.needs_redraw = false;

        Ok(())
    }

    /// Blit canvas and toolbar to window.
    fn blit_to_window(&mut self) -> Result<()> {
        let width = self.canvas.width();
        let height = self.canvas.height();

        // Draw toolbar to reusable surface
        self.toolbar.draw(&self.toolbar_surface)?;

        // Get toolbar data and convert to BGRA
        let toolbar_data = self.toolbar_surface.to_rgba()?;
        let toolbar_bgra = rgba_to_bgra(&toolbar_data);

        // Blit toolbar at y=0 (toolbar is small, no chunking needed)
        self.conn.inner().put_image(
            ImageFormat::Z_PIXMAP,
            self.window.id(),
            self.gc,
            width as u16,
            TOOLBAR_HEIGHT as u16,
            0,
            0,
            0,
            24,
            &toolbar_bgra,
        )?;

        // Get image composite data
        let surface = self.canvas.composite_surface_mut();
        let data = surface.to_rgba()?;
        let bgra = rgba_to_bgra(&data);

        // Blit image at y=TOOLBAR_HEIGHT using chunked put_image for large images
        put_image_chunked(
            self.conn.inner(),
            self.window.id(),
            self.gc,
            width as u16,
            height as u16,
            0,
            self.image_offset_y as i16,
            &bgra,
        )?;

        self.conn.inner().flush()?;

        Ok(())
    }

    /// Set window to full opacity to prevent compositor transparency.
    fn set_full_opacity(&self) -> Result<()> {
        // Intern the _NET_WM_WINDOW_OPACITY atom
        let opacity_atom = self.conn.inner()
            .intern_atom(false, b"_NET_WM_WINDOW_OPACITY")?
            .reply()
            .context("Failed to intern opacity atom")?
            .atom;

        // Full opacity = 0xFFFFFFFF (max u32)
        let opacity: u32 = 0xFFFFFFFF;

        self.conn.inner().change_property32(
            PropMode::REPLACE,
            self.window.id(),
            opacity_atom,
            AtomEnum::CARDINAL,
            &[opacity],
        )?;

        self.conn.inner().flush()?;
        Ok(())
    }
}

/// Convert RGBA to BGRA (X11 native format).
fn rgba_to_bgra(data: &[u8]) -> Vec<u8> {
    let mut bgra = data.to_vec();
    for chunk in bgra.chunks_exact_mut(4) {
        chunk.swap(0, 2); // Swap R and B
    }
    bgra
}

/// Put image data in chunks to avoid exceeding X11 max request size.
fn put_image_chunked<C: X11Connection>(
    conn: &C,
    drawable: u32,
    gc: u32,
    width: u16,
    height: u16,
    dst_x: i16,
    dst_y: i16,
    data: &[u8],
) -> Result<()> {
    let bytes_per_row = width as usize * 4;
    let rows_per_chunk = (MAX_PUT_IMAGE_BYTES / bytes_per_row).max(1);

    let mut y = 0u16;
    while (y as usize) < height as usize {
        let chunk_height = ((height as usize - y as usize).min(rows_per_chunk)) as u16;
        let start = y as usize * bytes_per_row;
        let end = start + chunk_height as usize * bytes_per_row;
        let chunk_data = &data[start..end];

        conn.put_image(
            ImageFormat::Z_PIXMAP,
            drawable,
            gc,
            width,
            chunk_height,
            dst_x,
            dst_y + y as i16,
            0,
            24,
            chunk_data,
        )?;

        y += chunk_height;
    }

    Ok(())
}
