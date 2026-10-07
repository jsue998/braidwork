use crate::{
    args::{Cli, Command, ResourceCommand, TaskCommand},
    error::CliError,
    output,
};
use braidwork_core::{resource::Resource, task::Task};
use braidwork_project::Project;
use braidwork_store::StoreError;
mod execution;
mod manual;
pub fn run(cli: Cli) -> Result<String, CliError> {
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
        command => execution::run(&mut project, command, cli.json),
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
            project.store_mut().insert_task(&task).map_err(|error| {
                if matches!(error, StoreError::Integrity { .. }) {
                    CliError::TaskReferences(error)
                } else {
                    error.into()
                }
            })?;
            output::task(&task, json)
        }
        TaskCommand::Status { id, status } => {
            output::task(&project.set_task_status(&id, status.into())?, json)
        }
        TaskCommand::List => output::tasks(&project.store().list_tasks()?, json),
        TaskCommand::Show { id } => output::task(&project.store().get_task(&id)?, json),
    }
}
