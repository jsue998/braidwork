//! Metadata for meaningful outputs; content storage belongs outside the core.

use serde::{Deserialize, Serialize};

use crate::id::{ArtifactId, TaskId};

/// The semantic purpose of an artifact, independent of its storage or media type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    /// Proposed changes to software or other project files.
    Patch,
    /// Reasoning or investigation relevant to project work.
    Analysis,
    /// A specific observation or discovered issue.
    Finding,
    /// Evidence produced by running tests.
    TestResult,
    /// Documentation intended for the project.
    Documentation,
    /// A question requiring clarification or a decision.
    Question,
    /// Generic text without a more specific semantic classification.
    Text,
    /// Generic structured or binary data.
    Data,
}

/// Metadata referencing a meaningful result of a task, rather than a chat message.
///
/// Content is external. A future store can use content-addressed keys, while
/// this object retains identity, provenance, and optional descriptive metadata.
/// The core neither resolves the reference nor computes or verifies digests.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Artifact {
    /// Identity of this artifact record, not necessarily a content digest.
    pub id: ArtifactId,
    /// Task that produced the content.
    pub task_id: TaskId,
    /// Semantic purpose of the content.
    pub kind: ArtifactKind,
    /// Opaque storage reference, such as an algorithm-qualified content digest.
    pub content_ref: String,
    /// Media type, when known, for example `text/plain`.
    pub media_type: Option<String>,
    /// Content size in bytes when known; `None` does not mean zero bytes.
    pub size_bytes: Option<u64>,
}
