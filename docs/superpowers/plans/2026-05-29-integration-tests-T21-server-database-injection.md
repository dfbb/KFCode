# T21 — server `ServerState` Database 注入改造

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 `ServerState` 上加 `pub fn from_database(db: Database, ...) -> Self` 入口，让集成测试能注入用 `Database::open_at(tempdir)` 构造的实例，避免触碰 `dirs::data_local_dir()`（spec §2.5 server 启动条目）。

**Architecture:** `new_with_storage_for_url` 在 `crates/kfcode-server/src/server.rs:146` 内部直接 `Database::new()`。把这条逻辑抽出来，改名 `new_with_database`，接受 `Database` 参数；`new_with_storage_for_url` 调用 `Database::new()` + `new_with_database`。tests 走 `new_with_database` 注入 `Database::open_at`。

**Tech Stack:** `kfcode_storage::Database` / `kfcode_server::ServerState`。

**依赖:** Batch 1 T03（`Database::open_at` 已存在）

---

### Task 3.1：Database 注入

**Files:**
- Modify: `crates/kfcode-server/src/server.rs:140-179`（重构 storage 构造）
- Create: `crates/kfcode-server/tests/server_database_injection.rs`

- [ ] **Step 1: 写失败测试**

写入 `crates/kfcode-server/tests/server_database_injection.rs`：

```rust
mod common;

use kfcode_storage::Database;
use kfcode_server::ServerState;
use tempfile::TempDir;

#[tokio::test]
async fn server_state_accepts_external_database() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("server-test.db");
    let db = Database::open_at(&path).await.expect("open_at");

    // 走新 API
    let state = ServerState::new_with_database(db, "http://test".to_string())
        .await
        .expect("inject db");

    // 验证有 storage backend
    assert!(state.has_storage(), "state should have storage backend");
}

#[tokio::test]
async fn injected_database_persists_at_explicit_path() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("explicit.db");
    let db = Database::open_at(&path).await.unwrap();
    let _state = ServerState::new_with_database(db, "http://t".into())
        .await
        .expect("inject");

    assert!(path.exists(), "db file must be at explicit path");
}
```

> 上面用到 `state.has_storage()`——下一步把它加上。

- [ ] **Step 2: 跑测试，确认 fail**

```
cargo test -p kfcode-server --test server_database_injection
```

预期：编译失败 `no function or associated item named new_with_database` / `no method has_storage`。

- [ ] **Step 3: 重构 server.rs**

修改 `crates/kfcode-server/src/server.rs`：

1. 在 `ServerState` 上加 `has_storage()` 与 `new_with_database` 方法：

```rust
impl ServerState {
    /// 是否有 storage backend 注入。
    pub fn has_storage(&self) -> bool {
        self.session_repo.is_some() && self.message_repo.is_some()
    }

    /// 用外部传入的 `Database` 实例构造 state（测试与自定义部署用）。
    pub async fn new_with_database(
        db: Database,
        server_url: String,
    ) -> anyhow::Result<Self> {
        let mut state = Self::new();
        let auth_manager = Arc::new(AuthManager::load_from_file(&auth_data_dir()).await);
        state.auth_manager = auth_manager.clone();
        load_plugin_auth_store(&server_url, auth_manager.clone()).await;
        let auth_store = auth_manager.list().await;

        let cwd = std::env::current_dir().unwrap_or_default();
        let bootstrap_config = match load_config(&cwd) {
            Ok(config) => {
                let providers = convert_config_providers_for_bootstrap(&config);
                bootstrap_config_from_raw(
                    providers,
                    config.disabled_providers.clone(),
                    config.enabled_providers.clone(),
                    config.model.clone(),
                    config.small_model.clone(),
                )
            }
            Err(error) => {
                tracing::warn!(%error, "failed to load config for provider bootstrap, using defaults");
                kfcode_provider::BootstrapConfig::default()
            }
        };

        state.providers = create_registry_from_bootstrap_config(&bootstrap_config, &auth_store);
        let pool = db.pool().clone();
        state.session_repo = Some(SessionRepository::new(pool.clone()));
        state.message_repo = Some(MessageRepository::new(pool));
        state.load_sessions_from_storage().await?;
        Ok(state)
    }
}
```

2. 把 `new_with_storage_for_url` 改成调用 `new_with_database`：

```rust
pub async fn new_with_storage_for_url(server_url: String) -> anyhow::Result<Self> {
    let db = Database::new().await?;
    Self::new_with_database(db, server_url).await
}
```

- [ ] **Step 4: 跑测试**

```
cargo test -p kfcode-server --test server_database_injection
cargo test -p kfcode-server
```

预期：2 条注入测试 pass，全 crate 无回归。

- [ ] **Step 5: 提交**

```bash
git add crates/kfcode-server/src/server.rs crates/kfcode-server/tests/server_database_injection.rs
git commit -m "$(cat <<'EOF'
feat(server): inject Database into ServerState via new_with_database

new_with_storage_for_url buried Database::new() inside, leaving tests
no way to point storage at a tempdir. Extract a public
ServerState::new_with_database(db, server_url) and have the existing
constructor delegate to it. Adds has_storage() observer for tests.
EOF
)"
```
