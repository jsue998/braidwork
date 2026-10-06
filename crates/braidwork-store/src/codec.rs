use braidwork_core::id::InvalidId;
use rusqlite::{Connection, OptionalExtension, Row, types::FromSql};
use serde::{Serialize, de::DeserializeOwned};

use crate::{EntityId, ReconstructionError, StoreError};

pub(crate) fn column<T: FromSql>(row: &Row<'_>, name: &str) -> Result<T, StoreError> {
    row.get(name)
        .map_err(|error| ReconstructionError::Column(error).into())
}

pub(crate) fn id<T: TryFrom<String, Error = InvalidId>>(
    row: &Row<'_>,
    name: &str,
) -> Result<T, StoreError> {
    T::try_from(column::<String>(row, name)?)
        .map_err(|error| ReconstructionError::InvalidId(error).into())
}

pub(crate) fn optional_id<T: TryFrom<String, Error = InvalidId>>(
    row: &Row<'_>,
    name: &str,
) -> Result<Option<T>, StoreError> {
    column::<Option<String>>(row, name)?
        .map(T::try_from)
        .transpose()
        .map_err(|error| ReconstructionError::InvalidId(error).into())
}

pub(crate) fn unsigned(row: &Row<'_>, name: &str) -> Result<u64, StoreError> {
    column::<String>(row, name)?
        .parse()
        .map_err(|error| ReconstructionError::Unsigned(error).into())
}

pub(crate) fn optional_unsigned(row: &Row<'_>, name: &str) -> Result<Option<u64>, StoreError> {
    column::<Option<String>>(row, name)?
        .map(|value| value.parse())
        .transpose()
        .map_err(|error| ReconstructionError::Unsigned(error).into())
}

pub(crate) fn json<T: DeserializeOwned>(row: &Row<'_>, name: &str) -> Result<T, StoreError> {
    serde_json::from_str(&column::<String>(row, name)?)
        .map_err(|error| ReconstructionError::Json(error).into())
}

// Unit enums occupy ordinary TEXT columns, using the domain's snake_case names.
pub(crate) fn enum_text<T: Serialize>(value: T) -> Result<String, StoreError> {
    Ok(serde_json::from_value(serde_json::to_value(value)?)?)
}

pub(crate) fn enum_column<T: DeserializeOwned>(row: &Row<'_>, name: &str) -> Result<T, StoreError> {
    serde_json::from_value(serde_json::Value::String(column(row, name)?))
        .map_err(|error| ReconstructionError::Json(error).into())
}

pub(crate) fn read<T>(
    connection: &Connection,
    sql: &str,
    id: &str,
    entity: EntityId,
    decode: impl FnOnce(&Row<'_>) -> Result<T, StoreError>,
) -> Result<T, StoreError> {
    // Keep domain failures separate from SQL query failures rather than wrapping
    // them in rusqlite's conversion errors or exposing public persistence DTOs.
    connection
        .query_row(sql, [id], |row| Ok(decode(row)))
        .optional()?
        .ok_or(StoreError::NotFound { entity })?
}
