# T27 — server `/pty/{id}/connect` WebSocket（真端口）

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 验证 PTY WebSocket 端到端：`POST /pty/` 创建 PTY → 用 `tokio-tungstenite` 连 `ws://.../pty/{id}/connect` → 收到至少一帧（PTY 输出帧或 cursor 元数据帧）→ 客户端关闭。

**Architecture:** 必须用真端口（spec §4.3 路线 B）。`pty_routes` 在 `routes.rs:3401`，handler `pty_connect` 在 `routes.rs:3463`。`POST /pty/` 创建一个 PTY 实例，body 含 `command`/`cwd`/cols/rows 等；handler 查 PTY manager 后 upgrade。

**Tech Stack:** axum::serve / tokio-tungstenite / reqwest。

**依赖:** T20

---

### Task 3.7：PTY WebSocket

**Files:**
- Create: `crates/kfcode-server/tests/pty_websocket.rs`

- [ ] **Step 1: 写测试**

写入 `crates/kfcode-server/tests/pty_websocket.rs`：

```rust
mod common;

use futures::{SinkExt, StreamExt};
use tokio::time::{timeout, Duration};
use tokio_tungstenite::connect_async;
use serde_json::json;

#[tokio::test]
async fn pty_websocket_connects_and_receives_frame() {
    let state = common::fresh_state_in_memory();
    let app = common::app_with(state);
    let server = common::TestServer::spawn(app).await;

    // 1. 创建 PTY，跑 echo
    let create_url = server.http_url("/pty/");
    let client = reqwest::Client::new();
    let create_res = client
        .post(&create_url)
        .json(&json!({
            "command": "echo",
            "args": ["hello-pty"],
            "cwd": "/",
            "cols": 80,
            "rows": 24
        }))
        .send()
        .await
        .expect("POST /pty/ — server may not accept this body shape; adjust per real CreatePtyRequest");
    assert!(create_res.status().is_success(), "create_pty failed: {}", create_res.status());

    let info: serde_json::Value = create_res.json().await.unwrap();
    let id = info.get("id").and_then(|v| v.as_str()).expect("pty id").to_string();

    // 2. 连 WebSocket
    let ws_url = server.ws_url(&format!("/pty/{id}/connect"));
    let (mut ws, _resp) = connect_async(ws_url).await.expect("ws connect");

    // 3. 在 2 秒内必须收到至少一帧
    let frame = timeout(Duration::from_secs(2), ws.next()).await
        .expect("timed out waiting for frame")
        .expect("ws stream ended")
        .expect("ws frame error");
    let _ = frame; // 内容由真实 PTY 决定（echo 输出"hello-pty"，或先收 cursor 元数据帧 0x00 + JSON）

    // 4. 关闭
    ws.close(None).await.unwrap();
}
```

> **如果 `POST /pty/` 实际接受的 body 形状与上面不同**：从源码 `crates/kfcode-server/src/routes.rs` 的 `create_pty` handler 取真实 `CreatePtyRequest` struct（grep `struct CreatePtyRequest` 与 `pub command: String` 之类），按真实字段构造 body。本 plan 列出的字段为常见组合，需要实施时核对。
>
> **如 PTY 在测试环境不可用**（CI 缺 portable-pty 后端、缺 echo binary 等）：标 `#[ignore = "depends on real PTY backend"]`。

- [ ] **Step 2: 跑测试**

```
cargo test -p kfcode-server --test pty_websocket
```

预期：1 条 pass。

- [ ] **Step 3: 提交**

```bash
git add crates/kfcode-server/tests/pty_websocket.rs
git commit -m "$(cat <<'EOF'
test(server): cover PTY WebSocket via real port + tokio-tungstenite

POST /pty creates a PTY, then ws connect to /pty/{id}/connect; the
test asserts at least one frame arrives within 2s and the client
closes cleanly. RAII TestServer aborts on drop.
EOF
)"
```
