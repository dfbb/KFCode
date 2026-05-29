//! Generic dialog overlay with open/close state.

use ratatui::{layout::Rect, Frame};

/// A simple dialog that tracks whether it is open and renders into a given area.
pub struct Dialog {
    _title: String,
    open: bool,
}

impl Dialog {
    /// Create a new dialog with the given title, initially closed.
    pub fn new(title: &str) -> Self {
        Self {
            _title: title.to_string(),
            open: false,
        }
    }

    /// Open the dialog.
    pub fn open(&mut self) {
        self.open = true;
    }

    /// Close the dialog.
    pub fn close(&mut self) {
        self.open = false;
    }

    /// Returns true if the dialog is currently open.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Render the dialog into the given area (currently a no-op placeholder).
    pub fn render(&self, _frame: &mut Frame, _area: Rect) {}
}
