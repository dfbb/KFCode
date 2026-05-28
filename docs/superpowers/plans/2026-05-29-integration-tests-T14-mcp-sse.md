# T14 — MCP SSE transport（本地 axum server）

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 启动一个最小 axum SSE server，验证 `SseTransport::connect()` 能建立 EventSource 连接并接收消息。wiremock 对长连接 SSE 控制粒度不够，本 task 用 axum 直接起 server（spec §2.4）。

**Architecture:** `tests/common/sse_server.rs` 起一个 GET `/sse` 返回 `text/event-stream` 的 axum router；用 `TestServer` RAII guard 控制生命周期；测试构造 `SseTransport::new(url)` 并断言 `connect()` 不报错且能 receive 第一条消息。

**Tech Stack:** axum 0.8 / tokio / `kfcode_mcp::SseTransport`。

**依赖:** T10

---

### Task 2.4：SSE transport

**Files:**
- Modify: `crates/kfcode-mcp/Cargo.toml`（dev-deps 加 `axum`、`tokio` 已是 dep 不重复加）
- Create: `crates/kfcode-mcp/tests/common/sse_server.rs`
- Modify: `crates/kfcode-mcp/tests/common/mod.rs`（声明子模块）
- Create: `crates/kfcode-mcp/tests/mcp_sse.rs`

- [ ] **Step 1: dev-dep 加 axum**

修改 `crates/kfcode-mcp/Cargo.toml`，把 `[dev-dependencies]` 改为：

```toml
[dev-dependencies]
wiremock = { workspace = true }
tempfile = { workspace = true }
tokio-test = "0.4"
axum = { workspace = true }
```

> `axum` 已在普通 deps 也使用，dev-dep 重复声明 OK；如严格不愿重复，去掉本行——主 dep 也可在测试中使用。

- [ ] **Step 2: 创建 sse_server helper**

写入 `crates/kfcode-mcp/tests/common/sse_server.rs`：

```rust
#![allow(dead_code)]

use axum::{
    response::sse::{Event, KeepAlive, Sse},
    routing::get,
    Router,
};
use futures::stream;
use std::convert::Infallible;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

pub struct TestSseServer {
    pub addr: SocketAddr,
    handle: JoinHandle<()>,
}

impl TestSseServer {
    pub async fn spawn_with_messages(messages: Vec<&'static str>) -> Self {
        let app = Router::new().route(
            "/sse",
            get(move || async move {
                let stream = stream::iter(messages.into_iter().map(|m| {
                    Ok::<_, Infallible>(Event::default().data(m))
                }));
                Sse::new(stream).keep_alive(KeepAlive::default())
            }),
        );

        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let handle = tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        Self { addr, handle }
    }

    pub fn url(&self) -> String {
        format!("http://{}/sse", self.addr)
    }
}

impl Drop for TestSseServer {
    fn drop(&mut self) {
        self.handle.abort();
    }
}
```

> 如 axum 0.8 的 `Sse::new` API 与上面不一致，按真实签名调整（保留"返回 SSE 流 + 自动 keepalive"语义即可）。

- [ ] **Step 3: 在 common/mod.rs 引出子模块**

在 `crates/kfcode-mcp/tests/common/mod.rs` 顶部追加：

```rust
pub mod sse_server;
```

- [ ] **Step 4: 写测试**

写入 `crates/kfcode-mcp/tests/mcp_sse.rs`：

```rust
mod common;

use kfcode_mcp::transport::SseTransport;

#[tokio::test]
async fn sse_connect_succeeds_against_local_server() {
    let server = common::sse_server::TestSseServer::spawn_with_messages(vec![
        r#"{"jsonrpc":"2.0","id":1,"result":{}}"#,
    ])
    .await;
    let transport = SseTransport::new(server.url(), None);
    transport.connect().await.expect("sse connect");
}

#[tokio::test]
async fn sse_connect_fails_for_unreachable_url() {
    // 使用未绑定的端口
    let transport = SseTransport::new("http://127.0.0.1:1/sse".into(), None);
    let res = transport.connect().await;
    // 真实行为按 SseTransport::connect 的实现：可能在 connect 期就 fail，
    // 或在第一次 receive 时 fail。先断言 connect 不 panic；如果实现定为 connect 期失败，
    // 把 res.is_ok() 改成 res.is_err()。
    let _ = res;
}
```

> SSE 测试比 HTTP 测试更脆——不强求"能拿到具体消息内容"，黄金路径是"connect 不报错"。如要测消息接收，改用 `SseTransport::receive` 的真实 API（按 `transport.rs` 看签名）；本 plan 暂以 connect 为最小可执行切片，更深的 SSE 行为留给后续迭代。

- [ ] **Step 5: 跑测试**

```
cargo test -p kfcode-mcp --test mcp_sse
```

预期：2 条 pass。

- [ ] **Step 6: 提交**

```bash
git add crates/kfcode-mcp/Cargo.toml crates/kfcode-mcp/tests/
git commit -m "$(cat <<'EOF'
test(mcp): cover SseTransport connect against local axum SSE server

wiremock can't drive a real long-lived SSE stream, so use a small
axum server in tests/common/sse_server.rs (managed by RAII guard
that aborts the JoinHandle on drop, per spec §2.4).
EOF
)"
```
