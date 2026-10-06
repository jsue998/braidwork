use rusqlite::{Connection, TransactionBehavior};

use crate::StoreError;

/// Current schema version; also used when writing `PRAGMA user_version`.
pub const SCHEMA_VERSION: u32 = 2;

pub(crate) fn apply(connection: &mut Connection) -> Result<(), StoreError> {
    // Acquire the migration write lock before reading the version, so two new
    // connections cannot both decide to apply a pending migration.
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let version: u32 = transaction.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version > SCHEMA_VERSION {
        return Err(StoreError::UnsupportedSchema {
            found: version,
            supported: SCHEMA_VERSION,
        });
    }
    if version == 0 {
        transaction.execute_batch(include_str!("../migrations/001_initial.sql"))?;
    }
    if version < SCHEMA_VERSION {
        transaction.execute_batch(include_str!("../migrations/002_execution_model.sql"))?;
        transaction.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    }
    transaction.commit()?;
    Ok(())
}
