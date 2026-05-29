//! Specialized view components for individual tool types.
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::diff::DiffView;
use super::tool_call::ToolCallStatus;
use crate::context::TodoStatus;
use crate::theme::Theme;

/// View for a glob file-search tool call.
pub struct GlobToolView {
    /// The glob pattern that was searched.
    pub pattern: String,
    /// Matched file paths, once the tool has returned.
    pub matches: Option<Vec<String>>,
}

impl GlobToolView {
    /// Create a new glob view for the given pattern.
    pub fn new(pattern: String) -> Self {
        Self {
            pattern,
            matches: None,
        }
    }

    /// Render the glob view into the given frame area.
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let count = self.matches.as_ref().map(|m| m.len()).unwrap_or(0);

        let line = Line::from(vec![
            Span::styled("glob ", Style::default().fg(theme.primary)),
            Span::styled(&self.pattern, Style::default().fg(theme.text)),
            Span::styled(
                format!(" ({} matches)", count),
                Style::default().fg(theme.text_muted),
            ),
        ]);

        let paragraph = Paragraph::new(line);
        frame.render_widget(paragraph, area);
    }
}

/// View for a grep text-search tool call.
pub struct GrepToolView {
    /// The regex or literal pattern that was searched.
    pub pattern: String,
    /// Optional path scope for the search.
    pub path: Option<String>,
    /// Number of matches found, once the tool has returned.
    pub matches: Option<u32>,
}

impl GrepToolView {
    /// Create a new grep view for the given pattern.
    pub fn new(pattern: String) -> Self {
        Self {
            pattern,
            path: None,
            matches: None,
        }
    }

    /// Render the grep view into the given frame area.
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let count = self.matches.unwrap_or(0);

        let line = Line::from(vec![
            Span::styled("grep ", Style::default().fg(theme.primary)),
            Span::styled(&self.pattern, Style::default().fg(theme.text)),
            if let Some(p) = &self.path {
                Span::styled(format!(" in {}", p), Style::default().fg(theme.text_muted))
            } else {
                Span::raw("")
            },
            Span::styled(
                format!(" ({} matches)", count),
                Style::default().fg(theme.text_muted),
            ),
        ]);

        let paragraph = Paragraph::new(line);
        frame.render_widget(paragraph, area);
    }
}

/// View for a directory listing tool call.
pub struct ListToolView {
    /// The directory path that was listed.
    pub path: String,
}

impl ListToolView {
    /// Create a new list view for the given path.
    pub fn new(path: String) -> Self {
        Self { path }
    }

    /// Render the list view into the given frame area.
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let line = Line::from(vec![
            Span::styled("list ", Style::default().fg(theme.primary)),
            Span::styled(&self.path, Style::default().fg(theme.text)),
        ]);

        let paragraph = Paragraph::new(line);
        frame.render_widget(paragraph, area);
    }
}

/// View for a web-fetch tool call.
pub struct WebfetchToolView {
    /// The URL that was fetched.
    pub url: String,
}

impl WebfetchToolView {
    /// Create a new webfetch view for the given URL.
    pub fn new(url: String) -> Self {
        Self { url }
    }

    /// Render the webfetch view into the given frame area, truncating long URLs.
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let display_url = if self.url.chars().count() > 50 {
            format!("{}...", self.url.chars().take(47).collect::<String>())
        } else {
            self.url.clone()
        };

        let line = Line::from(vec![
            Span::styled("webfetch ", Style::default().fg(theme.primary)),
            Span::styled(display_url, Style::default().fg(theme.text)),
        ]);

        let paragraph = Paragraph::new(line);
        frame.render_widget(paragraph, area);
    }
}

/// View for a web-search tool call.
pub struct WebsearchToolView {
    /// The search query string.
    pub query: String,
    /// Number of results returned, once the tool has finished.
    pub results: Option<u32>,
}

impl WebsearchToolView {
    /// Create a new websearch view for the given query.
    pub fn new(query: String) -> Self {
        Self {
            query,
            results: None,
        }
    }

    /// Render the websearch view into the given frame area.
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let count = self.results.unwrap_or(0);

        let line = Line::from(vec![
            Span::styled("websearch ", Style::default().fg(theme.primary)),
            Span::styled(&self.query, Style::default().fg(theme.text)),
            Span::styled(
                format!(" ({} results)", count),
                Style::default().fg(theme.text_muted),
            ),
        ]);

        let paragraph = Paragraph::new(line);
        frame.render_widget(paragraph, area);
    }
}

/// View for a long-running task tool call.
pub struct TaskToolView {
    /// Human-readable name of the task.
    pub task_name: String,
    /// Optional category label for the task.
    pub category: Option<String>,
    /// Current execution status.
    pub status: ToolCallStatus,
}

impl TaskToolView {
    /// Create a new task view with the given name, defaulting to running status.
    pub fn new(task_name: String) -> Self {
        Self {
            task_name,
            category: None,
            status: ToolCallStatus::Running,
        }
    }

    /// Render the task view into the given frame area.
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let status_icon = match self.status {
            ToolCallStatus::Pending => "◯",
            ToolCallStatus::Running => "◐",
            ToolCallStatus::Completed => "●",
            ToolCallStatus::Failed => "✗",
        };

        let status_color = match self.status {
            ToolCallStatus::Pending => theme.text_muted,
            ToolCallStatus::Running => theme.warning,
            ToolCallStatus::Completed => theme.success,
            ToolCallStatus::Failed => theme.error,
        };

