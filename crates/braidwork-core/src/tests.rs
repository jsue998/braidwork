use std::{
    collections::{BTreeSet, HashSet},
    fmt::Debug,
    str::FromStr,
};

use serde::{Serialize, de::DeserializeOwned};

use crate::{
    artifact::{Artifact, ArtifactKind},
    capsule::{ContextBudget, ContextInput, ExpectedOutput, TaskCapsule},
    id::{
        AgentSpecId, ArtifactId, AssignmentId, CapsuleId, DelegationId, InvalidId, ModelId,
        ReceiptId, ResourceId, SessionId, TaskId,
    },
    receipt::{ExecutionOutcome, MonetaryCost, Receipt, Usage, Verification},
    resource::{AccessMode, Resource, ResourceStatus, Scarcity},
    task::{Task, TaskError, TaskStatus},
};

fn resource() -> Resource {
    Resource {
        id: ResourceId::new("chatgpt-plus-main").unwrap(),
        name: "Main subscription".into(),
        provider: "OpenAI".into(),
        access_mode: AccessMode::Manual,
        scarcity: Scarcity::Scarce,
        status: ResourceStatus::Available,
    }
}

fn task() -> Task {
    Task::new(
        TaskId::new("task-42").unwrap(),
        "Fix parser",
        "Reject empty input without panicking",
        Some(TaskId::new("task-40").unwrap()),
        [TaskId::new("task-41").unwrap()],
    )
    .unwrap()
}

fn capsule() -> TaskCapsule {
    TaskCapsule {
        id: CapsuleId::new("capsule-42-v1").unwrap(),
        task_id: task().id().clone(),
        role: "implementer".into(),
        mission: "Produce reliable work".into(),
        instructions: vec!["Explain decisions".into()],
        objective: "Reject empty input without panicking".into(),
        inputs: vec![
            ContextInput::Inline {
                label: "Requirement".into(),
                text: "Empty input must return a typed error.".into(),
            },
            ContextInput::Reference {
                label: "Parser source".into(),
                reference: "src/parser.rs".into(),
            },
        ],
        constraints: vec!["Keep the public API unchanged".into()],
        acceptance_criteria: vec!["Existing and new parser tests pass".into()],
        expected_outputs: vec![ExpectedOutput {
            kind: ArtifactKind::Patch,
            description: "Parser fix and regression test".into(),
        }],
        context_budget: ContextBudget {
            max_estimated_tokens: 4_096,
        },
    }
}

fn artifact() -> Artifact {
    Artifact {
        id: ArtifactId::new("artifact-42-patch").unwrap(),
        task_id: task().id().clone(),
        kind: ArtifactKind::Patch,
        content_ref: "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
            .into(),
        media_type: Some("text/x-diff".into()),
        size_bytes: Some(512),
    }
}

fn receipt() -> Receipt {
    let resource = resource();
    let capsule = capsule();
    let artifact = artifact();
    Receipt {
        id: ReceiptId::new("receipt-42-attempt-1").unwrap(),
        task_id: capsule.task_id,
        capsule_id: capsule.id,
        resource_id: resource.id,
        model_id: None,
        access_mode: resource.access_mode,
        artifacts: vec![artifact.id],
        execution: ExecutionOutcome::Completed,
        verification: Verification::Accepted {
            summary: "Manually reviewed the patch and ran parser tests".into(),
            evidence: vec![ArtifactId::new("artifact-42-tests").unwrap()],
        },
        usage: Usage::default(),
    }
}

fn assert_json_round_trip<T>(value: &T)
where
    T: Serialize + DeserializeOwned + PartialEq + Debug,
{
    let json = serde_json::to_string(value).unwrap();
    let decoded: T = serde_json::from_str(&json).unwrap();
    assert_eq!(&decoded, value);
}

#[test]
fn ids_preserve_strings_and_support_display_and_parsing() {
    let resource = ResourceId::new("resource main").unwrap();
    assert_eq!(resource.as_str(), "resource main");
    assert_eq!(resource.to_string(), "resource main");
    assert_eq!("resource main".parse::<ResourceId>().unwrap(), resource);
    assert_eq!(
        ResourceId::try_from(String::from("resource main")).unwrap(),
        resource
    );
    assert_eq!(ResourceId::try_from("resource main").unwrap(), resource);

    assert_eq!(TaskId::new("task-42").unwrap().to_string(), "task-42");
    assert_eq!(CapsuleId::new("capsule-1").unwrap().as_str(), "capsule-1");
    assert_eq!(
        ArtifactId::new("artifact-1").unwrap().as_str(),
        "artifact-1"
    );
    assert_eq!(ReceiptId::new("receipt-1").unwrap().as_str(), "receipt-1");
}

