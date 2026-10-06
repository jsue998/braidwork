use crate::{Project, ProjectError};
use braidwork_core::{
    agent::AgentSpec,
    artifact::{Artifact, ArtifactKind},
    assignment::{Assignment, AssignmentStatus},
    capsule::{ContextBudget, ContextInput, ExpectedOutput, TaskCapsule},
    id::{
        ArtifactId, AssignmentId, CapsuleId, DelegationId, ModelId, ReceiptId, SessionId, TaskId,
    },
    receipt::{ExecutionOutcome, Receipt, Usage, Verification},
    resource::{AccessMode, Resource},
    session::Session,
    task::Task,
};
use serde::Serialize;
use std::fmt::Write;
use uuid::Uuid;

/// Declared selected-context budget used by preparation unless overridden.
pub const DEFAULT_CONTEXT_TOKENS: u64 = 4096;

/// Explicit selected context and requirements; no retrieval or token estimation.
#[derive(Clone, Debug)]
pub struct CapsulePreparation {
    /// Already selected portable inputs.
    pub inputs: Vec<ContextInput>,
    /// Conditions the worker must respect.
    pub constraints: Vec<String>,
    /// Caller-supplied criteria, separate from the task's objective.
    pub acceptance_criteria: Vec<String>,
    /// Requested semantic outputs; the default requests generic text.
    pub expected_outputs: Vec<ExpectedOutput>,
    /// Declared estimated-token bound; not a measured token count.
    pub max_context_tokens: u64,
}
impl Default for CapsulePreparation {
    fn default() -> Self {
        Self {
            inputs: Vec::new(),
            constraints: Vec::new(),
            acceptance_criteria: Vec::new(),
            expected_outputs: vec![ExpectedOutput {
                kind: ArtifactKind::Text,
                description: "Provide the task result".into(),
            }],
            max_context_tokens: DEFAULT_CONTEXT_TOKENS,
        }
    }
}

/// Optional honest observations and semantic metadata for a manually returned result.
#[derive(Clone, Debug)]
pub struct IngestOptions {
    /// Actual execution model when known; never inferred from resource/provider.
    pub model_id: Option<ModelId>,
    /// Independent optional measurements, including known zero.
    pub usage: Usage,
    /// Purpose of the returned content, chosen by the caller.
    pub kind: ArtifactKind,
    /// Content media type, when known.
    pub media_type: Option<String>,
}
impl Default for IngestOptions {
    fn default() -> Self {
        Self {
            model_id: None,
            usage: Usage::default(),
            kind: ArtifactKind::Text,
            media_type: Some("text/markdown".into()),
        }
    }
}

/// Persistent records produced by one manual ingestion, suitable for JSON output.
#[derive(Clone, Debug, Serialize)]
pub struct IngestResult {
    /// Allocation now at result-received, without implied acceptance.
    pub assignment: Assignment,
    /// Metadata referencing the exact returned bytes.
    pub artifact: Artifact,
    /// Execution outcome, verification, usage, and execution identities.
    pub receipt: Receipt,
}
/// Everything a person needs to locate a session and copy its portable instructions.
#[derive(Clone, Debug, Serialize)]
pub struct Dispatch {
    /// Historical allocation.
    pub assignment: Assignment,
    /// Execution surface and opaque external reference.
    pub session: Session,
    /// Access capacity shared independently of session identity.
    pub resource: Resource,
    /// Exact historical agent definition.
    pub agent: AgentSpec,
    /// Assigned work.
    pub task: Task,
    /// Provider-independent instructions for copying manually.
    pub rendered: String,
}

