# T06 — `TodoRepository` CRUD 集成测试

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 覆盖 `TodoRepository::{list_for_session, upsert, delete, delete_for_session}` 与外键级联。

**Architecture:** `tests/todos.rs`，复用 `make_session`。`TodoItem` 是简单 struct（id/content/status/priority/position）。

**Tech Stack:** sqlx 0.8 / `kfcode_storage::TodoItem`。

**依赖:** T01 / T02 / T04

---

### Task 1.5：TodoRepository 集成测试

**Files:**
- Create: `crates/kfcode-storage/tests/todos.rs`

- [ ] **Step 1: 写失败测试**

写入 `crates/kfcode-storage/tests/todos.rs`：

```rust
mod common;

use kfcode_storage::{SessionRepository, TodoItem, TodoRepository};

fn todo(id: &str, content: &str, position: i64) -> TodoItem {
    TodoItem {
        id: id.to_string(),
        content: content.to_string(),
        status: "pending".to_string(),
        priority: "normal".to_string(),
        position,
    }
}

async fn setup_session(db: &kfcode_storage::Database, sid: &str) {
    let repo = SessionRepository::new(db.pool().clone());
    repo.create(&common::make_session(sid, "p")).await.unwrap();
}

#[tokio::test]
async fn list_returns_empty_for_new_session() {
    let db = common::fresh_db().await;
    setup_session(&db, "s").await;
    let repo = TodoRepository::new(db.pool().clone());
    let list = repo.list_for_session("s").await.unwrap();
    assert!(list.is_empty());
}

#[tokio::test]
async fn upsert_inserts_and_updates() {
    let db = common::fresh_db().await;
    setup_session(&db, "s").await;
    let repo = TodoRepository::new(db.pool().clone());

    repo.upsert("s", &todo("t1", "first", 0)).await.expect("insert");
    let list = repo.list_for_session("s").await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].content, "first");

    let mut updated = todo("t1", "first updated", 0);
    updated.status = "done".to_string();
    repo.upsert("s", &updated).await.expect("update");
    let list = repo.list_for_session("s").await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].content, "first updated");
    assert_eq!(list[0].status, "done");
}

#[tokio::test]
async fn list_orders_by_position() {
    let db = common::fresh_db().await;
    setup_session(&db, "s").await;
    let repo = TodoRepository::new(db.pool().clone());
    repo.upsert("s", &todo("c", "third", 2)).await.unwrap();
    repo.upsert("s", &todo("a", "first", 0)).await.unwrap();
    repo.upsert("s", &todo("b", "second", 1)).await.unwrap();
    let list = repo.list_for_session("s").await.unwrap();
    let positions: Vec<i64> = list.iter().map(|t| t.position).collect();
    assert_eq!(positions, vec![0, 1, 2], "expected ascending position order");
}

#[tokio::test]
async fn delete_removes_single_todo() {
    let db = common::fresh_db().await;
    setup_session(&db, "s").await;
    let repo = TodoRepository::new(db.pool().clone());
    repo.upsert("s", &todo("a", "x", 0)).await.unwrap();
    repo.upsert("s", &todo("b", "y", 1)).await.unwrap();
    repo.delete("s", "a").await.expect("delete");
    let list = repo.list_for_session("s").await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, "b");
}

#[tokio::test]
async fn delete_for_session_clears_all() {
    let db = common::fresh_db().await;
    setup_session(&db, "s").await;
    let repo = TodoRepository::new(db.pool().clone());
    repo.upsert("s", &todo("a", "x", 0)).await.unwrap();
    repo.upsert("s", &todo("b", "y", 1)).await.unwrap();
    repo.delete_for_session("s").await.expect("delete_for_session");
    assert!(repo.list_for_session("s").await.unwrap().is_empty());
}

#[tokio::test]
async fn deleting_session_cascades_todos() {
    let db = common::fresh_db().await;
    setup_session(&db, "s").await;
    let trepo = TodoRepository::new(db.pool().clone());
    trepo.upsert("s", &todo("a", "x", 0)).await.unwrap();
    SessionRepository::new(db.pool().clone()).delete("s").await.unwrap();
    // todos 表 FK ON DELETE CASCADE 必须把 todo 一起删掉
    let list = trepo.list_for_session("s").await.unwrap();
    assert!(list.is_empty(), "cascade should remove todos");
}

#[tokio::test]
async fn rejects_todo_for_nonexistent_session() {
    let db = common::fresh_db().await;
    let repo = TodoRepository::new(db.pool().clone());
    let err = repo.upsert("missing", &todo("a", "x", 0)).await.expect_err("FK");
    let msg = format!("{err}").to_uppercase();
    assert!(msg.contains("FOREIGN KEY") || msg.contains("CONSTRAINT"), "got: {msg}");
}
```

- [ ] **Step 2: 跑测试**

```
cargo test -p kfcode-storage --test todos
```

预期：7 条全 pass。

- [ ] **Step 3: 提交**

```bash
git add crates/kfcode-storage/tests/todos.rs
git commit -m "$(cat <<'EOF'
test(storage): cover TodoRepository CRUD with FK cascade

Empty list, upsert insert/update, position ordering, delete single +
bulk, cascade on session delete, and FK rejection.
EOF
)"
```
