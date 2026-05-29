//! Slash-command palette popup for browsing and selecting commands.

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use crate::command::{CommandAction, CommandRegistry};
use crate::theme::Theme;

/// Floating popup that lists and filters slash commands.
pub struct SlashCommandPopup {
    pub registry: CommandRegistry,
    pub query: String,
    pub filtered: Vec<String>,
    pub state: ListState,
    pub open: bool,
    pub selected_action: Option<CommandAction>,
}

impl SlashCommandPopup {
    /// Create a new slash command popup.
    pub fn new() -> Self {
        Self {
            registry: CommandRegistry::new(),
            query: String::new(),
            filtered: Vec::new(),
            state: ListState::default(),
            open: false,
            selected_action: None,
        }
    }

    /// Open the popup and reset the query.
    pub fn open(&mut self) {
        self.query = String::new();
        self.refresh_filter();
        self.state.select(Some(0));
        self.open = true;
        self.selected_action = None;
    }

    /// Close the popup and clear state.
    pub fn close(&mut self) {
        self.open = false;
        self.query.clear();
        self.filtered.clear();
    }

    /// Returns true if the popup is currently open.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Consume and return the action selected by the user, if any.
    pub fn take_action(&mut self) -> Option<CommandAction> {
        self.selected_action.take()
    }

    /// Return the current search query.
    pub fn query(&self) -> &str {
        &self.query
    }

    fn refresh_filter(&mut self) {
        if self.query.is_empty() {
            self.filtered = self
                .registry
                .suggested_commands()
                .iter()
                .map(|cmd| cmd.name.clone())
                .collect();
        } else {
            self.filtered = self
                .registry
                .search(&self.query)
                .iter()
                .map(|cmd| cmd.name.clone())
                .collect();
        }
        self.state.select(Some(0));
    }

    /// Append a character to the query and refresh the filtered list.
    pub fn handle_input(&mut self, c: char) {
        self.query.push(c);
        self.refresh_filter();
    }

    /// Remove the last character from the query; returns false if the query was already empty.
    pub fn handle_backspace(&mut self) -> bool {
        if self.query.pop().is_some() {
            self.refresh_filter();
            true
        } else {
            false
        }
    }

    /// Move the selection cursor up.
    pub fn move_up(&mut self) {
        if let Some(selected) = self.state.selected() {
            let new = selected.saturating_sub(1);
            self.state.select(Some(new));
        }
    }

    /// Move the selection cursor down.
    pub fn move_down(&mut self) {
        if let Some(selected) = self.state.selected() {
            let new = (selected + 1).min(self.filtered.len().saturating_sub(1));
            self.state.select(Some(new));
        }
    }

    /// Confirm the currently highlighted command and store its action.
    pub fn select_current(&mut self) {
        if let Some(idx) = self.state.selected() {
            if let Some(name) = self.filtered.get(idx) {
                if let Some(cmd) = self.registry.get(name) {
                    self.selected_action = Some(cmd.action.clone());
                    self.close();
                }
            }
        }
    }

    /// Render the popup above the given area.
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if !self.open || self.filtered.is_empty() {
            return;
        }

        let width = 50.min(area.width.saturating_sub(4));
        let height = (10.min(self.filtered.len()) as u16).saturating_add(2);

        let x = area.x + (area.width - width) / 2;
        let y = area.y.saturating_sub(height + 1);

        let popup_area = Rect::new(x.max(1), y.max(1), width, height);

        let query_line = Line::from(vec![
            Span::raw("/"),
            Span::styled(&self.query, Style::default().fg(theme.primary)),
        ]);

        let items: Vec<ListItem> = self
            .filtered
            .iter()
            .enumerate()
            .map(|(idx, name)| {
                let cmd = self.registry.get(name);
                let title = cmd.map(|c| c.title.as_str()).unwrap_or(name);
                let _desc = cmd.map(|c| c.description.as_str()).unwrap_or("");
                let keybind = cmd.and_then(|c| c.keybind.clone());

                let is_selected = self.state.selected() == Some(idx);
                let style = if is_selected {
                    Style::default()
                        .fg(theme.primary)
                        .bg(theme.background_element)
                } else {
                    Style::default().fg(theme.text)
                };

                let content = if let Some(kb) = keybind {
                    Line::from(vec![
                        Span::styled(title, style),
                        Span::styled(format!("  ({})", kb), Style::default().fg(theme.text_muted)),
                    ])
                } else {
                    Line::from(Span::styled(title, style))
                };

                ListItem::new(content)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title(query_line)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.border)),
            )
            .highlight_style(Style::default().fg(theme.primary));

        frame.render_widget(list, popup_area);
    }
}

impl Default for SlashCommandPopup {
    fn default() -> Self {
        Self::new()
    }
}
