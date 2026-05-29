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
