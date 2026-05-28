# T11 — mcp `auth.rs` 显式 path 注入改造

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 `kfcode-mcp::auth` 模块改造成"接受显式 path"的形态，让集成测试每个测试用独立 tempdir，避免触碰用户真实 `data_dir` 与并行竞态（spec §2.5 OAuth 条目方案 1，禁用全局 setter）。

**Architecture:** 引入 `pub struct AuthStore { path: PathBuf }`，保留所有现有 free functions 作为 `AuthStore` 的方法；现有 free functions 改成"调用 `AuthStore::default_user_store()`"以保持调用者无感。`default_user_store()` 内部走 `dirs::data_dir()`。

**Tech Stack:** Rust async / tokio::fs。

**依赖:** T10

---

### Task 2.1：auth.rs path 注入

**Files:**
- Modify: `crates/kfcode-mcp/src/auth.rs`（约 200 行整体重构）
- Create: `crates/kfcode-mcp/tests/auth_store.rs`
- Modify: `crates/kfcode-mcp/tests/common/mod.rs`（追加 `fresh_auth_store`）

- [ ] **Step 1: 写失败测试**

写入 `crates/kfcode-mcp/tests/auth_store.rs`：

```rust
mod common;

use kfcode_mcp::auth::{AuthEntry, AuthStore, OAuthTokens};
use tempfile::TempDir;

fn fresh_store() -> (AuthStore, TempDir) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("mcp-auth.json");
    (AuthStore::new(path), dir)
}

#[tokio::test]
async fn round_trips_auth_entry() {
    let (store, _dir) = fresh_store();
    let mut entry = AuthEntry::default();
    entry.tokens = Some(OAuthTokens {
        access_token: "tok".into(),
        refresh_token: Some("ref".into()),
        expires_at: None,
        scope: None,
    });
    store.set("server-1", entry).await.expect("set");
    let got = store.get("server-1").await.expect("must exist");
    assert_eq!(got.tokens.unwrap().access_token, "tok");
}

#[tokio::test]
async fn isolates_independent_stores() {
    let (a, _da) = fresh_store();
    let (b, _db) = fresh_store();
    let mut e = AuthEntry::default();
    e.server_url = Some("https://a".into());
    a.set("name", e).await.unwrap();
    assert!(b.get("name").await.is_none(), "stores at distinct paths must not bleed");
}

#[tokio::test]
async fn remove_clears_entry() {
    let (store, _dir) = fresh_store();
    store.set("x", AuthEntry::default()).await.unwrap();
    store.remove("x").await.expect("remove");
    assert!(store.get("x").await.is_none());
}

#[tokio::test]
async fn update_tokens_preserves_other_fields() {
    let (store, _dir) = fresh_store();
    let mut e = AuthEntry::default();
    e.server_url = Some("https://example".into());
    store.set("x", e).await.unwrap();

    store
        .update_tokens(
            "x",
            OAuthTokens {
                access_token: "new".into(),
                refresh_token: None,
                expires_at: None,
                scope: None,
            },
        )
        .await
        .expect("update_tokens");

    let got = store.get("x").await.unwrap();
    assert_eq!(got.tokens.unwrap().access_token, "new");
    assert_eq!(got.server_url.as_deref(), Some("https://example"));
}
```

- [ ] **Step 2: 跑测试，确认 fail（编译错误：`AuthStore` 不存在）**

```
cargo test -p kfcode-mcp --test auth_store
```

预期：编译失败 `cannot find type AuthStore in module kfcode_mcp::auth`。

- [ ] **Step 3: 重构 auth.rs**

把 `crates/kfcode-mcp/src/auth.rs` 重构为以下骨架（保持现有 free functions 为兼容外壳）：

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;

// ... (保留 OAuthTokens / OAuthClientInfo / AuthEntry 定义不变)

/// 单个 auth JSON 文件的存储抽象，可注入 path 以便测试隔离。
pub struct AuthStore {
    path: PathBuf,
}

