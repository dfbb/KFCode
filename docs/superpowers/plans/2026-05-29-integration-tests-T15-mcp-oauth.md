# T15 — MCP OAuth flow（wiremock + 注入 path）

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 验证 OAuth 凭据存取走 `AuthStore`（不触碰用户真实 path）；token 过期检测；refresh 流程能更新 store。

**Architecture:** 直接对 `AuthStore` API 做端到端测试（不引入完整 `oauth2` crate 流程，那要起授权页 mock，过重）；refresh 用 wiremock 模拟 token endpoint。这条 task 在 T11 注入改造之上写出"凭据隔离 + token 生命周期"的覆盖。

**Tech Stack:** wiremock / `kfcode_mcp::auth::AuthStore` / `kfcode_mcp::oauth::*`。

**依赖:** T10 / T11

---

### Task 2.5：OAuth flow

**Files:**
- Create: `crates/kfcode-mcp/tests/mcp_oauth.rs`

- [ ] **Step 1: 写失败测试**

写入 `crates/kfcode-mcp/tests/mcp_oauth.rs`：

```rust
mod common;

use kfcode_mcp::auth::{AuthEntry, AuthStore, OAuthTokens, is_token_expired};

#[tokio::test]
async fn token_expiry_detection() {
    // expires_at 在过去 → expired
    let mut e = AuthEntry::default();
    e.tokens = Some(OAuthTokens {
        access_token: "old".into(),
        refresh_token: None,
        expires_at: Some(0.0),
        scope: None,
    });
    assert_eq!(is_token_expired(&e), Some(true));

    // expires_at 在未来 → not expired
    let mut e2 = AuthEntry::default();
    let future = (chrono::Utc::now().timestamp() + 3600) as f64;
    e2.tokens = Some(OAuthTokens {
        access_token: "fresh".into(),
        refresh_token: None,
        expires_at: Some(future),
        scope: None,
    });
    assert_eq!(is_token_expired(&e2), Some(false));

    // 缺少 expires_at → None
    let mut e3 = AuthEntry::default();
    e3.tokens = Some(OAuthTokens {
        access_token: "x".into(), refresh_token: None, expires_at: None, scope: None,
    });
    assert_eq!(is_token_expired(&e3), None);
}

#[tokio::test]
async fn store_persists_across_reload() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("auth.json");

    {
        let store = AuthStore::new(&path);
        let mut e = AuthEntry::default();
        e.tokens = Some(OAuthTokens {
            access_token: "tok".into(),
            refresh_token: Some("ref".into()),
            expires_at: None,
            scope: None,
        });
        e.server_url = Some("https://api.example.com".into());
        store.set("server-1", e).await.unwrap();
    }

    let store = AuthStore::new(&path);
    let got = store.get("server-1").await.expect("must persist");
    assert_eq!(got.tokens.unwrap().access_token, "tok");
    assert_eq!(got.server_url.as_deref(), Some("https://api.example.com"));
}

#[tokio::test]
async fn get_for_url_invalidates_on_url_change() {
    let dir = tempfile::TempDir::new().unwrap();
    let store = AuthStore::new(dir.path().join("auth.json"));
    let mut e = AuthEntry::default();
    e.server_url = Some("https://old".into());
    store.set("name", e).await.unwrap();

    assert!(store.get_for_url("name", "https://old").await.is_some());
    assert!(
        store.get_for_url("name", "https://new").await.is_none(),
        "URL change must invalidate"
    );
}
```

- [ ] **Step 2: 跑测试**

```
cargo test -p kfcode-mcp --test mcp_oauth
```

预期：3 条 pass。

- [ ] **Step 3: 提交**

```bash
git add crates/kfcode-mcp/tests/mcp_oauth.rs
git commit -m "$(cat <<'EOF'
test(mcp): cover OAuth token expiry detection and AuthStore persistence

Each test gets its own AuthStore at a tempdir path (per spec §2.5)
so they run in parallel without touching the real ~/.local/share
or each other.
EOF
)"
```

> 完整的 authorization-code 流程（启动 listener、走 OAuth2 client、回调）涉及 oauth2 crate 与本地 redirect server 协作，超出最小集成测试范围；本 task 限于 AuthStore 行为与 token 检测，更深的 flow 可在后续 batch 增加。
