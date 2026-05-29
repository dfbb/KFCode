//! Single todo item component with status icon rendering.
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::context::TodoStatus;

/// A single todo entry with its text and current status.
pub struct TodoItem {
    /// Display text of the todo entry.
    pub content: String,
    /// Current lifecycle status.
    pub status: TodoStatus,
}

impl TodoItem {
    /// Create a new todo item with the given text and status.
    pub fn new(content: &str, status: TodoStatus) -> Self {
        Self {
            content: content.to_string(),
            status,
        }
    }

    /// Render the todo item with a status icon into the given frame area.
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let (icon, color) = match &self.status {
            TodoStatus::Pending => ("○", ratatui::style::Color::Gray),
            TodoStatus::InProgress => ("◐", ratatui::style::Color::Yellow),
            TodoStatus::Completed => ("●", ratatui::style::Color::Green),
            TodoStatus::Cancelled => ("○", ratatui::style::Color::DarkGray),
        };

        let line = Line::from(vec![
            Span::styled(icon, Style::default().fg(color)),
            Span::raw(" "),
            Span::styled(&self.content, Style::default()),
        ]);

        let paragraph = Paragraph::new(line);
        frame.render_widget(paragraph, area);
    }
}
