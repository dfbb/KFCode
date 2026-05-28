# T08 — `permissions` 表 schema/migration（最小 SQL）

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 覆盖 `permissions` 表的存在性、列定义、PK 约束。当前 crate 没有 `PermissionRepository`，本 task 限于 schema/migration 级断言（spec §3.1 storage 黄金路径明确允许此情况下写最小 SQL）。

**Architecture:** `tests/permissions_schema.rs`，用 `sqlx::query_as` 读 `PRAGMA table_info(permissions)` 与 INSERT/SELECT。

**Tech Stack:** sqlx 0.8 / 直接 SQL。

**依赖:** T01 / T02

---

### Task 1.7：permissions 表 schema 测试

**Files:**
- Create: `crates/kfcode-storage/tests/permissions_schema.rs`

- [ ] **Step 1: 写失败测试**

写入 `crates/kfcode-storage/tests/permissions_schema.rs`：

```rust
mod common;

use sqlx::Row;

#[tokio::test]
async fn permissions_table_exists_after_migrations() {
    let db = common::fresh_db().await;
    let row: (i64,) = sqlx::query_as(
        "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='permissions'",
    )
    .fetch_one(db.pool())
    .await
    .unwrap();
    assert_eq!(row.0, 1, "permissions table missing");
}

#[tokio::test]
async fn permissions_table_has_expected_columns() {
    let db = common::fresh_db().await;
    let rows = sqlx::query("PRAGMA table_info(permissions)")
        .fetch_all(db.pool())
        .await
        .unwrap();
    let names: Vec<String> = rows.iter().map(|r| r.get::<String, _>("name")).collect();
    for col in ["project_id", "created_at", "updated_at", "data"] {
        assert!(names.contains(&col.to_string()), "missing column {col}; got {names:?}");
    }
}

#[tokio::test]
async fn permissions_project_id_is_primary_key() {
    let db = common::fresh_db().await;
    let rows = sqlx::query("PRAGMA table_info(permissions)")
        .fetch_all(db.pool())
        .await
        .unwrap();
    let pk_col = rows
        .iter()
        .find(|r| r.get::<i64, _>("pk") > 0)
        .expect("must have a primary key");
    assert_eq!(pk_col.get::<String, _>("name"), "project_id");
}

#[tokio::test]
async fn permissions_round_trip_insert_and_select() {
    let db = common::fresh_db().await;
    sqlx::query("INSERT INTO permissions (project_id, created_at, updated_at, data) VALUES (?, ?, ?, ?)")
        .bind("proj-x")
        .bind(0_i64)
        .bind(0_i64)
        .bind("[]")
        .execute(db.pool())
        .await
        .unwrap();

    let row: (String, String) = sqlx::query_as("SELECT project_id, data FROM permissions WHERE project_id = ?")
        .bind("proj-x")
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert_eq!(row.0, "proj-x");
    assert_eq!(row.1, "[]");
}

#[tokio::test]
async fn permissions_rejects_duplicate_project_id() {
    let db = common::fresh_db().await;
    sqlx::query("INSERT INTO permissions (project_id, created_at, updated_at, data) VALUES (?, ?, ?, ?)")
        .bind("dup")
        .bind(0_i64)
        .bind(0_i64)
        .bind("[]")
        .execute(db.pool())
        .await
        .unwrap();

    let err = sqlx::query("INSERT INTO permissions (project_id, created_at, updated_at, data) VALUES (?, ?, ?, ?)")
        .bind("dup")
        .bind(0_i64)
        .bind(0_i64)
        .bind("[]")
        .execute(db.pool())
        .await
        .unwrap_err();
    let msg = format!("{err}").to_uppercase();
    assert!(msg.contains("UNIQUE") || msg.contains("CONSTRAINT"), "got: {msg}");
}
```

- [ ] **Step 2: 跑测试**

```
cargo test -p kfcode-storage --test permissions_schema
```

预期：5 条全 pass。

- [ ] **Step 3: 提交**

```bash
git add crates/kfcode-storage/tests/permissions_schema.rs
git commit -m "$(cat <<'EOF'
test(storage): cover permissions table schema and constraints

permissions table currently has no Rust repository wrapper, so the
integration tests work at schema/migration level via raw SQL: table
exists, columns present, project_id is PK, round-trip insert/select,
and PK uniqueness. Per spec §3.1 storage section.
EOF
)"
```
