# T20 — kfcode-server 集成测试脚手架

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 准备 server 集成测试目录，加 dev-deps（`tower` 启用 `util` feature、`reqwest`、`tokio-tungstenite`、`tempfile`），写 common helper 含 `oneshot_call(app, req)` 与 `TestServer` RAII guard。

**Architecture:** spec §4.3 路线 A（`tower::ServiceExt::oneshot`）作为 HTTP 黄金路径主线；路线 B（真端口 + reqwest / tokio-tungstenite）只在 WebSocket / SSE 任务用。

**Tech Stack:** axum 0.8 / tower 0.5 (util) / reqwest / tokio-tungstenite。

**依赖:** Batch 1 完成（依赖 `Database::open_at`、`AuthStore` 不直接需要——server 用自己的 Database）

---

### Task 3.0：server 脚手架

**Files:**
- Modify: `crates/kfcode-server/Cargo.toml`（新增 `[dev-dependencies]`）
- Create: `crates/kfcode-server/tests/common/mod.rs`
- Create: `crates/kfcode-server/tests/smoke.rs`

- [ ] **Step 1: dev-deps**

修改 `crates/kfcode-server/Cargo.toml`，在文件末尾追加：

```toml
[dev-dependencies]
tower = { workspace = true, features = ["util"] }
reqwest = { workspace = true }
tokio-tungstenite = { workspace = true }
tempfile = { workspace = true }
tokio-test = "0.4"
```

> `tower` 在普通 deps 已有 `tower = { workspace = true }`。这里在 dev-deps 用 `features = ["util"]` 启用 `ServiceExt::oneshot`。如 cargo 对同一 crate 同名声明报"重复定义"，把 features 上提到普通 deps 行：`tower = { workspace = true, features = ["util"] }`，并删除 dev-dep 行——该 feature 在 prod 不会被多余编译开销影响（tower::util 是空 feature）。

- [ ] **Step 2: common helper**

写入 `crates/kfcode-server/tests/common/mod.rs`：

```rust
#![allow(dead_code)]

use axum::body::{Body, to_bytes};
use axum::http::{Request, Response, StatusCode};
use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tower::util::ServiceExt;

use kfcode_server::{router, ServerState};

/// 构造一个空 ServerState（无 storage backend）；适用于不依赖 db 的路由。
pub fn fresh_state_in_memory() -> Arc<ServerState> {
    Arc::new(ServerState::new())
}

/// 把 router 与给定 state 拼起来，返回完整 axum::Router。
pub fn app_with(state: Arc<ServerState>) -> Router {
    router().with_state(state)
}

/// 用 ServiceExt::oneshot 在内存中调用 app，免起端口。
pub async fn oneshot_call(app: Router, req: Request<Body>) -> Response<Body> {
    app.oneshot(req).await.expect("oneshot")
}

pub async fn body_to_bytes(res: Response<Body>) -> bytes::Bytes {
    to_bytes(res.into_body(), 1024 * 1024).await.expect("body")
}

/// 仅用于 WebSocket / SSE 测试：起真实端口，drop 时 abort。
pub struct TestServer {
    pub addr: SocketAddr,
    handle: JoinHandle<()>,
}

impl TestServer {
    pub async fn spawn(app: Router) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let handle = tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        Self { addr, handle }
    }

    pub fn http_url(&self, path: &str) -> String {
        format!("http://{}{}", self.addr, path)
    }

    pub fn ws_url(&self, path: &str) -> String {
        format!("ws://{}{}", self.addr, path)
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.handle.abort();
    }
}
```

> 测试代码使用 `kfcode_server::router` 与 `kfcode_server::ServerState`——确认这两项已 re-export。如未 re-export，T21 会同时补上。

- [ ] **Step 3: smoke 测试**

写入 `crates/kfcode-server/tests/smoke.rs`：

```rust
mod common;

#[tokio::test]
async fn server_state_constructs() {
    let _state = common::fresh_state_in_memory();
}
```

- [ ] **Step 4: 跑测试**

```
cargo test -p kfcode-server --test smoke
```

预期：1 条 pass。如出现"router 未 re-export"编译错，跳到 T21 完成 Database 注入与 re-export 后再回到本 task。

- [ ] **Step 5: 提交**

```bash
git add crates/kfcode-server/Cargo.toml crates/kfcode-server/tests/
git commit -m "$(cat <<'EOF'
test(server): scaffold integration tests

dev-deps include tower with util feature for ServiceExt::oneshot,
plus reqwest and tokio-tungstenite for WebSocket-bound tests.
tests/common/mod.rs offers oneshot_call (per spec §4.3 route A) and
TestServer RAII (route B).
EOF
)"
```
