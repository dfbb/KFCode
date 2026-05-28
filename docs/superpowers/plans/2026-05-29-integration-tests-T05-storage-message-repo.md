# T05 — `MessageRepository` CRUD 集成测试

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 覆盖 `MessageRepository::{create, upsert, get, list_for_session, delete, delete_for_session}` 与外键级联删除（依赖 T02 PRAGMA）。

**Architecture:** 在 `tests/messages.rs` 内组织。每个测试先 create 一个父 session（FK 必须），再操作 messages。`SessionMessage` fixture 用 helper 构造。

**Tech Stack:** sqlx 0.8 / `kfcode_types::message::SessionMessage`。

**依赖:** T01 / T02 / T04（沿用 `make_session` helper）

---

### Task 1.4：MessageRepository 集成测试

**Files:**
- Modify: `crates/kfcode-storage/tests/common/mod.rs`（追加 `make_message`）
- Create: `crates/kfcode-storage/tests/messages.rs`

- [ ] **Step 1: 在 common 加 message fixture**

在 `crates/kfcode-storage/tests/common/mod.rs` 追加：

```rust
use kfcode_types::message::{MessageRole, SessionMessage};

pub fn make_message(id: &str, session_id: &str, role: MessageRole) -> SessionMessage {
    SessionMessage {
        id: id.to_string(),
        session_id: session_id.to_string(),
        role,
        parts: Vec::new(),
        created_at: Utc::now(),
        metadata: HashMap::new(),
    }
}
```

- [ ] **Step 2: 写失败测试**

写入 `crates/kfcode-storage/tests/messages.rs`：

```rust
mod common;

use kfcode_storage::{MessageRepository, SessionRepository};
use kfcode_types::message::MessageRole;

async fn setup_session(db: &kfcode_storage::Database, sid: &str) {
    let repo = SessionRepository::new(db.pool().clone());
    repo.create(&common::make_session(sid, "p")).await.unwrap();
}

#[tokio::test]
async fn round_trips_user_message() {
    let db = common::fresh_db().await;
    setup_session(&db, "s").await;
    let repo = MessageRepository::new(db.pool().clone());
    let m = common::make_message("msg-1", "s", MessageRole::User);
    repo.create(&m).await.expect("create");
    let got = repo.get("msg-1").await.unwrap().unwrap();
    assert_eq!(got.id, "msg-1");
    assert_eq!(got.role, MessageRole::User);
}

#[tokio::test]
async fn rejects_message_for_nonexistent_session() {
    let db = common::fresh_db().await;
    let repo = MessageRepository::new(db.pool().clone());
    let m = common::make_message("msg-orphan", "no-such-session", MessageRole::User);
    let err = repo.create(&m).await.expect_err("expected FK error");
    let msg = format!("{err}").to_uppercase();
    assert!(msg.contains("FOREIGN KEY") || msg.contains("CONSTRAINT"), "got: {msg}");
}

#[tokio::test]
async fn upsert_overwrites_existing_row() {
    let db = common::fresh_db().await;
    setup_session(&db, "s").await;
    let repo = MessageRepository::new(db.pool().clone());
    let mut m = common::make_message("msg", "s", MessageRole::User);
    repo.create(&m).await.unwrap();
    m.role = MessageRole::Assistant;
    repo.upsert(&m).await.expect("upsert");
    let got = repo.get("msg").await.unwrap().unwrap();
    assert_eq!(got.role, MessageRole::Assistant);
}

#[tokio::test]
async fn list_for_session_returns_only_session_messages() {
    let db = common::fresh_db().await;
    setup_session(&db, "s1").await;
    setup_session(&db, "s2").await;
    let repo = MessageRepository::new(db.pool().clone());
    repo.create(&common::make_message("a", "s1", MessageRole::User)).await.unwrap();
    repo.create(&common::make_message("b", "s2", MessageRole::User)).await.unwrap();
    repo.create(&common::make_message("c", "s1", MessageRole::Assistant)).await.unwrap();

    let list1 = repo.list_for_session("s1").await.unwrap();
    assert_eq!(list1.len(), 2);
    assert!(list1.iter().all(|m| m.session_id == "s1"));

    let list2 = repo.list_for_session("s2").await.unwrap();
    assert_eq!(list2.len(), 1);
}

#[tokio::test]
async fn delete_removes_single_message() {
    let db = common::fresh_db().await;
    setup_session(&db, "s").await;
    let repo = MessageRepository::new(db.pool().clone());
    repo.create(&common::make_message("m1", "s", MessageRole::User)).await.unwrap();
    repo.create(&common::make_message("m2", "s", MessageRole::User)).await.unwrap();
    repo.delete("m1").await.expect("delete");
    assert!(repo.get("m1").await.unwrap().is_none());
    assert!(repo.get("m2").await.unwrap().is_some());
}

#[tokio::test]
async fn delete_for_session_clears_only_target() {
    let db = common::fresh_db().await;
    setup_session(&db, "s1").await;
    setup_session(&db, "s2").await;
    let repo = MessageRepository::new(db.pool().clone());
    repo.create(&common::make_message("a", "s1", MessageRole::User)).await.unwrap();
    repo.create(&common::make_message("b", "s2", MessageRole::User)).await.unwrap();
    repo.delete_for_session("s1").await.expect("delete_for_session");
    assert!(repo.list_for_session("s1").await.unwrap().is_empty());
    assert_eq!(repo.list_for_session("s2").await.unwrap().len(), 1);
}

#[tokio::test]
async fn deleting_session_cascades_messages() {
    let db = common::fresh_db().await;
    setup_session(&db, "s").await;
    let mrepo = MessageRepository::new(db.pool().clone());
    mrepo.create(&common::make_message("m", "s", MessageRole::User)).await.unwrap();
    let srepo = SessionRepository::new(db.pool().clone());
    srepo.delete("s").await.expect("session delete");
    assert!(mrepo.get("m").await.unwrap().is_none(), "message should be cascaded");
}
```

- [ ] **Step 3: 跑测试**

```
cargo test -p kfcode-storage --test messages
```

预期：7 条全 pass。`deleting_session_cascades_messages` 依赖 T02 的 `foreign_keys=ON`——若 fail，回查 T02 是否合并。

- [ ] **Step 4: 提交**

```bash
git add crates/kfcode-storage/tests/messages.rs crates/kfcode-storage/tests/common/mod.rs
git commit -m "$(cat <<'EOF'
test(storage): cover MessageRepository CRUD and FK cascade

Round-trip, FK rejection for orphan, upsert idempotency, list scoped
by session, single + bulk delete, and cascade-on-session-delete (which
requires PRAGMA foreign_keys=ON from T02).
EOF
)"
```
