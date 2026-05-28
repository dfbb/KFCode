# T04 — `SessionRepository` CRUD 集成测试

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 覆盖 `SessionRepository` 的 create / get / list / update / delete / list_children 黄金路径与关键错误路径。

**Architecture:** 在 `tests/sessions.rs` 内组织，使用 `common::fresh_db()`；session fixture 用 helper `make_session(id, project_id)` 构造，避免每个测试重复。

**Tech Stack:** sqlx 0.8 / `kfcode_types::Session`。

**依赖:** T01（脚手架）、T02（PRAGMA）

---

### Task 1.3：SessionRepository 集成测试

**Files:**
- Modify: `crates/kfcode-storage/tests/common/mod.rs`（追加 `make_session`）
- Create: `crates/kfcode-storage/tests/sessions.rs`

- [ ] **Step 1: 在 common 加 session fixture helper**

在 `crates/kfcode-storage/tests/common/mod.rs` 追加：

```rust
use kfcode_types::session::{Session, SessionStatus, SessionTime};
use std::collections::HashMap;
use chrono::Utc;

pub fn make_session(id: &str, project_id: &str) -> Session {
    let now = Utc::now();
    Session {
        id: id.to_string(),
        slug: id.to_string(),
        project_id: project_id.to_string(),
        directory: "/tmp/test".to_string(),
        parent_id: None,
        title: format!("Test {id}"),
        version: "1.0.0".to_string(),
        time: SessionTime::default(),
        messages: Vec::new(),
        summary: None,
        share: None,
        revert: None,
        permission: None,
        usage: None,
        status: SessionStatus::default(),
        metadata: HashMap::new(),
        created_at: now,
        updated_at: now,
    }
}
```

> 如 `Session` 字段在源码已发生变化（编译报错），按编译器提示对齐字段；不要静默改源码字段以匹配测试。

如果 `kfcode-types` 不在 `kfcode-storage` 的 dev/dep 里，加入 dev-dep（已是 dep，无需重复）。

- [ ] **Step 2: 写失败测试集**

写入 `crates/kfcode-storage/tests/sessions.rs`：

```rust
mod common;

use kfcode_storage::SessionRepository;

#[tokio::test]
async fn round_trips_session() {
    let db = common::fresh_db().await;
    let repo = SessionRepository::new(db.pool().clone());
    let s = common::make_session("ses-1", "proj");
    repo.create(&s).await.expect("create");
    let got = repo.get("ses-1").await.expect("get").expect("some");
    assert_eq!(got.id, "ses-1");
    assert_eq!(got.project_id, "proj");
}

#[tokio::test]
async fn get_returns_none_for_missing() {
    let db = common::fresh_db().await;
    let repo = SessionRepository::new(db.pool().clone());
    assert!(repo.get("does-not-exist").await.unwrap().is_none());
}

#[tokio::test]
async fn rejects_duplicate_session_id() {
    let db = common::fresh_db().await;
    let repo = SessionRepository::new(db.pool().clone());
    let s = common::make_session("dup", "proj");
    repo.create(&s).await.expect("first ok");
    let err = repo.create(&s).await.expect_err("expected dup error");
    let msg = format!("{err}");
    assert!(msg.to_lowercase().contains("unique") || msg.to_lowercase().contains("constraint"), "got: {msg}");
}

#[tokio::test]
async fn list_filters_by_project() {
    let db = common::fresh_db().await;
    let repo = SessionRepository::new(db.pool().clone());
    repo.create(&common::make_session("a", "p1")).await.unwrap();
    repo.create(&common::make_session("b", "p2")).await.unwrap();
    repo.create(&common::make_session("c", "p1")).await.unwrap();

    let all = repo.list(None, 100).await.unwrap();
    assert_eq!(all.len(), 3);

    let p1 = repo.list(Some("p1"), 100).await.unwrap();
    assert_eq!(p1.len(), 2);
    assert!(p1.iter().all(|s| s.project_id == "p1"));
}

#[tokio::test]
async fn list_respects_limit() {
    let db = common::fresh_db().await;
    let repo = SessionRepository::new(db.pool().clone());
    for i in 0..5 {
        repo.create(&common::make_session(&format!("s{i}"), "p")).await.unwrap();
    }
    let limited = repo.list(None, 2).await.unwrap();
    assert_eq!(limited.len(), 2);
}

#[tokio::test]
async fn update_persists_changes() {
    let db = common::fresh_db().await;
    let repo = SessionRepository::new(db.pool().clone());
    let mut s = common::make_session("s", "p");
    repo.create(&s).await.unwrap();
    s.title = "Renamed".to_string();
    repo.update(&s).await.expect("update");
    let got = repo.get("s").await.unwrap().unwrap();
    assert_eq!(got.title, "Renamed");
}

#[tokio::test]
async fn delete_removes_session() {
    let db = common::fresh_db().await;
    let repo = SessionRepository::new(db.pool().clone());
    let s = common::make_session("s", "p");
    repo.create(&s).await.unwrap();
    repo.delete("s").await.expect("delete");
    assert!(repo.get("s").await.unwrap().is_none());
}

#[tokio::test]
async fn list_children_finds_forked_sessions() {
    let db = common::fresh_db().await;
    let repo = SessionRepository::new(db.pool().clone());
    repo.create(&common::make_session("parent", "p")).await.unwrap();

    let mut child = common::make_session("child-1", "p");
    child.parent_id = Some("parent".to_string());
    repo.create(&child).await.unwrap();

    let mut child2 = common::make_session("child-2", "p");
    child2.parent_id = Some("parent".to_string());
    repo.create(&child2).await.unwrap();

    repo.create(&common::make_session("unrelated", "p")).await.unwrap();

    let kids = repo.list_children("parent").await.unwrap();
    assert_eq!(kids.len(), 2);
    assert!(kids.iter().all(|s| s.parent_id.as_deref() == Some("parent")));
}

#[tokio::test]
async fn unicode_session_fields_round_trip() {
    let db = common::fresh_db().await;
    let repo = SessionRepository::new(db.pool().clone());
    let mut s = common::make_session("uni-😀", "项目");
    s.title = "中文标题 🚀".to_string();
    s.directory = "/tmp/路径".to_string();
    repo.create(&s).await.unwrap();
    let got = repo.get("uni-😀").await.unwrap().unwrap();
    assert_eq!(got.title, "中文标题 🚀");
    assert_eq!(got.directory, "/tmp/路径");
    assert_eq!(got.project_id, "项目");
}
```

- [ ] **Step 3: 跑测试**

```
cargo test -p kfcode-storage --test sessions
```

预期：9 条全 pass。如有 fail，**先确认是真实 bug 还是测试错**——若是源码 bug（如 list 没按 limit 截断），按 spec §2.8 走独立 commit 修复。

- [ ] **Step 4: 提交**

```bash
git add crates/kfcode-storage/tests/sessions.rs crates/kfcode-storage/tests/common/mod.rs
git commit -m "$(cat <<'EOF'
test(storage): cover SessionRepository CRUD and edge cases

Round-trip, get-missing, dup-id rejection, project-filtered list,
limit, update, delete, list_children for forked sessions, and
unicode fields. Uses in-memory db with foreign keys ON.
EOF
)"
```
