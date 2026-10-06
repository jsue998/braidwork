use std::{
    fmt,
    num::{NonZeroU32, ParseIntError},
};

use braidwork_core::{
    delegation::SelfDelegation,
    id::{
        AgentSpecId, ArtifactId, AssignmentId, CapsuleId, DelegationId, InvalidId, ReceiptId,
        ResourceId, SessionId, TaskId,
    },
    task::TaskError,
};
use thiserror::Error;

/// A typed identity identifying the entity involved in a store error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntityId {
    /// A precise historical agent revision.
    AgentSpec(AgentSpecId, NonZeroU32),
    /// A persistent execution instance.
    Session(SessionId),
    /// A historical work allocation.
    Assignment(AssignmentId),
    /// An explicit delegation edge.
    Delegation(DelegationId),
    /// An AI resource record.
    Resource(ResourceId),
    /// A task record.
    Task(TaskId),
    /// A capsule record.
    Capsule(CapsuleId),
    /// An artifact metadata record.
    Artifact(ArtifactId),
    /// An execution receipt record.
    Receipt(ReceiptId),
}

impl fmt::Display for EntityId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AgentSpec(id, revision) => write!(formatter, "agent {id}@{revision}"),
            Self::Session(id) => write!(formatter, "session {id}"),
            Self::Assignment(id) => write!(formatter, "assignment {id}"),
            Self::Delegation(id) => write!(formatter, "delegation {id}"),
            Self::Resource(id) => write!(formatter, "resource {id}"),
            Self::Task(id) => write!(formatter, "task {id}"),
            Self::Capsule(id) => write!(formatter, "capsule {id}"),
            Self::Artifact(id) => write!(formatter, "artifact {id}"),
            Self::Receipt(id) => write!(formatter, "receipt {id}"),
        }
    }
}

/// Persisted data could not be reconstructed as a valid domain value.
#[derive(Debug, Error)]
pub enum ReconstructionError {
    /// Persisted revision zero cannot identify a historical agent definition.
    #[error("invalid persisted agent revision {0}; revisions start at one")]
    Revision(u32),
    /// A persisted edge delegates directly to itself.
    #[error(transparent)]
    Delegation(#[from] SelfDelegation),
    /// A persisted identifier violates domain validation.
    #[error("invalid persisted identifier: {0}")]
    InvalidId(#[from] InvalidId),
    /// Persisted task relationships violate local domain invariants.
    #[error("invalid persisted task: {0}")]
    Task(#[from] TaskError),
    /// Nested JSON or an enum value is malformed or incompatible with the domain.
    #[error("invalid persisted JSON or enum: {0}")]
    Json(#[from] serde_json::Error),
    /// A persisted unsigned decimal is outside the domain's `u64` representation.
    #[error("invalid persisted unsigned integer: {0}")]
    Unsigned(#[from] ParseIntError),
    /// A selected column has an incompatible `SQLite` representation.
    #[error("invalid persisted column: {0}")]
    Column(#[source] rusqlite::Error),
}

/// Explicit failure categories for the store's library API.
#[derive(Debug, Error)]
pub enum StoreError {
    /// A required referenced entity is absent; the relationship is named explicitly.
    #[error("missing {relation}: {entity}")]
    MissingReference {
        /// Relationship requiring this entity.
        relation: &'static str,
        /// Precise missing identity.
        entity: EntityId,
    },
    /// The assignment did not capture the session's configured historical revision.
    #[error("assignment {0} must use the session's exact agent revision")]
    AgentMismatch(AssignmentId),
    /// A workflow operation is invalid for the persisted allocation.
    #[error("assignment {assignment}: {reason}")]
    Workflow {
        /// Allocation involved.
        assignment: AssignmentId,
        /// Specific workflow rule violated.
        reason: &'static str,
    },
    /// The parent agent's stored policy disallows this edge or child count.
    #[error("delegation from assignment {0} is denied by its historical agent policy")]
    DelegationDenied(AssignmentId),
    /// Task or execution references disagree across a composed operation.
    #[error("inconsistent execution provenance: {0}")]
    Provenance(&'static str),
    /// `SQLite` opening, querying, or transaction failure outside integrity checks.
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    /// Domain metadata could not be encoded for persistence.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    /// Persisted data failed domain reconstruction; no fallback value is invented.
    #[error("domain reconstruction error: {0}")]
    Reconstruction(#[from] ReconstructionError),
    /// An insertion attempted to reuse the identity of an existing entity.
    #[error("entity already exists: {entity}")]
    AlreadyExists {
        /// Typed identity of the existing entity.
        entity: EntityId,
    },
    /// The requested entity was not present.
    #[error("entity not found: {entity}")]
    NotFound {
        /// Typed identity requested by the caller.
        entity: EntityId,
    },
    /// A reference, provenance relationship, or SQL constraint was violated.
    ///
    /// Callers can match this variant without interpreting `SQLite` error messages.
    #[error("referential or integrity error: {source}")]
    Integrity {
        /// Original `SQLite` constraint failure retained for diagnostics.
        #[source]
        source: rusqlite::Error,
    },
    /// This implementation cannot open the persisted schema version.
    #[error("unsupported schema version {found}; supported migrations end at {supported}")]
    UnsupportedSchema {
        /// Persisted version found in the database.
        found: u32,
        /// Schema version supported by this implementation.
        supported: u32,
    },
}

pub(crate) fn insertion_error(source: rusqlite::Error, entity: EntityId) -> StoreError {
    if let rusqlite::Error::SqliteFailure(error, _) = &source
        && matches!(
            error.extended_code,
            rusqlite::ffi::SQLITE_CONSTRAINT_PRIMARYKEY | rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE
        )
    {
        return StoreError::AlreadyExists { entity };
    }
    integrity_error(source)
}

pub(crate) fn integrity_error(source: rusqlite::Error) -> StoreError {
    if source.sqlite_error_code() == Some(rusqlite::ErrorCode::ConstraintViolation) {
        StoreError::Integrity { source }
    } else {
        StoreError::Database(source)
    }
}
