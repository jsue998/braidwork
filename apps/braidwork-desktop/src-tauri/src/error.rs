//! Structured IPC failures with human messages and retained diagnostic causes.
use braidwork_project::{ArtifactContentError, ProjectError};
use braidwork_store::StoreError;
use serde::Serialize;
use std::error::Error;

/// Serializable failure contract; codes are stable within this desktop version.
#[derive(Debug, Serialize)]
pub struct IpcError {
    /// Machine-readable category, never selected by parsing error messages.
    pub code: &'static str,
    /// Human-facing description.
    pub message: String,
    /// Expandable diagnostics without a backtrace or Debug dump.
    pub detail: Option<String>,
}
impl IpcError {
    pub(crate) fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            detail: None,
        }
    }
    pub(crate) fn input(message: impl Into<String>) -> Self {
        Self::new("invalid_input", message)
    }
}
impl From<StoreError> for IpcError {
    fn from(error: StoreError) -> Self {
        let (code, message) = match &error {
            StoreError::NotFound { entity } => ("not_found", format!("Could not find {entity}. Refresh the project and try again.")),
            StoreError::MissingReference { relation, entity } => ("missing_reference", format!("The {relation} requires an existing {entity}.")),
            StoreError::AlreadyExists { entity } => ("already_exists", format!("That {entity} already exists.")),
            StoreError::DelegationDenied(_) => ("delegation_denied", "That agent is not allowed to delegate more work. Check its delegation policy and direct-delegate limit.".into()),
            StoreError::Workflow { reason, .. } => ("workflow", (*reason).into()),
            StoreError::Reconstruction(_) => ("invalid_project_data", "Stored project data could not be reconstructed. No data was repaired or replaced.".into()),
            StoreError::UnsupportedSchema { .. } => ("unsupported_schema", "This database version is not supported by this Braidwork version.".into()),
            StoreError::Integrity { .. } | StoreError::Provenance(_) | StoreError::AgentMismatch(_) => ("integrity", "These references are inconsistent with the project. Refresh and check the selected work, session, and agent revision.".into()),
            StoreError::Database(_) | StoreError::Serialization(_) => ("store", "The project operation could not be saved or read.".into()),
        };
        Self {
            code,
            message,
            detail: Some(diagnostic(&error)),
        }
    }
}
impl From<ProjectError> for IpcError {
    fn from(error: ProjectError) -> Self {
        if let ProjectError::Store(error) = error {
            return error.into();
        }
        let (code, message) = match &error {
            ProjectError::AlreadyInitialized(_) => (
                "already_initialized",
                "This folder already contains a Braidwork project. Open it instead.",
            ),
            ProjectError::InvalidName => (
                "invalid_name",
                "The project name must contain non-whitespace text.",
            ),
            ProjectError::MarkerMissing(_) | ProjectError::NotFound(_) => (
                "project_not_found",
                "No Braidwork project was found in this folder. Select its root or create a project.",
            ),
            ProjectError::DatabaseMissing(_) => (
                "database_missing",
                "This project's database is missing. It has not been recreated.",
            ),
            ProjectError::UnsupportedFormat { .. } => (
                "unsupported_format",
                "This project format is not supported by this Braidwork version.",
            ),
            ProjectError::ContextForbidden => (
                "context_forbidden",
                "A zero context budget does not allow selected context.",
            ),
            ProjectError::Content(ArtifactContentError::Missing(_)) => (
                "content_missing",
                "The result's content file is missing. Its metadata has been preserved.",
            ),
            ProjectError::Content(_) => (
                "artifact_content",
                "The result content could not be read or written.",
            ),
            ProjectError::SelfDelegation(_) => (
                "self_delegation",
                "An assignment cannot delegate work to itself.",
            ),
            _ => (
                "project",
                "The project could not be opened or changed. Existing data has been preserved.",
            ),
        };
        Self {
            code,
            message: message.into(),
            detail: Some(diagnostic(&error)),
        }
    }
}
fn diagnostic(error: &dyn Error) -> String {
    let mut text = error.to_string();
    let mut current = error.source();
    while let Some(cause) = current {
        use std::fmt::Write;
        let _ = write!(text, "\nCaused by: {cause}");
        current = cause.source();
    }
    text
}
