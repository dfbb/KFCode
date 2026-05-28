//! Configuration loading and schema types for kfcode.
//! Exposes `ConfigLoader` for merging configs from global, project, and remote sources,
//! along with all serializable schema types.

/// Config loading logic: file discovery, env substitution, and merge ordering.
pub mod loader;
/// Serializable schema types that represent the full kfcode configuration.
pub mod schema;
/// Remote `.well-known/kfcode` fetching with in-process caching.
pub mod wellknown;

pub use loader::*;
pub use schema::*;
