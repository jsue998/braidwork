use braidwork_core::{
    artifact::{Artifact, ArtifactKind},
    capsule::{ContextBudget, ContextInput, ExpectedOutput, TaskCapsule},
    id::{ArtifactId, CapsuleId, ModelId, ReceiptId, ResourceId, TaskId},
    receipt::{ExecutionOutcome, MonetaryCost, Receipt, Usage, Verification},
    resource::{AccessMode, Resource, ResourceStatus, Scarcity},
    task::{Task, TaskStatus},
};
use rusqlite::{Connection, params};

use crate::{EntityId, ReconstructionError, SCHEMA_VERSION, SqliteStore, StoreError, migrations};

fn task(id: &str, parent: Option<&str>, dependencies: &[&str]) -> Task {
    Task::new(
        TaskId::new(id).unwrap(),
        "Fix parser",
        "Reject empty input without panicking",
        parent.map(|id| TaskId::new(id).unwrap()),
        dependencies.iter().map(|id| TaskId::new(*id).unwrap()),
    )
    .unwrap()
}

fn capsule(id: &str, task_id: &TaskId) -> TaskCapsule {
    TaskCapsule {
        id: CapsuleId::new(id).unwrap(),
        task_id: task_id.clone(),
        role: "implementer".into(),
        mission: "Produce reliable work".into(),
        instructions: vec!["Explain decisions".into()],
        objective: "Handle empty input".into(),
        inputs: vec![
            ContextInput::Inline {
                label: "Requirement".into(),
                text: "Return a typed error: entrada vacía.".into(),
            },
            ContextInput::Reference {
                label: "Parser source".into(),
                reference: "src/parser.rs".into(),
            },
        ],
        constraints: vec!["No unsafe Rust".into(), "Keep the public API".into()],
        acceptance_criteria: vec!["Parser tests pass".into()],
        expected_outputs: vec![ExpectedOutput {
            kind: ArtifactKind::Patch,
            description: "Fix and regression test".into(),
        }],
        context_budget: ContextBudget {
            max_estimated_tokens: 4096,
        },
    }
}

fn artifact(id: &str, task_id: &TaskId) -> Artifact {
    Artifact {
        id: ArtifactId::new(id).unwrap(),
        task_id: task_id.clone(),
        kind: ArtifactKind::Patch,
        content_ref: "store:parser-patch".into(),
        media_type: Some("text/x-diff".into()),
        size_bytes: Some(512),
    }
}

struct Fixture {
    resource: Resource,
    parent: Task,
    prerequisites: Vec<Task>,
    task: Task,
    capsule: TaskCapsule,
    artifacts: Vec<Artifact>,
    receipt: Receipt,
}

impl Fixture {
    fn new() -> Self {
        let resource = Resource {
            id: ResourceId::new("subscription-main").unwrap(),
            name: "Main subscription".into(),
            provider: "Example".into(),
            access_mode: AccessMode::Manual,
            scarcity: Scarcity::Scarce,
            status: ResourceStatus::Available,
        };
        let parent = task("parent", None, &[]);
        let prerequisites = vec![
            task("a-prerequisite", None, &[]),
            task("b-prerequisite", None, &[]),
        ];
        let mut task = task(
            "task 42",
            Some("parent"),
            &["b-prerequisite", "a-prerequisite", "a-prerequisite"],
        );
        task.set_status(TaskStatus::InProgress);
        let capsule = capsule("capsule-42", task.id());
        let mut tests = artifact("a-tests", task.id());
        tests.kind = ArtifactKind::TestResult;
        tests.content_ref = "store:parser-tests".into();
        tests.media_type = Some("text/plain".into());
        // Intentionally not in ID order: receipt ordering is meaningful domain data.
        let artifacts = vec![artifact("z-patch", task.id()), tests];
        let receipt = Receipt {
            id: ReceiptId::new("receipt-42").unwrap(),
            task_id: task.id().clone(),
            capsule_id: capsule.id.clone(),
            resource_id: resource.id.clone(),
            model_id: None,
            access_mode: AccessMode::Manual,
            artifacts: artifacts
                .iter()
                .map(|artifact| artifact.id.clone())
                .collect(),
            execution: ExecutionOutcome::Completed,
            verification: Verification::Accepted {
                summary: "Reviewed patch and ran parser tests".into(),
                evidence: vec![artifacts[1].id.clone()],
            },
            usage: Usage::default(),
        };
        Self {
            resource,
            parent,
            prerequisites,
            task,
            capsule,
            artifacts,
            receipt,
        }
    }

