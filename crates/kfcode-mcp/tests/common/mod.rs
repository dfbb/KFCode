#![allow(dead_code)]

pub mod sse_server;

use std::sync::Arc;
use kfcode_mcp::McpToolRegistry;
use wiremock::{matchers::{method, path}, Mock, MockServer, ResponseTemplate};
use wiremock::matchers::body_partial_json;

/// 启动一个空的 wiremock server。测试结束 drop 自动停止。
pub async fn fresh_mock_server() -> MockServer {
    MockServer::start().await
}

/// 默认空的 MCP tool registry。
pub fn fresh_registry() -> Arc<McpToolRegistry> {
    Arc::new(McpToolRegistry::new())
}

use kfcode_mcp::auth::AuthStore;
use tempfile::TempDir;

pub fn fresh_auth_store() -> (AuthStore, TempDir) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("mcp-auth.json");
    (AuthStore::new(path), dir)
}

/// 给 wiremock server mount 一个按 JSON-RPC method 字段匹配的 POST 响应。
pub async fn mount_jsonrpc_method(
    server: &MockServer,
    route: &str,
    method_name: &str,
    result: serde_json::Value,
) {
    Mock::given(method("POST"))
        .and(path(route))
        .and(body_partial_json(serde_json::json!({"method": method_name})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": result,
        })))
        .mount(server)
        .await;
}
