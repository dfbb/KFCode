//! Standalone dialog for renaming a session.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::theme::Theme;

/// Dialog that presents a single text input for renaming a session.
pub struct SessionRenameDialog {
    open: bool,
    session_id: Option<String>,
    input: String,
}

impl SessionRenameDialog {
    /// Creates a new, closed rename dialog.
    pub fn new() -> Self {
        Self {
            open: false,
            session_id: None,
            input: String::new(),
        }
    }

    /// Opens the dialog pre-filled with the session's current title.
    pub fn open(&mut self, session_id: String, title: String) {
        self.open = true;
        self.session_id = Some(session_id);
        self.input = title;
    }

    /// Closes the dialog and clears the input.
    pub fn close(&mut self) {
        self.open = false;
        self.session_id = None;
        self.input.clear();
    }

    /// Returns `true` if the dialog is currently visible.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Appends a character to the title input.
    pub fn handle_input(&mut self, c: char) {
        self.input.push(c);
    }

    /// Removes the last character from the title input.
    pub fn handle_backspace(&mut self) {
        self.input.pop();
    }

    /// Confirms the rename and returns `(session_id, new_title)`, or `None` if the title is empty.
    pub fn confirm(&mut self) -> Option<(String, String)> {
        let session_id = self.session_id.clone()?;
        let title = self.input.trim().to_string();
        if title.is_empty() {
            return None;
        }
        self.close();
        Some((session_id, title))
    }

    /// Renders the dialog into `frame` if it is open.
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if !self.open {
            return;
        }

        let dialog_area = centered_rect(70, 8, area);
        frame.render_widget(Clear, dialog_area);

        let block = Block::default()
            .title(Span::styled(
                " Rename Session ",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .style(Style::default().bg(theme.background_panel));
        let inner = super::dialog_inner(block.inner(dialog_area));
        frame.render_widget(block, dialog_area);

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(inner);

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("> ", Style::default().fg(theme.primary)),
                Span::styled(&self.input, Style::default().fg(theme.text)),
                Span::styled("▏", Style::default().fg(theme.primary)),
            ])),
            layout[0],
        );

        frame.render_widget(
            Paragraph::new("Enter save  Esc cancel").style(Style::default().fg(theme.text_muted)),
            layout[2],
        );
    }
}

impl Default for SessionRenameDialog {
    fn default() -> Self {
        Self::new()
    }
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    super::centered_rect(width, height, area)
}

