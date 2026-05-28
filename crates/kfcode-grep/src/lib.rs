//! Public API surface for the kfcode-grep crate, re-exporting all search types and the main search engine.

/// Core search types and the `Ripgrep` search engine implementation.
pub mod search;

pub use search::{FileSearchOptions, MatchResult, Ripgrep, Stats, SubMatch};
