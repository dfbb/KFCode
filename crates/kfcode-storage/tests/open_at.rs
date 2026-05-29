mod common;

use kfcode_storage::Database;
use tempfile::TempDir;

#[tokio::test]
async fn open_at_creates_db_at_explicit_path() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.db");
    let _db = Database::open_at(&path).await.expect("open at tempdir");
    assert!(path.exists(), "db file should be created at {}", path.display());
}

#[tokio::test]
async fn open_at_runs_migrations() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.db");
    let db = Database::open_at(&path).await.expect("open");

    let row: (i64,) = sqlx::query_as("SELECT count(*) FROM sqlite_master WHERE type='table' AND name='sessions'")
        .fetch_one(db.pool())
        .await
        .expect("query");
    assert_eq!(row.0, 1);
}

#[tokio::test]
async fn open_at_reopen_preserves_data() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.db");

    {
        let db = Database::open_at(&path).await.unwrap();
        sqlx::query("INSERT INTO sessions (id, slug, project_id, directory, title, version, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)")
            .bind("ses-persist").bind("persist").bind("proj").bind("/tmp")
            .bind("t").bind("1").bind("active").bind(0_i64).bind(0_i64)
            .execute(db.pool()).await.unwrap();
    }

    let db = Database::open_at(&path).await.unwrap();
    let row: (i64,) = sqlx::query_as("SELECT count(*) FROM sessions WHERE id = 'ses-persist'")
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert_eq!(row.0, 1, "data should persist across reopens");
}
