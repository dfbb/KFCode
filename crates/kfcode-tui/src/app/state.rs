//! Application lifecycle state machine.

/// High-level state of the running TUI application.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum AppState {
    /// Normal operation — the event loop is running.
    #[default]
    Running,
    /// The user has requested exit; the loop will terminate after this frame.
    Exiting,
    /// The text prompt has keyboard focus.
    PromptFocused,
    /// A modal dialog is open and consuming input.
    DialogOpen,
    /// The command palette overlay is open.
    CommandPalette,
}
