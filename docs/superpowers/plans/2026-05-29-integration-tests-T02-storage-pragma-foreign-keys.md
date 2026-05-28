# T02 — 启用 SQLite 外键约束 PRAGMA

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 修复 `kfcode-storage` 当前未启用 SQLite 外键约束的 bug；后续外键测试依赖此修复。

**Architecture:** 在 `Database::new()` 与 `Database::in_memory()` 公共路径上，连接成功后立即执行 `PRAGMA foreign_keys = ON`；并在 `Database` 上加一个 `pragma_foreign_keys_enabled` getter 用于断言。

**Tech Stack:** sqlx 0.8 / SQLite。

**依赖:** T01（已有 fresh_db helper）

---

### Task 1.1：PRAGMA foreign_keys=ON

**Files:**
- Modify: `crates/kfcode-storage/src/database.rs:45-87`（`new` / `in_memory`）
- Modify: `crates/kfcode-storage/src/database.rs:88-95`（追加 getter）
- Create: `crates/kfcode-storage/tests/pragma.rs`

- [ ] **Step 1: 写失败测试**

写入 `crates/kfcode-storage/tests/pragma.rs`：

```rust
mod common;

use sqlx::Row;

#[tokio::test]
async fn in_memory_enables_foreign_keys() {
    let db = common::fresh_db().await;
    let row = sqlx::query("PRAGMA foreign_keys")
        .fetch_one(db.pool())
        .await
        .expect("query pragma");
    let value: i64 = row.get(0);
    assert_eq!(value, 1, "foreign_keys must be ON for in-memory db");
}

#[tokio::test]
async fn rejects_orphan_message_when_session_missing() {
    let db = common::fresh_db().await;
    let res = sqlx::query("INSERT INTO messages (id, session_id, role, created_at) VALUES (?, ?, ?, ?)")
        .bind("msg-orphan")
        .bind("session-does-not-exist")
        .bind("user")
        .bind(0_i64)
        .execute(db.pool())
        .await;
    assert!(res.is_err(), "expected FK violation for orphan message");
    let err = res.unwrap_err().to_string();
    assert!(err.contains("FOREIGN KEY") || err.contains("constraint"), "got: {err}");
}
```

- [ ] **Step 2: 跑测试，确认两条都 fail**

```
cargo test -p kfcode-storage --test pragma
```

预期：两条都 fail（`foreign_keys` PRAGMA 默认值为 0；FK 违反不会触发）。

- [ ] **Step 3: 在 `Database::in_memory` 与 `Database::new` 启用 PRAGMA**

修改 `crates/kfcode-storage/src/database.rs`。在 `new()` 与 `in_memory()` 内、`db.run_migrations().await?;` 之**前**，加入一段统一调用：

```rust
        // 在 new() 内：pool 创建之后、 run_migrations 之前
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;
```

```rust
        // 在 in_memory() 内：pool 创建之后、 run_migrations 之前
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;
```

> 注意：SQLite 的 `foreign_keys` PRAGMA 是 per-connection 的；本 crate 的连接池配置使其在每个连接上都启用——`new()` 的 pool 默认 `max_connections=5`，**必须**改为通过 `SqlitePoolOptions::after_connect` 在每个连接上执行 PRAGMA，而不是仅对当前一条连接执行。把 pool 构造改为：

```rust
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
```

`in_memory()` 同理（`max_connections=1`），用同一 `after_connect` 即可。删掉前面那两段独立的 `PRAGMA` 调用（被 after_connect 覆盖）。

- [ ] **Step 4: 跑测试，确认两条都 pass**

```
cargo test -p kfcode-storage --test pragma
```

预期：两条 pass。

- [ ] **Step 5: 跑全 crate 测试，确认无回归**

```
cargo test -p kfcode-storage
```

预期：全部 pass。

- [ ] **Step 6: 提交**

```bash
git add crates/kfcode-storage/src/database.rs crates/kfcode-storage/tests/pragma.rs
git commit -m "$(cat <<'EOF'
fix(storage): enable SQLite foreign_keys via after_connect

SQLite's foreign_keys pragma is per-connection and defaults to OFF, so
declared FK constraints in schema were not enforced. Use
SqlitePoolOptions::after_connect to run "PRAGMA foreign_keys = ON" on
every connection in both Database::new and Database::in_memory.

Adds tests/pragma.rs covering pragma value and FK violation behavior
(test discovered while writing integration tests per spec §3.1).
EOF
)"
```
