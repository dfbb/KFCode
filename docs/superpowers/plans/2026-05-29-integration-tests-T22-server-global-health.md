# T22 — server `/health` 黄金路径（oneshot 路线）

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 验证 spec §4.3 路线 A（`tower::ServiceExt::oneshot` + 内存 router）能跑通最简单 `/health` 路由。

**Architecture:** `/health` 在 `crates/kfcode-server/src/routes.rs:48` 顶级注册，handler 见 `routes.rs:4238`。返回 `{"status":"ok","version":"..."}`。

**Tech Stack:** axum 0.8 / tower 0.5 util。

**依赖:** T20 / T21（state 不需要 storage 即可，所以技术上只依赖 T20）

---

### Task 3.2：/health 黄金路径

**Files:**
- Create: `crates/kfcode-server/tests/health.rs`

- [ ] **Step 1: 写测试**

写入 `crates/kfcode-server/tests/health.rs`：

```rust
mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};

#[tokio::test]
async fn health_returns_200_ok() {
    let state = common::fresh_state_in_memory();
    let app = common::app_with(state);

    let res = common::oneshot_call(
        app,
        Request::builder().uri("/health").body(Body::empty()).unwrap(),
    )
    .await;

    assert_eq!(res.status(), StatusCode::OK);
    let body = common::body_to_bytes(res).await;
    let body_str = std::str::from_utf8(&body).unwrap();
    assert!(body_str.contains(r#""status":"ok""#), "got: {body_str}");
}
```

- [ ] **Step 2: 跑测试**

```
cargo test -p kfcode-server --test health
```

预期：1 条 pass。

- [ ] **Step 3: 提交**

```bash
git add crates/kfcode-server/tests/health.rs
git commit -m "test(server): cover /health golden path via tower::oneshot"
```
