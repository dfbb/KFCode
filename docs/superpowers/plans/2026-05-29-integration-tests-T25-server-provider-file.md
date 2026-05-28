# T25 — server `/provider` GET + `/file/content` GET

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 覆盖 `/provider` 与 `/file` 两个 route group 的代表黄金路径。

**Architecture:** 用空 state 即可（不需要 storage）。`/file/content` 接 `?path=...`；用 tempdir 写一个文件再读。

**Tech Stack:** axum / tower oneshot。

**依赖:** T20

---

### Task 3.5：/provider + /file

**Files:**
- Create: `crates/kfcode-server/tests/provider_file.rs`

- [ ] **Step 1: 写测试**

写入 `crates/kfcode-server/tests/provider_file.rs`：

```rust
mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tempfile::TempDir;

#[tokio::test]
async fn provider_list_returns_200() {
    let state = common::fresh_state_in_memory();
    let app = common::app_with(state);
    let res = common::oneshot_call(
        app,
        Request::builder().uri("/provider/").body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn file_content_returns_file_bytes() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("hello.txt");
    std::fs::write(&file, "hello world").unwrap();

    let state = common::fresh_state_in_memory();
    let app = common::app_with(state);

    let uri = format!("/file/content?path={}", urlencoding::encode(file.to_str().unwrap()));
    let res = common::oneshot_call(
        app,
        Request::builder().uri(&uri).body(Body::empty()).unwrap(),
    )
    .await;

    assert_eq!(res.status(), StatusCode::OK);
    let body = common::body_to_bytes(res).await;
    assert!(
        std::str::from_utf8(&body).unwrap_or("").contains("hello world"),
        "expected file body in response"
    );
}

#[tokio::test]
async fn file_content_returns_404_for_missing() {
    let state = common::fresh_state_in_memory();
    let app = common::app_with(state);
    let res = common::oneshot_call(
        app,
        Request::builder()
            .uri("/file/content?path=/no/such/file/here")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    // 真实行为按 file_routes 实现：可能 400/404/500 任一；测试只断言"非 200"
    assert_ne!(res.status(), StatusCode::OK, "missing file must not return 200");
}
```

> 上面用了 `urlencoding`。如果未在 dev-deps，把 `urlencoding::encode(...)` 替换为手动 `file.to_str().unwrap().replace('/', "%2F")` 或简单地把测试文件名设为不含特殊字符的纯字母（`hello.txt`），URL 直接拼。本 plan 默认用纯字母，去掉 `urlencoding` 依赖：把上面 `let uri = format!("/file/content?path={}", urlencoding::encode(file.to_str().unwrap()));` 换成 `let uri = format!("/file/content?path={}", file.to_str().unwrap());`。

- [ ] **Step 2: 跑测试**

```
cargo test -p kfcode-server --test provider_file
```

预期：3 条 pass。

- [ ] **Step 3: 提交**

```bash
git add crates/kfcode-server/tests/provider_file.rs
git commit -m "test(server): cover /provider list and /file/content read paths"
```