impl AuthStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// 用户默认 store，走 `dirs::data_dir() / kfcode / mcp-auth.json`。
    pub fn default_user_store() -> Self {
        let path = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("kfcode")
            .join("mcp-auth.json");
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    async fn read_all(&self) -> HashMap<String, AuthEntry> {
        match fs::read_to_string(&self.path).await {
            Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
            Err(_) => HashMap::new(),
        }
    }

    async fn write_all(&self, data: &HashMap<String, AuthEntry>) -> Result<(), std::io::Error> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).await?;
        }
        let json = serde_json::to_string_pretty(data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        fs::write(&self.path, json).await
    }

    pub async fn get(&self, mcp_name: &str) -> Option<AuthEntry> {
        self.read_all().await.get(mcp_name).cloned()
    }

    pub async fn get_for_url(&self, mcp_name: &str, server_url: &str) -> Option<AuthEntry> {
        let entry = self.get(mcp_name).await?;
        if entry.server_url.as_deref() == Some(server_url) {
            Some(entry)
        } else {
            None
        }
    }

    pub async fn set(&self, mcp_name: &str, entry: AuthEntry) -> Result<(), std::io::Error> {
        let mut data = self.read_all().await;
        data.insert(mcp_name.to_string(), entry);
        self.write_all(&data).await
    }

    pub async fn remove(&self, mcp_name: &str) -> Result<(), std::io::Error> {
        let mut data = self.read_all().await;
        data.remove(mcp_name);
        self.write_all(&data).await
    }

    pub async fn update_tokens(
        &self,
        mcp_name: &str,
        tokens: OAuthTokens,
    ) -> Result<(), std::io::Error> {
        let mut data = self.read_all().await;
        let entry = data.entry(mcp_name.to_string()).or_default();
        entry.tokens = Some(tokens);
        self.write_all(&data).await
    }

    // 同样把 update_client_info / update_code_verifier / clear_code_verifier /
    // update_oauth_state / get_oauth_state / clear_oauth_state 都改成 self 方法。
    // 行为与现有 free functions 一致；这里略，照抄原有逻辑。
}

// 兼容外壳：保留所有原有 pub free functions，内部委托给 default_user_store()。
// 例如：
pub async fn get(mcp_name: &str) -> Option<AuthEntry> {
    AuthStore::default_user_store().get(mcp_name).await
}

pub async fn set(mcp_name: &str, entry: AuthEntry) -> Result<(), std::io::Error> {
    AuthStore::default_user_store().set(mcp_name, entry).await
}

// ...其余 free functions 同样委托 default_user_store。
// is_token_expired 不动（纯函数，不接 IO）。
```

> 关键：所有 free functions 必须保留并签名不变；只是把实现改成 `default_user_store()` 委托。这样 `oauth.rs` / `client.rs` 等调用者无需修改。

- [ ] **Step 4: 在 common 加 helper**

在 `crates/kfcode-mcp/tests/common/mod.rs` 追加：

```rust
use kfcode_mcp::auth::AuthStore;
use tempfile::TempDir;

pub fn fresh_auth_store() -> (AuthStore, TempDir) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("mcp-auth.json");
    (AuthStore::new(path), dir)
}
```

- [ ] **Step 5: 跑测试**

```
cargo test -p kfcode-mcp --test auth_store
cargo test -p kfcode-mcp
```

预期：4 条 auth_store 测试全 pass，全 crate 无回归（因为 free functions 仍可用）。

- [ ] **Step 6: 提交**

```bash
git add crates/kfcode-mcp/src/auth.rs crates/kfcode-mcp/tests/auth_store.rs crates/kfcode-mcp/tests/common/mod.rs
git commit -m "$(cat <<'EOF'
feat(mcp): introduce AuthStore for path-injectable auth persistence

Existing auth module wrote to dirs::data_dir() globally, making
integration tests touch the user's real auth file. Introduce
AuthStore { path } as the testable primitive; existing free
functions become thin wrappers around AuthStore::default_user_store().

Tests use AuthStore::new(tempdir.path()) to isolate per-test, no
global setter, no env vars (per spec §2.5).
EOF
)"
```
