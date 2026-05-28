# T24 — server `/session` POST/GET（跨 storage 协作 + 测试隔离）

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 验证 `POST /session` 能创建 session、`GET /session` 列出、`GET /session/{id}` 取回；带 storage 注入演示真实跨 crate 协作。每个测试独立 tempdir DB + 独立 ServerState，规避 spec §3.1 server 切面"测试隔离"风险。

**Architecture:** 用 T21 的 `ServerState::new_with_database`；每个测试新建 `Database::open_at(tempdir)` + 新 `ServerState`，避免共享全局 singleton。

**Tech Stack:** axum / tower oneshot / kfcode_storage::Database。

**依赖:** T20 / T21 / Batch 1 T03

---

### Task 3.4：/session POST/GET

**Files:**
- Modify: `crates/kfcode-server/tests/common/mod.rs`（追加 `state_with_temp_db`）
- Create: `crates/kfcode-server/tests/sessions.rs`

- [ ] **Step 1: helper**

在 `crates/kfcode-server/tests/common/mod.rs` 追加：

```rust
use kfcode_storage::Database;

/// 构造带 tempdir storage 的 ServerState，避免触碰用户真实 db。
/// 返回 (state, tempdir guard)；guard drop 时清理文件。
pub async fn state_with_temp_db() -> (Arc<ServerState>, tempfile::TempDir) {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("server-test.db");
    let db = Database::open_at(&path).await.expect("open_at");
    let state = ServerState::new_with_database(db, "http://test".into())
        .await
        .expect("inject");
    (Arc::new(state), dir)
}
```

- [ ] **Step 2: 写测试**

写入 `crates/kfcode-server/tests/sessions.rs`：

```rust
mod common;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header::CONTENT_TYPE};
use serde_json::Value;

#[tokio::test]
async fn create_session_returns_200_with_id() {
    let (state, _dir) = common::state_with_temp_db().await;
    let app = common::app_with(state);
    let res = common::oneshot_call(
        app,
        Request::builder()
            .method(Method::POST)
            .uri("/session/")
            .header(CONTENT_TYPE, "application/json")
            .header("x-kfcode-directory", "/tmp/test")
            .body(Body::from(r#"{}"#))
            .unwrap(),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let body = common::body_to_bytes(res).await;
    let v: Value = serde_json::from_slice(&body).unwrap();
    assert!(v.get("id").is_some(), "response missing id: {v}");
}

#[tokio::test]
async fn get_session_returns_404_for_missing() {
    let (state, _dir) = common::state_with_temp_db().await;
    let app = common::app_with(state);
    let res = common::oneshot_call(
        app,
        Request::builder()
            .uri("/session/does-not-exist")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn list_sessions_returns_200() {
    let (state, _dir) = common::state_with_temp_db().await;
    let app = common::app_with(state);
    let res = common::oneshot_call(
        app,
        Request::builder().uri("/session/").body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn each_test_has_isolated_storage() {
    // 同一函数内开两个 state，共用同一进程，要确保互不干扰
    let (state_a, _da) = common::state_with_temp_db().await;
    let (state_b, _db) = common::state_with_temp_db().await;
    let app_a = common::app_with(state_a.clone());
    let app_b = common::app_with(state_b.clone());

    let res_a = common::oneshot_call(
        app_a,
        Request::builder()
            .method(Method::POST)
            .uri("/session/")
            .header(CONTENT_TYPE, "application/json")
            .header("x-kfcode-directory", "/tmp/a")
            .body(Body::from(r#"{}"#))
            .unwrap(),
    )
    .await;
    assert_eq!(res_a.status(), StatusCode::OK);

    // state_b 看不见 state_a 创建的 session
    let res_b = common::oneshot_call(
        app_b,
        Request::builder().uri("/session/").body(Body::empty()).unwrap(),
    )
    .await;
    let body_b = common::body_to_bytes(res_b).await;
    let v: Value = serde_json::from_slice(&body_b).unwrap();
    let arr = v.as_array().expect("array");
    assert!(arr.is_empty(), "state_b should not see state_a sessions; got {v}");
}
```

- [ ] **Step 3: 跑测试**

```
cargo test -p kfcode-server --test sessions
```

预期：4 条 pass。注意：如果 `each_test_has_isolated_storage` 失败，意味着 server 内部仍用进程级 singleton（如全局 `SESSION_RUN_STATUS` 影响 list 输出）——按 spec §3.1 server 切面"测试隔离（关键）"加 reset hook，作为 §2.8 修源码独立提交。

- [ ] **Step 4: 提交**

```bash
git add crates/kfcode-server/tests/sessions.rs crates/kfcode-server/tests/common/mod.rs
git commit -m "$(cat <<'EOF'
test(server): cover /session POST/GET with isolated tempdir storage

Each test gets its own Database::open_at(tempdir) and new ServerState,
verifying no cross-test storage bleed. Sets x-kfcode-directory header
because create_session middleware injects KFCodeDirectory from it.
EOF
)"
```