#[test]
fn ids_are_comparable_ordered_and_hashable() {
    let first = ResourceId::new("a").unwrap();
    let second = ResourceId::new("b").unwrap();
    assert_ne!(first, second);
    assert!(first < second);
    assert_eq!(
        HashSet::from([first.clone(), first.clone(), second.clone()]).len(),
        2
    );
    let ordered = BTreeSet::from([second.clone(), first.clone()]);
    assert_eq!(ordered.into_iter().collect::<Vec<_>>(), vec![first, second]);
}

#[test]
fn model_id_preserves_strings_and_supports_the_shared_id_operations() {
    let first = ModelId::new("model a").unwrap();
    let second = ModelId::new("model-b").unwrap();
    assert_eq!(first.as_str(), "model a");
    assert_eq!(first.to_string(), "model a");
    assert_eq!("model a".parse::<ModelId>().unwrap(), first);
    assert_eq!(ModelId::try_from(String::from("model a")).unwrap(), first);
    assert_eq!(ModelId::try_from("model a").unwrap(), first);
    assert_ne!(first, second);
    assert!(first < second);
    assert_eq!(
        HashSet::from([first.clone(), first.clone(), second.clone()]).len(),
        2
    );
    assert_eq!(
        BTreeSet::from([second.clone(), first.clone()])
            .into_iter()
            .collect::<Vec<_>>(),
        vec![first, second]
    );
}

fn assert_id_conversions_reject<T>(value: &str)
where
    T: for<'a> TryFrom<&'a str, Error = InvalidId>
        + TryFrom<String, Error = InvalidId>
        + FromStr<Err = InvalidId>
        + Debug,
{
    assert_eq!(T::try_from(value).unwrap_err(), InvalidId);
    assert_eq!(T::try_from(value.to_owned()).unwrap_err(), InvalidId);
    assert_eq!(value.parse::<T>().unwrap_err(), InvalidId);
}

fn assert_all_ids_reject(value: &str) {
    assert_eq!(AgentSpecId::new(value), Err(InvalidId));
    assert_id_conversions_reject::<AgentSpecId>(value);
    assert_eq!(SessionId::new(value), Err(InvalidId));
    assert_id_conversions_reject::<SessionId>(value);
    assert_eq!(AssignmentId::new(value), Err(InvalidId));
    assert_id_conversions_reject::<AssignmentId>(value);
    assert_eq!(DelegationId::new(value), Err(InvalidId));
    assert_id_conversions_reject::<DelegationId>(value);
    assert_eq!(ResourceId::new(value), Err(InvalidId));
    assert_eq!(ModelId::new(value), Err(InvalidId));
    assert_eq!(TaskId::new(value), Err(InvalidId));
    assert_eq!(CapsuleId::new(value), Err(InvalidId));
    assert_eq!(ArtifactId::new(value), Err(InvalidId));
    assert_eq!(ReceiptId::new(value), Err(InvalidId));
    assert_id_conversions_reject::<ResourceId>(value);
    assert_id_conversions_reject::<ModelId>(value);
    assert_id_conversions_reject::<TaskId>(value);
    assert_id_conversions_reject::<CapsuleId>(value);
    assert_id_conversions_reject::<ArtifactId>(value);
    assert_id_conversions_reject::<ReceiptId>(value);
}

#[test]
fn every_id_rejects_empty_strings() {
    assert_all_ids_reject("");
}

#[test]
fn every_id_rejects_whitespace_only_strings() {
    for invalid in [" ", "\t\n", "\u{2003}"] {
        assert_all_ids_reject(invalid);
    }
}

#[test]
fn every_id_rejects_leading_whitespace() {
    for invalid in [" task-42", "\ttask-42", "\ntask-42", "\u{2003}task-42"] {
        assert_all_ids_reject(invalid);
    }
}

#[test]
fn every_id_rejects_trailing_whitespace() {
    for invalid in ["task-42 ", "task-42\t", "task-42\n", "task-42\u{2003}"] {
        assert_all_ids_reject(invalid);
    }
}

