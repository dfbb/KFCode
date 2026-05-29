mod common;

use kfcode_server::ServerState;
use kfcode_storage::Database;
use tempfile::TempDir;

#[tokio::test]
async fn server_state_accepts_external_database() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("server-test.db");
    let db = Database::open_at(&path).await.expect("open_at");
    let state = ServerState::new_with_database(db, "http://test".to_string())
        .await
        .expect("inject db");
    assert!(state.has_storage(), "state should have storage backend");
}

#[tokio::test]
async fn injected_database_persists_at_explicit_path() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("explicit.db");
    let db = Database::open_at(&path).await.unwrap();
    let _state = ServerState::new_with_database(db, "http://t".into())
        .await
        .expect("inject");
    assert!(path.exists(), "db file must be at explicit path");
}
