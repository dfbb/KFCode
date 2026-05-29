//! HTTP server library for kfcode, exposing REST and WebSocket endpoints for sessions, providers, MCP, PTY, and TUI control.

#![allow(ambiguous_glob_reexports)]

/// Bearer token authentication middleware.
pub mod auth_middleware;
/// Error types and the `Result` alias used across all route handlers.
pub mod error;
/// MCP server lifecycle and OAuth flow management.
pub mod mcp_oauth;
/// Provider authentication helpers wrapping the plugin auth bridge.
pub mod oauth;
/// Pseudo-terminal session management and WebSocket bridge.
pub mod pty;
/// Axum route definitions for all API endpoints.
pub mod routes;
/// Server state, startup, and storage synchronization.
pub mod server;
/// Git worktree utilities used by the experimental worktree endpoints.
pub mod worktree;

pub use error::*;
pub use mcp_oauth::*;
pub use oauth::*;
pub use pty::*;
pub use routes::*;
pub use server::*;
pub use worktree::*;
