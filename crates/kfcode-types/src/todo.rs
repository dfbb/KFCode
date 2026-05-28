//! Todo item types and helpers for tracking tasks within a session.
//!
//! [`TodoItem`] is the persisted record; [`TodoStatus`] and [`TodoPriority`] are
//! typed enums with string-parsing helpers for use with external data sources.
use serde::{Deserialize, Serialize};

/// A lightweight todo descriptor without session binding, used for creation requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoInfo {
    /// Human-readable description of the task.
    pub content: String,
    /// Current status string (e.g. `"pending"`, `"completed"`).
    pub status: String,
    /// Priority string (e.g. `"high"`, `"medium"`, `"low"`).
    pub priority: String,
}

/// A persisted todo item associated with a specific session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    /// Identifier of the session this todo belongs to.
    pub session_id: String,
    /// Human-readable description of the task.
    pub content: String,
    /// Current status string (e.g. `"pending"`, `"completed"`).
    pub status: String,
    /// Priority string (e.g. `"high"`, `"medium"`, `"low"`).
    pub priority: String,
    /// Zero-based display order within the session's todo list.
    pub position: u32,
}

/// Lifecycle state of a todo item.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TodoStatus {
    /// The task has not been started.
    Pending,
    /// The task is currently being worked on.
    InProgress,
    /// The task has been finished successfully.
    Completed,
    /// The task was abandoned without completion.
    Cancelled,
}

impl TodoStatus {
    /// Returns the canonical lowercase string representation of this status.
    pub fn as_str(&self) -> &'static str {
        match self {
            TodoStatus::Pending => "pending",
            TodoStatus::InProgress => "in_progress",
            TodoStatus::Completed => "completed",
            TodoStatus::Cancelled => "cancelled",
        }
    }
}

impl std::fmt::Display for TodoStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Importance level of a todo item.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TodoPriority {
    /// Must be addressed before lower-priority items.
    High,
    /// Normal importance; the default when priority is unspecified.
    Medium,
    /// Can be deferred in favour of higher-priority work.
    Low,
}

impl TodoPriority {
    /// Returns the canonical lowercase string representation of this priority.
    pub fn as_str(&self) -> &'static str {
        match self {
            TodoPriority::High => "high",
            TodoPriority::Medium => "medium",
            TodoPriority::Low => "low",
        }
    }
}

impl std::fmt::Display for TodoPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Parses a status string into a [`TodoStatus`], defaulting to `Pending` for unknown values.
pub fn parse_status(status: &str) -> TodoStatus {
    match status.to_lowercase().as_str() {
        "pending" => TodoStatus::Pending,
        "in_progress" | "in progress" => TodoStatus::InProgress,
        "completed" => TodoStatus::Completed,
        "cancelled" => TodoStatus::Cancelled,
        // Unknown strings are treated as pending rather than failing.
        _ => TodoStatus::Pending,
    }
}

/// Parses a priority string into a [`TodoPriority`], defaulting to `Medium` for unknown values.
pub fn parse_priority(priority: &str) -> TodoPriority {
    match priority.to_lowercase().as_str() {
        "high" => TodoPriority::High,
        "medium" => TodoPriority::Medium,
        "low" => TodoPriority::Low,
        // Unknown strings fall back to medium priority.
        _ => TodoPriority::Medium,
    }
}
