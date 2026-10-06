use braidwork_core::{
    resource::Resource,
    task::{Task, TaskError},
};
use braidwork_project::{Project, ProjectError};
use braidwork_store::StoreError;
use clap::Parser;
use std::{
    io::{self, Write},
    process::ExitCode,
};
use thiserror::Error;
mod args;
mod output;
use args::{Cli, Command, ResourceCommand, TaskCommand};

#[derive(Debug, Error)]
enum CliError {
    #[error(transparent)]
    Project(#[from] ProjectError),
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Task(#[from] TaskError),
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("JSON output error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("output formatting error: {0}")]
    Format(#[from] std::fmt::Error),
    #[error(
        "--project selects an existing project; use init <PATH> to choose an initialization directory"
    )]
    InitProjectConflict,
}
fn main() -> ExitCode {
    let result = run(Cli::parse()).and_then(|text| {
        writeln!(io::stdout().lock(), "{text}")?;
        Ok(())
    });
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let _ = writeln!(io::stderr().lock(), "error: {error}");
            if matches!(error, CliError::Store(StoreError::Integrity { .. })) {
                let _ = writeln!(
                    io::stderr().lock(),
                    "Check that the parent and prerequisite tasks exist."
                );
            }
            ExitCode::FAILURE
        }
    }
}
fn run(cli: Cli) -> Result<String, CliError> {
    if let Command::Init { path, name } = cli.command {
        if cli.project.is_some() {
            return Err(CliError::InitProjectConflict);
        }
        let path = match path {
            Some(path) => path,
            None => std::env::current_dir()?,
        };
        return output::init(&Project::init(path, name.as_deref())?, cli.json);
    }
    let mut project = match cli.project {
        Some(root) => Project::open(root)?,
        None => Project::discover(std::env::current_dir()?)?,
    };
    match cli.command {
        Command::Status => output::status(&project, cli.json),
        Command::Resource { command } => resource_command(&mut project, command, cli.json),
        Command::Task { command } => task_command(&mut project, command, cli.json),
        Command::Init { .. } => unreachable!("init returned before project discovery"),
    }
}
fn resource_command(
    project: &mut Project,
    command: ResourceCommand,
    json: bool,
) -> Result<String, CliError> {
    match command {
        ResourceCommand::Add(args) => {
            let resource = Resource {
                id: args.id,
                name: args.name,
                provider: args.provider,
                access_mode: args.access_mode.into(),
                scarcity: args.scarcity.into(),
                status: args.status.into(),
            };
            project.store_mut().insert_resource(&resource)?;
            output::resource(&resource, json)
        }
        ResourceCommand::List => output::resources(&project.store().list_resources()?, json),
        ResourceCommand::Show { id } => output::resource(&project.store().get_resource(&id)?, json),
    }
}
fn task_command(
    project: &mut Project,
    command: TaskCommand,
    json: bool,
) -> Result<String, CliError> {
    match command {
        TaskCommand::Add(args) => {
            let task = Task::new(
                args.id,
                args.title,
                args.objective,
                args.parent,
                args.depends_on,
            )?;
            project.store_mut().insert_task(&task)?;
            output::task(&task, json)
        }
        TaskCommand::List => output::tasks(&project.store().list_tasks()?, json),
        TaskCommand::Show { id } => output::task(&project.store().get_task(&id)?, json),
    }
}
