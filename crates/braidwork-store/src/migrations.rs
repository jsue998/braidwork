use rusqlite::{Connection, TransactionBehavior};

use crate::StoreError;

/// Current schema version; also used when writing `PRAGMA user_version`.
pub const SCHEMA_VERSION: u32 = 1;

pub(crate) fn apply(connection: &mut Connection) -> Result<(), StoreError> {
    // Acquire the migration write lock before reading the version, so two new
    // connections cannot both decide to apply the initial migration.
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let version: u32 = transaction.pragma_query_value(None, "user_version", |row| row.get(0))?;
    match version {
        0 => {
            transaction.execute_batch(include_str!("../migrations/001_initial.sql"))?;
            transaction.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        }
        SCHEMA_VERSION => {}
        found => {
            return Err(StoreError::UnsupportedSchema {
                found,
                supported: SCHEMA_VERSION,
            });
        }
    }
    transaction.commit()?;
    Ok(())
}
