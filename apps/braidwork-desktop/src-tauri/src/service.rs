//! Serialized desktop operations. No SQL, CLI process, or domain authority lives here.
use crate::{
    dto::{
        self, AgentInput, ArtifactPreview, AssignmentDetail, IngestInput, Instructions,
        PreparationInput, PreviewStatus, ProjectInfo, ResourceInput, ResultDetail, ResultSummary,
        SessionInput, TaskInput, WorkflowSummary, WorkspaceSnapshot,
    },
    error::IpcError,
    state::ProjectState,
};
use braidwork_core::{
    agent::AgentSpec,
    assignment::{Assignment, AssignmentStatus},
    capsule::ContextInput,
    delegation::Delegation,
    id::{AgentSpecId, AssignmentId, CapsuleId, ResourceId, SessionId, TaskId},
    receipt::{MonetaryCost, Usage},
    resource::Resource,
    session::{Session, SessionStatus},
    task::{Task, TaskStatus},
};
use braidwork_project::{CapsulePreparation, IngestOptions, Project, ProjectError};
use braidwork_store::StoreError;
use std::{fs, num::NonZeroU32, path::Path};
use uuid::Uuid;

/// Maximum automatic text preview size; canonical content is never truncated.
pub const PREVIEW_LIMIT: u64 = 1024 * 1024;

/// Owns only a canonical root, serializing operations without sharing a connection.
#[derive(Default)]
pub struct DesktopService {
    state: ProjectState,
}
impl DesktopService {
    /// Creates a new child directory and initializes a project there.
    ///
    /// Human metadata is preserved; folder names are validated for portability.
    /// A failed initialization removes only a still-empty new directory and
    /// preserves both the original error and the previously active project.
    ///
    /// # Errors
    /// Returns invalid-name/folder/parent, destination-exists, I/O, or project errors.
    pub fn create_project(
        &self,
        parent_directory: &Path,
        folder_name: &str,
        display_name: &str,
    ) -> Result<ProjectInfo, IpcError> {
        if display_name.trim().is_empty() {
            return Err(ProjectError::InvalidName.into());
        }
        let root = crate::creation::destination(parent_directory, folder_name)?;
        let mut active = self.state.lock()?;
        crate::creation::create_directory(&root)?;
        let initialized = Project::init(&root, Some(display_name));
        let project = match initialized {
            Ok(project) => project,
            Err(error) => {
                // remove_dir never removes nonempty content, including partial init state.
                let _ = fs::remove_dir(&root);
                return Err(error.into());
            }
        };
        let info = project_info(&project)?;
        *active = Some(project.root().to_owned());
        Ok(info)
    }
    /// Initializes a real project and selects it only after successful initialization.
    ///
    /// # Errors
    /// Returns structured project, input, or state-access errors.
    pub fn init_project(&self, root: &Path, name: &str) -> Result<ProjectInfo, IpcError> {
        let mut active = self.state.lock()?;
        let project = Project::init(root, Some(name))?;
        let info = project_info(&project)?;
        *active = Some(project.root().to_owned());
        Ok(info)
    }
    /// Opens exactly this root; a failed switch preserves the previous active project.
    ///
    /// # Errors
    /// Returns structured project or state-access errors; no missing files are recreated.
    pub fn open_project(&self, root: &Path) -> Result<ProjectInfo, IpcError> {
        let mut active = self.state.lock()?;
        let project = Project::open(root)?;
        let info = project_info(&project)?;
        *active = Some(project.root().to_owned());
        Ok(info)
    }
    /// Closes the view without deleting or changing any project data.
    ///
    /// # Errors
    /// Returns a state-access error if an earlier operation poisoned the lock.
    pub fn close_project(&self) -> Result<(), IpcError> {
        *self.state.lock()? = None;
        Ok(())
    }

