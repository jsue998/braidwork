//! Native IPC adapter: filesystem and database work execute outside the UI thread.
use crate::{
    dto::{
        AgentInput, AssignmentDetail, IngestInput, Instructions, PreparationInput, ProjectInfo,
        ResourceInput, ResultDetail, ResultSummary, SessionInput, TaskInput, WorkspaceSnapshot,
    },
    error::IpcError,
    service::DesktopService,
};
use braidwork_core::{
    agent::AgentSpec,
    assignment::Assignment,
    delegation::Delegation,
    id::{AssignmentId, CapsuleId, SessionId, TaskId},
    resource::Resource,
    session::Session,
    task::{Task, TaskStatus},
};
use std::{path::PathBuf, sync::Arc};
use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

async fn call<T: Send + 'static>(
    app: AppHandle,
    action: impl FnOnce(&DesktopService) -> Result<T, IpcError> + Send + 'static,
) -> Result<T, IpcError> {
    tauri::async_runtime::spawn_blocking(move || {
        let service = app.try_state::<Arc<DesktopService>>().ok_or_else(|| {
            IpcError::new(
                "state_unavailable",
                "The desktop service is unavailable. Restart Braidwork.",
            )
        })?;
        action(service.inner().as_ref())
    })
    .await
    .map_err(|_| {
        IpcError::new(
            "operation_interrupted",
            "The operation was interrupted. Refresh or reopen the project.",
        )
    })?
}
#[tauri::command(rename_all = "snake_case")]
async fn init_project(
    app: AppHandle,
    root: PathBuf,
    name: String,
) -> Result<ProjectInfo, IpcError> {
    call(app, move |s| s.init_project(&root, &name)).await
}
#[tauri::command(rename_all = "snake_case")]
async fn open_project(app: AppHandle, root: PathBuf) -> Result<ProjectInfo, IpcError> {
    call(app, move |s| s.open_project(&root)).await
}
#[tauri::command]
async fn close_project(app: AppHandle) -> Result<(), IpcError> {
    call(app, DesktopService::close_project).await
}
#[tauri::command]
async fn workspace_snapshot(app: AppHandle) -> Result<WorkspaceSnapshot, IpcError> {
    call(app, DesktopService::workspace_snapshot).await
}
#[tauri::command]
async fn create_resource(app: AppHandle, input: ResourceInput) -> Result<Resource, IpcError> {
    call(app, move |s| s.create_resource(input)).await
}
#[tauri::command]
async fn create_agent(app: AppHandle, input: AgentInput) -> Result<AgentSpec, IpcError> {
    call(app, move |s| s.create_agent(input)).await
}
#[tauri::command]
async fn create_session(app: AppHandle, input: SessionInput) -> Result<Session, IpcError> {
    call(app, move |s| s.create_session(input)).await
}
#[tauri::command]
async fn create_task(app: AppHandle, input: TaskInput) -> Result<Task, IpcError> {
    call(app, move |s| s.create_task(input)).await
}
#[tauri::command]
async fn set_task_status(app: AppHandle, id: TaskId, status: TaskStatus) -> Result<Task, IpcError> {
    call(app, move |s| s.set_task_status(&id, status)).await
}
#[tauri::command(rename_all = "snake_case")]
async fn create_assignment(
    app: AppHandle,
    task_id: TaskId,
    session_id: SessionId,
) -> Result<Assignment, IpcError> {
    call(app, move |s| s.create_assignment(&task_id, &session_id)).await
}
#[tauri::command]
async fn create_delegation(
    app: AppHandle,
    from: AssignmentId,
    to: AssignmentId,
) -> Result<Delegation, IpcError> {
    call(app, move |s| s.create_delegation(from, to)).await
}
#[tauri::command(rename_all = "snake_case")]
async fn prepare_capsule(
    app: AppHandle,
    assignment_id: AssignmentId,
    input: PreparationInput,
) -> Result<Instructions, IpcError> {
    call(app, move |s| s.prepare_capsule(&assignment_id, input)).await
}
#[tauri::command(rename_all = "snake_case")]
async fn render_capsule(app: AppHandle, capsule_id: CapsuleId) -> Result<Instructions, IpcError> {
    call(app, move |s| s.render_capsule(&capsule_id)).await
}
#[tauri::command(rename_all = "snake_case")]
async fn mark_dispatched(
    app: AppHandle,
    assignment_id: AssignmentId,
) -> Result<Assignment, IpcError> {
    call(app, move |s| s.mark_dispatched(&assignment_id)).await
}
#[tauri::command(rename_all = "snake_case")]
async fn ingest_text(
    app: AppHandle,
    assignment_id: AssignmentId,
    text: String,
    input: IngestInput,
) -> Result<ResultSummary, IpcError> {
    call(app, move |s| s.ingest_text(&assignment_id, &text, input)).await
}
#[tauri::command(rename_all = "snake_case")]
async fn ingest_file(
    app: AppHandle,
    assignment_id: AssignmentId,
    path: PathBuf,
    input: IngestInput,
) -> Result<ResultSummary, IpcError> {
    call(app, move |s| s.ingest_file(&assignment_id, &path, input)).await
}
#[tauri::command(rename_all = "snake_case")]
async fn assignment_detail(
    app: AppHandle,
    assignment_id: AssignmentId,
) -> Result<AssignmentDetail, IpcError> {
    call(app, move |s| s.assignment_detail(&assignment_id)).await
}
#[tauri::command(rename_all = "snake_case")]
async fn result_detail(
    app: AppHandle,
    assignment_id: AssignmentId,
) -> Result<ResultDetail, IpcError> {
    call(app, move |s| s.result_detail(&assignment_id)).await
}
#[tauri::command(rename_all = "snake_case")]
async fn open_external_reference(app: AppHandle, session_id: SessionId) -> Result<(), IpcError> {
    let browser = app.clone();
    call(app, move |s| {
        let url = s.external_url(&session_id)?;
        browser
            .opener()
            .open_url(url, None::<&str>)
            .map_err(|error| IpcError {
                code: "external_open",
                message: "Could not open the external reference in your browser.".into(),
                detail: Some(error.to_string()),
            })
    })
    .await
}
pub(crate) fn handler() -> impl Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        init_project,
        open_project,
        close_project,
        workspace_snapshot,
        create_resource,
        create_agent,
        create_session,
        create_task,
        set_task_status,
        create_assignment,
        create_delegation,
        prepare_capsule,
        render_capsule,
        mark_dispatched,
        ingest_text,
        ingest_file,
        assignment_detail,
        result_detail,
        open_external_reference
    ]
}