        let lines = vec![Line::from(vec![
            Span::styled(status_icon, Style::default().fg(status_color)),
            Span::raw(" "),
            Span::styled("task ", Style::default().fg(theme.primary)),
            Span::styled(
                &self.task_name,
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
            if let Some(cat) = &self.category {
                Span::styled(format!(" [{}]", cat), Style::default().fg(theme.text_muted))
            } else {
                Span::raw("")
            },
        ])];

        let paragraph = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::LEFT)
                .border_style(Style::default().fg(status_color)),
        );

        frame.render_widget(paragraph, area);
    }
}

/// View for a skill invocation tool call.
pub struct SkillToolView {
    /// Name of the skill that was invoked.
    pub skill_name: String,
}

impl SkillToolView {
    /// Create a new skill view for the given skill name.
    pub fn new(skill_name: String) -> Self {
        Self { skill_name }
    }

    /// Render the skill view into the given frame area.
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let line = Line::from(vec![
            Span::styled("skill ", Style::default().fg(theme.primary)),
            Span::styled(&self.skill_name, Style::default().fg(theme.text)),
        ]);

        let paragraph = Paragraph::new(line);
        frame.render_widget(paragraph, area);
    }
}

/// View for a file edit tool call, showing the diff and any diagnostics.
pub struct EditToolView {
    /// Path of the file being edited.
    pub file_path: String,
    /// Unified diff content for the edit.
    pub diff_content: String,
    /// Diagnostic messages produced after the edit.
    pub diagnostics: Vec<String>,
}

impl EditToolView {
    /// Create a new edit view for the given file path and diff.
    pub fn new(file_path: String, diff_content: String) -> Self {
        Self {
            file_path,
            diff_content,
            diagnostics: Vec::new(),
        }
    }

    /// Render the edit view into the given frame area.
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let mut lines = vec![Line::from(vec![
            Span::styled(
                "edit ",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(&self.file_path, Style::default().fg(theme.text)),
        ])];

        // Render diff using DiffView
        if !self.diff_content.is_empty() {
            let diff_view = DiffView::new().with_content(&self.diff_content);
            lines.extend(diff_view.to_lines(theme));
        }

        // Render diagnostics if any
        if !self.diagnostics.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Diagnostics:",
                Style::default()
                    .fg(theme.warning)
                    .add_modifier(Modifier::BOLD),
            )));
            for diag in &self.diagnostics {
                lines.push(Line::from(Span::styled(
                    format!("  {}", diag),
                    Style::default().fg(theme.warning),
                )));
            }
        }

        let paragraph = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::LEFT)
                .border_style(Style::default().fg(theme.info)),
        );

        frame.render_widget(paragraph, area);
    }
}

/// View for an apply-patch tool call that may touch multiple files.
pub struct ApplyPatchToolView {
    /// File paths extracted from the diff headers.
    pub files: Vec<String>,
    /// Full unified diff content.
    pub diff_content: String,
}

impl ApplyPatchToolView {
    /// Create a new apply-patch view, extracting affected file paths from the diff.
    pub fn new(diff_content: String) -> Self {
        // Extract file paths from diff headers
        let files: Vec<String> = diff_content
            .lines()
            .filter_map(|line| {
                line.strip_prefix("+++ b/")
                    .or_else(|| line.strip_prefix("+++ "))
                    .map(|s| s.to_string())
            })
            .collect();

        Self {
            files,
            diff_content,
        }
    }

    /// Render the apply-patch view into the given frame area.
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let mut lines = vec![Line::from(vec![
            Span::styled(
                "apply_patch ",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("({} files)", self.files.len()),
                Style::default().fg(theme.text_muted),
            ),
        ])];

        // List affected files
        for file in &self.files {
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(file.as_str(), Style::default().fg(theme.info)),
            ]));
        }

        // Render diff
        if !self.diff_content.is_empty() {
            lines.push(Line::from(""));
            let diff_view = DiffView::new().with_content(&self.diff_content);
            lines.extend(diff_view.to_lines(theme));
        }

        let paragraph = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::LEFT)
                .border_style(Style::default().fg(theme.info)),
        );

        frame.render_widget(paragraph, area);
    }
}

/// View for a todo-write tool call that displays the full todo list.
pub struct TodoWriteToolView {
    /// List of (content, status) pairs representing each todo entry.
    pub items: Vec<(String, TodoStatus)>,
}

impl TodoWriteToolView {
    /// Create a new todo-write view from a list of items.
    pub fn new(items: Vec<(String, TodoStatus)>) -> Self {
        Self { items }
    }

    /// Render the todo-write view into the given frame area.
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let mut lines = vec![Line::from(Span::styled(
            "Todo List",
            Style::default()
                .fg(theme.primary)
                .add_modifier(Modifier::BOLD),
        ))];

        for (content, status) in &self.items {
            let (icon, color) = match status {
                TodoStatus::Pending => ("○", theme.text_muted),
                TodoStatus::InProgress => ("◐", theme.warning),
                TodoStatus::Completed => ("●", theme.success),
                TodoStatus::Cancelled => ("○", theme.text_muted),
            };

            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", icon), Style::default().fg(color)),
                Span::styled(content.as_str(), Style::default().fg(theme.text)),
            ]));
        }

        let paragraph = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::LEFT)
                .border_style(Style::default().fg(theme.primary)),
        );

        frame.render_widget(paragraph, area);
    }
}
