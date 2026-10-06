use super::manual;
use crate::{
    args::Command,
    error::CliError,
    execution_args::{AgentCommand, AssignmentCommand, DelegationCommand, SessionCommand},
    output,
};
use braidwork_core::{
    agent::{AgentSpec, DelegationPolicy},
    session::{Session, SessionStatus},
};
use braidwork_project::Project;
use serde::Serialize;
use std::fmt::Write;

#[derive(Serialize)]
struct Detail {
    assignment: braidwork_core::assignment::Assignment,
    capsule: Option<braidwork_core::capsule::TaskCapsule>,
    receipt: Option<braidwork_core::receipt::Receipt>,
}

pub(super) fn run(project: &mut Project, command: Command, json: bool) -> Result<String, CliError> {
    match command {
        Command::Agent { command } => agent(project, command, json),
        Command::Session { command } => session(project, command, json),
        Command::Assignment { command } => assignment(project, command, json),
        Command::Delegation { command } => delegation(project, command, json),
        Command::Capsule { command } => manual::capsule(project, command, json),
        Command::Dispatch(args) => manual::dispatch(project, &args, json),
        Command::Ingest(args) => manual::ingest(project, args, json),
        _ => unreachable!("existing commands are handled by the parent dispatcher"),
    }
}
fn agent(project: &mut Project, command: AgentCommand, json: bool) -> Result<String, CliError> {
    match command {
        AgentCommand::Add(args) => {
            let agent = AgentSpec {
                id: args.id,
                revision: args.revision,
                name: args.name,
                role: args.role,
                mission: args.mission,
                instructions: args.instruction,
                expertise: args.expertise,
                delegation: DelegationPolicy {
                    allowed: args.allow_delegation,
                    max_children: args.max_children,
                },
            };
            project.store_mut().insert_agent_spec(&agent)?;
            output::details(&agent, json)
        }
        AgentCommand::Show { id, revision } => {
            output::details(&project.store().get_agent_spec(&id, revision)?, json)
        }
        AgentCommand::List => list(&project.store().list_agent_specs()?, json),
    }
}
fn session(project: &mut Project, command: SessionCommand, json: bool) -> Result<String, CliError> {
    match command {
        SessionCommand::Add(args) => {
            let session = Session {
                id: args.id,
                label: args.label,
                resource_id: args.resource,
                agent_spec_id: args.agent,
                agent_spec_revision: args.agent_revision,
                external_ref: args.external_ref,
                status: SessionStatus::Ready,
            };
            project.store_mut().insert_session(&session)?;
            output::details(&session, json)
        }
        SessionCommand::Show { id } => output::details(&project.store().get_session(&id)?, json),
        SessionCommand::List => list(&project.store().list_sessions()?, json),
    }
}
fn assignment(
    project: &mut Project,
    command: AssignmentCommand,
    json: bool,
) -> Result<String, CliError> {
    match command {
        AssignmentCommand::Create { task_id, session } => {
            output::details(&project.create_assignment(&task_id, &session)?, json)
        }
        AssignmentCommand::List => output::assignments(&project.store().list_assignments()?, json),
        AssignmentCommand::Show { id } => {
            let assignment = project.store().get_assignment(&id)?;
            let capsule = match project.store().get_assignment_capsule(&id) {
                Ok(value) => Some(value),
                Err(braidwork_store::StoreError::Workflow { .. }) => None,
                Err(error) => return Err(error.into()),
            };
            let receipt = match project.store().get_assignment_receipt(&id) {
                Ok(value) => Some(value),
                Err(braidwork_store::StoreError::Workflow { .. }) => None,
                Err(error) => return Err(error.into()),
            };
            let detail = Detail {
                assignment,
                capsule,
                receipt,
            };
            output::emit(json, &detail, || assignment_detail(&detail))
        }
    }
}
fn assignment_detail(detail: &Detail) -> Result<String, CliError> {
    let assignment = &detail.assignment;
    let mut text = format!(
        "ID: {}\nTask: {}\nSession: {}\nAgent: {}@{}\nStatus: {}\nCapsule: {}",
        assignment.id,
        assignment.task_id,
        assignment.session_id,
        assignment.agent_spec_id,
        assignment.agent_spec_revision,
        output::label(assignment.status)?,
        detail
            .capsule
            .as_ref()
            .map_or("(none)", |capsule| capsule.id.as_str())
    );
    if let Some(receipt) = &detail.receipt {
        write!(text, "\n\nReceipt:\n{}", output::details(receipt, false)?)?;
    } else {
        text.push_str("\nReceipt: (none)");
    }
    Ok(text)
}
fn delegation(
    project: &mut Project,
    command: DelegationCommand,
    json: bool,
) -> Result<String, CliError> {
    match command {
        DelegationCommand::Add { from, to } => {
            output::details(&project.add_delegation(from, to)?, json)
        }
        DelegationCommand::Show { id } => {
            output::details(&project.store().get_delegation(&id)?, json)
        }
        DelegationCommand::List => list(&project.store().list_delegations()?, json),
    }
}
fn list<T: Serialize>(values: &[T], json: bool) -> Result<String, CliError> {
    output::emit(json, values, || {
        if values.is_empty() {
            return Ok("(none)".into());
        }
        values
            .iter()
            .map(|value| output::details(value, false))
            .collect::<Result<Vec<_>, _>>()
            .map(|values| values.join("\n\n"))
    })
}
