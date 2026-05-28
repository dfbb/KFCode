# T03 — 新增 `Database::open_at(path)` 多连接入口

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让集成测试能用 tempfile 路径打开多连接 SQLite，覆盖 `Database::in_memory()` 单连接覆盖不到的并发/锁场景（spec §2.5 方式 2）。

**Architecture:** 在 `Database` 上加 `pub async fn open_at(path: &Path) -> Result<Self, DatabaseError>`，复用 `new()` 的 pool 构造逻辑（含 `after_connect` PRAGMA），但接受外部 path、不调用 `get_database_path()`。`new()` 改为调用 `open_at(get_database_path()?)`。

**Tech Stack:** sqlx 0.8 / SQLite / std::path。

**依赖:** T02（PRAGMA via `after_connect`）

---

### Task 1.2：`Database::open_at`

**Files:**
- Modify: `crates/kfcode-storage/src/database.rs:45-67`（重构 `new()` 调用 `open_at`）
- Modify: `crates/kfcode-storage/src/database.rs:88+`（新增 `open_at` 方法）
- Create: `crates/kfcode-storage/tests/open_at.rs`
- Modify: `crates/kfcode-storage/tests/common/mod.rs`（追加 `fresh_tempdir_db` helper）

- [ ] **Step 1: 写失败测试**

写入 `crates/kfcode-storage/tests/open_at.rs`：

```rust
mod common;

use kfcode_storage::Database;
use tempfile::TempDir;

#[tokio::test]
async fn open_at_creates_db_at_explicit_path() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.db");
    let _db = Database::open_at(&path).await.expect("open at tempdir");
    assert!(path.exists(), "db file should be created at {}", path.display());
}

#[tokio::test]
async fn open_at_runs_migrations() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.db");
    let db = Database::open_at(&path).await.expect("open");

    // sessions 表应当存在
    let row: (i64,) = sqlx::query_as("SELECT count(*) FROM sqlite_master WHERE type='table' AND name='sessions'")
        .fetch_one(db.pool())
        .await
        .expect("query");
    assert_eq!(row.0, 1);
}

#[tokio::test]
async fn open_at_reopen_preserves_data() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.db");

    {
        let db = Database::open_at(&path).await.unwrap();
        sqlx::query("INSERT INTO sessions (id, slug, project_id, directory, title, version, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)")
            .bind("ses-persist").bind("persist").bind("proj").bind("/tmp")
            .bind("t").bind("1").bind("active").bind(0_i64).bind(0_i64)
            .execute(db.pool()).await.unwrap();
    }

    let db = Database::open_at(&path).await.unwrap();
    let row: (i64,) = sqlx::query_as("SELECT count(*) FROM sessions WHERE id = 'ses-persist'")
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert_eq!(row.0, 1, "data should persist across reopens");
}
```

- [ ] **Step 2: 跑测试，确认 fail（编译错误：`open_at` 不存在）**

```
cargo test -p kfcode-storage --test open_at
```

预期：编译失败 `no function or associated item named open_at found`。

- [ ] **Step 3: 实现 `Database::open_at`，并把 `new()` 改为调用它**

修改 `crates/kfcode-storage/src/database.rs`：

```rust
impl Database {
    /// 在用户默认数据目录打开数据库。
    pub async fn new() -> Result<Self, DatabaseError> {
        let db_path = Self::get_database_path()?;
        Self::open_at(&db_path).await
    }

    /// 在显式 path 打开数据库（用于集成测试 / 自定义部署）。多连接 pool。
    pub async fn open_at(path: &std::path::Path) -> Result<Self, DatabaseError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;
        }

        let db_url = format!("sqlite:{}?mode=rwc", path.display());

        info!("Connecting to database at {}", path.display());

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .after_connect(|conn, _meta| Box::pin(async move {
                sqlx::query("PRAGMA foreign_keys = ON")
                    .execute(conn)
                    .await
                    .map(|_| ())
            }))
            .connect(&db_url)
            .await
            .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

        let db = Self { pool };
        db.run_migrations().await?;
        Ok(db)
    }

    pub async fn in_memory() -> Result<Self, DatabaseError> {
        // 保持原样：单连接 + after_connect PRAGMA。
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .after_connect(|conn, _meta| Box::pin(async move {
                sqlx::query("PRAGMA foreign_keys = ON")
                    .execute(conn)
                    .await
                    .map(|_| ())
            }))
            .connect("sqlite::memory:")
            .await
            .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

        let db = Self { pool };
        db.run_migrations().await?;
        Ok(db)
    }

    // ...其它方法不变
}
```

- [ ] **Step 4: 在 `tests/common/mod.rs` 追加 helper**

把 `crates/kfcode-storage/tests/common/mod.rs` 改成：

```rust
#![allow(dead_code)]

use kfcode_storage::{Database, DatabaseError};
use tempfile::TempDir;

pub async fn fresh_db() -> Database {
    Database::in_memory().await.expect("init in-memory db")
}

/// 多连接 tempdir 数据库；返回 (Database, TempDir)，drop TempDir 时清理文件。
pub async fn fresh_tempdir_db() -> (Database, TempDir) {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("test.db");
    let db = Database::open_at(&path).await.expect("open_at");
    (db, dir)
}

pub use kfcode_storage::DatabaseError as DbErr;
```

- [ ] **Step 5: 跑测试**

```
cargo test -p kfcode-storage --test open_at
cargo test -p kfcode-storage
```

预期：3 条 open_at 测试全 pass，全 crate 无回归。

- [ ] **Step 6: 提交**

```bash
git add crates/kfcode-storage/src/database.rs crates/kfcode-storage/tests/open_at.rs crates/kfcode-storage/tests/common/mod.rs
git commit -m "$(cat <<'EOF'
feat(storage): add Database::open_at for explicit-path connections

Database::new() walks dirs::data_local_dir(), preventing tests from
isolating storage from user state. Add Database::open_at(path) that
takes an explicit path and runs the same pool/PRAGMA setup. new() now
delegates to open_at(get_database_path()).

Use it via tests/common/fresh_tempdir_db() for multi-connection /
concurrency / reopen-persistence tests.
EOF
)"
```
