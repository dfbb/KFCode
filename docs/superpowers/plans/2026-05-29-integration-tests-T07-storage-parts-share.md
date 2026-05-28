# T07 — `PartRepository` + `ShareRepository` 集成测试（公开模块路径）

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 覆盖 `PartRepository` 与 `ShareRepository` 的黄金路径与 FK 级联。这两个 repo 当前未在 `lib.rs` re-export，但模块本身 `pub mod repository`，测试通过 `kfcode_storage::repository::PartRepository` / `ShareRepository` 直接可达。

**Architecture:** 一个文件覆盖两个 repo（用户偏好"5 个文件"，把 parts/share 合并）；`tests/parts_share.rs`。

**Tech Stack:** sqlx 0.8 / `kfcode_storage::repository::{PartRepository, PartRow, ShareRepository, SessionShareRow}`。

**依赖:** T01 / T02 / T04 / T05

---

### Task 1.6：PartRepository + ShareRepository 集成测试

**Files:**
- Create: `crates/kfcode-storage/tests/parts_share.rs`
- Optional: `crates/kfcode-storage/src/lib.rs`（如 plan 顺手补 re-export，则在本 task 单独 commit）

- [ ] **Step 1: 写失败测试**

写入 `crates/kfcode-storage/tests/parts_share.rs`：

