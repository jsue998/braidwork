use crate::CliError;
use braidwork_core::{
    resource::Resource,
    task::{Task, TaskStatus},
};
use braidwork_project::Project;
use serde::Serialize;
use std::{fmt::Write, path::PathBuf};

#[derive(Serialize)]
pub struct ProjectInfo {
    pub name: String,
    pub root: PathBuf,
    pub state_path: PathBuf,
    pub database_path: PathBuf,
    pub project_format_version: u32,
    pub store_schema_version: u32,
}
impl ProjectInfo {
    pub fn new(project: &Project) -> Result<Self, CliError> {
        Ok(Self {
            name: project.metadata().name().to_owned(),
            root: project.root().to_owned(),
            state_path: project.state_path().to_owned(),
            database_path: project.database_path().to_owned(),
            project_format_version: project.format_version(),
            store_schema_version: project.schema_version()?,
        })
    }
}
#[derive(Default, Serialize)]
pub struct TaskCounts {
    pub total: usize,
    pub pending: usize,
    pub in_progress: usize,
    pub completed: usize,
}
#[derive(Serialize)]
pub struct Status {
    pub project: ProjectInfo,
    pub resource_count: usize,
    pub tasks: TaskCounts,
}

pub fn init(project: &Project, json: bool) -> Result<String, CliError> {
    let info = ProjectInfo::new(project)?;
    emit(json, &info, || {
        Ok(format!(
            "Initialized Braidwork project {:?}\nRoot: {}\nState: {}\nSchema: {}",
            info.name,
            info.root.display(),
            info.state_path.display(),
            info.store_schema_version
        ))
    })
}

pub fn status(project: &Project, json: bool) -> Result<String, CliError> {
    let resources = project.store().list_resources()?;
    let tasks = project.store().list_tasks()?;
    let mut counts = TaskCounts {
        total: tasks.len(),
        ..TaskCounts::default()
    };
    for task in tasks {
        match task.status() {
            TaskStatus::Pending => counts.pending += 1,
            TaskStatus::InProgress => counts.in_progress += 1,
            TaskStatus::Completed => counts.completed += 1,
        }
    }
    let status = Status {
        project: ProjectInfo::new(project)?,
        resource_count: resources.len(),
        tasks: counts,
    };
    emit(json, &status, || {
        Ok(format!(
            "Project: {}\nRoot: {}\nProject format: {}\nStore schema: {}\n\nResources: {}\n\nTasks:\n  total:       {}\n  pending:     {}\n  in_progress: {}\n  completed:   {}",
            status.project.name,
            status.project.root.display(),
            status.project.project_format_version,
            status.project.store_schema_version,
            status.resource_count,
            status.tasks.total,
            status.tasks.pending,
            status.tasks.in_progress,
            status.tasks.completed
        ))
    })
}

pub fn resource(value: &Resource, json: bool) -> Result<String, CliError> {
    emit(json, value, || {
        Ok(format!(
            "ID: {}\nName: {}\nProvider: {}\nAccess: {}\nScarcity: {}\nStatus: {}",
            value.id.as_str(),
            value.name,
            value.provider,
            label(value.access_mode)?,
            label(value.scarcity)?,
            label(value.status)?
        ))
    })
}
pub fn resources(values: &[Resource], json: bool) -> Result<String, CliError> {
    emit(json, values, || {
        let mut text = format!(
            "{:<24} {:<24} {:<16} {:<10} {:<10} STATUS",
            "ID", "NAME", "PROVIDER", "ACCESS", "SCARCITY"
        );
        for value in values {
            write!(
                text,
                "\n{:<24} {:<24} {:<16} {:<10} {:<10} {}",
                value.id.as_str(),
                value.name,
                value.provider,
                label(value.access_mode)?,
                label(value.scarcity)?,
                label(value.status)?
            )?;
        }
        Ok(text)
    })
}
pub fn task(value: &Task, json: bool) -> Result<String, CliError> {
    emit(json, value, || {
        let dependencies: Vec<_> = value
            .dependencies()
            .iter()
            .map(ToString::to_string)
            .collect();
        Ok(format!(
            "ID: {}\nTitle: {}\nObjective: {}\nStatus: {}\nParent: {}\nDependencies: {}",
            value.id().as_str(),
            value.title(),
            value.objective(),
            label(value.status())?,
            value
                .parent()
                .map_or_else(|| "(none)".into(), ToString::to_string),
            if dependencies.is_empty() {
                "(none)".into()
            } else {
                dependencies.join(", ")
            }
        ))
    })
}
pub fn tasks(values: &[Task], json: bool) -> Result<String, CliError> {
    emit(json, values, || {
        let mut text = format!("{:<24} {:<12} TITLE", "ID", "STATUS");
        for value in values {
            write!(
                text,
                "\n{:<24} {:<12} {}",
                value.id().as_str(),
                label(value.status())?,
                value.title()
            )?;
        }
        Ok(text)
    })
}
fn label(value: impl Serialize) -> Result<String, CliError> {
    Ok(serde_json::from_value(serde_json::to_value(value)?)?)
}
fn emit<T: Serialize + ?Sized>(
    json: bool,
    value: &T,
    human: impl FnOnce() -> Result<String, CliError>,
) -> Result<String, CliError> {
    if json {
        Ok(serde_json::to_string_pretty(value)?)
    } else {
        human()
    }
}
