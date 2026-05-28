# T23 — server `/config` GET + CORS 行为

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 验证 `GET /config` 返回 200 + JSON；验证 CORS 中间件（按真实 router 配置；如 router 未挂 CORS 中间件，本任务只覆盖 `/config` 的功能性而不强制覆盖 CORS）。

**Architecture:** `/config` 路由在 `routes.rs:2440`，`get_config` handler 直接读全局 `CONFIG_STATE`（`Lazy<RwLock<AppConfig>>`），不依赖 storage——可用 `fresh_state_in_memory()`。CORS：在 router 顶层是否挂 `CorsLayer` 由 `routes.rs:45` 与外层 `run_server_with_state` 决定，本 task 只覆盖功能性。

**Tech Stack:** axum / tower oneshot。

**依赖:** T20

---

### Task 3.3：/config GET

**Files:**
- Create: `crates/kfcode-server/tests/config.rs`

- [ ] **Step 1: 写测试**

写入 `crates/kfcode-server/tests/config.rs`：

```rust
mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};

#[tokio::test]
async fn config_get_returns_200() {
    let state = common::fresh_state_in_memory();
    let app = common::app_with(state);
    let res = common::oneshot_call(
        app,
        Request::builder().uri("/config/").body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn config_providers_returns_200() {
    let state = common::fresh_state_in_memory();
    let app = common::app_with(state);
    let res = common::oneshot_call(
        app,
        Request::builder().uri("/config/providers").body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn unknown_path_returns_404() {
    let state = common::fresh_state_in_memory();
    let app = common::app_with(state);
    let res = common::oneshot_call(
        app,
        Request::builder().uri("/no/such/path").body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
```

- [ ] **Step 2: 跑测试**

```
cargo test -p kfcode-server --test config
```

预期：3 条 pass。

- [ ] **Step 3: 提交**

```bash
git add crates/kfcode-server/tests/config.rs
git commit -m "test(server): cover /config GET, /config/providers GET, and 404 fallthrough"
```

> CORS 行为留给 batch 3 后续 plan：CORS layer 是否挂在 `router()` 之外（在 `run_server_with_state`）会影响测试构造方式；如挂在外层，CORS 测试需起真实端口走路线 B。本 task 限于路由功能性。
