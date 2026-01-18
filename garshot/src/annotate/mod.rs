//! Annotation mode for screenshot editing.
//!
//! This module provides an interactive annotation editor that can be used
//! after capturing a screenshot or for editing existing images.

mod canvas;
mod history;
mod overlay;
mod state;
pub mod tools;
pub mod ui;

pub use canvas::AnnotationCanvas;
pub use history::History;
pub use overlay::AnnotationOverlay;
pub use state::{AnnotationResult, AnnotationState, ToolProperties, ToolType};
