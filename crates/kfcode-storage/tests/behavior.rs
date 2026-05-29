mod common;

use kfcode_storage::{Database, SessionRepository};

// Tests transaction commit/rollback using db.begin() directly, which avoids
// the higher-ranked lifetime issue with the FnOnce closure in db.transaction().

#[tokio::test]
async fn transaction_commits_on_ok() {
    let db = common::fresh_db().await;

    let mut tx = db.begin().await.expect("begin");
    sqlx::query(
        "INSERT INTO permissions (project_id, created_at, updated_at, data) VALUES (?, ?, ?, ?)",
    )
    .bind("p-tx")
    .bind(0_i64)
    .bind(0_i64)
    .bind("[]")
    .execute(&mut *tx)
    .await
    .unwrap();
    tx.commit().await.expect("commit");

    let row: (i64,) =
        sqlx::query_as("SELECT count(*) FROM permissions WHERE project_id = 'p-tx'")
            .fetch_one(db.pool())
            .await
            .unwrap();
    assert_eq!(row.0, 1);
}

#[tokio::test]
async fn transaction_rolls_back_on_err() {
    let db = common::fresh_db().await;

    {
        let mut tx = db.begin().await.expect("begin");
        sqlx::query(
            "INSERT INTO permissions (project_id, created_at, updated_at, data) VALUES (?, ?, ?, ?)",
        )
        .bind("p-rb")
        .bind(0_i64)
        .bind(0_i64)
        .bind("[]")
        .execute(&mut *tx)
        .await
        .unwrap();
        // Drop tx without committing — SQLite rolls back automatically.
        drop(tx);
    }

    let row: (i64,) =
        sqlx::query_as("SELECT count(*) FROM permissions WHERE project_id = 'p-rb'")
            .fetch_one(db.pool())
            .await
            .unwrap();
    assert_eq!(row.0, 0, "rollback must drop the insert");
}

#[tokio::test(flavor = "multi_thread")]
async fn concurrent_writes_to_distinct_sessions_succeed() {
    let (db, _dir) = common::fresh_tempdir_db().await;

    let mut handles = Vec::new();
    for i in 0..16 {
        let pool = db.pool().clone();
        let s = common::make_session(&format!("c{i}"), "p");
        handles.push(tokio::spawn(async move {
            let r = SessionRepository::new(pool);
            r.create(&s).await
        }));
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

#[tokio::test]
async fn corrupted_json_field_falls_back_silently() {
    let db = common::fresh_db().await;
    SessionRepository::new(db.pool().clone())
        .create(&common::make_session("s", "p"))
        .await
        .unwrap();

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

    {
        let db = Database::open_at(&path)
            .await
            .expect("reopen runs migrations cleanly");
        let got = SessionRepository::new(db.pool().clone())
            .get("survivor")
            .await
            .unwrap();
        assert!(got.is_some(), "data must survive reopen");
    }

    {
        let _db = Database::open_at(&path).await.expect("reopen #2");
    }
}
