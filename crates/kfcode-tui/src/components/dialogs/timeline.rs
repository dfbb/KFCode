//! Dialog for navigating the message timeline of the current session.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState},
    Frame,
};

use crate::theme::Theme;

/// A single message entry shown in the timeline dialog.
#[derive(Clone, Debug)]
pub struct TimelineEntry {
    pub message_id: String,
    pub role: String,
    pub preview: String,
    pub timestamp: String,
}

/// Dialog that lists session messages in chronological order for quick navigation.
pub struct TimelineDialog {
    entries: Vec<TimelineEntry>,
    state: ListState,
    open: bool,
}

impl TimelineDialog {
    /// Creates a new, closed timeline dialog with no entries.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            state: ListState::default(),
            open: false,
        }
    }

    /// Opens the dialog populated with the given entries.
    pub fn open(&mut self, entries: Vec<TimelineEntry>) {
        self.entries = entries;
        self.state.select(if self.entries.is_empty() {
            None
        } else {
            Some(0)
        });
        self.open = true;
    }

    /// Closes the dialog.
    pub fn close(&mut self) {
        self.open = false;
    }

    /// Returns `true` if the dialog is currently visible.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Moves the selection one row up.
    pub fn move_up(&mut self) {
        if let Some(i) = self.state.selected() {
            if i > 0 {
                self.state.select(Some(i - 1));
            }
        }
    }

    /// Moves the selection one row down.
    pub fn move_down(&mut self) {
        if let Some(i) = self.state.selected() {
            if i < self.entries.len().saturating_sub(1) {
                self.state.select(Some(i + 1));
            }
        }
    }

    /// Returns a reference to the currently highlighted entry, if any.
    pub fn selected_entry(&self) -> Option<&TimelineEntry> {
        self.state.selected().and_then(|i| self.entries.get(i))
    }

    /// Returns the message ID of the currently highlighted entry, if any.
    pub fn selected_message_id(&self) -> Option<&str> {
        self.selected_entry().map(|e| e.message_id.as_str())
    }

    /// Renders the dialog into `frame` if it is open.
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if !self.open {
            return;
        }
        let dialog_width = 70u16.min(area.width.saturating_sub(4));
        let dialog_height = 20u16.min(area.height.saturating_sub(4));
        let dialog_area = centered_rect(dialog_width, dialog_height, area);
        frame.render_widget(Clear, dialog_area);
        let block = Block::default()
            .title(Span::styled(
                " Timeline ",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .style(Style::default().bg(theme.background_panel));
        let inner = super::dialog_inner(block.inner(dialog_area));
        frame.render_widget(block, dialog_area);
        if self.entries.is_empty() {
            let empty = ratatui::widgets::Paragraph::new("No messages in timeline")
                .style(Style::default().fg(theme.text_muted));
            frame.render_widget(empty, inner);
            return;
        }
        let items: Vec<ListItem> = self
            .entries
            .iter()
            .map(|entry| {
                let role_icon = match entry.role.as_str() {
                    "user" => "[U]",
                    "assistant" => "[A]",
                    _ => "[S]",
                };
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{} ", role_icon),
                        Style::default().fg(theme.primary),
                    ),
                    Span::styled(&entry.preview, Style::default().fg(theme.text)),
                    Span::styled(
                        format!("  {}", entry.timestamp),
                        Style::default().fg(theme.text_muted),
                    ),
                ]))
            })
            .collect();
        let list = List::new(items).highlight_style(
            Style::default()
                .bg(theme.background_element)
                .add_modifier(Modifier::BOLD),
        );
        frame.render_stateful_widget(list, inner, &mut self.state.clone());
    }
}

impl Default for TimelineDialog {
    fn default() -> Self {
        Self::new()
    }
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    super::centered_rect(width, height, area)
}

