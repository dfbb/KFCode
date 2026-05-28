//! Lightweight JSON-RPC 2.0 types for the plugin subprocess protocol.
//!
//! These are intentionally independent of `kfcode-mcp` so the plugin crate
//! does not pull in the full MCP dependency graph.

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ---------------------------------------------------------------------------
// JSON-RPC envelope
// ---------------------------------------------------------------------------

/// A JSON-RPC 2.0 request sent from Rust to the plugin host.
#[derive(Debug, Serialize)]
pub struct RpcRequest {
    pub jsonrpc: &'static str,
    pub id: u64,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl RpcRequest {
    /// Construct a request with the given `id`, `method`, and optional `params`.
    pub fn new(id: u64, method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            method: method.into(),
            params,
        }
    }
}

/// A JSON-RPC 2.0 response received from the plugin host.
#[derive(Debug, Deserialize)]
pub struct RpcResponse {
    #[allow(dead_code)]
    pub jsonrpc: String,
    pub id: u64,
    pub result: Option<Value>,
    pub error: Option<RpcError>,
}

/// A JSON-RPC 2.0 error object embedded in a response.
#[derive(Debug, Deserialize)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
}

/// A JSON-RPC 2.0 notification (no `id`) sent from the plugin host to Rust.
#[derive(Debug, Deserialize)]
pub struct RpcNotification {
    #[allow(dead_code)]
    pub jsonrpc: String,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}
