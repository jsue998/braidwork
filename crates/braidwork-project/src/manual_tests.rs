use super::*;
use braidwork_core::{
    agent::{AgentSpec, DelegationPolicy},
    artifact::ArtifactKind,
    assignment::AssignmentStatus,
    capsule::{ContextInput, ExpectedOutput},
    id::{AgentSpecId, ArtifactId, ModelId, ResourceId, SessionId, TaskId},
    receipt::{MonetaryCost, Usage, Verification},
    resource::{AccessMode, Resource, ResourceStatus, Scarcity},
    session::{Session, SessionStatus},
    task::{Task, TaskStatus},
};
use std::{fs, num::NonZeroU32};
fn setup(root: &std::path::Path) -> Project {
    let mut project = Project::init(root, Some("travel-planning")).unwrap();
    project
        .store_mut()
        .insert_resource(&Resource {
            id: ResourceId::new("account").unwrap(),
            name: "Primary account".into(),
            provider: "Example".into(),
            access_mode: AccessMode::Api,
            scarcity: Scarcity::Normal,
            status: ResourceStatus::Available,
        })
        .unwrap();
    project
        .store_mut()
        .insert_task(
            &Task::new(
                TaskId::new("travel").unwrap(),
                "Travel plan",
                "Research and prepare a structured travel plan",
                None,
                [],
            )
            .unwrap(),
        )
        .unwrap();
    for (name, revision) in [
        ("coordinator", 1),
        ("researcher", 1),
        ("worker", 1),
        ("researcher", 2),
    ] {
        project
            .store_mut()
            .insert_agent_spec(&AgentSpec {
                id: AgentSpecId::new(name).unwrap(),
                revision: NonZeroU32::new(revision).unwrap(),
                name: name.into(),
                role: name.into(),
                mission: format!("Prepare rigorous travel research revision {revision}"),
                instructions: vec!["Distinguish facts from inference".into()],
                expertise: vec!["research".into()],
                delegation: DelegationPolicy {
                    allowed: true,
                    max_children: Some(2),
                },
            })
            .unwrap();
        if revision == 1 {
            project
                .store_mut()
                .insert_session(&Session {
                    id: SessionId::new(name).unwrap(),
                    label: name.into(),
                    resource_id: ResourceId::new("account").unwrap(),
                    agent_spec_id: AgentSpecId::new(name).unwrap(),
                    agent_spec_revision: NonZeroU32::MIN,
                    external_ref: Some(format!("Manual {name} chat")),
                    status: SessionStatus::Ready,
                })
                .unwrap();
        }
    }
    project
}
#[test]
fn content_is_lazy_portable_reopenable_and_never_overwritten() {
    let directory = tempfile::tempdir().unwrap();
    let project = Project::init(directory.path(), Some("demo")).unwrap();
    assert!(!project.state_path().join(ARTIFACT_DIRECTORY).exists());
    let id = ArtifactId::new("arbitrary / identity .. with whitespace").unwrap();
    let bytes = b"result\0\xff";
    let reference = project.write_artifact_content(&id, bytes).unwrap();
    assert!(reference.starts_with("artifact:"));
    assert!(!reference.contains('/'));
    assert!(!reference.contains(directory.path().to_str().unwrap()));
    assert_eq!(project.read_artifact_content(&reference).unwrap(), bytes);
    assert!(matches!(
        project.write_artifact_content(&id, b"replacement"),
        Err(ArtifactContentError::AlreadyExists(_))
    ));
    drop(project);
    let project = Project::open(directory.path()).unwrap();
    assert_eq!(project.read_artifact_content(&reference).unwrap(), bytes);
    assert!(matches!(
        project.read_artifact_content("artifact:6162.bin"),
        Err(ArtifactContentError::Missing(_))
    ));
    for bad in [
        "artifact:../escape.bin",
        "artifact:/absolute.bin",
        "/tmp/content",
        "artifact:01\\escape.bin",
    ] {
        assert!(matches!(
            project.read_artifact_content(bad),
            Err(ArtifactContentError::InvalidReference(_))
        ));
    }
}
#[test]
fn full_manual_bridge_survives_reopen_and_preserves_unknowns_and_revision() {
    let directory = tempfile::tempdir().unwrap();
    let mut project = setup(directory.path());
    let assignment = project
        .create_assignment(
            &TaskId::new("travel").unwrap(),
            &SessionId::new("researcher").unwrap(),
        )
        .unwrap();
    // Simulate a future session reconfiguration after allocation, before prepare.
    let connection = rusqlite::Connection::open(project.database_path()).unwrap();
    connection
        .execute(
            "UPDATE sessions SET agent_spec_revision = 2 WHERE id = 'researcher'",
            [],
        )
        .unwrap();
    drop(connection);
    let preparation = CapsulePreparation {
        inputs: vec![ContextInput::Inline {
            label: "Travel requirements".into(),
            text: "Two travelers, five days, accessible transport".into(),
        }],
        constraints: vec!["Avoid fabricated prices".into()],
        acceptance_criteria: vec!["Explain budget assumptions".into()],
        expected_outputs: vec![ExpectedOutput {
            kind: ArtifactKind::Analysis,
            description: "Structured plan".into(),
        }],
        max_context_tokens: 2048,
    };
    let capsule = project
        .prepare_capsule(&assignment.id, preparation)
        .unwrap();
    assert_eq!(
        capsule.mission,
        "Prepare rigorous travel research revision 1"
    );
    let rendered = project.render_capsule(&capsule.id).unwrap();
    assert_rendered_contract(&rendered);
    project.dispatch(&assignment.id).unwrap();
    assert_eq!(
        project
            .store()
            .get_assignment(&assignment.id)
            .unwrap()
            .status,
        AssignmentStatus::Prepared
    );
    drop(project);
    let mut project = Project::open(directory.path()).unwrap();
    let bytes = b"# Braidwork Result\n\n## Output\nA travel plan.\n";
    let result = project
        .ingest(&assignment.id, bytes, IngestOptions::default())
        .unwrap();
    assert_eq!(result.assignment.status, AssignmentStatus::ResultReceived);
    assert_eq!(result.receipt.model_id, None);
    assert_eq!(result.receipt.usage, Usage::default());
    assert_eq!(result.receipt.access_mode, AccessMode::Manual); // Actual bridge transport, not declared API capacity.
    assert_eq!(result.receipt.verification, Verification::NotPerformed);
    assert_eq!(
        project
            .store()
            .get_task(&assignment.task_id)
            .unwrap()
            .status(),
        TaskStatus::Pending
    );
    drop(project);
    let project = Project::open(directory.path()).unwrap();
    assert_eq!(
        project
            .store()
            .get_assignment_receipt(&assignment.id)
            .unwrap(),
        result.receipt
    );
    assert_eq!(
        project
            .read_artifact_content(&result.artifact.content_ref)
            .unwrap(),
        bytes
    );
    assert_eq!(
        project.store().get_artifact(&result.artifact.id).unwrap(),
        result.artifact
    );
    assert_eq!(
        project
            .store()
            .get_assignment_capsule(&assignment.id)
            .unwrap(),
        capsule
    );
    assert_eq!(
        project
            .store()
            .get_assignment(&assignment.id)
            .unwrap()
            .agent_spec_revision,
        NonZeroU32::MIN
    );
}
#[test]
fn ingest_preserves_known_model_and_known_zero_usage_and_raw_bytes() {
    let directory = tempfile::tempdir().unwrap();
    let mut project = setup(directory.path());
    let assignment = project
        .create_assignment(
            &TaskId::new("travel").unwrap(),
            &SessionId::new("worker").unwrap(),
        )
        .unwrap();
    project
        .prepare_capsule(&assignment.id, CapsulePreparation::default())
        .unwrap();
    project
        .store_mut()
        .mark_assignment_dispatched(&assignment.id)
        .unwrap();
    let options = IngestOptions {
        model_id: Some(ModelId::new("actual-model").unwrap()),
        usage: Usage {
            input_tokens: Some(0),
            output_tokens: None,
            cost: Some(MonetaryCost {
                amount_micros: 0,
                currency: "USD".into(),
            }),
        },
        kind: ArtifactKind::Data,
        media_type: None,
    };
    let result = project.ingest(&assignment.id, b"\0\xff", options).unwrap();
    assert_eq!(result.receipt.model_id.unwrap().as_str(), "actual-model");
    assert_eq!(result.receipt.usage.input_tokens, Some(0));
    assert_eq!(result.receipt.usage.cost.unwrap().amount_micros, 0);
    assert_eq!(
        project
            .read_artifact_content(&result.artifact.content_ref)
            .unwrap(),
        b"\0\xff"
    );
    assert!(
        project
            .ingest(&assignment.id, b"second result", IngestOptions::default())
            .is_err()
    );
    assert_eq!(
        fs::read_dir(project.state_path().join(ARTIFACT_DIRECTORY))
            .unwrap()
            .count(),
        1
    );
}
#[test]
fn zero_budget_rejects_selected_context_without_persisting_a_capsule() {
    let directory = tempfile::tempdir().unwrap();
    let mut project = setup(directory.path());
    let assignment = project
        .create_assignment(
            &TaskId::new("travel").unwrap(),
            &SessionId::new("worker").unwrap(),
        )
        .unwrap();
    let preparation = CapsulePreparation {
        max_context_tokens: 0,
        inputs: vec![ContextInput::Inline {
            label: "Context".into(),
            text: "Selected".into(),
        }],
        ..CapsulePreparation::default()
    };
    assert!(matches!(
        project.prepare_capsule(&assignment.id, preparation),
        Err(ProjectError::ContextForbidden)
    ));
    assert!(
        project
            .store()
            .get_assignment_capsule(&assignment.id)
            .is_err()
    );
    project
        .prepare_capsule(
            &assignment.id,
            CapsulePreparation {
                max_context_tokens: 0,
                ..CapsulePreparation::default()
            },
        )
        .unwrap();
}
#[test]
fn hierarchical_delegation_uses_distinct_sessions_and_survives_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let mut project = setup(directory.path());
    let mut assignments = Vec::new();
    for name in ["coordinator", "researcher", "worker"] {
        assignments.push(
            project
                .create_assignment(
                    &TaskId::new("travel").unwrap(),
                    &SessionId::new(name).unwrap(),
                )
                .unwrap(),
        );
    }
    let first = project
        .add_delegation(assignments[0].id.clone(), assignments[1].id.clone())
        .unwrap();
    let second = project
        .add_delegation(assignments[1].id.clone(), assignments[2].id.clone())
        .unwrap();
    drop(project);
    let project = Project::open(directory.path()).unwrap();
    assert_eq!(project.store().get_delegation(first.id()).unwrap(), first);
    assert_eq!(project.store().get_delegation(second.id()).unwrap(), second);
    assert_eq!(project.store().list_sessions().unwrap().len(), 3);
    for assignment in assignments {
        assert_eq!(
            project.store().get_assignment(&assignment.id).unwrap(),
            assignment
        );
    }
}
#[test]
fn missing_capsule_ingest_fails_before_creating_artifact_directory() {
    let directory = tempfile::tempdir().unwrap();
    let mut project = setup(directory.path());
    let assignment = project
        .create_assignment(
            &TaskId::new("travel").unwrap(),
            &SessionId::new("worker").unwrap(),
        )
        .unwrap();
    assert!(
        project
            .ingest(&assignment.id, b"result", IngestOptions::default())
            .is_err()
    );
    assert!(!project.state_path().join(ARTIFACT_DIRECTORY).exists());
}