impl Project {
    /// Allocates an existing task to an existing session, capturing its agent revision.
    ///
    /// # Errors
    /// Returns typed persistence or identity-construction failures.
    pub fn create_assignment(
        &mut self,
        task_id: &TaskId,
        session_id: &SessionId,
    ) -> Result<Assignment, ProjectError> {
        self.store().get_task(task_id)?;
        let session = self.store().get_session(session_id)?;
        self.store()
            .get_agent_spec(&session.agent_spec_id, session.agent_spec_revision)?;
        let assignment = Assignment {
            id: AssignmentId::new(Uuid::new_v4().to_string())?,
            task_id: task_id.clone(),
            session_id: session_id.clone(),
            agent_spec_id: session.agent_spec_id,
            agent_spec_revision: session.agent_spec_revision,
            status: AssignmentStatus::Prepared,
        };
        self.store_mut().insert_assignment(&assignment)?;
        Ok(assignment)
    }
    /// Records an explicit delegation between existing allocations.
    ///
    /// # Errors
    /// Returns core self-delegation, policy, reference, or persistence failures.
    pub fn add_delegation(
        &mut self,
        from: AssignmentId,
        to: AssignmentId,
    ) -> Result<braidwork_core::delegation::Delegation, ProjectError> {
        let edge = braidwork_core::delegation::Delegation::new(
            DelegationId::new(Uuid::new_v4().to_string())?,
            from,
            to,
        )?;
        self.store_mut().insert_delegation(&edge)?;
        Ok(edge)
    }
    /// Snapshots task and historical agent text into one portable capsule.
    ///
    /// # Errors
    /// Returns workflow/persistence failures or rejects selected context with zero budget.
    pub fn prepare_capsule(
        &mut self,
        assignment_id: &AssignmentId,
        preparation: CapsulePreparation,
    ) -> Result<TaskCapsule, ProjectError> {
        if preparation.max_context_tokens == 0 && !preparation.inputs.is_empty() {
            return Err(ProjectError::ContextForbidden);
        }
        let assignment = self.store().get_assignment(assignment_id)?;
        let task = self.store().get_task(&assignment.task_id)?;
        let agent = self
            .store()
            .get_agent_spec(&assignment.agent_spec_id, assignment.agent_spec_revision)?;
        let capsule = TaskCapsule {
            id: CapsuleId::new(Uuid::new_v4().to_string())?,
            task_id: task.id().clone(),
            role: agent.role,
            mission: agent.mission,
            instructions: agent.instructions,
            objective: task.objective().to_owned(),
            inputs: preparation.inputs,
            constraints: preparation.constraints,
            acceptance_criteria: preparation.acceptance_criteria,
            expected_outputs: preparation.expected_outputs,
            context_budget: ContextBudget {
                max_estimated_tokens: preparation.max_context_tokens,
            },
        };
        self.store_mut()
            .prepare_assignment_capsule(assignment_id, &capsule)?;
        Ok(capsule)
    }
    /// Renders a capsule using only its portable data; external references stay opaque.
    ///
    /// # Errors
    /// Returns typed capsule-not-found or reconstruction/database failures.
    pub fn render_capsule(&self, capsule_id: &CapsuleId) -> Result<String, ProjectError> {
        Ok(render_capsule(&self.store().get_capsule(capsule_id)?))
    }
    /// Returns a manual dispatch bundle; inspection has no state-changing side effect.
    ///
    /// # Errors
    /// Returns missing-workflow/entity or reconstruction/persistence failures.
    pub fn dispatch(&self, assignment_id: &AssignmentId) -> Result<Dispatch, ProjectError> {
        let assignment = self.store().get_assignment(assignment_id)?;
        let capsule = self.store().get_assignment_capsule(assignment_id)?;
        let session = self.store().get_session(&assignment.session_id)?;
        let resource = self.store().get_resource(&session.resource_id)?;
        let agent = self
            .store()
            .get_agent_spec(&assignment.agent_spec_id, assignment.agent_spec_revision)?;
        let task = self.store().get_task(&assignment.task_id)?;
        Ok(Dispatch {
            assignment,
            session,
            resource,
            agent,
            task,
            rendered: render_capsule(&capsule),
        })
    }
    /// Stores exact manually returned bytes, then commits metadata, receipt, and status.
    ///
    /// Receipt verification is not performed and the task is never marked completed.
    /// Access mode records this manual transport, independently of resource configuration.
    /// On database failure, only the newly written unreferenced content is removed.
    /// A process crash between filesystem and DB operations may leave an orphan;
    /// no distributed filesystem/SQLite transaction or recovery engine is claimed.
    ///
    /// # Errors
    /// Returns content, workflow, or persistence failures; cleanup failure retains
    /// both errors in [`ProjectError::IngestCleanup`].
    pub fn ingest(
        &mut self,
        assignment_id: &AssignmentId,
        bytes: &[u8],
        options: IngestOptions,
    ) -> Result<IngestResult, ProjectError> {
        let assignment = self.store().get_assignment(assignment_id)?;
        let capsule = self.store().get_assignment_capsule(assignment_id)?;
        let session = self.store().get_session(&assignment.session_id)?;
        if assignment.status == AssignmentStatus::ResultReceived {
            return Err(braidwork_store::StoreError::Workflow { assignment: assignment_id.clone(), reason: "a result has already been received; create a new assignment for another attempt" }.into());
        }
        let artifact_id = ArtifactId::new(Uuid::new_v4().to_string())?;
        let receipt_id = ReceiptId::new(Uuid::new_v4().to_string())?;
        let content_ref = self.write_artifact_content(&artifact_id, bytes)?;
        let artifact = Artifact {
            id: artifact_id,
            task_id: assignment.task_id.clone(),
            kind: options.kind,
            content_ref,
            media_type: options.media_type,
            size_bytes: Some(bytes.len() as u64),
        };
        let receipt = Receipt {
            id: receipt_id,
            task_id: assignment.task_id,
            capsule_id: capsule.id,
            resource_id: session.resource_id,
            model_id: options.model_id,
            access_mode: AccessMode::Manual,
            artifacts: vec![artifact.id.clone()],
            execution: ExecutionOutcome::Completed,
            verification: Verification::NotPerformed,
            usage: options.usage,
        };
        match self
            .store_mut()
            .record_assignment_result(assignment_id, &artifact, &receipt)
        {
            Ok(assignment) => Ok(IngestResult {
                assignment,
                artifact,
                receipt,
            }),
            Err(store) => match self.remove_new_content(&artifact.content_ref) {
                Ok(()) => Err(store.into()),
                Err(cleanup) => Err(ProjectError::IngestCleanup { store, cleanup }),
            },
        }
    }
}

