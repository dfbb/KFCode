//! Session types representing a coding session and its associated metadata.
//!
//! The central type is [`Session`], which aggregates messages, usage statistics,
//! permission rules, and lifecycle timestamps.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Aggregated diff statistics for a session's file changes.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionSummary {
    /// Total lines added across all changed files.
    pub additions: u64,
    /// Total lines deleted across all changed files.
    pub deletions: u64,
    /// Number of files that were changed.
    pub files: u64,
    /// Per-file diff details; omitted from serialization when absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diffs: Option<Vec<FileDiff>>,
}

/// Line-level diff statistics for a single file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    /// Relative path of the changed file.
    pub path: String,
    /// Lines added in this file.
    pub additions: u64,
    /// Lines deleted in this file.
    pub deletions: u64,
}

/// A shareable URL for a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionShare {
    /// Publicly accessible URL for viewing the session.
    pub url: String,
}

/// Parameters needed to revert a session to a prior state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRevert {
    /// Identifier of the message to revert to.
    pub message_id: String,
    /// Optional part identifier within the target message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub part_id: Option<String>,
    /// Full content snapshot at the revert point, if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<String>,
    /// Unified diff representing the changes to undo, if available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<String>,
}

/// Allow/deny permission rules applied to a session.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PermissionRuleset {
    /// Patterns or tool names that are explicitly permitted.
    #[serde(default)]
    pub allow: Vec<String>,
    /// Patterns or tool names that are explicitly denied.
    #[serde(default)]
    pub deny: Vec<String>,
    /// Optional mode string controlling how rules are evaluated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
}

/// Unix-millisecond timestamps tracking a session's lifecycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTime {
    /// When the session was first created, in milliseconds since the Unix epoch.
    pub created: i64,
    /// When the session was last modified, in milliseconds since the Unix epoch.
    pub updated: i64,
    /// When context compaction started, if in progress.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compacting: Option<i64>,
    /// When the session was archived, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archived: Option<i64>,
}

impl Default for SessionTime {
    fn default() -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            created: now,
            updated: now,
            compacting: None,
            archived: None,
        }
    }
}

/// Cumulative token and cost usage for a session.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionUsage {
    /// Total prompt tokens consumed.
    pub input_tokens: u64,
    /// Total completion tokens generated.
    pub output_tokens: u64,
    /// Tokens used for chain-of-thought reasoning.
    pub reasoning_tokens: u64,
    /// Tokens written to the prompt cache.
    pub cache_write_tokens: u64,
    /// Tokens served from the prompt cache.
    pub cache_read_tokens: u64,
    /// Estimated total cost in USD.
    pub total_cost: f64,
}

/// High-level lifecycle state of a session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionStatus {
    /// The session is open and accepting new messages.
    Active,
    /// The session has finished normally.
    Completed,
    /// The session has been archived and is read-only.
    Archived,
    /// The session's context is currently being compacted.
    Compacting,
}

impl Default for SessionStatus {
    fn default() -> Self {
        Self::Active
    }
}

/// Execution state of the agent within a session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RunStatus {
    /// No agent task is currently running.
    Idle,
    /// An agent task is actively running.
    Busy,
    /// The agent is retrying a failed task.
    Retrying {
        /// Number of retry attempts made so far.
        attempt: u32,
    },
}

use crate::message::SessionMessage;

/// A complete coding session, including its messages, metadata, and lifecycle state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique session identifier.
    pub id: String,
    /// URL-friendly slug derived from the session title.
    pub slug: String,
    /// Identifier of the project this session belongs to.
    pub project_id: String,
    /// Absolute path of the working directory for this session.
    pub directory: String,
    /// Identifier of the parent session, for forked sessions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    /// Human-readable title of the session.
    pub title: String,
    /// Schema or protocol version string.
    pub version: String,
    /// Lifecycle timestamps for the session.
    pub time: SessionTime,
    /// Ordered list of messages exchanged in this session.
    pub messages: Vec<SessionMessage>,
    /// Aggregated diff statistics, if computed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<SessionSummary>,
    /// Share URL, if the session has been published.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub share: Option<SessionShare>,
    /// Revert target, if a revert operation is pending.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revert: Option<SessionRevert>,
    /// Permission rules applied to this session.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission: Option<PermissionRuleset>,
    /// Token and cost usage accumulated during this session.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<SessionUsage>,
    /// Current lifecycle status of the session.
    #[serde(default)]
    pub status: SessionStatus,
    /// Arbitrary key-value metadata attached to the session.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
    /// Deserialization-only creation timestamp; not written back to storage.
    #[serde(default, skip_serializing)]
    pub created_at: DateTime<Utc>,
    /// Deserialization-only last-update timestamp; not written back to storage.
    #[serde(default, skip_serializing)]
    pub updated_at: DateTime<Utc>,
}

impl Session {
    /// Updates the session's `updated` timestamp to the current wall-clock time.
    pub fn touch(&mut self) {
        let now = Utc::now();
        self.time.updated = now.timestamp_millis();
        self.updated_at = now;
    }
}
