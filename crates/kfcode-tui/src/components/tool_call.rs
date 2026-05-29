//! Tool call data types and generic tool call / result views.
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::theme::Theme;

/// Controls how a tool call is laid out in the message stream.
#[derive(Clone, Debug, PartialEq)]
pub enum ToolRenderMode {
    /// Compact single-line display alongside surrounding text.
    Inline,
    /// Full-width block with a left border.
    Block,
}

/// A single tool invocation with its arguments and current execution status.
#[derive(Clone, Debug)]
pub struct ToolCall {
    /// Unique identifier for this invocation.
    pub id: String,
    /// Name of the tool being called.
    pub name: String,
    /// Raw JSON arguments string.
    pub arguments: String,
    /// Current execution status.
    pub status: ToolCallStatus,
}

impl ToolCall {
    /// Return the preferred render mode for this tool by name.
    pub fn render_mode(&self) -> ToolRenderMode {
        match self.name.as_str() {
            "glob" | "grep" | "list" | "webfetch" | "websearch" | "skill" | "read" => {
                ToolRenderMode::Inline
            }
            "bash" | "write" | "edit" | "apply_patch" | "task" | "todowrite" | "question" => {
                ToolRenderMode::Block
            }
            _ => ToolRenderMode::Block,
        }
    }
}

/// Lifecycle state of a tool invocation.
#[derive(Clone, Debug, PartialEq)]
pub enum ToolCallStatus {
    /// Queued but not yet started.
    Pending,
    /// Currently executing.
    Running,
    /// Finished successfully.
    Completed,
    /// Finished with an error.
    Failed,
}

/// Generic view for a tool call that has no dedicated specialized view.
pub struct ToolCallView {
    tool_call: ToolCall,
}

impl ToolCallView {
    /// Wrap a `ToolCall` for rendering.
    pub fn new(tool_call: ToolCall) -> Self {
        Self { tool_call }
    }

    /// Render the tool call block into the given frame area.
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let status_icon = match self.tool_call.status {
            ToolCallStatus::Pending => "◯",
            ToolCallStatus::Running => "◐",
            ToolCallStatus::Completed => "●",
            ToolCallStatus::Failed => "✗",
        };

        let status_color = match self.tool_call.status {
            ToolCallStatus::Pending => theme.text_muted,
            ToolCallStatus::Running => theme.warning,
            ToolCallStatus::Completed => theme.success,
            ToolCallStatus::Failed => theme.error,
        };

        let title_line = Line::from(vec![
            Span::styled(status_icon, Style::default().fg(status_color)),
            Span::raw(" "),
            Span::styled(
                &self.tool_call.name,
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                match self.tool_call.status {
                    ToolCallStatus::Pending => " (pending)",
                    ToolCallStatus::Running => " (running...)",
                    ToolCallStatus::Completed => " (completed)",
                    ToolCallStatus::Failed => " (failed)",
                },
                Style::default().fg(theme.text_muted),
            ),
        ]);

        let mut lines = vec![title_line];

        if !self.tool_call.arguments.is_empty() {
            lines.push(Line::from(""));
            for line in self.tool_call.arguments.lines().take(5) {
                lines.push(Line::from(Span::styled(
                    format!("  {}", line),
                    Style::default().fg(theme.text_muted),
                )));
            }
            if self.tool_call.arguments.lines().count() > 5 {
                lines.push(Line::from(Span::styled(
                    "  ...",
                    Style::default().fg(theme.text_muted),
                )));
            }
        }

        let paragraph = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::LEFT)
                .border_style(Style::default().fg(status_color)),
        );

        frame.render_widget(paragraph, area);
    }
}

/// Displays the result returned by a tool call.
pub struct ToolResultView {
    /// Name of the tool that produced this result.
    pub tool_name: String,
    /// Raw result text.
    pub result: String,
    /// Whether the tool reported an error.
    pub is_error: bool,
    /// Whether the result was truncated for display.
    pub truncated: bool,
}

impl ToolResultView {
    /// Create a result view, automatically setting `truncated` when the result exceeds 20 lines.
    pub fn new(tool_name: String, result: String, is_error: bool) -> Self {
        let line_count = result.lines().count();
        Self {
            tool_name,
            result,
            is_error,
            truncated: line_count > 20,
        }
    }

    /// Render the tool result block into the given frame area.
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let icon = if self.is_error { "✗" } else { "✓" };
        let color = if self.is_error {
            theme.error
        } else {
            theme.success
        };

        let display_result = if self.truncated {
            self.result.lines().take(20).collect::<Vec<_>>().join("\n")
        } else {
            self.result.clone()
        };

        let mut lines = vec![Line::from(vec![
            Span::styled(icon, Style::default().fg(color)),
            Span::raw(" "),
            Span::styled(
                &self.tool_name,
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" result", Style::default().fg(theme.text_muted)),
        ])];

        lines.push(Line::from(""));

        for line in display_result.lines() {
            let styled_line = if self.is_error {
                Span::styled(line.to_string(), Style::default().fg(theme.error))
            } else {
                Span::styled(line.to_string(), Style::default().fg(theme.text))
            };
            lines.push(Line::from(styled_line));
        }

