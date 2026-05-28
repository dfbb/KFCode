//! Persistent storage layer for kfcode, backed by a SQLite database.
//! Exposes repository types for sessions, messages, todos, and related entities.

/// SQLite connection pool management and migration runner.
pub mod database;
/// Repository types that provide CRUD access to each database table.
pub mod repository;
/// SQL DDL statements and the ordered migration slice applied at startup.
pub mod schema;

pub use database::{Database, DatabaseError};
pub use repository::{MessageRepository, SessionRepository, TodoItem, TodoRepository};
