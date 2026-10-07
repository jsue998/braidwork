//! Snake-case IPC data. Unsigned 64-bit quantities use decimal strings so
//! JavaScript cannot round canonical token, money, budget, or size values.
use braidwork_core::{
    agent::{AgentSpec, DelegationPolicy},
    artifact::{Artifact, ArtifactKind},
    assignment::Assignment,
    capsule::{ContextInput, ExpectedOutput},
    delegation::Delegation,
    id::{
        AgentSpecId, ArtifactId, AssignmentId, CapsuleId, ModelId, ReceiptId, ResourceId, TaskId,
    },
    receipt::{ExecutionOutcome, Receipt, Usage, Verification},
    resource::{AccessMode, Resource, ResourceStatus, Scarcity},
    session::Session,
    task::Task,
};
use serde::{Deserialize, Serialize};
use std::{num::NonZeroU32, path::PathBuf};

/// Human project identity without database internals.
#[derive(Debug, Serialize)]
pub struct ProjectInfo {
    /// Human project name.
    pub name: String,
    /// Canonical root for display.
    pub root: PathBuf,
    /// Marker format, independently versioned from the store.
    pub format_version: u32,
    /// Persisted store schema.
    pub schema_version: u32,
}
/// Aggregated workspace read model; domain entities retain their existing Serde shapes.
#[derive(Debug, Serialize)]
pub struct WorkspaceSnapshot {
    /// Active project.
    pub project: ProjectInfo,
    /// Available access capacities.
    pub resources: Vec<Resource>,
    /// Exact historical definitions, never implicit latest revisions.
    pub agents: Vec<AgentSpec>,
    /// Independent execution surfaces.
    pub sessions: Vec<Session>,
    /// Work graph entities.
    pub tasks: Vec<Task>,
    /// Allocations, distinct from sessions and tasks.
    pub assignments: Vec<Assignment>,
    /// Explicit delegation edges.
    pub delegations: Vec<Delegation>,
    /// Manual workflow links and result metadata, without loading content.
    pub workflow: Vec<WorkflowSummary>,
}
/// Persisted links needed to derive needs-attention and results views.
#[derive(Debug, Serialize)]
pub struct WorkflowSummary {
    /// Allocation owning this workflow.
    pub assignment_id: AssignmentId,
    /// Prepared capsule, if any.
    pub capsule_id: Option<CapsuleId>,
    /// Received result, if any; content is fetched on selection only.
    pub result: Option<ResultSummary>,
}
/// Metadata for a received execution, with no automatic content reads.
#[derive(Debug, Serialize)]
pub struct ResultSummary {
    /// Real historical receipt.
    pub receipt: ReceiptView,
    /// Real artifact metadata.
    pub artifacts: Vec<ArtifactView>,
}
/// Lossless IPC projection of a receipt; optional observations remain optional.
#[derive(Debug, Serialize)]
pub struct ReceiptView {
    /// Audit identity.
    pub id: ReceiptId,
    /// Work identity.
    pub task_id: TaskId,
    /// Portable instructions used.
    pub capsule_id: CapsuleId,
    /// Actual access resource.
    pub resource_id: ResourceId,
    /// Actual model, only when known.
    pub model_id: Option<ModelId>,
    /// Recorded transport, manual for Manual Bridge ingestion.
    pub access_mode: AccessMode,
    /// Ordered canonical artifact identities, including any recorded repetitions.
    pub artifacts: Vec<ArtifactId>,
    /// Execution outcome, distinct from verification and task status.
    pub execution: ExecutionOutcome,
    /// Acceptance evidence; ingest never automatically verifies.
    pub verification: Verification,
    /// Optional observations with lossless decimal representation.
    pub usage: UsageView,
}
/// Optional observed usage, preserving unknown separately from known zero.
#[derive(Debug, Serialize)]
pub struct UsageView {
    /// Observed input tokens as a decimal integer.
    pub input_tokens: Option<String>,
    /// Observed output tokens as a decimal integer.
    pub output_tokens: Option<String>,
    /// Observed money, with explicit currency.
    pub cost: Option<CostView>,
}
/// Lossless monetary observation.
#[derive(Debug, Serialize)]
pub struct CostView {
    /// Millionths of the stated currency, as a decimal integer.
    pub amount_micros: String,
    /// Caller-supplied currency.
    pub currency: String,
}
impl From<&Usage> for UsageView {
    fn from(value: &Usage) -> Self {
        Self {
            input_tokens: value.input_tokens.map(|n| n.to_string()),
            output_tokens: value.output_tokens.map(|n| n.to_string()),
            cost: value.cost.as_ref().map(|c| CostView {
                amount_micros: c.amount_micros.to_string(),
                currency: c.currency.clone(),
            }),
        }
    }
}
impl From<Receipt> for ReceiptView {
    fn from(value: Receipt) -> Self {
        Self {
            id: value.id,
            task_id: value.task_id,
            capsule_id: value.capsule_id,
            resource_id: value.resource_id,
            model_id: value.model_id,
            access_mode: value.access_mode,
            artifacts: value.artifacts,
            execution: value.execution,
            verification: value.verification,
            usage: UsageView::from(&value.usage),
        }
    }
}
/// Metadata suitable for the inspector, without a local content path.
#[derive(Debug, Serialize)]
pub struct ArtifactView {
    /// Artifact identity.
    pub id: ArtifactId,
    /// Semantic purpose.
    pub kind: ArtifactKind,
    /// Observed media type.
    pub media_type: Option<String>,
    /// Lossless known size, independently of preview availability.
    pub size_bytes: Option<String>,
}
impl From<Artifact> for ArtifactView {
    fn from(value: Artifact) -> Self {
        Self {
            id: value.id,
            kind: value.kind,
            media_type: value.media_type,
            size_bytes: value.size_bytes.map(|n| n.to_string()),
        }
    }
}
/// A bounded, inert preview of untrusted result content.
#[derive(Debug, Serialize)]
pub struct ArtifactPreview {
    /// Metadata remains available for every preview outcome.
    pub artifact: ArtifactView,
    /// Why text is or is not available.
    pub status: PreviewStatus,
    /// Exact small UTF-8 text; never HTML-rendered or executed.
    pub text: Option<String>,
}
/// Preview is a display decision, never a modification of canonical bytes.
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PreviewStatus {
    /// Text is small enough and valid UTF-8.
    Text,
    /// Actual content exceeded the UI limit.
    TooLarge,
    /// Media type is not known to be text-like.
    Binary,
    /// Text-like metadata did not contain valid UTF-8.
    InvalidUtf8,
}
/// Result inspector fetched only on selection.
#[derive(Debug, Serialize)]
pub struct ResultDetail {
    /// Historical receipt.
    pub receipt: ReceiptView,
    /// Bounded previews for its artifacts.
    pub artifacts: Vec<ArtifactPreview>,
}
/// Portable rendered instruction snapshot.
#[derive(Debug, Serialize)]
pub struct Instructions {
    /// Persisted capsule identity.
    pub capsule_id: CapsuleId,
    /// Markdown ready to copy to any external AI.
    pub rendered: String,
    /// Declared estimate, not a counted provider limit.
    pub max_estimated_tokens: String,
}
/// Assignment inspector, using its captured exact agent revision.
#[derive(Debug, Serialize)]
pub struct AssignmentDetail {
    /// Allocation.
    pub assignment: Assignment,
    /// Assigned work.
    pub task: Task,
    /// Execution surface.
    pub session: Session,
    /// Exact historical agent.
    pub agent: AgentSpec,
    /// Shared access capacity.
    pub resource: Resource,
    /// Openable HTTP/HTTPS reference, validated in Rust.
    pub external_url: Option<String>,
    /// Prepared portable instructions.
    pub instructions: Option<Instructions>,
    /// Result metadata, if received.
    pub result: Option<ResultSummary>,
}
/// Resource creation; Rust generates its canonical identity.
#[derive(Debug, Deserialize)]
pub struct ResourceInput {
    /// Human name.
    pub name: String,
    /// Free-form provider, without credentials.
    pub provider: String,
    /// Declared transport.
    pub access_mode: AccessMode,
    /// Declared scarcity.
    pub scarcity: Scarcity,
    /// Declared availability.
    pub status: ResourceStatus,
}
/// Initial revision-one agent definition.
#[derive(Debug, Deserialize)]
pub struct AgentInput {
    /// Human name.
    pub name: String,
    /// Specialization.
    pub role: String,
    /// Purpose.
    pub mission: String,
    /// Explicit behavioral instructions.
    pub instructions: Vec<String>,
    /// Human expertise labels.
    pub expertise: Vec<String>,
    /// Actual domain policy, enforced by the store.
    pub delegation: DelegationPolicy,
}
/// Session creation binds exact revision, never an implicit latest agent.
#[derive(Debug, Deserialize)]
pub struct SessionInput {
    /// Human label.
    pub label: String,
    /// Existing access capacity.
    pub resource_id: ResourceId,
    /// Existing agent identity.
    pub agent_spec_id: AgentSpecId,
    /// Exact historical revision.
    pub agent_spec_revision: NonZeroU32,
    /// Opaque external reference, optionally openable as HTTP/HTTPS.
    pub external_ref: Option<String>,
}
/// Pending task creation with existing optional graph references.
#[derive(Debug, Deserialize)]
pub struct TaskInput {
    /// Human title.
    pub title: String,
    /// Intended work.
    pub objective: String,
    /// Existing enclosing task.
    pub parent: Option<TaskId>,
    /// Existing prerequisites; core collapses duplicates.
    pub dependencies: Vec<TaskId>,
}
/// Explicit selected context, with files snapshotted inline by Rust.
#[derive(Debug, Deserialize)]
pub struct PreparationInput {
    /// Explicit portable inputs.
    pub inputs: Vec<ContextInput>,
    /// Files selected by a native dialog, not persisted as absolute references.
    pub context_files: Vec<PathBuf>,
    /// Worker constraints.
    pub constraints: Vec<String>,
    /// User criteria, not automatic verification.
    pub acceptance_criteria: Vec<String>,
    /// Explicit expected outputs.
    pub expected_outputs: Vec<ExpectedOutput>,
    /// Decimal estimated token bound; no counting occurs here.
    pub max_context_tokens: String,
}
/// Optional observations for manual ingestion; unknown values must remain `None`.
#[derive(Debug, Default, Deserialize)]
pub struct IngestInput {
    /// Actual model, if known.
    pub model_id: Option<ModelId>,
    /// Semantic output kind; defaults to text.
    pub kind: Option<ArtifactKind>,
    /// Media type; defaults to text/markdown.
    pub media_type: Option<String>,
    /// Observed decimal input tokens.
    pub input_tokens: Option<String>,
    /// Observed decimal output tokens.
    pub output_tokens: Option<String>,
    /// Observed decimal monetary millionths, requiring a currency.
    pub cost_micros: Option<String>,
    /// Currency, requiring a cost observation.
    pub currency: Option<String>,
}
