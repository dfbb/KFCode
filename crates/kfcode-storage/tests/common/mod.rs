#![allow(dead_code)]

use kfcode_storage::{Database, DatabaseError};
use tempfile::TempDir;

pub async fn fresh_db() -> Database {
    Database::in_memory().await.expect("init in-memory db")
}

/// 多连接 tempdir 数据库；返回 (Database, TempDir)，drop TempDir 时清理文件。
pub async fn fresh_tempdir_db() -> (Database, TempDir) {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("test.db");
    let db = Database::open_at(&path).await.expect("open_at");
    (db, dir)
}

pub use kfcode_storage::DatabaseError as DbErr;
