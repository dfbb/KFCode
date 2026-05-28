# T13 — MCP HTTP 错误路径（4xx/5xx + JSON-RPC error + 超时）

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 验证 `McpClient::http` / `call_tool` 在服务器返回 4xx/5xx、JSON-RPC error envelope、网络超时下，返回 `McpClientError` 而非 panic。

**Architecture:** wiremock 不同响应模板；超时用 wiremock 的 `set_delay` 加 `McpClient::with_timeout`。

**Tech Stack:** wiremock / `kfcode_mcp::{McpClient, McpClientError}`。

**依赖:** T10 / T12

---

### Task 2.3：MCP HTTP 错误路径

**Files:**
- Create: `crates/kfcode-mcp/tests/mcp_http_errors.rs`

- [ ] **Step 1: 写失败测试**

写入 `crates/kfcode-mcp/tests/mcp_http_errors.rs`：

```rust
mod common;

use kfcode_mcp::{McpClient, McpClientError};
use std::time::Duration;
use wiremock::{matchers::{method, path}, Mock, ResponseTemplate};
use wiremock::matchers::body_partial_json;

#[tokio::test]
async fn http_connect_fails_on_5xx() {
    let server = common::fresh_mock_server().await;
    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
        .mount(&server)
        .await;

    let res = McpClient::http("t".into(), common::fresh_registry(), server.uri(), None).await;
    assert!(res.is_err(), "expected error on 500");
}

#[tokio::test]
async fn http_connect_fails_on_4xx() {
    let server = common::fresh_mock_server().await;
    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(400).set_body_string("bad"))
        .mount(&server)
        .await;

    let res = McpClient::http("t".into(), common::fresh_registry(), server.uri(), None).await;
    assert!(res.is_err());
}

#[tokio::test]
async fn http_connect_propagates_jsonrpc_error_envelope() {
    let server = common::fresh_mock_server().await;
    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_partial_json(serde_json::json!({"method": "initialize"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "error": {"code": -32603, "message": "internal error"}
        })))
        .mount(&server)
        .await;

    let res = McpClient::http("t".into(), common::fresh_registry(), server.uri(), None).await;
    let err = res.unwrap_err();
    let s = format!("{err}");
    assert!(s.to_lowercase().contains("internal") || s.contains("-32603"), "got: {s}");
}

#[tokio::test]
async fn http_call_times_out_on_slow_response() {
    // Mock 延迟 2 秒响应；client timeout 设 200ms。
    let server = common::fresh_mock_server().await;
    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_secs(2))
                .set_body_json(serde_json::json!({
                    "jsonrpc": "2.0", "id": 1,
                    "result": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": {"tools": {}},
                        "serverInfo": {"name":"x", "version":"1"}
                    }
                })),
        )
        .mount(&server)
        .await;

    // McpClient::with_timeout 在 client 侧设阈
    // 注意：with_timeout 是 sync builder（见 client.rs:228 之后），但 connect_http 异步。
    // 构造空 client 后再 connect_http。
    let registry = common::fresh_registry();
    let client = kfcode_mcp::McpClient::new("t".into(), registry).with_timeout(200);
    let res = client.connect_http(server.uri(), None).await;
    assert!(res.is_err(), "expected timeout error");
}
```

> 如 `with_timeout` 不是 `&self -> Self` 的链式 API（见 `client.rs` 真实签名），按真实签名调整调用方式。

- [ ] **Step 2: 跑测试**

```
cargo test -p kfcode-mcp --test mcp_http_errors
```

预期：4 条 pass。

- [ ] **Step 3: 提交**

```bash
git add crates/kfcode-mcp/tests/mcp_http_errors.rs
git commit -m "test(mcp): cover HTTP errors (4xx/5xx, JSON-RPC error envelope, timeout)"
```
