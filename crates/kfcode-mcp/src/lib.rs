//! MCP (Model Context Protocol) client library for kfcode.
//!
//! Provides transports, protocol types, OAuth support, and a client registry
//! for connecting to and calling tools on remote MCP servers.

/// Persistent OAuth credential storage.
pub mod auth;
/// MCP client and registry for managing server connections.
pub mod client;
/// OAuth 2.0 authorization-code + PKCE flow for MCP servers.
pub mod oauth;
/// JSON-RPC 2.0 message types used by the MCP protocol.
pub mod protocol;
/// Tool metadata and in-memory tool registry.
pub mod tool;
/// Transport implementations (stdio, HTTP, SSE).
pub mod transport;

pub use client::{
    McpClient, McpClientError, McpClientRegistry, McpServerConfig, McpStatus,
    MCP_TOOLS_CHANGED_EVENT,
};
pub use oauth::{AuthStatus, McpOAuthConfig, McpOAuthManager, OAuthError, OAuthRegistry};
pub use protocol::{JsonRpcNotification, JsonRpcRequest, JsonRpcResponse};
pub use tool::{McpTool, McpToolRegistry};
pub use transport::{HttpTransport, McpTransport, SseTransport, StdioTransport};
