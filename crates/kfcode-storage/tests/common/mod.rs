#![allow(dead_code)]

use chrono::Utc;
use kfcode_storage::{Database, DatabaseError};
use kfcode_types::message::{MessageRole, SessionMessage};
use kfcode_types::session::{Session, SessionStatus, SessionTime};
use std::collections::HashMap;
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
