//! Interactive region selection with blur overlay.

pub mod blur;
pub mod events;
pub mod overlay;

pub use events::SelectionResult;
pub use overlay::interactive_selection;
