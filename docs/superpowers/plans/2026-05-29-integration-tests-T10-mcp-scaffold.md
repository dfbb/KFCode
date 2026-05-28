# T10 — kfcode-mcp 集成测试脚手架

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 建 mcp 集成测试目录、helper 与 dev-deps（wiremock、tempfile、tokio-tungstenite 用不到这里只 mcp 部分）。

**Architecture:** `crates/kfcode-mcp/tests/common/mod.rs` 提供 wiremock helper 与 `make_registry()`；外加 workspace dev-dep 声明。

**Tech Stack:** wiremock 0.6 / tempfile / serde_json。

**依赖:** 无（与 batch 1 并行可行）

---

### Task 2.0：mcp 脚手架

**Files:**
- Modify: `Cargo.toml`（workspace 根，加 wiremock 到 `[workspace.dependencies]`）
- Modify: `crates/kfcode-mcp/Cargo.toml`（新增 `[dev-dependencies]` 段）
- Create: `crates/kfcode-mcp/tests/common/mod.rs`
- Create: `crates/kfcode-mcp/tests/smoke.rs`

- [ ] **Step 1: workspace 加 wiremock 与 tempfile**

修改根 `Cargo.toml`，在 `[workspace.dependencies]` 末尾追加：

```toml
wiremock = "0.6"
tempfile = "3"
```

`tempfile` 若已在 workspace（前面 batch 1 已加）则跳过，避免重复声明。

- [ ] **Step 2: kfcode-mcp 加 dev-dependencies**

修改 `crates/kfcode-mcp/Cargo.toml`，在文件末尾追加：

```toml
[dev-dependencies]
wiremock = { workspace = true }
tempfile = { workspace = true }
tokio-test = "0.4"
```

- [ ] **Step 3: 创建 common helper**

写入 `crates/kfcode-mcp/tests/common/mod.rs`：

```rust
#![allow(dead_code)]

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

/// 给 wiremock server mount 一个按 JSON-RPC method 字段匹配的 POST 响应。
/// MCP HTTP transport 把所有 RPC 都打到同一路径，必须按 method 区分。
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
```

- [ ] **Step 4: smoke 测试**

写入 `crates/kfcode-mcp/tests/smoke.rs`：

```rust
mod common;

#[tokio::test]
async fn mock_server_starts_and_stops() {
    let server = common::fresh_mock_server().await;
    let url = server.uri();
    assert!(url.starts_with("http://"));
    drop(server); // 应当干净停止
}

#[tokio::test]
async fn registry_constructs() {
    let _r = common::fresh_registry();
}
```

- [ ] **Step 5: 跑测试**

```
cargo test -p kfcode-mcp --test smoke
```

预期：2 条 pass。

- [ ] **Step 6: 提交**

```bash
git add Cargo.toml crates/kfcode-mcp/Cargo.toml crates/kfcode-mcp/tests/
git commit -m "$(cat <<'EOF'
test(mcp): scaffold integration tests

Workspace declares wiremock 0.6 and tempfile (if not already). mcp
crate gets dev-deps wiremock + tempfile + tokio-test, plus a
tests/common/mod.rs with fresh_mock_server / fresh_registry /
mount_jsonrpc_method helpers (the last matches POST bodies by
JSON-RPC "method" field per spec §4.1).
EOF
)"
```