    fn insert(&self, store: &mut SqliteStore) {
        store.insert_resource(&self.resource).unwrap();
        store.insert_task(&self.parent).unwrap();
        for prerequisite in &self.prerequisites {
            store.insert_task(prerequisite).unwrap();
        }
        store.insert_task(&self.task).unwrap();
        store.insert_capsule(&self.capsule).unwrap();
        for artifact in &self.artifacts {
            store.insert_artifact(artifact).unwrap();
        }
        store.insert_receipt(&self.receipt).unwrap();
    }

    fn assert_restored(&self, store: &SqliteStore) {
        assert_eq!(
            store.get_resource(&self.resource.id).unwrap(),
            self.resource
        );
        assert_eq!(store.get_task(self.parent.id()).unwrap(), self.parent);
        for prerequisite in &self.prerequisites {
            assert_eq!(store.get_task(prerequisite.id()).unwrap(), *prerequisite);
        }
        assert_eq!(store.get_task(self.task.id()).unwrap(), self.task);
        assert_eq!(store.get_capsule(&self.capsule.id).unwrap(), self.capsule);
        for artifact in &self.artifacts {
            assert_eq!(store.get_artifact(&artifact.id).unwrap(), *artifact);
        }
        assert_eq!(store.get_receipt(&self.receipt.id).unwrap(), self.receipt);
    }
}

fn populated() -> (SqliteStore, Fixture) {
    let mut store = SqliteStore::open_in_memory().unwrap();
    let fixture = Fixture::new();
    fixture.insert(&mut store);
    (store, fixture)
}

fn assert_integrity(result: Result<(), StoreError>) {
    let error = result.expect_err("expected integrity failure");
    assert!(
        matches!(error, StoreError::Integrity { .. }),
        "expected integrity error, got {error:?}"
    );
}

fn assert_exists(result: Result<(), StoreError>, expected: &EntityId) {
    assert!(matches!(result, Err(StoreError::AlreadyExists { entity }) if &entity == expected));
}

