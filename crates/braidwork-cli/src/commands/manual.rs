use crate::{
    error::CliError,
    execution_args::{CapsuleCommand, DispatchArgs, IngestArgs, PrepareArgs},
    output,
};
use braidwork_core::{
    capsule::ContextInput,
    receipt::{MonetaryCost, Usage},
};
use braidwork_project::{CapsulePreparation, IngestOptions, Project};
use std::{
    fs,
    io::{self, Read},
};

pub(super) fn capsule(
    project: &mut Project,
    command: CapsuleCommand,
    json: bool,
) -> Result<String, CliError> {
    match command {
        CapsuleCommand::Prepare(args) => prepare(project, args, json),
        CapsuleCommand::Render { id } => {
            let rendered = project.render_capsule(&id)?;
            output::emit(
                json,
                &serde_json::json!({"capsule_id":id,"rendered":rendered}),
                || Ok(rendered.clone()),
            )
        }
    }
}
fn prepare(project: &mut Project, args: PrepareArgs, json: bool) -> Result<String, CliError> {
    let mut inputs = Vec::new();
    for path in args.context_file {
        let text = fs::read_to_string(&path).map_err(|source| CliError::InputFile {
            path: path.clone(),
            source,
        })?;
        let label = path.file_name().map_or_else(
            || "Manual context".into(),
            |name| name.to_string_lossy().into_owned(),
        );
        inputs.push(ContextInput::Inline { label, text });
    }
    for (index, text) in args.context_text.into_iter().enumerate() {
        inputs.push(ContextInput::Inline {
            label: format!("Manual context {}", index + 1),
            text,
        });
    }
    let outputs = if args.output.is_empty() {
        CapsulePreparation::default().expected_outputs
    } else {
        args.output
    };
    let capsule = project.prepare_capsule(
        &args.assignment_id,
        CapsulePreparation {
            inputs,
            constraints: args.constraint,
            acceptance_criteria: args.accept,
            expected_outputs: outputs,
            max_context_tokens: args.max_context_tokens,
        },
    )?;
    output::details(&capsule, json)
}
pub(super) fn dispatch(
    project: &mut Project,
    args: &DispatchArgs,
    json: bool,
) -> Result<String, CliError> {
    let mut dispatch = project.dispatch(&args.assignment_id)?;
    if args.mark_dispatched {
        dispatch.assignment = project
            .store_mut()
            .mark_assignment_dispatched(&args.assignment_id)?;
    }
    output::emit(json, &dispatch, || {
        Ok(format!(
            "Session: {} ({})\nResource: {}\nExternal reference: {}\nAgent: {}@{}\nTask: {}\n\n{}",
            dispatch.session.id,
            dispatch.session.label,
            dispatch.resource.id,
            dispatch.session.external_ref.as_deref().unwrap_or("(none)"),
            dispatch.agent.id,
            dispatch.agent.revision,
            dispatch.task.id(),
            dispatch.rendered
        ))
    })
}
pub(super) fn ingest(
    project: &mut Project,
    args: IngestArgs,
    json: bool,
) -> Result<String, CliError> {
    let assignment = project.store().get_assignment(&args.assignment_id)?;
    project
        .store()
        .get_assignment_capsule(&args.assignment_id)?;
    if assignment.status == braidwork_core::assignment::AssignmentStatus::ResultReceived {
        return Err(braidwork_store::StoreError::Workflow { assignment: args.assignment_id, reason: "a result has already been received; create a new assignment for another attempt" }.into());
    }
    let bytes = if let Some(path) = args.file {
        fs::read(&path).map_err(|source| CliError::InputFile { path, source })?
    } else {
        let mut bytes = Vec::new();
        io::stdin().lock().read_to_end(&mut bytes)?;
        bytes
    };
    let cost = match (args.cost_micros, args.currency) {
        (Some(amount_micros), Some(currency)) => Some(MonetaryCost {
            amount_micros,
            currency,
        }),
        (None, None) => None,
        _ => return Err(CliError::IncompleteCost),
    };
    let options = IngestOptions {
        model_id: args.model,
        usage: Usage {
            input_tokens: args.input_tokens,
            output_tokens: args.output_tokens,
            cost,
        },
        kind: args.kind,
        media_type: Some(args.media_type),
    };
    output::details(&project.ingest(&args.assignment_id, &bytes, options)?, json)
}
