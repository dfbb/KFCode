//! Permission management for kfcode: rule evaluation, engine state, and command arity lookup.

/// Command arity lookup for bash permission matching.
pub mod arity;
/// Runtime permission engine that tracks pending and approved permissions per session.
pub mod engine;
/// Permission rule types, ruleset construction, and evaluation helpers.
pub mod ruleset;

pub use arity::*;
pub use engine::*;
pub use ruleset::*;
