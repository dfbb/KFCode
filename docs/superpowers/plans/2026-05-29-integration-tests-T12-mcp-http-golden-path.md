# T12 — MCP HTTP transport 黄金路径（initialize → tools/list → tools/call）

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 用 wiremock 模拟完整 MCP HTTP RPC 序列，验证 `McpClient::http(...)` 能完成 initialize 握手、加载 tools、调用 tool。

**Architecture:** 用 `mount_jsonrpc_method` helper 按 method 字段分别挂 `initialize`、`notifications/initialized`、`tools/list`、`tools/call` 的响应。`McpClient::http` 内部调用顺序见 `client.rs:228 → connect_http → connect_http_inner` → `initialize` → `notifications/initialized` → `load_tools`。

**Tech Stack:** wiremock / serde_json / `kfcode_mcp::McpClient`。

**依赖:** T10

---

### Task 2.2：MCP HTTP 黄金路径

**Files:**
- Create: `crates/kfcode-mcp/tests/mcp_http_golden.rs`

- [ ] **Step 1: 写失败测试**

写入 `crates/kfcode-mcp/tests/mcp_http_golden.rs`：

```rust
mod common;

use kfcode_mcp::McpClient;
use wiremock::{matchers::{method, path}, Mock, ResponseTemplate};
use wiremock::matchers::body_partial_json;

async fn mount_initialize_sequence(server: &wiremock::MockServer) {
    // initialize
    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_partial_json(serde_json::json!({"method": "initialize"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "test-server", "version": "1.0" },
            }
        })))
        .mount(server)
        .await;

    // notifications/initialized — 服务器对 notification 通常 200 空 body
    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_partial_json(serde_json::json!({"method": "notifications/initialized"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
        .mount(server)
        .await;

    // tools/list
    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_partial_json(serde_json::json!({"method": "tools/list"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "tools": [
                    {
                        "name": "echo",
                        "description": "echo input",
                        "inputSchema": { "type": "object" }
                    }
                ]
            }
        })))
        .mount(server)
        .await;
}

#[tokio::test]
async fn http_connect_completes_handshake_and_lists_tools() {
    let server = common::fresh_mock_server().await;
    mount_initialize_sequence(&server).await;

    let registry = common::fresh_registry();
    let client = McpClient::http(
        "test".to_string(),
        registry.clone(),
        server.uri(),
        None,
    )
    .await
    .expect("connect http");

    assert!(client.is_initialized().await, "must be initialized");
    let tools = registry.list().await;
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].definition.name, "echo");
}

#[tokio::test]
async fn http_call_tool_returns_result() {
    let server = common::fresh_mock_server().await;
    mount_initialize_sequence(&server).await;

    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_partial_json(serde_json::json!({"method": "tools/call"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "content": [{"type": "text", "text": "hello"}],
                "isError": false
            }
        })))
        .mount(&server)
        .await;

    let client = McpClient::http(
        "t".to_string(),
        common::fresh_registry(),
        server.uri(),
        None,
    )
    .await
    .unwrap();

    let res = client
        .call_tool("echo", Some(serde_json::json!({"text": "hello"})))
        .await
        .expect("call_tool");
    let txt = serde_json::to_string(&res).unwrap();
    assert!(txt.contains("hello"), "result must echo input: {txt}");
}
```

> **若 `McpToolRegistry::list()` 返回的字段名不是 `definition.name`**，按 source 真实字段调整（参考 `crates/kfcode-mcp/src/tool.rs`）。如返回的是 `McpTool { name, description, ... }`，把 `tools[0].definition.name` 改成 `tools[0].name`。

- [ ] **Step 2: 跑测试**

```
cargo test -p kfcode-mcp --test mcp_http_golden
```

预期：2 条 pass。

- [ ] **Step 3: 提交**

```bash
git add crates/kfcode-mcp/tests/mcp_http_golden.rs
git commit -m "$(cat <<'EOF'
test(mcp): cover HTTP transport initialize → tools/list → tools/call

Wiremock matches POST bodies by JSON-RPC method (per spec §4.1) so
all three RPCs share a single path but get distinct responses.
Verifies handshake completes, tools load into registry, and
tools/call result threads through.
EOF
)"
```
