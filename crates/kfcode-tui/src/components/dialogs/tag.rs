//! Dialog for selecting one or more tags to apply to a session.

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use crate::theme::Theme;

/// Metadata for a single tag.
#[derive(Clone, Debug)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
}

/// Dialog that shows a checkable list of tags.
pub struct TagDialog {
    pub tags: Vec<Tag>,
    pub selected_tags: Vec<String>,
    pub state: ListState,
    pub open: bool,
}

impl TagDialog {
    /// Creates a new, closed tag dialog with no tags loaded.
    pub fn new() -> Self {
        Self {
            tags: Vec::new(),
            selected_tags: Vec::new(),
            state: ListState::default(),
            open: false,
        }
    }

    /// Opens the dialog and selects the first tag.
    pub fn open(&mut self) {
        self.open = true;
        self.state.select(Some(0));
    }

    /// Closes the dialog.
    pub fn close(&mut self) {
        self.open = false;
    }

    /// Returns `true` if the dialog is currently visible.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Replaces the tag list.
    pub fn set_tags(&mut self, tags: Vec<Tag>) {
        self.tags = tags;
    }

    /// Toggles the checked state of the currently highlighted tag.
    pub fn toggle_selection(&mut self) {
        if let Some(selected) = self.state.selected() {
            if let Some(tag) = self.tags.get(selected) {
                if self.selected_tags.contains(&tag.id) {
                    self.selected_tags.retain(|id| id != &tag.id);
                } else {
                    self.selected_tags.push(tag.id.clone());
                }
            }
        }
    }

    /// Moves the selection one row up.
    pub fn move_up(&mut self) {
        if let Some(selected) = self.state.selected() {
            let new = selected.saturating_sub(1);
            self.state.select(Some(new));
        }
    }

    /// Moves the selection one row down.
    pub fn move_down(&mut self) {
        if let Some(selected) = self.state.selected() {
            let new = (selected + 1).min(self.tags.len().saturating_sub(1));
            self.state.select(Some(new));
        }
    }

    /// Returns the IDs of all currently checked tags.
    pub fn selected_tags(&self) -> &[String] {
        &self.selected_tags
    }

    /// Renders the dialog into `frame` if it is open and the tag list is non-empty.
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if !self.open || self.tags.is_empty() {
            return;
        }

        let height = (self.tags.len() as u16 + 2).min(15);
        let width = 40u16;
        let popup_area = super::centered_rect(width, height, area);
        let block = Block::default()
            .title(" Select Tags ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .style(Style::default().bg(theme.background_panel));
        let content_area = super::dialog_inner(block.inner(popup_area));

        let items: Vec<ListItem> = self
            .tags
            .iter()
            .map(|tag| {
                let is_checked = self.selected_tags.contains(&tag.id);
                let check_mark = if is_checked { "☑" } else { "☐" };
                ListItem::new(Line::from(vec![
                    Span::styled(check_mark, Style::default().fg(theme.primary)),
                    Span::styled(format!(" {}", tag.name), Style::default().fg(theme.text)),
                ]))
            })
            .collect();

        frame.render_widget(block, popup_area);

        let list = List::new(items).highlight_style(Style::default().fg(theme.primary));

        frame.render_widget(list, content_area);
    }
}

impl Default for TagDialog {
    fn default() -> Self {
        Self::new()
    }
}
