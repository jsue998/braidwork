//! Auditable execution and verification records shared by every access mode.

use serde::{Deserialize, Serialize};

use crate::{
    id::{ArtifactId, CapsuleId, ModelId, ReceiptId, ResourceId, TaskId},
    resource::AccessMode,
};

/// Whether an execution finished producing its result, independently of acceptance.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum ExecutionOutcome {
    /// The worker finished; verification may still reject the result.
    Completed,
    /// The attempt failed; any partial artifacts can still be recorded.
    Failed {
        /// Human-readable explanation, without a provider response object.
        reason: String,
    },
}

/// The recorded verification decision, with optional artifact evidence.
///
/// A manual review can provide a summary without artifact evidence. Evidence may
/// also originate from a separate verification task. This records a decision;
/// it does not run checks or certify the truth of the supplied information.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum Verification {
    /// No verification decision has been recorded.
    NotPerformed,
    /// The result was accepted according to the recorded checks or review.
    Accepted {
        /// Explanation of the review or checks supporting acceptance.
        summary: String,
        /// Artifacts supporting the decision, possibly from another task.
        evidence: Vec<ArtifactId>,
    },
    /// The result was rejected according to the recorded checks or review.
    Rejected {
        /// Explanation of the unmet criteria or failed checks.
        summary: String,
        /// Artifacts supporting the decision, possibly from another task.
        evidence: Vec<ArtifactId>,
    },
}

/// A known monetary cost recorded with integer precision.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonetaryCost {
    /// Millionths of the named currency unit; zero means a known zero cost.
    pub amount_micros: u64,
    /// Currency label, conventionally an ISO 4217 code such as `USD`.
    pub currency: String,
}

/// Observed usage, preserving missing measurements rather than inventing zeros.
///
/// Each measurement is independently optional. `Default` means entirely unknown.
/// Token counts are reported observations, not estimates computed by the core.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    /// Input tokens reported by the execution resource, when known.
    pub input_tokens: Option<u64>,
    /// Output tokens reported by the execution resource, when known.
    pub output_tokens: Option<u64>,
    /// Monetary cost when known, including an explicitly measured zero cost.
    pub cost: Option<MonetaryCost>,
}

/// An auditable record linking work, its context, resource, known model, and outputs.
///
/// Manual, API, harness, and local executions use the same representation.
/// The access mode and known model identity are recorded at execution time so
/// later resource changes do not alter those historical facts. An unknown model
/// remains `None` rather than being inferred from the resource's identity.
/// Related entity existence and matching task
/// provenance must be checked by the project/application layer, not this record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    /// Identity of this execution record.
    pub id: ReceiptId,
    /// Task whose work was attempted.
    pub task_id: TaskId,
    /// Specific capsule supplied to the worker.
    pub capsule_id: CapsuleId,
    /// Resource that performed the work.
    pub resource_id: ResourceId,
    /// Concrete model used, when known and recorded; `None` means undetermined.
    pub model_id: Option<ModelId>,
    /// Access mode actually used for this attempt.
    pub access_mode: AccessMode,
    /// Resulting artifacts, including partial outputs of a failed attempt.
    pub artifacts: Vec<ArtifactId>,
    /// Outcome of execution, separate from verification.
    pub execution: ExecutionOutcome,
    /// Review or checks and the resulting acceptance decision.
    pub verification: Verification,
    /// Usage observations, which may all be unknown for manual work.
    pub usage: Usage,
}