    fn with_project<T>(
        &self,
        action: impl FnOnce(&mut Project) -> Result<T, IpcError>,
    ) -> Result<T, IpcError> {
        let active = self.state.lock()?;
        let root = active
            .as_ref()
            .ok_or_else(|| IpcError::new("no_project", "Open or create a project first."))?;
        let mut project = Project::open(root)?;
        let result = action(&mut project);
        drop(active);
        result
    }
    /// Reads real workspace entities and manual-workflow links without loading artifact bytes.
    ///
    /// # Errors
    /// Returns no-project, project, store, or reconstruction errors.
    pub fn workspace_snapshot(&self) -> Result<WorkspaceSnapshot, IpcError> {
        self.with_project(|project| {
            let store = project.store();
            let assignments = store.list_assignments()?;
            let workflow = assignments
                .iter()
                .map(|assignment| {
                    Ok(WorkflowSummary {
                        assignment_id: assignment.id.clone(),
                        capsule_id: optional_capsule(project, assignment)?.map(|c| c.id),
                        result: result_summary(project, assignment)?,
                    })
                })
                .collect::<Result<Vec<_>, IpcError>>()?;
            Ok(WorkspaceSnapshot {
                project: project_info(project)?,
                resources: store.list_resources()?,
                agents: store.list_agent_specs()?,
                sessions: store.list_sessions()?,
                tasks: store.list_tasks()?,
                delegations: store.list_delegations()?,
                assignments,
                workflow,
            })
        })
    }
    /// Creates a resource with a Rust-generated identity and caller-declared metadata.
    ///
    /// # Errors
    /// Returns input, project, or persistence errors.
    pub fn create_resource(&self, input: ResourceInput) -> Result<Resource, IpcError> {
        required(&input.name, "Resource name")?;
        required(&input.provider, "Provider")?;
        self.with_project(|project| {
            let resource = Resource {
                id: ResourceId::new(Uuid::new_v4().to_string())
                    .map_err(|e| IpcError::input(e.to_string()))?,
                name: input.name,
                provider: input.provider,
                access_mode: input.access_mode,
                scarcity: input.scarcity,
                status: input.status,
            };
            project.store_mut().insert_resource(&resource)?;
            Ok(resource)
        })
    }
    /// Creates revision one of a provider-independent agent with a Rust-generated ID.
    ///
    /// # Errors
    /// Returns input, project, or persistence errors.
    pub fn create_agent(&self, input: AgentInput) -> Result<AgentSpec, IpcError> {
        required(&input.name, "Agent name")?;
        required(&input.role, "Role")?;
        required(&input.mission, "Mission")?;
        self.with_project(|project| {
            let agent = AgentSpec {
                id: AgentSpecId::new(Uuid::new_v4().to_string())
                    .map_err(|e| IpcError::input(e.to_string()))?,
                revision: NonZeroU32::MIN,
                name: input.name,
                role: input.role,
                mission: input.mission,
                instructions: input.instructions,
                expertise: input.expertise,
                delegation: input.delegation,
            };
            project.store_mut().insert_agent_spec(&agent)?;
            Ok(agent)
        })
    }
    /// Creates an independent ready session bound to an exact existing agent revision.
    ///
    /// # Errors
    /// Returns input, project, or typed reference/persistence errors.
    pub fn create_session(&self, input: SessionInput) -> Result<Session, IpcError> {
        required(&input.label, "Session label")?;
        self.with_project(|project| {
            let session = Session {
                id: SessionId::new(Uuid::new_v4().to_string())
                    .map_err(|e| IpcError::input(e.to_string()))?,
                label: input.label,
                resource_id: input.resource_id,
                agent_spec_id: input.agent_spec_id,
                agent_spec_revision: input.agent_spec_revision,
                external_ref: input.external_ref,
                status: SessionStatus::Ready,
            };
            project.store_mut().insert_session(&session)?;
            Ok(session)
        })
    }
    /// Creates pending work through the validated core constructor.
    ///
    /// # Errors
    /// Returns input, domain relationship, project, or persistence errors.
    pub fn create_task(&self, input: TaskInput) -> Result<Task, IpcError> {
        required(&input.title, "Task title")?;
        required(&input.objective, "Objective")?;
        self.with_project(|project| {
            let task = Task::new(
                TaskId::new(Uuid::new_v4().to_string())
                    .map_err(|e| IpcError::input(e.to_string()))?,
                input.title,
                input.objective,
                input.parent,
                input.dependencies,
            )
            .map_err(|e| IpcError::input(e.to_string()))?;
            project.store_mut().insert_task(&task)?;
            Ok(task)
        })
    }
    /// Records explicit user-declared progress; it does not verify a receipt.
    ///
    /// # Errors
    /// Returns project, task-not-found, or persistence errors.
    pub fn set_task_status(&self, id: &TaskId, status: TaskStatus) -> Result<Task, IpcError> {
        self.with_project(|project| Ok(project.set_task_status(id, status)?))
    }
    /// Allocates work through the real project API, freezing the exact agent revision.
    ///
    /// # Errors
    /// Returns project, missing-reference, or persistence errors.
    pub fn create_assignment(
        &self,
        task: &TaskId,
        session: &SessionId,
    ) -> Result<Assignment, IpcError> {
        self.with_project(|project| Ok(project.create_assignment(task, session)?))
    }
    /// Registers explicit delegation under the historical parent's real policy.
    ///
    /// # Errors
    /// Returns self-delegation, missing-reference, policy, or persistence errors.
    pub fn create_delegation(
        &self,
        from: AssignmentId,
        to: AssignmentId,
    ) -> Result<Delegation, IpcError> {
        self.with_project(|project| Ok(project.add_delegation(from, to)?))
    }
    /// Snapshots explicit UTF-8 context files inline and prepares real portable instructions.
    ///
    /// # Errors
    /// Returns input, file/UTF-8, zero-budget, workflow, or persistence errors.
    pub fn prepare_capsule(
        &self,
        assignment: &AssignmentId,
        input: PreparationInput,
    ) -> Result<Instructions, IpcError> {
        self.with_project(|project| {
            let mut inputs = input.inputs;
            for path in input.context_files {
                let text = fs::read_to_string(&path).map_err(|source| {
                    IpcError::from(ProjectError::Io {
                        path: path.clone(),
                        source,
                    })
                })?;
                let label = path.file_name().map_or_else(
                    || "Selected context".into(),
                    |n| n.to_string_lossy().into_owned(),
                );
                inputs.push(ContextInput::Inline { label, text });
            }
            let capsule = project.prepare_capsule(
                assignment,
                CapsulePreparation {
                    inputs,
                    constraints: input.constraints,
                    acceptance_criteria: input.acceptance_criteria,
                    expected_outputs: input.expected_outputs,
                    max_context_tokens: decimal(&input.max_context_tokens, "Context budget")?,
                },
            )?;
            Ok(instructions(&capsule))
        })
    }
    /// Renders a persisted capsule without dispatching or changing state.
    ///
    /// # Errors
    /// Returns project, missing-capsule, or reconstruction/persistence errors.
    pub fn render_capsule(&self, id: &CapsuleId) -> Result<Instructions, IpcError> {
        self.with_project(|project| Ok(instructions(&project.store().get_capsule(id)?)))
    }
    /// Explicitly records dispatch; inspection and copying never call this implicitly.
    ///
    /// # Errors
    /// Returns project, workflow, or persistence errors.
    pub fn mark_dispatched(&self, id: &AssignmentId) -> Result<Assignment, IpcError> {
        self.with_project(|project| Ok(project.store_mut().mark_assignment_dispatched(id)?))
    }
    /// Ingests exact pasted text through Manual Bridge, with no verification or task completion.
    ///
    /// # Errors
    /// Returns input, project, content, workflow, or persistence errors.
    pub fn ingest_text(
        &self,
        id: &AssignmentId,
        text: &str,
        input: IngestInput,
    ) -> Result<ResultSummary, IpcError> {
        self.with_project(|project| {
            let result = project.ingest(id, text.as_bytes(), ingest_options(input)?)?;
            Ok(ResultSummary {
                receipt: result.receipt.into(),
                artifacts: vec![result.artifact.into()],
            })
        })
    }
    /// Imports exact selected file bytes; arbitrary HTML is never executed or rendered.
    ///
    /// # Errors
    /// Returns file, input, project, content, workflow, or persistence errors.
    pub fn ingest_file(
        &self,
        id: &AssignmentId,
        path: &Path,
        input: IngestInput,
    ) -> Result<ResultSummary, IpcError> {
        self.with_project(|project| {
            let bytes = fs::read(path).map_err(|source| {
                IpcError::from(ProjectError::Io {
                    path: path.to_owned(),
                    source,
                })
            })?;
            let result = project.ingest(id, &bytes, ingest_options(input)?)?;
            Ok(ResultSummary {
                receipt: result.receipt.into(),
                artifacts: vec![result.artifact.into()],
            })
        })
    }
    /// Resolves assignment provenance and prepared instructions, without changing state.
    ///
    /// # Errors
    /// Returns project, entity, workflow, or reconstruction/persistence errors.
    pub fn assignment_detail(&self, id: &AssignmentId) -> Result<AssignmentDetail, IpcError> {
        self.with_project(|project| {
            let assignment = project.store().get_assignment(id)?;
            let session = project.store().get_session(&assignment.session_id)?;
            Ok(AssignmentDetail {
                task: project.store().get_task(&assignment.task_id)?,
                agent: project
                    .store()
                    .get_agent_spec(&assignment.agent_spec_id, assignment.agent_spec_revision)?,
                resource: project.store().get_resource(&session.resource_id)?,
                external_url: session.external_ref.as_deref().and_then(http_url),
                session,
                instructions: optional_capsule(project, &assignment)?
                    .as_ref()
                    .map(instructions),
                result: result_summary(project, &assignment)?,
                assignment,
            })
        })
    }
    /// Fetches metadata and bounded inert previews for one received assignment.
    ///
    /// # Errors
    /// Returns project, missing-result, missing-content, or reconstruction/persistence errors.
    pub fn result_detail(&self, id: &AssignmentId) -> Result<ResultDetail, IpcError> {
        self.with_project(|project| {
            let receipt = project.store().get_assignment_receipt(id)?;
            let artifacts = receipt
                .artifacts
                .iter()
                .map(|id| preview(project, project.store().get_artifact(id)?))
                .collect::<Result<Vec<_>, IpcError>>()?;
            Ok(ResultDetail {
                receipt: receipt.into(),
                artifacts,
            })
        })
    }
    /// Resolves a session reference for a native browser action, permitting only HTTP/HTTPS.
    ///
    /// # Errors
    /// Returns invalid-reference, project, or persistence errors; opaque handles are never opened.
    pub fn external_url(&self, id: &SessionId) -> Result<String, IpcError> {
        self.with_project(|project| {
            project.store().get_session(id)?.external_ref.as_deref().and_then(http_url).ok_or_else(|| IpcError::input("This session reference is not an HTTP or HTTPS URL. Copy the reference instead."))
        })
    }
}
fn project_info(project: &Project) -> Result<ProjectInfo, IpcError> {
    Ok(ProjectInfo {
        name: project.metadata().name().into(),
        root: project.root().to_owned(),
        format_version: project.format_version(),
        schema_version: project.schema_version()?,
    })
}
fn required(value: &str, field: &str) -> Result<(), IpcError> {
    if value.trim().is_empty() {
        Err(IpcError::input(format!(
            "{field} must contain non-whitespace text."
        )))
    } else {
        Ok(())
    }
}
fn decimal(value: &str, field: &str) -> Result<u64, IpcError> {
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err(IpcError::input(format!(
            "{field} must be an unsigned decimal integer."
        )));
    }
    value.parse().map_err(|_| {
        IpcError::input(format!(
            "{field} exceeds the supported unsigned 64-bit range."
        ))
    })
}
fn optional_decimal(value: Option<&str>, field: &str) -> Result<Option<u64>, IpcError> {
    value.map(|v| decimal(v, field)).transpose()
}
fn ingest_options(input: IngestInput) -> Result<IngestOptions, IpcError> {
    let cost = match (input.cost_micros, input.currency) {
        (None, None) => None,
        (Some(amount), Some(currency)) => {
            required(&currency, "Currency")?;
            Some(MonetaryCost {
                amount_micros: decimal(&amount, "Cost")?,
                currency,
            })
        }
        _ => {
            return Err(IpcError::input(
                "Provide both cost micros and currency, or leave both unknown.",
            ));
        }
    };
    Ok(IngestOptions {
        model_id: input.model_id,
        usage: Usage {
            input_tokens: optional_decimal(input.input_tokens.as_deref(), "Input tokens")?,
            output_tokens: optional_decimal(input.output_tokens.as_deref(), "Output tokens")?,
            cost,
        },
        kind: input
            .kind
            .unwrap_or(braidwork_core::artifact::ArtifactKind::Text),
        media_type: input.media_type.or_else(|| Some("text/markdown".into())),
    })
}
fn instructions(capsule: &braidwork_core::capsule::TaskCapsule) -> Instructions {
    Instructions {
        capsule_id: capsule.id.clone(),
        rendered: braidwork_project::render_capsule(capsule),
        max_estimated_tokens: capsule.context_budget.max_estimated_tokens.to_string(),
    }
}
fn optional_capsule(
    project: &Project,
    assignment: &Assignment,
) -> Result<Option<braidwork_core::capsule::TaskCapsule>, IpcError> {
    match project.store().get_assignment_capsule(&assignment.id) {
        Ok(capsule) => Ok(Some(capsule)),
        Err(StoreError::Workflow { .. }) if assignment.status == AssignmentStatus::Prepared => {
            Ok(None)
        }
        Err(error) => Err(error.into()),
    }
}
fn result_summary(
    project: &Project,
    assignment: &Assignment,
) -> Result<Option<ResultSummary>, IpcError> {
    match project.store().get_assignment_receipt(&assignment.id) {
        Ok(receipt) => {
            let artifacts = receipt
                .artifacts
                .iter()
                .map(|id| {
                    project
                        .store()
                        .get_artifact(id)
                        .map(dto::ArtifactView::from)
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Some(ResultSummary {
                receipt: receipt.into(),
                artifacts,
            }))
        }
        Err(StoreError::Workflow { .. })
            if assignment.status != AssignmentStatus::ResultReceived =>
        {
            Ok(None)
        }
        Err(error) => Err(error.into()),
    }
}
fn http_url(value: &str) -> Option<String> {
    let url = url::Url::parse(value).ok()?;
    (matches!(url.scheme(), "http" | "https") && url.host_str().is_some()).then(|| url.to_string())
}
fn preview(
    project: &Project,
    artifact: braidwork_core::artifact::Artifact,
) -> Result<ArtifactPreview, IpcError> {
    let text_like = artifact.media_type.as_deref().is_some_and(|media| {
        let media = media
            .split(';')
            .next()
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        media.starts_with("text/")
            || matches!(media.as_str(), "application/json" | "application/xml")
            || media.ends_with("+json")
            || media.ends_with("+xml")
    });
    let (status, text) = if text_like {
        match project
            .read_artifact_content_up_to(&artifact.content_ref, PREVIEW_LIMIT)
            .map_err(ProjectError::from)?
        {
            None => (PreviewStatus::TooLarge, None),
            Some(bytes) => match String::from_utf8(bytes) {
                Ok(text) => (PreviewStatus::Text, Some(text)),
                Err(_) => (PreviewStatus::InvalidUtf8, None),
            },
        }
    } else {
        (PreviewStatus::Binary, None)
    };
    Ok(ArtifactPreview {
        artifact: artifact.into(),
        status,
        text,
    })
}

#[cfg(test)]
mod tests;