fn assert_rendered_contract(rendered: &str) {
    for heading in [
        "Role",
        "Mission",
        "Objective",
        "Context",
        "Instructions",
        "Constraints",
        "Acceptance Criteria",
        "Expected Output",
        "Response Contract",
    ] {
        assert!(rendered.contains(&format!("## {heading}")));
    }
    assert!(rendered.contains("Two travelers"));
    assert!(rendered.contains("# Braidwork Result"));
}

#[test]
fn ingest_database_failure_rolls_back_rows_and_removes_only_new_content() {
    let directory = tempfile::tempdir().unwrap();
    let mut project = setup(directory.path());
    let assignment = project
        .create_assignment(
            &TaskId::new("travel").unwrap(),
            &SessionId::new("worker").unwrap(),
        )
        .unwrap();
    project
        .prepare_capsule(&assignment.id, CapsulePreparation::default())
        .unwrap();
    let sentinel = project
        .write_artifact_content(&ArtifactId::new("existing-content").unwrap(), b"preserve")
        .unwrap();
    let connection = rusqlite::Connection::open(project.database_path()).unwrap();
    connection.execute_batch("CREATE TRIGGER reject_receipt BEFORE INSERT ON receipts BEGIN SELECT RAISE(ABORT, 'test insertion failure'); END;").unwrap();
    assert!(matches!(
        project.ingest(&assignment.id, b"new bytes", IngestOptions::default()),
        Err(ProjectError::Store(
            braidwork_store::StoreError::Integrity { .. }
        ))
    ));
    assert_eq!(
        project
            .store()
            .get_assignment(&assignment.id)
            .unwrap()
            .status,
        AssignmentStatus::Prepared
    );
    assert!(
        project
            .store()
            .get_assignment_receipt(&assignment.id)
            .is_err()
    );
    assert_eq!(
        connection
            .query_row::<u32, _, _>("SELECT count(*) FROM artifacts", [], |row| row.get(0))
            .unwrap(),
        0
    );
    assert_eq!(
        connection
            .query_row::<u32, _, _>("SELECT count(*) FROM receipts", [], |row| row.get(0))
            .unwrap(),
        0
    );
    assert_eq!(
        fs::read_dir(project.state_path().join(ARTIFACT_DIRECTORY))
            .unwrap()
            .count(),
        1
    );
    assert_eq!(
        project.read_artifact_content(&sentinel).unwrap(),
        b"preserve"
    );
    connection
        .execute("DROP TRIGGER reject_receipt", [])
        .unwrap();
    project
        .ingest(
            &assignment.id,
            b"retry after failed commit",
            IngestOptions::default(),
        )
        .unwrap();
}
#[test]
fn heterogeneous_resources_preserve_session_and_receipt_provenance() {
    let directory = tempfile::tempdir().unwrap();
    let mut project = setup(directory.path());
    let resource = Resource {
        id: ResourceId::new("other-capacity").unwrap(),
        name: "Another provider".into(),
        provider: "Different".into(),
        access_mode: AccessMode::Local,
        scarcity: Scarcity::Abundant,
        status: ResourceStatus::Available,
    };
    project.store_mut().insert_resource(&resource).unwrap();
    let session = Session {
        id: SessionId::new("other-chat").unwrap(),
        label: "Other worker".into(),
        resource_id: resource.id.clone(),
        agent_spec_id: AgentSpecId::new("worker").unwrap(),
        agent_spec_revision: NonZeroU32::MIN,
        external_ref: None,
        status: SessionStatus::Ready,
    };
    project.store_mut().insert_session(&session).unwrap();
    for (session_id, expected_resource) in [
        (
            SessionId::new("researcher").unwrap(),
            ResourceId::new("account").unwrap(),
        ),
        (session.id, resource.id),
    ] {
        let assignment = project
            .create_assignment(&TaskId::new("travel").unwrap(), &session_id)
            .unwrap();
        project
            .prepare_capsule(&assignment.id, CapsulePreparation::default())
            .unwrap();
        let result = project
            .ingest(&assignment.id, b"Manual result", IngestOptions::default())
            .unwrap();
        assert_eq!(result.receipt.resource_id, expected_resource);
        assert_eq!(result.receipt.model_id, None);
    }
}
