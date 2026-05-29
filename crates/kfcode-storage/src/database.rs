//! SQLite connection pool wrapper with automatic schema migration.

use anyhow::Result;
use sqlx::sqlite::{SqliteConnection, SqlitePool, SqlitePoolOptions};
use sqlx::{Sqlite, Transaction};
use std::future::Future;
use std::path::PathBuf;
use thiserror::Error;
use tracing::info;

/// Errors that can occur during database operations.
#[derive(Debug, Error)]
pub enum DatabaseError {
    /// Failed to open or acquire a connection from the pool.
    #[error("Database connection error: {0}")]
    ConnectionError(String),

    /// A migration statement failed to execute.
    #[error("Migration error: {0}")]
    MigrationError(String),

    /// A SQL query returned an error.
    #[error("Query error: {0}")]
    QueryError(String),

    /// Beginning or committing a transaction failed.
    #[error("Transaction error: {0}")]
    TransactionError(String),
}

/// Owned handle to the SQLite connection pool used by all repositories.
pub struct Database {
    pool: SqlitePool,
}

/// Convenience alias for an active SQLite transaction.
pub type SqliteTransaction<'a> = Transaction<'a, Sqlite>;

impl Database {
    /// Opens (or creates) the application database file and runs all pending migrations.
    ///
    /// # Errors
    /// Returns `DatabaseError::ConnectionError` if the file cannot be created or the pool
    /// cannot connect, and `DatabaseError::MigrationError` if any migration statement fails.
    pub async fn new() -> Result<Self, DatabaseError> {
        let db_path = Self::get_database_path()?;

        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;
        }

        let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

        info!("Connecting to database at {}", db_path.display());

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .after_connect(|conn, _meta| Box::pin(async move {
                sqlx::query("PRAGMA foreign_keys = ON")
                    .execute(conn)
                    .await
                    .map(|_| ())
            }))
            .connect(&db_url)
            .await
            .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

        let db = Self { pool };
        db.run_migrations().await?;

        Ok(db)
    }

    /// Creates a transient in-memory database and runs all migrations.
    ///
    /// # Note
    /// The in-memory pool is limited to a single connection so that all callers
    /// share the same SQLite in-memory instance; multiple connections would each
    /// see an empty, independent database.
    pub async fn in_memory() -> Result<Self, DatabaseError> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .after_connect(|conn, _meta| Box::pin(async move {
                sqlx::query("PRAGMA foreign_keys = ON")
                    .execute(conn)
                    .await
                    .map(|_| ())
            }))
            .connect("sqlite::memory:")
            .await
            .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

        let db = Self { pool };
        db.run_migrations().await?;

        Ok(db)
    }

    /// Returns a reference to the underlying connection pool.
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Begins a new database transaction and returns it to the caller.
    ///
    /// # Errors
    /// Returns `DatabaseError::TransactionError` if the pool cannot start a transaction.
    pub async fn begin(&self) -> Result<SqliteTransaction<'_>, DatabaseError> {
        self.pool
            .begin()
            .await
            .map_err(|e| DatabaseError::TransactionError(e.to_string()))
    }

    /// Runs `f` inside a transaction, committing on success or propagating the error on failure.
    ///
    /// # Errors
    /// Returns `DatabaseError::TransactionError` if begin or commit fails, or any error
    /// returned by `f`.
    pub async fn transaction<F, T, Fut>(&self, f: F) -> Result<T, DatabaseError>
    where
        F: FnOnce(&mut SqliteTransaction<'_>) -> Fut,
        Fut: Future<Output = Result<T, DatabaseError>>,
    {
        let mut tx = self.begin().await?;
        let result = f(&mut tx).await?;
        tx.commit()
            .await
            .map_err(|e| DatabaseError::TransactionError(e.to_string()))?;
        Ok(result)
    }

    /// Acquires a detached connection from the pool for one-off use.
    ///
    /// # Note
    /// The returned connection is detached from the pool and must be dropped
    /// explicitly; prefer `pool()` or `transaction()` for normal use.
    pub async fn get_connection(&self) -> Result<SqliteConnection, DatabaseError> {
        self.pool
            .acquire()
            .await
            .map(|conn| conn.detach())
            .map_err(|e| DatabaseError::ConnectionError(e.to_string()))
    }

    async fn run_migrations(&self) -> Result<(), DatabaseError> {
        info!("Running database migrations");

        for migration in crate::schema::ALL_MIGRATIONS {
            sqlx::query(migration)
                .execute(&self.pool)
                .await
                .map_err(|e| DatabaseError::MigrationError(e.to_string()))?;
        }

        Ok(())
    }

    fn get_database_path() -> Result<PathBuf, DatabaseError> {
        let data_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("kfcode");

        Ok(data_dir.join("kfcode.db"))
    }
}