```rust
mod common;

use kfcode_storage::{MessageRepository, SessionRepository};
use kfcode_storage::repository::{PartRepository, PartRow, ShareRepository, SessionShareRow};
use kfcode_types::message::MessageRole;

async fn setup(db: &kfcode_storage::Database) {
    SessionRepository::new(db.pool().clone())
        .create(&common::make_session("s", "p"))
        .await
        .unwrap();
    MessageRepository::new(db.pool().clone())
        .create(&common::make_message("m", "s", MessageRole::User))
        .await
        .unwrap();
}

fn text_part(id: &str, text: &str, order: i64) -> PartRow {
    PartRow {
        id: id.to_string(),
        message_id: "m".to_string(),
        session_id: "s".to_string(),
        part_type: "text".to_string(),
        text: Some(text.to_string()),
        tool_name: None,
        tool_call_id: None,
        tool_arguments: None,
        tool_result: None,
        tool_error: None,
        tool_status: None,
        sort_order: order,
    }
}

#[tokio::test]
async fn parts_upsert_and_list_in_sort_order() {
    let db = common::fresh_db().await;
    setup(&db).await;
    let repo = PartRepository::new(db.pool().clone());
    repo.upsert(&text_part("p2", "second", 1)).await.unwrap();
    repo.upsert(&text_part("p1", "first", 0)).await.unwrap();

    let list = repo.list_for_message("m").await.unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].id, "p1");
    assert_eq!(list[1].id, "p2");
}

#[tokio::test]
async fn parts_list_for_session_aggregates_across_messages() {
    let db = common::fresh_db().await;
    setup(&db).await;
    MessageRepository::new(db.pool().clone())
        .create(&common::make_message("m2", "s", MessageRole::Assistant))
        .await
        .unwrap();
    let repo = PartRepository::new(db.pool().clone());
    repo.upsert(&text_part("p1", "a", 0)).await.unwrap();

    let mut p2 = text_part("p2", "b", 0);
    p2.message_id = "m2".to_string();
    repo.upsert(&p2).await.unwrap();

    let list = repo.list_for_session("s").await.unwrap();
    assert_eq!(list.len(), 2);
}

#[tokio::test]
async fn parts_delete_for_message_only_removes_target() {
    let db = common::fresh_db().await;
    setup(&db).await;
    MessageRepository::new(db.pool().clone())
        .create(&common::make_message("m2", "s", MessageRole::User))
        .await
        .unwrap();
    let repo = PartRepository::new(db.pool().clone());
    repo.upsert(&text_part("p1", "x", 0)).await.unwrap();
    let mut p2 = text_part("p2", "y", 0);
    p2.message_id = "m2".to_string();
    repo.upsert(&p2).await.unwrap();

    repo.delete_for_message("m").await.unwrap();
    assert!(repo.list_for_message("m").await.unwrap().is_empty());
    assert_eq!(repo.list_for_message("m2").await.unwrap().len(), 1);
}

#[tokio::test]
async fn deleting_message_cascades_parts() {
    let db = common::fresh_db().await;
    setup(&db).await;
    let prepo = PartRepository::new(db.pool().clone());
    prepo.upsert(&text_part("p", "x", 0)).await.unwrap();
    MessageRepository::new(db.pool().clone()).delete("m").await.unwrap();
    assert!(prepo.list_for_message("m").await.unwrap().is_empty(), "FK cascade");
}

// ShareRepository

fn share_for(session_id: &str) -> SessionShareRow {
    SessionShareRow {
        session_id: session_id.to_string(),
        id: format!("share-{session_id}"),
        secret: "secret".to_string(),
        url: format!("https://example.com/{session_id}"),
    }
}

#[tokio::test]
async fn share_upsert_and_get_round_trip() {
    let db = common::fresh_db().await;
    SessionRepository::new(db.pool().clone())
        .create(&common::make_session("s", "p"))
        .await
        .unwrap();
    let repo = ShareRepository::new(db.pool().clone());

    repo.upsert(&share_for("s")).await.expect("insert share");
    let got = repo.get("s").await.unwrap().unwrap();
    assert_eq!(got.id, "share-s");
    assert_eq!(got.url, "https://example.com/s");
}

#[tokio::test]
async fn share_get_returns_none_when_absent() {
    let db = common::fresh_db().await;
    SessionRepository::new(db.pool().clone())
        .create(&common::make_session("s", "p"))
        .await
        .unwrap();
    let repo = ShareRepository::new(db.pool().clone());
    assert!(repo.get("s").await.unwrap().is_none());
}

#[tokio::test]
async fn share_delete_removes_row() {
    let db = common::fresh_db().await;
    SessionRepository::new(db.pool().clone())
        .create(&common::make_session("s", "p"))
        .await
        .unwrap();
    let repo = ShareRepository::new(db.pool().clone());
    repo.upsert(&share_for("s")).await.unwrap();
    repo.delete("s").await.expect("delete");
    assert!(repo.get("s").await.unwrap().is_none());
}

#[tokio::test]
async fn share_cascades_when_session_deleted() {
    let db = common::fresh_db().await;
    SessionRepository::new(db.pool().clone())
        .create(&common::make_session("s", "p"))
        .await
        .unwrap();
    let srepo = ShareRepository::new(db.pool().clone());
    srepo.upsert(&share_for("s")).await.unwrap();
    SessionRepository::new(db.pool().clone()).delete("s").await.unwrap();
    assert!(srepo.get("s").await.unwrap().is_none(), "FK cascade");
}
```

- [ ] **Step 2: 跑测试**

```
cargo test -p kfcode-storage --test parts_share
```

预期：8 条全 pass。

- [ ] **Step 3：（可选）补 re-export**

如 plan 决定顺手补 re-export，单独 commit。修改 `crates/kfcode-storage/src/lib.rs`：

```rust
pub use repository::{
    MessageRepository, PartRepository, PartRow,
    SessionRepository, SessionShareRow, ShareRepository,
    TodoItem, TodoRepository,
};
```

- [ ] **Step 4: 提交**

```bash
git add crates/kfcode-storage/tests/parts_share.rs
git commit -m "$(cat <<'EOF'
test(storage): cover PartRepository and ShareRepository

Access via kfcode_storage::repository::* (PartRepository / PartRow /
ShareRepository / SessionShareRow are not in lib.rs re-exports but
the module itself is pub). Covers upsert, list ordering, scoped
list_for_session, delete_for_message, and FK cascade for both.
EOF
)"
```

如果做了 Step 3 的 re-export，单独再提一个 commit：

```bash
git add crates/kfcode-storage/src/lib.rs
git commit -m "feat(storage): re-export Part/Share repository types from lib root"
```
