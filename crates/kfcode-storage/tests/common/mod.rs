#![allow(dead_code)]

use kfcode_storage::{Database, DatabaseError};

/// 创建一个跑完所有迁移的 in-memory 数据库（单连接）。
pub async fn fresh_db() -> Database {
    Database::in_memory()
        .await
        .expect("init in-memory db failed")
}

/// 暴露 DatabaseError 给测试做匹配断言（避免每个测试都 use）。
pub use kfcode_storage::DatabaseError as DbErr;
