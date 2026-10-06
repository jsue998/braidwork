use braidwork_core::{
    artifact::ArtifactKind,
    capsule::ExpectedOutput,
    id::{
        AgentSpecId, AssignmentId, CapsuleId, DelegationId, ModelId, ResourceId, SessionId, TaskId,
    },
};
use braidwork_project::DEFAULT_CONTEXT_TOKENS;
use clap::{ArgGroup, Args, Subcommand};
use std::{num::NonZeroU32, path::PathBuf};

#[derive(Subcommand)]
pub enum AgentCommand {
    /// Insert an immutable agent revision; definitions are provider-independent
    Add(AgentArgs),
    /// List all agent revisions in ID/revision order
    List,
    /// Show one exact agent revision
    Show {
        id: AgentSpecId,
        #[arg(long, default_value = "1")]
        revision: NonZeroU32,
    },
}
#[derive(Args)]
pub struct AgentArgs {
    /// Identity shared by the historical revisions
    pub id: AgentSpecId,
    /// Positive immutable revision; inserting an existing revision fails
    #[arg(long, default_value = "1")]
    pub revision: NonZeroU32,
    /// Human name
    #[arg(long)]
    pub name: String,
    /// Specialist responsibility
    #[arg(long)]
    pub role: String,
    /// Purpose of this agent
    #[arg(long)]
    pub mission: String,
    /// Working instruction; repeat for multiple instructions
    #[arg(long)]
    pub instruction: Vec<String>,
    /// Declared specialty; repeat for multiple specialties
    #[arg(long)]
    pub expertise: Vec<String>,
    /// Permit explicit delegation by this revision
    #[arg(long)]
    pub allow_delegation: bool,
    /// Maximum direct delegated children per assignment (requires permission)
    #[arg(long, requires = "allow_delegation")]
    pub max_children: Option<u32>,
}
#[derive(Subcommand)]
pub enum SessionCommand {
    /// Register an execution instance; many may share a resource
    Add(SessionArgs),
    /// List execution instances by ID
    List,
    /// Show an instance and its opaque external reference
    Show { id: SessionId },
}
#[derive(Args)]
pub struct SessionArgs {
    /// Persistent instance identity
    pub id: SessionId,
    /// Human label
    #[arg(long)]
    pub label: String,
    /// Existing capacity identity
    #[arg(long)]
    pub resource: ResourceId,
    /// Existing agent definition
    #[arg(long)]
    pub agent: AgentSpecId,
    /// Exact configured agent revision; not a latest-revision lookup
    #[arg(long, default_value = "1")]
    pub agent_revision: NonZeroU32,
    /// Opaque chat label, URL, thread ID, or manual handle
    #[arg(long)]
    pub external_ref: Option<String>,
}
#[derive(Subcommand)]
pub enum AssignmentCommand {
    /// Allocate an existing task to a session and capture its exact agent revision
    Create {
        task_id: TaskId,
        #[arg(long)]
        session: SessionId,
    },
    /// List allocations by ID
    List,
    /// Show allocation, prepared capsule, and received receipt when present
    Show { id: AssignmentId },
}
#[derive(Subcommand)]
pub enum DelegationCommand {
    /// Record an explicit edge under the parent's historical agent policy
    Add {
        #[arg(long)]
        from: AssignmentId,
        #[arg(long)]
        to: AssignmentId,
    },
    /// List delegation edges by ID
    List,
    /// Show an exact delegation edge
    Show { id: DelegationId },
}
#[derive(Subcommand)]
pub enum CapsuleCommand {
    /// Prepare one portable capsule from the assignment's historical agent and task
    Prepare(PrepareArgs),
    /// Render provider-independent instructions ready to copy manually
    Render { id: CapsuleId },
}
#[derive(Args)]
pub struct PrepareArgs {
    /// Allocation whose capsule is prepared once
    pub assignment_id: AssignmentId,
    /// Worker condition; repeat as needed
    #[arg(long)]
    pub constraint: Vec<String>,
    /// Acceptance criterion; repeat as needed
    #[arg(long)]
    pub accept: Vec<String>,
    /// Expected output KIND:DESCRIPTION; defaults to generic text
    #[arg(long, value_parser = parse_output, value_name = "KIND:DESCRIPTION")]
    pub output: Vec<ExpectedOutput>,
    /// UTF-8 file to snapshot as inline context; repeat as needed
    #[arg(long)]
    pub context_file: Vec<PathBuf>,
    /// Explicit inline context; repeat as needed
    #[arg(long)]
    pub context_text: Vec<String>,
    /// Declared estimated-token budget; no token counting is performed
    #[arg(long, default_value_t = DEFAULT_CONTEXT_TOKENS)]
    pub max_context_tokens: u64,
}
#[derive(Args)]
pub struct DispatchArgs {
    /// Allocation whose prepared instructions are displayed
    pub assignment_id: AssignmentId,
    /// Explicitly record the handoff; display alone has no state-changing effect
    #[arg(long)]
    pub mark_dispatched: bool,
}
#[derive(Args)]
#[command(group(ArgGroup::new("source").required(true).args(["file", "stdin"])))]
pub struct IngestArgs {
    /// Allocation receiving its first manual result
    pub assignment_id: AssignmentId,
    /// File containing the exact returned bytes
    #[arg(long)]
    pub file: Option<PathBuf>,
    /// Read exact result bytes from standard input instead of a file
    #[arg(long)]
    pub stdin: bool,
    /// Actual model identity when known; never inferred
    #[arg(long)]
    pub model: Option<ModelId>,
    /// Known input tokens (omit when unknown)
    #[arg(long)]
    pub input_tokens: Option<u64>,
    /// Known output tokens (omit when unknown)
    #[arg(long)]
    pub output_tokens: Option<u64>,
    /// Known cost in currency millionths, including zero
    #[arg(long, requires = "currency")]
    pub cost_micros: Option<u64>,
    /// Explicit currency for the known monetary cost
    #[arg(long, requires = "cost_micros")]
    pub currency: Option<String>,
    /// Semantic artifact kind: patch, analysis, finding, `test_result`, documentation, question, text, data
    #[arg(long, default_value = "text", value_parser = parse_kind)]
    pub kind: ArtifactKind,
    /// Media type of the returned bytes
    #[arg(long, default_value = "text/markdown")]
    pub media_type: String,
}
fn parse_kind(value: &str) -> Result<ArtifactKind, String> {
    serde_json::from_value(serde_json::Value::String(value.to_owned()))
        .map_err(|error| error.to_string())
}
fn parse_output(value: &str) -> Result<ExpectedOutput, String> {
    let (kind, description) = value
        .split_once(':')
        .ok_or("expected KIND:DESCRIPTION, for example analysis:Explain the evidence")?;
    Ok(ExpectedOutput {
        kind: parse_kind(kind)?,
        description: description.to_owned(),
    })
}
