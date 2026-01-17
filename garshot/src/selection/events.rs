//! Mouse and keyboard event handling for selection.

use x11rb::protocol::xproto::*;

use crate::capture::Region;

/// Result of the selection operation.
#[derive(Debug, Clone)]
pub enum SelectionResult {
    /// User selected a region.
    Selected(Region),
    /// User cancelled the selection.
    Cancelled,
}

/// Current state of the selection.
#[derive(Debug, Clone)]
pub enum SelectionState {
    /// Waiting for user to start dragging.
    Idle,
    /// User is dragging to select a region.
    Dragging {
        start_x: i16,
        start_y: i16,
        current_x: i16,
        current_y: i16,
    },
}

impl SelectionState {
    /// Get the current selection region (if dragging).
    pub fn current_region(&self) -> Option<Region> {
        match self {
            SelectionState::Idle => None,
            SelectionState::Dragging {
                start_x,
                start_y,
                current_x,
                current_y,
            } => Some(normalize_region(*start_x, *start_y, *current_x, *current_y)),
        }
    }
}

/// Selection event handler.
#[derive(Clone)]
pub struct SelectionHandler {
    state: SelectionState,
}

impl SelectionHandler {
    pub fn new() -> Self {
        Self {
            state: SelectionState::Idle,
        }
    }

    /// Get the current state.
    pub fn state(&self) -> &SelectionState {
        &self.state
    }

    /// Get current cursor position (if dragging).
    pub fn cursor_position(&self) -> Option<(i16, i16)> {
        match &self.state {
            SelectionState::Idle => None,
            SelectionState::Dragging { current_x, current_y, .. } => {
                Some((*current_x, *current_y))
            }
        }
    }

    /// Get the current selection region.
    pub fn current_region(&self) -> Option<Region> {
        self.state.current_region()
    }

    /// Handle any X11 event and return a result if the selection is complete.
    pub fn handle_event(&mut self, event: &x11rb::protocol::Event) -> Option<SelectionResult> {
        use x11rb::protocol::Event;
        match event {
            Event::ButtonPress(e) => self.handle_button_press(e),
            Event::ButtonRelease(e) => self.handle_button_release(e),
            Event::MotionNotify(e) => {
                self.handle_motion(e);
                None
            }
            Event::KeyPress(e) => self.handle_key_press(e),
            _ => None,
        }
    }

    /// Handle a button press event.
    pub fn handle_button_press(&mut self, event: &ButtonPressEvent) -> Option<SelectionResult> {
        match event.detail {
            1 => {
                // Left click: start selection
                self.state = SelectionState::Dragging {
                    start_x: event.event_x,
                    start_y: event.event_y,
                    current_x: event.event_x,
                    current_y: event.event_y,
                };
                None
            }
            3 => {
                // Right click: cancel
                Some(SelectionResult::Cancelled)
            }
            _ => None,
        }
    }

    /// Handle a button release event.
    pub fn handle_button_release(&mut self, event: &ButtonReleaseEvent) -> Option<SelectionResult> {
        if event.detail == 1 {
            // Left release: complete selection
            if let SelectionState::Dragging {
                start_x,
                start_y,
                current_x,
                current_y,
            } = self.state
            {
                let region = normalize_region(start_x, start_y, current_x, current_y);
                if region.width > 1 && region.height > 1 {
                    return Some(SelectionResult::Selected(region));
                }
            }
        }
        None
    }

    /// Handle a motion notify event.
    pub fn handle_motion(&mut self, event: &MotionNotifyEvent) {
        if let SelectionState::Dragging {
            start_x, start_y, ..
        } = self.state
        {
            self.state = SelectionState::Dragging {
                start_x,
                start_y,
                current_x: event.event_x,
                current_y: event.event_y,
            };
        }
    }

    /// Handle a key press event.
    pub fn handle_key_press(&mut self, event: &KeyPressEvent) -> Option<SelectionResult> {
        // Key codes for common keys (may vary by keyboard layout)
        // Escape is typically 9, Return is typically 36
        match event.detail {
            9 => Some(SelectionResult::Cancelled),  // Escape
            36 => {
                // Return/Enter: confirm current selection
                if let Some(region) = self.state.current_region() {
                    if region.width > 1 && region.height > 1 {
                        return Some(SelectionResult::Selected(region));
                    }
                }
                None
            }
            _ => None,
        }
    }
}

impl Default for SelectionHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Normalize a region so width/height are positive.
fn normalize_region(x1: i16, y1: i16, x2: i16, y2: i16) -> Region {
    let x = x1.min(x2);
    let y = y1.min(y2);
    let width = (x2 - x1).unsigned_abs();
    let height = (y2 - y1).unsigned_abs();
    Region::new(x, y, width, height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_region() {
        // Normal order
        let r = normalize_region(10, 20, 110, 120);
        assert_eq!(r.x, 10);
        assert_eq!(r.y, 20);
        assert_eq!(r.width, 100);
        assert_eq!(r.height, 100);

        // Reversed order
        let r = normalize_region(110, 120, 10, 20);
        assert_eq!(r.x, 10);
        assert_eq!(r.y, 20);
        assert_eq!(r.width, 100);
        assert_eq!(r.height, 100);
    }
}
