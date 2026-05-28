# T09 — Storage 行为测试（事务、并发、JSON 容错、迁移幂等）

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 覆盖 `Database::transaction()` 的 commit/rollback、tempfile 多连接的并发写、JSON 字段静默降级现状、迁移幂等。

**Architecture:** `tests/behavior.rs`。事务用 in-memory 即可；并发用 `fresh_tempdir_db()`（多连接）；迁移幂等通过对同一个 tempfile path 反复 `Database::open_at()`。

**Tech Stack:** sqlx 0.8 / tokio。

**依赖:** T01 / T02 / T03 / T04

---

### Task 1.8：Storage 行为测试

**Files:**
- Create: `crates/kfcode-storage/tests/behavior.rs`

- [ ] **Step 1: 写失败测试**

写入 `crates/kfcode-storage/tests/behavior.rs`：

```rust
mod common;

use kfcode_storage::{Database, DatabaseError, SessionRepository};

#[tokio::test]
async fn transaction_commits_on_ok() {
    let db = common::fresh_db().await;
    db.transaction(|tx| async move {
        sqlx::query("INSERT INTO permissions (project_id, created_at, updated_at, data) VALUES (?, ?, ?, ?)")
            .bind("p-tx").bind(0_i64).bind(0_i64).bind("[]")
            .execute(&mut **tx)
            .await
            .map_err(|e| DatabaseError::QueryError(e.to_string()))?;
        Ok::<_, DatabaseError>(())
    })
    .await
    .expect("commit");

    let row: (i64,) = sqlx::query_as("SELECT count(*) FROM permissions WHERE project_id = 'p-tx'")
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert_eq!(row.0, 1);
}

#[tokio::test]
async fn transaction_rolls_back_on_err() {
    let db = common::fresh_db().await;
    let res = db
        .transaction(|tx| async move {
            sqlx::query("INSERT INTO permissions (project_id, created_at, updated_at, data) VALUES (?, ?, ?, ?)")
                .bind("p-rb").bind(0_i64).bind(0_i64).bind("[]")
                .execute(&mut **tx)
                .await
                .map_err(|e| DatabaseError::QueryError(e.to_string()))?;
            Err::<(), _>(DatabaseError::QueryError("forced rollback".into()))
        })
        .await;
    assert!(res.is_err());

    let row: (i64,) = sqlx::query_as("SELECT count(*) FROM permissions WHERE project_id = 'p-rb'")
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert_eq!(row.0, 0, "rollback must drop the insert");
}

#[tokio::test(flavor = "multi_thread")]
async fn concurrent_writes_to_distinct_sessions_succeed() {
    let (db, _dir) = common::fresh_tempdir_db().await;
    let repo = SessionRepository::new(db.pool().clone());

    let mut handles = Vec::new();
    for i in 0..16 {
        let r = repo.clone_for_test(); // 详见下方说明
        let s = common::make_session(&format!("c{i}"), "p");
        handles.push(tokio::spawn(async move { r.create(&s).await }));
    }
    for h in handles {
        h.await.unwrap().expect("create");
    }

    let total: (i64,) = sqlx::query_as("SELECT count(*) FROM sessions")
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert_eq!(total.0, 16);
}
```

> 注意：`SessionRepository` 当前不实现 `Clone`。在测试中用 pool 重新构造即可，把 spawn 这里改成：
>
> ```rust
> let pool = db.pool().clone();
> handles.push(tokio::spawn(async move {
>     let r = SessionRepository::new(pool);
>     r.create(&s).await
> }));
> ```
>
> 上面伪代码中的 `clone_for_test` 仅作占位提示——实际写测试时去掉它，按 pool clone 重新构造 repo。

继续追加（同一文件）：

```rust
#[tokio::test]
async fn corrupted_json_field_falls_back_silently() {
    // 当前 SessionRow::into_session 对 revert/permission/summary_diffs 等字段
    // 用 .and_then(... .ok()) 静默降级。spec §3.1 默认按现状测：
    //   - 写入合法 JSON：读出一致
    //   - 写入损坏 JSON：读出 None / default 而不 panic
    let db = common::fresh_db().await;
    SessionRepository::new(db.pool().clone())
        .create(&common::make_session("s", "p"))
        .await
        .unwrap();

    // 直接 SQL 把 revert 字段改成损坏 JSON
    sqlx::query("UPDATE sessions SET revert = ? WHERE id = 's'")
        .bind("{not valid json")
        .execute(db.pool())
        .await
        .unwrap();

    let got = SessionRepository::new(db.pool().clone())
        .get("s")
        .await
        .expect("get must not error on corrupt JSON")
        .expect("session row still present");
    assert!(got.revert.is_none(), "corrupt JSON must fall back to None");
}

#[tokio::test]
async fn migrations_idempotent_across_reopens() {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("test.db");

    {
        let db = Database::open_at(&path).await.unwrap();
        SessionRepository::new(db.pool().clone())
            .create(&common::make_session("survivor", "p"))
            .await
            .unwrap();
    }

    // 第二次打开必须再跑一遍迁移而不报错，且数据保留
    {
        let db = Database::open_at(&path).await.expect("reopen runs migrations cleanly");
        let got = SessionRepository::new(db.pool().clone())
            .get("survivor")
            .await
            .unwrap();
        assert!(got.is_some(), "data must survive reopen");
    }

    // 第三次开仍然 OK
    {
        let _db = Database::open_at(&path).await.expect("reopen #2");
    }
}
```

- [ ] **Step 2: 跑测试**

```
cargo test -p kfcode-storage --test behavior
```

预期：5 条全 pass。
- 如果 `corrupted_json_field_falls_back_silently` 报错而不是返回 None，意味着 spec §3.1 描述的行为已被改成显式错误——确认是 plan T09 之外的预期改动还是回归 bug。
- 如果 `migrations_idempotent_across_reopens` 第二次 open 失败，按 spec §2.8 视为 bug：迁移语句都用了 `IF NOT EXISTS`，应当幂等；定位是哪条迁移失败。

- [ ] **Step 3: 跑全 storage 测试，确认无回归**

```
cargo test -p kfcode-storage
```

预期：T01-T09 全部 pass。

- [ ] **Step 4: 提交**

```bash
git add crates/kfcode-storage/tests/behavior.rs
git commit -m "$(cat <<'EOF'
test(storage): cover transactions, concurrency, JSON fallback, migration idempotency

- transaction commit/rollback on Ok/Err
- 16-way concurrent INSERT to distinct sessions over multi-connection
  tempdir pool (depends on Database::open_at from T03)
- corrupted-JSON revert field returns None (current silent-fallback
  behavior per spec §3.1; bug-fix is out of scope for this batch)
- reopening the same db file runs migrations cleanly multiple times
EOF
)"
```
