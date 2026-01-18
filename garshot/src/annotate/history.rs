//! Undo/redo history for annotation canvas.

/// A snapshot of the annotation state for undo/redo.
#[derive(Debug, Clone)]
pub struct Snapshot {
    /// RGBA pixel data of the annotations layer.
    pub data: Vec<u8>,
}

impl Snapshot {
    /// Create a new snapshot from pixel data.
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
}

/// Undo/redo history stack.
pub struct History {
    /// Stack of past states (for undo).
    undo_stack: Vec<Snapshot>,
    /// Stack of undone states (for redo).
    redo_stack: Vec<Snapshot>,
    /// Maximum history depth.
    max_depth: usize,
}

impl History {
    /// Create a new history with default max depth.
    pub fn new() -> Self {
        Self::with_max_depth(50)
    }

    /// Create a new history with specified max depth.
    pub fn with_max_depth(max_depth: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_depth,
        }
    }

    /// Push a new snapshot onto the undo stack.
    ///
    /// This clears the redo stack since we're branching history.
    pub fn push(&mut self, snapshot: Snapshot) {
        // Clear redo stack - new action invalidates redo history
        self.redo_stack.clear();

        // Add to undo stack
        self.undo_stack.push(snapshot);

        // Trim if over max depth
        while self.undo_stack.len() > self.max_depth {
            self.undo_stack.remove(0);
        }
    }

    /// Undo: pop from undo stack, return snapshot to restore.
    ///
    /// The `current` parameter is the current state to save for redo.
    pub fn undo(&mut self, current: Snapshot) -> Option<Snapshot> {
        if let Some(previous) = self.undo_stack.pop() {
            // Save current state for redo
            self.redo_stack.push(current);
            Some(previous)
        } else {
            None
        }
    }

    /// Redo: pop from redo stack, return snapshot to restore.
    ///
    /// The `current` parameter is the current state to save for undo.
    pub fn redo(&mut self, current: Snapshot) -> Option<Snapshot> {
        if let Some(next) = self.redo_stack.pop() {
            // Save current state for undo
            self.undo_stack.push(current);
            Some(next)
        } else {
            None
        }
    }

    /// Check if undo is available.
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if redo is available.
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Get the number of undo steps available.
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Get the number of redo steps available.
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }

    /// Clear all history.
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}

impl Default for History {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_undo_redo() {
        let mut history = History::new();

        // Push some states
        history.push(Snapshot::new(vec![1]));
        history.push(Snapshot::new(vec![2]));
        history.push(Snapshot::new(vec![3]));

        assert!(history.can_undo());
        assert!(!history.can_redo());

        // Undo
        let current = Snapshot::new(vec![4]);
        let restored = history.undo(current).unwrap();
        assert_eq!(restored.data, vec![3]);

        assert!(history.can_undo());
        assert!(history.can_redo());

        // Redo
        let current = Snapshot::new(vec![3]);
        let restored = history.redo(current).unwrap();
        assert_eq!(restored.data, vec![4]);
    }

    #[test]
    fn test_new_action_clears_redo() {
        let mut history = History::new();

        history.push(Snapshot::new(vec![1]));
        history.push(Snapshot::new(vec![2]));

        // Undo
        let _ = history.undo(Snapshot::new(vec![3]));
        assert!(history.can_redo());

        // New action should clear redo
        history.push(Snapshot::new(vec![4]));
        assert!(!history.can_redo());
    }
}