#[test]
fn every_id_accepts_and_preserves_internal_whitespace() {
    for valid in ["task 42", "task\t42", "task\n42", "task\u{2003}42"] {
        assert_eq!(AgentSpecId::new(valid).unwrap().as_str(), valid);
        assert_json_round_trip(&AgentSpecId::new(valid).unwrap());
        assert_eq!(SessionId::new(valid).unwrap().as_str(), valid);
        assert_json_round_trip(&SessionId::new(valid).unwrap());
        assert_eq!(AssignmentId::new(valid).unwrap().as_str(), valid);
        assert_json_round_trip(&AssignmentId::new(valid).unwrap());
        assert_eq!(DelegationId::new(valid).unwrap().as_str(), valid);
        assert_json_round_trip(&DelegationId::new(valid).unwrap());
        assert_eq!(ResourceId::new(valid).unwrap().as_str(), valid);
        assert_eq!(ModelId::new(valid).unwrap().as_str(), valid);
        assert_eq!(TaskId::new(valid).unwrap().as_str(), valid);
        assert_eq!(CapsuleId::new(valid).unwrap().as_str(), valid);
        assert_eq!(ArtifactId::new(valid).unwrap().as_str(), valid);
        assert_eq!(ReceiptId::new(valid).unwrap().as_str(), valid);
        assert_json_round_trip(&ResourceId::new(valid).unwrap());
        assert_json_round_trip(&ModelId::new(valid).unwrap());
        assert_json_round_trip(&TaskId::new(valid).unwrap());
        assert_json_round_trip(&CapsuleId::new(valid).unwrap());
        assert_json_round_trip(&ArtifactId::new(valid).unwrap());
        assert_json_round_trip(&ReceiptId::new(valid).unwrap());
    }
}

#[test]
fn deserialization_cannot_bypass_id_validation() {
    for invalid in [
        "",
        " \t\n",
        " task-42",
        "\ttask-42",
        "task-42 ",
        "task-42\n",
        "\u{2003}task-42",
        "task-42\u{2003}",
    ] {
        let json = serde_json::to_string(invalid).unwrap();
        assert!(serde_json::from_str::<AgentSpecId>(&json).is_err());
        assert!(serde_json::from_str::<SessionId>(&json).is_err());
        assert!(serde_json::from_str::<AssignmentId>(&json).is_err());
        assert!(serde_json::from_str::<DelegationId>(&json).is_err());
        assert!(serde_json::from_str::<ResourceId>(&json).is_err());
        assert!(serde_json::from_str::<ModelId>(&json).is_err());
        assert!(serde_json::from_str::<TaskId>(&json).is_err());
        assert!(serde_json::from_str::<CapsuleId>(&json).is_err());
        assert!(serde_json::from_str::<ArtifactId>(&json).is_err());
        assert!(serde_json::from_str::<ReceiptId>(&json).is_err());
    }
}

#[test]
fn ids_serialize_as_strings_and_round_trip() {
    let id = ResourceId::new("resource-1").unwrap();
    assert_eq!(serde_json::to_string(&id).unwrap(), "\"resource-1\"");
    assert_json_round_trip(&id);
    let model_id = ModelId::new("model-1").unwrap();
    assert_eq!(serde_json::to_string(&model_id).unwrap(), "\"model-1\"");
    assert_json_round_trip(&model_id);
    assert_json_round_trip(&TaskId::new("task-1").unwrap());
    assert_json_round_trip(&CapsuleId::new("capsule-1").unwrap());
    assert_json_round_trip(&ArtifactId::new("artifact-1").unwrap());
    assert_json_round_trip(&ReceiptId::new("receipt-1").unwrap());
}

#[test]
fn task_rejects_direct_self_dependency() {
    let id = TaskId::new("task-42").unwrap();
    assert_eq!(
        Task::new(
            id.clone(),
            "Fix parser",
            "Handle empty input",
            None,
            [id.clone()]
        ),
        Err(TaskError::SelfDependency(id)),
    );
}

#[test]
fn task_rejects_self_parent() {
    let id = TaskId::new("task-42").unwrap();
    assert_eq!(
        Task::new(
            id.clone(),
            "Fix parser",
            "Handle empty input",
            Some(id.clone()),
            []
        ),
        Err(TaskError::SelfParent(id)),
    );
}

#[test]
fn task_collapses_duplicate_dependencies_in_deterministic_order() {
    let first = TaskId::new("task-1").unwrap();
    let second = TaskId::new("task-2").unwrap();
    let task = Task::new(
        TaskId::new("task-42").unwrap(),
        "Fix parser",
        "Handle empty input",
        None,
        [second.clone(), first.clone(), second.clone()],
    )
    .unwrap();
    assert_eq!(
        task.dependencies().iter().cloned().collect::<Vec<_>>(),
        vec![first, second]
    );
}

#[test]
fn task_parent_and_dependencies_have_separate_meanings() {
    let task = task();
    assert_eq!(task.id().as_str(), "task-42");
    assert_eq!(task.title(), "Fix parser");
    assert_eq!(task.objective(), "Reject empty input without panicking");
    let parent = task.parent().unwrap();
    assert_eq!(parent.as_str(), "task-40");
    assert!(!task.dependencies().contains(parent));
}

