//! Shared domain types for the kfcode workspace.
//!
//! Re-exports all public items from the `message`, `session`, and `todo` modules
//! so downstream crates can import everything from a single path.

/// Message types: [`SessionMessage`], [`MessageRole`], [`MessagePart`], and [`PartType`].
pub mod message;
/// Session types: [`Session`], [`SessionStatus`], [`SessionUsage`], and related structs.
pub mod session;
/// Todo types: [`TodoItem`], [`TodoStatus`], [`TodoPriority`], and parsing helpers.
pub mod todo;

pub use message::*;
pub use session::*;
pub use todo::*;
