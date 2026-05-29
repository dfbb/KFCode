//! Agent runtime for kfcode: defines agent types, conversation state, and the execution loop.

#![allow(ambiguous_glob_reexports)]

/// Agent configuration, registry, and permission evaluation.
pub mod agent;
/// Stateful executor that drives the provider/tool agentic loop.
pub mod executor;
/// Message and conversation types shared across the agent runtime.
pub mod message;

pub use agent::*;
pub use executor::*;
pub use message::*;
