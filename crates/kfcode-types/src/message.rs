//! Message types used to represent the content exchanged within a session.
//!
//! A [`SessionMessage`] is the top-level unit; it contains one or more [`MessagePart`]
//! values that carry the actual payload via the [`PartType`] enum.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single message within a session, authored by a specific role.
///
/// Each message holds an ordered list of [`MessagePart`] values that together
/// form the full content (text, tool calls, files, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    /// Unique message identifier, prefixed with `msg_`.
    pub id: String,
    /// Identifier of the session this message belongs to.
    pub session_id: String,
    /// Who authored this message.
    pub role: MessageRole,
    /// Ordered content parts that make up the message body.
    pub parts: Vec<MessagePart>,
    /// Wall-clock time when the message was created.
    pub created_at: DateTime<Utc>,
    /// Arbitrary key-value metadata attached to the message.
    pub metadata: HashMap<String, serde_json::Value>,
}

/// The author role of a [`SessionMessage`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageRole {
    /// A human user turn.
    User,
    /// An AI assistant turn.
    Assistant,
    /// A system-level instruction turn.
    System,
    /// A tool response turn.
    Tool,
}

/// A single typed segment within a [`SessionMessage`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagePart {
    /// Unique part identifier, prefixed with `prt_`.
    pub id: String,
    /// The typed payload carried by this part.
    pub part_type: PartType,
    /// Wall-clock time when this part was created.
    pub created_at: DateTime<Utc>,
    /// Back-reference to the owning message, if set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
}

/// The typed payload of a [`MessagePart`].
///
/// Serialized with a `"type"` discriminant tag using camelCase variant names.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PartType {
    /// A plain-text segment.
    Text {
        text: String,
    },
    /// A request to invoke a tool.
    ToolCall {
        /// Unique identifier for this tool call invocation.
        id: String,
        /// Name of the tool to invoke.
        name: String,
        /// JSON-encoded input arguments for the tool.
        input: serde_json::Value,
    },
    /// The result returned by a tool invocation.
    ToolResult {
        /// Identifier of the corresponding [`PartType::ToolCall`].
        tool_call_id: String,
        /// Textual output produced by the tool.
        content: String,
        /// Whether the tool reported an error.
        is_error: bool,
    },
    /// Internal chain-of-thought reasoning text (not shown to the user).
    Reasoning {
        text: String,
    },
    /// A file attachment referenced by URL.
    File {
        /// Publicly accessible URL of the file.
        url: String,
        /// Original filename.
        filename: String,
        /// MIME type of the file content.
        mime: String,
    },
    /// Marks the beginning of a named execution step.
    StepStart {
        /// Unique step identifier.
        id: String,
        /// Human-readable step name.
        name: String,
    },
    /// Marks the completion of a named execution step.
    StepFinish {
        /// Identifier matching the corresponding [`PartType::StepStart`].
        id: String,
        /// Optional output produced by the step.
        output: Option<String>,
    },
    /// A full content snapshot, used for revert/restore operations.
    Snapshot {
        content: String,
    },
    /// A text patch describing a file edit.
    Patch {
        /// The original text being replaced.
        old_string: String,
        /// The replacement text.
        new_string: String,
        /// Path of the file being patched.
        filepath: String,
    },
    /// Status update for a named sub-agent.
    Agent {
        /// Name of the sub-agent.
        name: String,
        /// Current status string of the sub-agent.
        status: String,
    },
    /// Status update for a named subtask.
    Subtask {
        /// Unique subtask identifier.
        id: String,
        /// Human-readable description of the subtask.
        description: String,
        /// Current status string of the subtask.
        status: String,
    },
    /// Records a retry attempt with a reason.
    Retry {
        /// Number of retries attempted so far.
        count: u32,
        /// Human-readable explanation for the retry.
        reason: String,
    },
    /// A context-compaction event carrying a condensed summary.
    Compaction {
        /// Condensed summary replacing the compacted context.
        summary: String,
    },
}

impl SessionMessage {
    /// Creates a new user message containing a single text part.
    pub fn user(session_id: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            id: format!("msg_{}", uuid::Uuid::new_v4()),
            session_id: session_id.into(),
            role: MessageRole::User,
            parts: vec![MessagePart {
                id: format!("prt_{}", uuid::Uuid::new_v4()),
                part_type: PartType::Text { text: text.into() },
                created_at: Utc::now(),
                message_id: None,
            }],
            created_at: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Creates a new assistant message with an empty parts list.
    ///
    /// Parts are typically appended incrementally as the assistant streams its response.
    pub fn assistant(session_id: impl Into<String>) -> Self {
        Self {
            id: format!("msg_{}", uuid::Uuid::new_v4()),
            session_id: session_id.into(),
            role: MessageRole::Assistant,
            parts: Vec::new(),
            created_at: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Concatenates all [`PartType::Text`] parts into a single string.
    pub fn get_text(&self) -> String {
        self.parts
            .iter()
            .filter_map(|p| match &p.part_type {
                PartType::Text { text } => Some(text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }
}
