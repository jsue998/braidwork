//! Canonical, synchronous `SQLite` persistence for Braidwork domain entities.
//!
//! Opening a store enables foreign keys and transactionally applies schema
//! migrations. Entity writes are explicit insertions: history is never replaced.
//! Specific manual-workflow operations also record assignment handoff status.
//! Reconstruction validates identifiers, revisions, and domain relationships;
//! SQL constraints enforce reference existence and execution provenance.

use std::path::Path;

use rusqlite::Connection;

mod bridge;
mod codec;
mod entities;
mod error;
mod execution;
mod migrations;

pub use error::{EntityId, ReconstructionError, StoreError};
pub use migrations::SCHEMA_VERSION;

/// A concrete store owning one private `SQLite` connection.
///
/// Supports file-backed and in-memory databases. Each successful open has foreign
/// keys enabled and the current schema applied. No connection pool, asynchronous
/// runtime, or artifact content storage is involved.
pub struct SqliteStore {
    connection: Connection,
}

impl SqliteStore {
    /// Opens or creates a database at the supplied path and applies migrations.
    ///
    /// Parent directories must already exist; no project layout is created.
    ///
    /// # Errors
    /// Returns [`StoreError`] if opening, connection setup, or migration fails,
    /// including when the persisted schema version is unsupported.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        Self::from_connection(Connection::open(path)?)
    }

    /// Creates a private in-memory database with the same schema and constraints.
    ///
    /// # Errors
    /// Returns [`StoreError`] if connection setup or migration fails.
    pub fn open_in_memory() -> Result<Self, StoreError> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    /// Reads the database's persisted `PRAGMA user_version`.
    ///
    /// # Errors
    /// Returns [`StoreError::Database`] if the version cannot be read.
    pub fn schema_version(&self) -> Result<u32, StoreError> {
        Ok(self
            .connection
            .pragma_query_value(None, "user_version", |row| row.get(0))?)
    }

    fn from_connection(mut connection: Connection) -> Result<Self, StoreError> {
        connection.pragma_update(None, "foreign_keys", true)?;
        migrations::apply(&mut connection)?;
        Ok(Self { connection })
    }
}

#[cfg(test)]
mod tests;