/// Renders provider-independent Markdown, without resolving references or counting tokens.
#[must_use]
pub fn render_capsule(capsule: &TaskCapsule) -> String {
    let mut text = format!(
        "# Braidwork Task Capsule\n\nCapsule: {}\nTask: {}\nDeclared context budget: {} estimated tokens (not counted)\n\n## Role\n{}\n\n## Mission\n{}\n\n## Objective\n{}\n\n## Context\n",
        capsule.id,
        capsule.task_id,
        capsule.context_budget.max_estimated_tokens,
        capsule.role,
        capsule.mission,
        capsule.objective
    );
    if capsule.inputs.is_empty() {
        text.push_str("(none)\n");
    }
    for input in &capsule.inputs {
        match input {
            ContextInput::Inline {
                label,
                text: content,
            } => {
                let _ = write!(text, "\n### {label}\n{content}\n");
            }
            ContextInput::Reference { label, reference } => {
                let _ = write!(
                    text,
                    "\n### {label}\nExternal context reference: {reference}\n"
                );
            }
        }
    }
    section(&mut text, "Instructions", &capsule.instructions);
    section(&mut text, "Constraints", &capsule.constraints);
    section(
        &mut text,
        "Acceptance Criteria",
        &capsule.acceptance_criteria,
    );
    text.push_str("\n## Expected Output\n");
    if capsule.expected_outputs.is_empty() {
        text.push_str("(none specified)\n");
    }
    for output in &capsule.expected_outputs {
        let _ = writeln!(text, "- {:?}: {}", output.kind, output.description);
    }
    text.push_str("\n## Response Contract\nReturn the result as readable Markdown. Use the following headings when useful; no JSON is required. Include substantive output rather than only a completion claim.\n\n# Braidwork Result\n\n## Summary\nBriefly describe what you did.\n\n## Output\nProvide the requested work.\n\n## Notes\nState uncertainty, sources, limitations, or questions.\n");
    text
}
fn section(text: &mut String, heading: &str, values: &[String]) {
    let _ = write!(text, "\n## {heading}\n");
    if values.is_empty() {
        text.push_str("(none)\n");
    }
    for value in values {
        let _ = writeln!(text, "- {value}");
    }
}