fn assert_no_receipt_rows(store: &SqliteStore, receipt_id: &ReceiptId) {
    assert!(
        matches!(store.get_receipt(receipt_id), Err(StoreError::NotFound { entity: EntityId::Receipt(id) }) if id == *receipt_id)
    );
    let count: u32 = store
        .connection
        .query_row(
            "SELECT count(*) FROM receipt_artifacts WHERE receipt_id = ?1",
            [receipt_id.as_str()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn new_database_applies_current_schema_and_enables_foreign_keys() {
    let store = SqliteStore::open_in_memory().unwrap();
    assert_eq!(store.schema_version().unwrap(), SCHEMA_VERSION);
    let enabled: u32 = store
        .connection
        .pragma_query_value(None, "foreign_keys", |row| row.get(0))
        .unwrap();
    assert_eq!(enabled, 1);
    let mut statement = store
        .connection
        .prepare("SELECT name FROM sqlite_schema WHERE type = 'table' ORDER BY name")
        .unwrap();
    let tables: Vec<String> = statement
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(
        tables,
        [
            "agent_specs",
            "artifacts",
            "assignment_capsules",
            "assignment_receipts",
            "assignments",
            "capsules",
            "delegations",
            "receipt_artifacts",
            "receipts",
            "resources",
            "sessions",
            "task_dependencies",
            "tasks"
        ]
    );
    assert!(
        matches!(store.connection.execute("INSERT INTO task_dependencies (task_id, dependency_id) VALUES ('absent', 'also-absent')", []),
        Err(rusqlite::Error::SqliteFailure(error, _)) if error.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_FOREIGNKEY)
    );
}

#[test]
fn failed_migration_rolls_back_schema_and_version() {
    let mut connection = Connection::open_in_memory().unwrap();
    connection
        .execute("CREATE TABLE tasks (sentinel TEXT)", [])
        .unwrap();
    assert!(matches!(
        migrations::apply(&mut connection),
        Err(StoreError::Database(_))
    ));
    let version: u32 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    assert_eq!(version, 0);
    let count: u32 = connection
        .query_row(
            "SELECT count(*) FROM sqlite_schema WHERE name = 'resources'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
    connection
        .execute("INSERT INTO tasks VALUES ('preserved')", [])
        .unwrap();
}

#[test]
fn unsupported_schema_is_rejected_without_altering_data_or_version() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("future.sqlite");
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE sentinel (value TEXT); INSERT INTO sentinel VALUES ('preserved');",
        )
        .unwrap();
    connection
        .pragma_update(None, "user_version", SCHEMA_VERSION + 1)
        .unwrap();
    drop(connection);
    assert!(
        matches!(SqliteStore::open(&path), Err(StoreError::UnsupportedSchema { found, supported }) if found == SCHEMA_VERSION + 1 && supported == SCHEMA_VERSION)
    );
    let connection = Connection::open(path).unwrap();
    let value: String = connection
        .query_row("SELECT value FROM sentinel", [], |row| row.get(0))
        .unwrap();
    assert_eq!(value, "preserved");
    let version: u32 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    assert_eq!(version, SCHEMA_VERSION + 1);
}

#[test]
fn file_backed_store_preserves_all_entities_across_idempotent_reopens() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("canonical.sqlite");
    let fixture = Fixture::new();
    {
        let mut store = SqliteStore::open(&path).unwrap();
        fixture.insert(&mut store);
    }
    assert!(path.is_file());
    for _ in 0..2 {
        let store = SqliteStore::open(&path).unwrap();
        assert_eq!(store.schema_version().unwrap(), SCHEMA_VERSION);
        let enabled: u32 = store
            .connection
            .pragma_query_value(None, "foreign_keys", |row| row.get(0))
            .unwrap();
        assert_eq!(enabled, 1);
        fixture.assert_restored(&store);
    }
}

#[test]
fn resources_round_trip_all_declared_enum_variants() {
    let mut store = SqliteStore::open_in_memory().unwrap();
    let fixture = Fixture::new();
    for (index, ((access_mode, scarcity), status)) in [
        AccessMode::Manual,
        AccessMode::Api,
        AccessMode::Harness,
        AccessMode::Local,
    ]
    .into_iter()
    .zip([
        Scarcity::Abundant,
        Scarcity::Normal,
        Scarcity::Scarce,
        Scarcity::Critical,
    ])
    .zip([
        ResourceStatus::Available,
        ResourceStatus::Unavailable,
        ResourceStatus::Exhausted,
        ResourceStatus::Available,
    ])
    .enumerate()
    {
        let resource = Resource {
            id: ResourceId::new(format!("resource-{index}")).unwrap(),
            access_mode,
            scarcity,
            status,
            ..fixture.resource.clone()
        };
        store.insert_resource(&resource).unwrap();
        assert_eq!(store.get_resource(&resource.id).unwrap(), resource);
    }
}

#[test]
fn task_round_trip_preserves_parent_status_and_unique_dependencies() {
    let (store, fixture) = populated();
    let restored = store.get_task(fixture.task.id()).unwrap();
    assert_eq!(restored, fixture.task);
    assert_eq!(restored.parent(), Some(fixture.parent.id()));
    assert_eq!(restored.status(), TaskStatus::InProgress);
    assert_eq!(restored.dependencies().len(), 2);
    let leaf = store.get_task(fixture.parent.id()).unwrap();
    assert_eq!(leaf.status(), TaskStatus::Pending);
    assert_eq!(leaf.parent(), None);
    assert!(leaf.dependencies().is_empty());
}

#[test]
fn nonexistent_parent_is_rejected() {
    let mut store = SqliteStore::open_in_memory().unwrap();
    let task = task("child", Some("absent-parent"), &[]);
    assert_integrity(store.insert_task(&task));
    assert!(matches!(
        store.get_task(task.id()),
        Err(StoreError::NotFound { .. })
    ));
}

#[test]
fn missing_dependency_rolls_back_task_and_already_inserted_dependency_rows() {
    let mut store = SqliteStore::open_in_memory().unwrap();
    let known = task("a-known", None, &[]);
    store.insert_task(&known).unwrap();
    let candidate = task("candidate", None, &["a-known", "z-absent"]);
    assert_integrity(store.insert_task(&candidate));
    assert!(matches!(
        store.get_task(candidate.id()),
        Err(StoreError::NotFound { .. })
    ));
    let count: u32 = store
        .connection
        .query_row(
            "SELECT count(*) FROM task_dependencies WHERE task_id = ?1",
            [candidate.id().as_str()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
    assert_eq!(store.get_task(known.id()).unwrap(), known);
}

#[test]
fn database_constraint_rejects_duplicate_dependencies() {
    let (store, fixture) = populated();
    let error = store
        .connection
        .execute(
            "INSERT INTO task_dependencies (task_id, dependency_id) VALUES (?1, ?2)",
            params![
                fixture.task.id().as_str(),
                fixture.prerequisites[0].id().as_str()
            ],
        )
        .unwrap_err();
    assert_eq!(
        error.sqlite_error_code(),
        Some(rusqlite::ErrorCode::ConstraintViolation)
    );
    assert_eq!(store.get_task(fixture.task.id()).unwrap(), fixture.task);
}

#[test]
fn capsule_round_trip_preserves_selected_context_and_requirements() {
    let (store, fixture) = populated();
    assert_eq!(
        store.get_capsule(&fixture.capsule.id).unwrap(),
        fixture.capsule
    );
}

#[test]
fn capsule_and_artifact_require_an_existing_task() {
    let mut store = SqliteStore::open_in_memory().unwrap();
    let absent = TaskId::new("absent").unwrap();
    let capsule = capsule("candidate-capsule", &absent);
    let artifact = artifact("candidate-artifact", &absent);
    assert_integrity(store.insert_capsule(&capsule));
    assert_integrity(store.insert_artifact(&artifact));
    assert!(matches!(
        store.get_capsule(&capsule.id),
        Err(StoreError::NotFound { .. })
    ));
    assert!(matches!(
        store.get_artifact(&artifact.id),
        Err(StoreError::NotFound { .. })
    ));
}

#[test]
fn artifact_metadata_preserves_unknown_zero_and_full_unsigned_size() {
    let (mut store, fixture) = populated();
    assert_eq!(
        store.get_artifact(&fixture.artifacts[0].id).unwrap(),
        fixture.artifacts[0]
    );
    for (index, (media_type, size_bytes)) in [
        (None, None),
        (Some("text/plain".to_owned()), Some(0)),
        (Some(String::new()), Some(u64::MAX)),
    ]
    .into_iter()
    .enumerate()
    {
        let mut artifact = artifact(&format!("metadata-{index}"), fixture.task.id());
        artifact.media_type = media_type;
        artifact.size_bytes = size_bytes;
        store.insert_artifact(&artifact).unwrap();
        assert_eq!(store.get_artifact(&artifact.id).unwrap(), artifact);
    }
}

#[test]
fn token_budget_and_json_usage_preserve_the_full_unsigned_domain_range() {
    let (mut store, fixture) = populated();
    for (index, max_estimated_tokens) in [0, u64::MAX].into_iter().enumerate() {
        let mut capsule = fixture.capsule.clone();
        capsule.id = CapsuleId::new(format!("budget-{index}")).unwrap();
        capsule.context_budget.max_estimated_tokens = max_estimated_tokens;
        store.insert_capsule(&capsule).unwrap();
        assert_eq!(store.get_capsule(&capsule.id).unwrap(), capsule);
    }
    let mut receipt = fixture.receipt;
    receipt.id = ReceiptId::new("max-usage").unwrap();
    receipt.usage = Usage {
        input_tokens: Some(u64::MAX),
        output_tokens: Some(u64::MAX),
        cost: Some(MonetaryCost {
            amount_micros: u64::MAX,
            currency: "USD".into(),
        }),
    };
    store.insert_receipt(&receipt).unwrap();
    assert_eq!(store.get_receipt(&receipt.id).unwrap(), receipt);
}

#[test]
fn manual_receipt_round_trip_preserves_unknown_model_and_usage() {
    let (store, fixture) = populated();
    let restored = store.get_receipt(&fixture.receipt.id).unwrap();
    assert_eq!(restored, fixture.receipt);
    assert_eq!(restored.access_mode, AccessMode::Manual);
    assert_eq!(restored.model_id, None);
    assert_eq!(restored.usage.input_tokens, None);
    assert_eq!(restored.usage.output_tokens, None);
    assert_eq!(restored.usage.cost, None);
}

#[test]
fn manual_receipt_preserves_known_model_and_known_zero_cost() {
    let (mut store, fixture) = populated();
    let mut receipt = fixture.receipt;
    receipt.id = ReceiptId::new("known-model-zero-cost").unwrap();
    receipt.model_id = Some(ModelId::new("model a").unwrap());
    receipt.usage = Usage {
        input_tokens: Some(120),
        output_tokens: Some(0),
        cost: Some(MonetaryCost {
            amount_micros: 0,
            currency: "EUR".into(),
        }),
    };
    store.insert_receipt(&receipt).unwrap();
    assert_eq!(store.get_receipt(&receipt.id).unwrap(), receipt);
}

#[test]
fn receipt_preserves_artifact_order_and_repetitions() {
    let (mut store, fixture) = populated();
    let mut receipt = fixture.receipt;
    receipt.id = ReceiptId::new("ordered-artifacts").unwrap();
    receipt.artifacts = vec![
        fixture.artifacts[1].id.clone(),
        fixture.artifacts[0].id.clone(),
        fixture.artifacts[1].id.clone(),
    ];
    store.insert_receipt(&receipt).unwrap();
    assert_eq!(store.get_receipt(&receipt.id).unwrap(), receipt);
}

#[test]
fn receipt_outcomes_verification_and_all_access_modes_round_trip() {
    let (mut store, fixture) = populated();
    for (index, access_mode) in [
        AccessMode::Manual,
        AccessMode::Api,
        AccessMode::Harness,
        AccessMode::Local,
    ]
    .into_iter()
    .enumerate()
    {
        let mut receipt = fixture.receipt.clone();
        receipt.id = ReceiptId::new(format!("attempt-{index}")).unwrap();
        receipt.access_mode = access_mode;
        if index == 0 {
            receipt.verification = Verification::Rejected {
                summary: "Parser regression failed".into(),
                evidence: vec![fixture.artifacts[1].id.clone()],
            };
        } else {
            receipt.execution = ExecutionOutcome::Failed {
                reason: "Attempt ended early".into(),
            };
            receipt.verification = Verification::NotPerformed;
            if index == 1 {
                receipt.artifacts.clear();
            }
        }
        store.insert_receipt(&receipt).unwrap();
        assert_eq!(store.get_receipt(&receipt.id).unwrap(), receipt);
    }
}

#[test]
fn receipts_require_existing_task_capsule_and_resource() {
    let (mut store, fixture) = populated();
    for missing in ["task", "capsule", "resource"] {
        let mut receipt = fixture.receipt.clone();
        receipt.id = ReceiptId::new(format!("missing-{missing}")).unwrap();
        match missing {
            "task" => receipt.task_id = TaskId::new("absent-task").unwrap(),
            "capsule" => receipt.capsule_id = CapsuleId::new("absent-capsule").unwrap(),
            "resource" => receipt.resource_id = ResourceId::new("absent-resource").unwrap(),
            _ => unreachable!(),
        }
        assert_integrity(store.insert_receipt(&receipt));
        assert_no_receipt_rows(&store, &receipt.id);
    }
}

#[test]
fn missing_artifact_rolls_back_receipt_and_already_inserted_artifact_rows() {
    let (mut store, fixture) = populated();
    let mut receipt = fixture.receipt.clone();
    receipt.id = ReceiptId::new("missing-artifact").unwrap();
    receipt
        .artifacts
        .push(ArtifactId::new("absent-artifact").unwrap());
    assert_integrity(store.insert_receipt(&receipt));
    assert_no_receipt_rows(&store, &receipt.id);
    fixture.assert_restored(&store);
}

#[test]
fn receipt_rejects_cross_task_outputs_but_allows_separate_verifier_evidence() {
    let (mut store, fixture) = populated();
    let other_task = task("other-task", None, &[]);
    store.insert_task(&other_task).unwrap();
    let other_capsule = capsule("other-capsule", other_task.id());
    store.insert_capsule(&other_capsule).unwrap();
    let evidence = artifact("other-artifact", other_task.id());
    store.insert_artifact(&evidence).unwrap();
    let mut wrong_capsule = fixture.receipt.clone();
    wrong_capsule.id = ReceiptId::new("wrong-capsule-task").unwrap();
    wrong_capsule.capsule_id = other_capsule.id;
    assert_integrity(store.insert_receipt(&wrong_capsule));
    assert_no_receipt_rows(&store, &wrong_capsule.id);
    let mut wrong_artifact = fixture.receipt.clone();
    wrong_artifact.id = ReceiptId::new("wrong-artifact-task").unwrap();
    wrong_artifact.artifacts.push(evidence.id.clone());
    assert_integrity(store.insert_receipt(&wrong_artifact));
    assert_no_receipt_rows(&store, &wrong_artifact.id);
    let mut verified = fixture.receipt;
    verified.id = ReceiptId::new("separate-verifier-evidence").unwrap();
    verified.verification = Verification::Accepted {
        summary: "Reviewed by another task".into(),
        evidence: vec![evidence.id],
    };
    store.insert_receipt(&verified).unwrap();
    assert_eq!(store.get_receipt(&verified.id).unwrap(), verified);
}

#[test]
fn duplicate_entity_ids_produce_typed_errors_without_overwriting_originals() {
    let (mut store, fixture) = populated();
    let mut resource = fixture.resource.clone();
    resource.name = "Changed".into();
    assert_exists(
        store.insert_resource(&resource),
        &EntityId::Resource(resource.id),
    );
    let mut task = fixture.task.clone();
    task.set_status(TaskStatus::Completed);
    assert_exists(store.insert_task(&task), &EntityId::Task(task.id().clone()));
    let mut capsule = fixture.capsule.clone();
    capsule.objective = "Changed".into();
    assert_exists(
        store.insert_capsule(&capsule),
        &EntityId::Capsule(capsule.id),
    );
    let mut artifact = fixture.artifacts[0].clone();
    artifact.content_ref = "changed".into();
    assert_exists(
        store.insert_artifact(&artifact),
        &EntityId::Artifact(artifact.id),
    );
    let mut receipt = fixture.receipt.clone();
    receipt.usage.input_tokens = Some(10);
    assert_exists(
        store.insert_receipt(&receipt),
        &EntityId::Receipt(receipt.id),
    );
    fixture.assert_restored(&store);
}

#[test]
fn absent_entities_return_their_typed_not_found_identity() {
    let store = SqliteStore::open_in_memory().unwrap();
    let fixture = Fixture::new();
    assert!(
        matches!(store.get_resource(&fixture.resource.id), Err(StoreError::NotFound { entity: EntityId::Resource(id) }) if id == fixture.resource.id)
    );
    assert!(
        matches!(store.get_task(fixture.task.id()), Err(StoreError::NotFound { entity: EntityId::Task(id) }) if id == *fixture.task.id())
    );
    assert!(
        matches!(store.get_capsule(&fixture.capsule.id), Err(StoreError::NotFound { entity: EntityId::Capsule(id) }) if id == fixture.capsule.id)
    );
    assert!(
        matches!(store.get_artifact(&fixture.artifacts[0].id), Err(StoreError::NotFound { entity: EntityId::Artifact(id) }) if id == fixture.artifacts[0].id)
    );
    assert!(
        matches!(store.get_receipt(&fixture.receipt.id), Err(StoreError::NotFound { entity: EntityId::Receipt(id) }) if id == fixture.receipt.id)
    );
}

#[test]
fn malformed_nested_json_returns_reconstruction_errors_without_panicking() {
    let (store, fixture) = populated();
    for column in [
        "inputs",
        "constraints",
        "acceptance_criteria",
        "expected_outputs",
    ] {
        let original: String = store
            .connection
            .query_row(
                &format!("SELECT {column} FROM capsules WHERE id = ?1"),
                [fixture.capsule.id.as_str()],
                |row| row.get(0),
            )
            .unwrap();
        let sql = format!("UPDATE capsules SET {column} = ?1 WHERE id = ?2");
        store
            .connection
            .execute(&sql, params!["[invalid JSON", fixture.capsule.id.as_str()])
            .unwrap();
        assert!(matches!(
            store.get_capsule(&fixture.capsule.id),
            Err(StoreError::Reconstruction(ReconstructionError::Json(_)))
        ));
        store
            .connection
            .execute(&sql, params![original, fixture.capsule.id.as_str()])
            .unwrap();
    }
    for column in ["execution", "verification", "usage"] {
        let original: String = store
            .connection
            .query_row(
                &format!("SELECT {column} FROM receipts WHERE id = ?1"),
                [fixture.receipt.id.as_str()],
                |row| row.get(0),
            )
            .unwrap();
        let sql = format!("UPDATE receipts SET {column} = ?1 WHERE id = ?2");
        store
            .connection
            .execute(&sql, params!["[invalid JSON", fixture.receipt.id.as_str()])
            .unwrap();
        assert!(matches!(
            store.get_receipt(&fixture.receipt.id),
            Err(StoreError::Reconstruction(ReconstructionError::Json(_)))
        ));
        store
            .connection
            .execute(&sql, params![original, fixture.receipt.id.as_str()])
            .unwrap();
    }
    fixture.assert_restored(&store);
}

#[test]
fn incompatible_json_domain_values_are_rejected_on_read() {
    let (store, fixture) = populated();
    for usage in [r#"{"input_tokens":-1}"#, r#"{"cost":{"amount_micros":0}}"#] {
        store
            .connection
            .execute(
                "UPDATE receipts SET usage = ?1 WHERE id = ?2",
                params![usage, fixture.receipt.id.as_str()],
            )
            .unwrap();
        assert!(matches!(
            store.get_receipt(&fixture.receipt.id),
            Err(StoreError::Reconstruction(ReconstructionError::Json(_)))
        ));
    }
    store
        .connection
        .execute(
            "UPDATE receipts SET usage = ?1, verification = ?2 WHERE id = ?3",
            params![
                serde_json::to_string(&Usage::default()).unwrap(),
                r#"{"decision":"accepted","summary":"reviewed","evidence":[" invalid-id"]}"#,
                fixture.receipt.id.as_str(),
            ],
        )
        .unwrap();
    assert!(matches!(
        store.get_receipt(&fixture.receipt.id),
        Err(StoreError::Reconstruction(ReconstructionError::Json(_)))
    ));
}

#[test]
fn persisted_model_ids_cannot_bypass_domain_validation() {
    let (store, fixture) = populated();
    for invalid in ["", " ", " model-1", "model-1\n"] {
        store
            .connection
            .execute(
                "UPDATE receipts SET model_id = ?1 WHERE id = ?2",
                params![invalid, fixture.receipt.id.as_str()],
            )
            .unwrap();
        assert!(matches!(
            store.get_receipt(&fixture.receipt.id),
            Err(StoreError::Reconstruction(ReconstructionError::InvalidId(
                _
            )))
        ));
    }
}

#[test]
fn invalid_persisted_unsigned_scalars_are_rejected_instead_of_truncated() {
    let (store, fixture) = populated();
    for invalid in ["-1", "18446744073709551616", "not-a-number"] {
        store
            .connection
            .execute(
                "UPDATE capsules SET max_estimated_tokens = ?1 WHERE id = ?2",
                params![invalid, fixture.capsule.id.as_str()],
            )
            .unwrap();
        assert!(matches!(
            store.get_capsule(&fixture.capsule.id),
            Err(StoreError::Reconstruction(ReconstructionError::Unsigned(_)))
        ));
        store
            .connection
            .execute(
                "UPDATE artifacts SET size_bytes = ?1 WHERE id = ?2",
                params![invalid, fixture.artifacts[0].id.as_str()],
            )
            .unwrap();
        assert!(matches!(
            store.get_artifact(&fixture.artifacts[0].id),
            Err(StoreError::Reconstruction(ReconstructionError::Unsigned(_)))
        ));
    }
}

#[test]
fn task_reconstruction_enforces_domain_rules_even_for_corrupt_persisted_rows() {
    let (store, fixture) = populated();
    // Simulate externally corrupted data by bypassing CHECKs only. Foreign keys
    // remain enabled throughout; the store never uses this setting.
    store
        .connection
        .pragma_update(None, "ignore_check_constraints", true)
        .unwrap();
    store
        .connection
        .execute(
            "INSERT INTO task_dependencies (task_id, dependency_id) VALUES (?1, ?1)",
            [fixture.task.id().as_str()],
        )
        .unwrap();
    store
        .connection
        .pragma_update(None, "ignore_check_constraints", false)
        .unwrap();
    assert!(matches!(
        store.get_task(fixture.task.id()),
        Err(StoreError::Reconstruction(ReconstructionError::Task(_)))
    ));
    store
        .connection
        .execute(
            "DELETE FROM task_dependencies WHERE task_id = ?1 AND dependency_id = ?1",
            [fixture.task.id().as_str()],
        )
        .unwrap();
    store
        .connection
        .pragma_update(None, "ignore_check_constraints", true)
        .unwrap();
    store
        .connection
        .execute(
            "UPDATE tasks SET parent = id WHERE id = ?1",
            [fixture.task.id().as_str()],
        )
        .unwrap();
    store
        .connection
        .pragma_update(None, "ignore_check_constraints", false)
        .unwrap();
    assert!(matches!(
        store.get_task(fixture.task.id()),
        Err(StoreError::Reconstruction(ReconstructionError::Task(_)))
    ));
    let enabled: u32 = store
        .connection
        .pragma_query_value(None, "foreign_keys", |row| row.get(0))
        .unwrap();
    assert_eq!(enabled, 1);
}

#[test]
fn resource_and_task_lists_are_empty_in_a_new_store() {
    let store = SqliteStore::open_in_memory().unwrap();
    assert!(store.list_resources().unwrap().is_empty());
    assert!(store.list_tasks().unwrap().is_empty());
}

#[test]
fn resource_list_returns_domain_values_in_id_order() {
    let mut store = SqliteStore::open_in_memory().unwrap();
    let fixture = Fixture::new();
    let mut expected = Vec::new();
    for name in ["z-resource", "a-resource", "m-resource"] {
        let resource = Resource {
            id: ResourceId::new(name).unwrap(),
            ..fixture.resource.clone()
        };
        store.insert_resource(&resource).unwrap();
        expected.push(resource);
    }
    expected.sort_by(|a, b| a.id.cmp(&b.id));
    assert_eq!(store.list_resources().unwrap(), expected);
    for resource in expected {
        assert_eq!(store.get_resource(&resource.id).unwrap(), resource);
    }
}

#[test]
fn task_list_preserves_relationships_and_all_statuses_in_id_order() {
    let (mut store, fixture) = populated();
    let mut completed = task(
        "completed",
        Some("parent"),
        &["b-prerequisite", "a-prerequisite"],
    );
    completed.set_status(TaskStatus::Completed);
    store.insert_task(&completed).unwrap();
    let mut expected = vec![fixture.task, fixture.parent, completed];
    expected.extend(fixture.prerequisites);
    expected.sort_by(|a, b| a.id().cmp(b.id()));
    let listed = store.list_tasks().unwrap();
    assert_eq!(listed, expected);
    for task in listed {
        assert_eq!(store.get_task(task.id()).unwrap(), task);
    }
}

#[test]
fn lists_apply_the_same_reconstruction_checks_as_getters() {
    let (store, fixture) = populated();
    // Use a resource without receipt references to simulate externally invalid IDs.
    store.connection.execute("INSERT INTO resources VALUES (' invalid', 'name', 'provider', 'manual', 'normal', 'available')", []).unwrap();
    assert!(matches!(
        store.list_resources(),
        Err(StoreError::Reconstruction(ReconstructionError::InvalidId(
            _
        )))
    ));
    store
        .connection
        .pragma_update(None, "ignore_check_constraints", true)
        .unwrap();
    store
        .connection
        .execute(
            "UPDATE tasks SET parent = id WHERE id = ?1",
            [fixture.task.id().as_str()],
        )
        .unwrap();
    store
        .connection
        .pragma_update(None, "ignore_check_constraints", false)
        .unwrap();
    assert!(matches!(
        store.list_tasks(),
        Err(StoreError::Reconstruction(ReconstructionError::Task(_)))
    ));
}

#[path = "execution_tests.rs"]
mod execution;