        if self.truncated {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "... (truncated)",
                Style::default().fg(theme.text_muted),
            )));
        }

        let paragraph = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::LEFT)
                .border_style(Style::default().fg(color)),
        );

        frame.render_widget(paragraph, area);
    }
}

/// Displays a bash command invocation and its output.
pub struct BashToolView {
    /// The shell command that was executed.
    pub command: String,
    /// Captured stdout/stderr output, if available.
    pub output: Option<String>,
    /// Process exit code, if the command has finished.
    pub exit_code: Option<i32>,
    /// Whether the command is still running.
    pub running: bool,
}

impl BashToolView {
    /// Create a new bash view for the given command.
    pub fn new(command: String) -> Self {
        Self {
            command,
            output: None,
            exit_code: None,
            running: false,
        }
    }

    /// Render the bash tool view into the given frame area.
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let status_color = if self.running {
            theme.warning
        } else if self.exit_code.map_or(false, |c| c != 0) {
            theme.error
        } else {
            theme.success
        };

        let mut lines = vec![Line::from(vec![
            Span::styled(
                "$ ",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(&self.command, Style::default().fg(theme.text)),
        ])];

        if let Some(ref output) = self.output {
            lines.push(Line::from(""));
            for line in output.lines().take(15) {
                lines.push(Line::from(Span::styled(
                    line.to_string(),
                    Style::default().fg(theme.text),
                )));
            }
            if output.lines().count() > 15 {
                lines.push(Line::from(Span::styled(
                    "... (output truncated)",
                    Style::default().fg(theme.text_muted),
                )));
            }
        }

        if let Some(code) = self.exit_code {
            lines.push(Line::from(""));
            let status_text = if code == 0 {
                format!("Exit code: {}", code)
            } else {
                format!("Exit code: {} (failed)", code)
            };
            lines.push(Line::from(Span::styled(
                status_text,
                Style::default().fg(status_color),
            )));
        }

        let paragraph = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::LEFT)
                .border_style(Style::default().fg(status_color)),
        );

        frame.render_widget(paragraph, area);
    }
}

/// Displays a file read operation with optional line-range and content preview.
pub struct ReadToolView {
    /// Path of the file being read.
    pub file_path: String,
    /// File content, if already loaded.
    pub content: Option<String>,
    /// Optional line range (start, end) that was requested.
    pub line_range: Option<(usize, usize)>,
}

impl ReadToolView {
    /// Create a new read view for the given file path.
    pub fn new(file_path: String) -> Self {
        Self {
            file_path,
            content: None,
            line_range: None,
        }
    }

    /// Render the read tool view into the given frame area.
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let mut lines = vec![Line::from(vec![
            Span::styled("📖 ", Style::default()),
            Span::styled(&self.file_path, Style::default().fg(theme.primary)),
            if let Some((start, end)) = self.line_range {
                Span::styled(
                    format!(" (lines {}-{})", start, end),
                    Style::default().fg(theme.text_muted),
                )
            } else {
                Span::raw("")
            },
        ])];

        if let Some(ref content) = self.content {
            lines.push(Line::from(""));
            for (i, line) in content.lines().take(20).enumerate() {
                let line_num = if let Some((start, _)) = self.line_range {
                    start + i
                } else {
                    i + 1
                };
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("{:4} ", line_num),
                        Style::default().fg(theme.text_muted),
                    ),
                    Span::styled(line.to_string(), Style::default().fg(theme.text)),
                ]));
            }
            if content.lines().count() > 20 {
                lines.push(Line::from(Span::styled(
                    "... (content truncated)",
                    Style::default().fg(theme.text_muted),
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

/// Displays a file write operation with an optional content preview.
pub struct WriteToolView {
    /// Path of the file being written.
    pub file_path: String,
    /// Short preview of the content being written.
    pub content_preview: Option<String>,
    /// Whether the write has completed.
    pub written: bool,
}

impl WriteToolView {
    /// Create a new write view for the given file path.
    pub fn new(file_path: String) -> Self {
        Self {
            file_path,
            content_preview: None,
            written: false,
        }
    }

    /// Render the write tool view into the given frame area.
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let status_icon = if self.written { "✓" } else { "..." };
        let status_color = if self.written {
            theme.success
        } else {
            theme.text_muted
        };

        let mut lines = vec![Line::from(vec![
            Span::styled(status_icon, Style::default().fg(status_color)),
            Span::raw(" "),
            Span::styled(&self.file_path, Style::default().fg(theme.primary)),
        ])];

        if let Some(ref preview) = self.content_preview {
            lines.push(Line::from(""));
            for line in preview.lines().take(5) {
                lines.push(Line::from(Span::styled(
                    format!("  {}", line),
                    Style::default().fg(theme.text_muted),
                )));
            }
        }

        let paragraph = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::LEFT)
                .border_style(Style::default().fg(status_color)),
        );

        frame.render_widget(paragraph, area);
    }
}