#[test]
fn task_status_records_manual_progress() {
    let mut task = task();
    assert_eq!(task.status(), TaskStatus::Pending);
    task.set_status(TaskStatus::InProgress);
    assert_eq!(task.status(), TaskStatus::InProgress);
    task.set_status(TaskStatus::Completed);
    assert_json_round_trip(&task);
    assert_eq!(task.status(), TaskStatus::Completed);
}

#[test]
fn deserialization_preserves_task_relationship_invariants() {
    let mut json = serde_json::to_value(task()).unwrap();
    json["dependencies"] = serde_json::json!(["task-42"]);
    assert!(serde_json::from_value::<Task>(json).is_err());

    let mut json = serde_json::to_value(task()).unwrap();
    json["parent"] = serde_json::json!("task-42");
    assert!(serde_json::from_value::<Task>(json).is_err());

    let mut json = serde_json::to_value(task()).unwrap();
    json["dependencies"] = serde_json::json!(["task-41", "task-41"]);
    let decoded = serde_json::from_value::<Task>(json).unwrap();
    assert_eq!(decoded.dependencies().len(), 1);

    let mut json = serde_json::to_value(task()).unwrap();
    json["id"] = serde_json::json!("");
    assert!(serde_json::from_value::<Task>(json).is_err());
}

#[test]
fn resource_describes_access_to_capacity_without_model_configuration() {
    let resource = resource();
    assert_eq!(resource.id.as_str(), "chatgpt-plus-main");
    assert_eq!(resource.name, "Main subscription");
    assert_eq!(resource.provider, "OpenAI");
    assert_eq!(resource.access_mode, AccessMode::Manual);
    assert_eq!(resource.scarcity, Scarcity::Scarce);
    assert_eq!(resource.status, ResourceStatus::Available);
}

#[test]
fn capsule_contains_selected_context_requirements_and_a_portable_limit() {
    let capsule = capsule();
    assert_eq!(capsule.task_id, *task().id());
    assert_eq!(capsule.role, "implementer");
    assert!(
        matches!(&capsule.inputs[0], ContextInput::Inline { text, .. } if text.contains("typed error"))
    );
    assert!(
        matches!(&capsule.inputs[1], ContextInput::Reference { reference, .. } if reference == "src/parser.rs")
    );
    assert_eq!(capsule.constraints, ["Keep the public API unchanged"]);
    assert_eq!(
        capsule.acceptance_criteria,
        ["Existing and new parser tests pass"]
    );
    assert_eq!(capsule.expected_outputs[0].kind, ArtifactKind::Patch);
    assert_eq!(capsule.context_budget.max_estimated_tokens, 4_096);
    let mut legacy = serde_json::to_value(&capsule).unwrap();
    legacy.as_object_mut().unwrap().remove("mission");
    legacy.as_object_mut().unwrap().remove("instructions");
    let restored: TaskCapsule = serde_json::from_value(legacy).unwrap();
    assert!(restored.mission.is_empty());
    assert!(restored.instructions.is_empty());
    assert_eq!(restored.inputs, capsule.inputs);
    assert_eq!(restored.objective, capsule.objective);
}

#[test]
fn context_budget_preserves_estimated_token_limits_including_zero() {
    for max_estimated_tokens in [0, 4_096] {
        let budget = ContextBudget {
            max_estimated_tokens,
        };
        assert_eq!(
            serde_json::to_value(budget).unwrap(),
            serde_json::json!({"max_estimated_tokens": max_estimated_tokens})
        );
        assert_json_round_trip(&budget);
    }
}

#[test]
fn artifact_references_external_content_with_known_or_unknown_metadata() {
    let mut artifact = artifact();
    assert_eq!(artifact.task_id, *task().id());
    assert_eq!(artifact.kind, ArtifactKind::Patch);
    assert!(artifact.content_ref.starts_with("sha256:"));
    assert_eq!(artifact.media_type.as_deref(), Some("text/x-diff"));
    assert_eq!(artifact.size_bytes, Some(512));
    artifact.media_type = None;
    artifact.size_bytes = None;
    assert_json_round_trip(&artifact);
    artifact.size_bytes = Some(0);
    assert_json_round_trip(&artifact);
}

