//! Core primitives shared across the kfcode workspace.
//!
//! Provides an in-process event bus and a sortable, prefixed ID generator.

/// In-process publish/subscribe event bus.
pub mod bus;
/// Sortable, prefixed unique identifier utilities.
pub mod id;

pub use bus::*;
pub use id::*;
