//! Helpers for setting the terminal window title using crossterm escape sequences.

use std::io;

use crate::branding::{APP_NAME, APP_SHORT_NAME};

/// Set the terminal window title to an arbitrary string.
pub fn set_title(title: &str) -> io::Result<()> {
    crossterm::execute!(io::stdout(), crossterm::terminal::SetTitle(title))
}

/// Set the terminal title to include the app name and the active session title.
pub fn set_session_title(session_title: &str) -> io::Result<()> {
    set_title(&format!("{} ({}) - {}", APP_NAME, APP_SHORT_NAME, session_title))
}

/// Reset the terminal title to the default app name without a session suffix.
pub fn reset_title() -> io::Result<()> {
    set_title(&format!("{} ({})", APP_NAME, APP_SHORT_NAME))
}