#[test]
fn manual_receipt_links_execution_outputs_and_verification_evidence() {
    let receipt = receipt();
    assert_eq!(receipt.task_id, *task().id());
    assert_eq!(receipt.capsule_id, capsule().id);
    assert_eq!(receipt.resource_id, resource().id);
    assert_eq!(receipt.access_mode, AccessMode::Manual);
    assert_eq!(receipt.artifacts, vec![artifact().id]);
    assert_eq!(receipt.execution, ExecutionOutcome::Completed);
    assert!(
        matches!(receipt.verification, Verification::Accepted { evidence, .. } if evidence == vec![ArtifactId::new("artifact-42-tests").unwrap()])
    );
}

#[test]
fn manual_receipt_records_a_known_model_and_round_trips() {
    let mut receipt = receipt();
    let model_id = ModelId::new("example-model-v1").unwrap();
    receipt.model_id = Some(model_id.clone());
    assert_eq!(receipt.access_mode, AccessMode::Manual);
    assert_eq!(receipt.resource_id, resource().id);
    assert_eq!(receipt.model_id, Some(model_id));
    assert_eq!(
        serde_json::to_value(&receipt).unwrap()["model_id"],
        "example-model-v1"
    );
    assert_json_round_trip(&receipt);
}

#[test]
fn manual_receipt_preserves_an_unknown_model_and_round_trips() {
    let receipt = receipt();
    assert_eq!(receipt.access_mode, AccessMode::Manual);
    assert_eq!(receipt.model_id, None);
    let mut json = serde_json::to_value(&receipt).unwrap();
    assert!(json["model_id"].is_null());
    assert_json_round_trip(&receipt);

    // An omitted optional identity is unknown, never inferred from the resource.
    json.as_object_mut().unwrap().remove("model_id");
    let decoded: Receipt = serde_json::from_value(json).unwrap();
    assert_eq!(decoded.model_id, None);
    assert_eq!(decoded, receipt);
}

#[test]
fn receipt_deserialization_cannot_bypass_model_id_validation() {
    for invalid in ["", " \t\n", "\u{2003}", " model-1", "model-1 "] {
        let mut json = serde_json::to_value(receipt()).unwrap();
        json["model_id"] = serde_json::json!(invalid);
        assert!(serde_json::from_value::<Receipt>(json).is_err());
    }
}

#[test]
fn unknown_usage_remains_unknown_after_serialization() {
    let original = receipt();
    let json = serde_json::to_string(&original).unwrap();
    let decoded: Receipt = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.usage.input_tokens, None);
    assert_eq!(decoded.usage.output_tokens, None);
    assert_eq!(decoded.usage.cost, None);
    assert_eq!(decoded, original);
}

#[test]
fn usage_distinguishes_partial_measurements_and_known_zero_from_unknown() {
    let mut receipt = receipt();
    receipt.usage = Usage {
        input_tokens: Some(120),
        output_tokens: None,
        cost: Some(MonetaryCost {
            amount_micros: 0,
            currency: "USD".into(),
        }),
    };
    assert_ne!(receipt.usage, Usage::default());
    assert_json_round_trip(&receipt);
    receipt.usage.output_tokens = Some(0);
    receipt.usage.cost.as_mut().unwrap().amount_micros = 25_000;
    assert_json_round_trip(&receipt);
}

#[test]
fn receipts_support_every_access_mode_without_provider_response_objects() {
    for access_mode in [
        AccessMode::Manual,
        AccessMode::Api,
        AccessMode::Harness,
        AccessMode::Local,
    ] {
        let mut receipt = receipt();
        receipt.access_mode = access_mode;
        assert_json_round_trip(&receipt);
    }
}

#[test]
fn execution_completion_does_not_imply_acceptance() {
    let mut receipt = receipt();
    receipt.verification = Verification::NotPerformed;
    assert_json_round_trip(&receipt);
    receipt.verification = Verification::Rejected {
        summary: "Regression test failed".into(),
        evidence: vec![ArtifactId::new("artifact-42-tests").unwrap()],
    };
    assert_eq!(receipt.execution, ExecutionOutcome::Completed);
    assert_json_round_trip(&receipt);
}

#[test]
fn failed_execution_can_record_partial_artifacts_or_no_outputs() {
    let mut receipt = receipt();
    receipt.execution = ExecutionOutcome::Failed {
        reason: "The manual session ended early".into(),
    };
    receipt.verification = Verification::NotPerformed;
    assert!(!receipt.artifacts.is_empty());
    assert_json_round_trip(&receipt);
    receipt.artifacts.clear();
    assert_json_round_trip(&receipt);
}

#[test]
fn representative_domain_entities_round_trip_through_json() {
    assert_json_round_trip(&resource());
    assert_json_round_trip(&task());
    assert_json_round_trip(&capsule());
    assert_json_round_trip(&artifact());
    assert_json_round_trip(&receipt());
}
