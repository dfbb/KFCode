//! Crossterm terminal initialization and teardown helpers.

use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, Stdout};

/// Type alias for the ratatui terminal backed by crossterm on stdout.
pub type Tui = Terminal<CrosstermBackend<Stdout>>;

/// Enable raw mode, enter the alternate screen, and return a configured `Tui`.
pub fn init() -> io::Result<Tui> {
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(
        io::stdout(),
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture,
        crossterm::event::EnableBracketedPaste,
    )?;
    let backend = CrosstermBackend::new(io::stdout());
    Terminal::new(backend)
}

/// Leave the alternate screen, disable raw mode, and restore the terminal.
pub fn restore() -> io::Result<()> {
    crossterm::execute!(
        io::stdout(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture,
        crossterm::event::DisableBracketedPaste,
    )?;
    crossterm::terminal::disable_raw_mode()?;
    Ok(())
}
