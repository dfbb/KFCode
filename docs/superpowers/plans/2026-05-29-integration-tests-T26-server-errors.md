# T26 — server `ApiError` 全分支映射

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 验证 `ApiError` 各变体（`crates/kfcode-server/src/error.rs`）映射到正确 HTTP 状态码：400 / 404 / 502 / 500。

**Architecture:** 直接对 `IntoResponse` impl 单元化测试——不通过 router，构造 `ApiError` 实例调 `into_response`，断言 status 与 body 中的 `error.type` 字段。这条 task 实质是单元级，但放集成测试目录便于与其它 server 测试一起跑。

**Tech Stack:** axum::response::IntoResponse / serde_json。

**依赖:** T20

---

### Task 3.6：ApiError → HTTP status

**Files:**
- Create: `crates/kfcode-server/tests/api_errors.rs`

- [ ] **Step 1: 写测试**

写入 `crates/kfcode-server/tests/api_errors.rs`：

```rust
mod common;

use axum::body::to_bytes;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use kfcode_server::ApiError;
use serde_json::Value;

async fn status_and_type(err: ApiError) -> (StatusCode, String) {
    let res = err.into_response();
    let status = res.status();
    let body_bytes = to_bytes(res.into_body(), 1024 * 1024).await.unwrap();
    let v: Value = serde_json::from_slice(&body_bytes).unwrap();
    let t = v.get("error")
        .and_then(|e| e.get("type"))
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();
    (status, t)
}

#[tokio::test]
async fn session_not_found_maps_to_404() {
    let (s, t) = status_and_type(ApiError::SessionNotFound("x".into())).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    assert_eq!(t, "session_not_found");
}

#[tokio::test]
async fn not_found_maps_to_404() {
    let (s, t) = status_and_type(ApiError::NotFound("nope".into())).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    assert_eq!(t, "not_found");
}

#[tokio::test]
async fn bad_request_maps_to_400() {
    let (s, t) = status_and_type(ApiError::BadRequest("bad".into())).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    assert_eq!(t, "bad_request");
}

#[tokio::test]
async fn invalid_request_maps_to_400() {
    let (s, t) = status_and_type(ApiError::InvalidRequest("invalid".into())).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    assert_eq!(t, "invalid_request");
}

#[tokio::test]
async fn provider_error_maps_to_502() {
    let (s, t) = status_and_type(ApiError::ProviderError("upstream".into())).await;
    assert_eq!(s, StatusCode::BAD_GATEWAY);
    assert_eq!(t, "provider_error");
}

#[tokio::test]
async fn internal_error_maps_to_500() {
    let (s, t) = status_and_type(ApiError::InternalError("boom".into())).await;
    assert_eq!(s, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(t, "internal_error");
}
```

- [ ] **Step 2: 跑测试**

```
cargo test -p kfcode-server --test api_errors
```

预期：6 条 pass。

- [ ] **Step 3: 提交**

```bash
git add crates/kfcode-server/tests/api_errors.rs
git commit -m "test(server): cover ApiError variant -> HTTP status mapping (400/404/502/500)"
```

> 401/403 不在 ApiError 现有变体中（spec §3.1 已说明），等鉴权实现后另开 task 补。
