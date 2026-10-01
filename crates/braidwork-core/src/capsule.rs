//! Portable task descriptions containing selected context and output requirements.

use serde::{Deserialize, Serialize};

use crate::{
    artifact::ArtifactKind,
    id::{CapsuleId, TaskId},
};

/// Context already selected for a worker by a person or a future context compiler.
///
/// References are opaque locations or storage keys. The core does not retrieve
/// them, rank context, or encode provider-specific chat messages.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ContextInput {
    /// Small context included directly in the portable description.
    Inline {
        /// Human-readable description of the context.
        label: String,
        /// Selected text, independent of any prompt syntax.
        text: String,
    },
    /// Context stored outside the capsule.
    Reference {
        /// Human-readable description of the context.
        label: String,
        /// Opaque location or key to be resolved outside the domain layer.
        reference: String,
    },
}

/// A provider-independent budget of estimated tokens for selected context.
///
/// This bounds context supplied to a worker, including resolved references.
/// The core does not count tokens; a future context compiler will estimate them
/// without requiring a particular provider or tokenizer in the domain model.
/// Actual provider/model limits must be checked outside the domain, including
/// any provider-specific rendering overhead. This does not budget response size.
/// Zero means no selected context is permitted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextBudget {
    /// Maximum estimated tokens of selected context, once references are resolved.
    pub max_estimated_tokens: u64,
}

/// A meaningful output the worker is expected to produce.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpectedOutput {
    /// Semantic type of the expected artifact.
    pub kind: ArtifactKind,
    /// What this output must contain.
    pub description: String,
}

/// Exactly what a worker needs to know and produce for a particular task.
///
/// A capsule is a portable view of project state, independent of the selected
/// resource. Its role and requirements are ordinary domain text, not chat roles,
/// API parameters, or prompt formatting conventions. Consumers should issue a
/// new capsule ID when changing a capsule already referenced by a receipt.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskCapsule {
    /// Identity of this particular task description.
    pub id: CapsuleId,
    /// Task this capsule describes.
    pub task_id: TaskId,
    /// Worker responsibility, such as implementer or reviewer.
    pub role: String,
    /// Result the worker should achieve for this execution.
    pub objective: String,
    /// Selected inline context or references to external context.
    pub inputs: Vec<ContextInput>,
    /// Conditions the worker must respect.
    pub constraints: Vec<String>,
    /// Conditions used to decide whether the work is acceptable.
    pub acceptance_criteria: Vec<String>,
    /// Meaningful outputs to turn into artifacts.
    pub expected_outputs: Vec<ExpectedOutput>,
    /// Estimated token limit on selected context, independent of any provider.
    pub context_budget: ContextBudget,
}
