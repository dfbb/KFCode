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
