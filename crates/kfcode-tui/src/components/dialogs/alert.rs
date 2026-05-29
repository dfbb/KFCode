//! Non-blocking alert dialog for displaying informational, success, warning, or error messages.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::theme::Theme;

/// Severity level of an alert.
pub enum AlertType {
    /// Neutral informational message.
    Info,
    /// Operation completed successfully.
    Success,
    /// Non-fatal warning that requires attention.
    Warning,
    /// Error that the user should act on.
    Error,
}

/// Dialog that displays a titled message with a severity-colored border.
pub struct AlertDialog {
    title: String,
    message: String,
    alert_type: AlertType,
    open: bool,
}

impl AlertDialog {
    /// Creates a new alert with the given title, message, and severity.
    pub fn new(title: &str, message: &str, alert_type: AlertType) -> Self {
        Self {
            title: title.to_string(),
            message: message.to_string(),
            alert_type,
            open: false,
        }
    }

    /// Creates an info-level alert.
    pub fn info(message: &str) -> Self {
        Self::new("Info", message, AlertType::Info)
    }

    /// Creates a success-level alert.
    pub fn success(message: &str) -> Self {
        Self::new("Success", message, AlertType::Success)
    }

    /// Creates a warning-level alert.
    pub fn warning(message: &str) -> Self {
        Self::new("Warning", message, AlertType::Warning)
    }

    /// Creates an error-level alert.
    pub fn error(message: &str) -> Self {
        Self::new("Error", message, AlertType::Error)
    }

    /// Makes the dialog visible.
    pub fn open(&mut self) {
        self.open = true;
    }

    /// Hides the dialog.
    pub fn close(&mut self) {
        self.open = false;
    }

    /// Returns `true` if the dialog is currently visible.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Replaces the displayed message text.
    pub fn set_message(&mut self, message: &str) {
        self.message = message.to_string();
    }

    /// Returns the current message text.
    pub fn message(&self) -> &str {
        &self.message
    }

    fn get_color(&self, theme: &Theme) -> ratatui::style::Color {
        match self.alert_type {
            AlertType::Info => theme.info,
            AlertType::Success => theme.success,
            AlertType::Warning => theme.warning,
            AlertType::Error => theme.error,
        }
    }

    fn get_icon(&self) -> &str {
        match self.alert_type {
            AlertType::Info => "ℹ",
            AlertType::Success => "✓",
            AlertType::Warning => "⚠",
            AlertType::Error => "✗",
        }
    }

    /// Renders the dialog into `frame` if it is open.
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if !self.open {
            return;
        }

        let color = self.get_color(theme);
        let icon = self.get_icon();

        let line_count = self.message.lines().count().max(1);
        let dialog_width = 50.min(area.width.saturating_sub(4));
        let dialog_height = (line_count + 6).min(16) as u16;

        let dialog_area = centered_rect(dialog_width, dialog_height, area);

        frame.render_widget(Clear, dialog_area);

        let block = Block::default()
            .title(Span::styled(
                format!(" {} {} ", icon, self.title),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(color))
            .style(Style::default().bg(theme.background_panel));

        let inner = super::dialog_inner(block.inner(dialog_area));
        frame.render_widget(block, dialog_area);

        let paragraph = Paragraph::new(self.message.clone())
            .style(Style::default().fg(theme.text))
            .wrap(Wrap { trim: false })
            .centered();

        frame.render_widget(
            paragraph,
            Rect {
                x: inner.x,
                y: inner.y + 1,
                width: inner.width,
                height: inner.height.saturating_sub(4),
            },
        );

        let hint = Line::from(vec![
            Span::styled("[Enter] ", Style::default().fg(theme.text_muted)),
            Span::styled("Close", Style::default().fg(theme.text)),
            Span::styled("  [Ctrl+C] ", Style::default().fg(theme.text_muted)),
            Span::styled("Copy", Style::default().fg(theme.text)),
        ]);
        frame.render_widget(
            Paragraph::new(hint).centered(),
            Rect {
                x: inner.x,
                y: inner.y + inner.height.saturating_sub(3),
                width: inner.width,
                height: 1,
            },
        );

        let ok_button = Line::from(vec![Span::styled(
            " [ OK ] ",
            Style::default().fg(theme.text).bg(color),
        )]);

        frame.render_widget(
            Paragraph::new(ok_button).centered(),
            Rect {
                x: inner.x,
                y: inner.y + inner.height.saturating_sub(2),
                width: inner.width,
                height: 1,
            },
        );
    }
}

impl Default for AlertDialog {
    fn default() -> Self {
        Self::info("Message")
    }
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    super::centered_rect(width, height, area)
}

