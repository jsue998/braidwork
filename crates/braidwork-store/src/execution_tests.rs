use super::*;
use crate::tests::{Fixture, artifact, capsule, task};
use braidwork_core::{
    agent::{AgentSpec, DelegationPolicy},
    assignment::{Assignment, AssignmentStatus},
    delegation::Delegation,
    id::{AgentSpecId, AssignmentId, DelegationId, SessionId},
    session::{Session, SessionStatus},
};
use std::num::NonZeroU32;
fn revision(n: u32) -> NonZeroU32 {
    NonZeroU32::new(n).unwrap()
}
fn agent(n: u32) -> AgentSpec {
    AgentSpec {
        id: AgentSpecId::new("researcher").unwrap(),
        revision: revision(n),
        name: "Research".into(),
        role: "Researcher".into(),
        mission: "Plan travel".into(),
        instructions: vec!["Cite sources".into()],
        expertise: vec!["research".into()],
        delegation: DelegationPolicy {
            allowed: true,
            max_children: None,
        },
    }
}
fn session(name: &str) -> Session {
    Session {
        id: SessionId::new(name).unwrap(),
        label: name.into(),
        resource_id: Fixture::new().resource.id,
        agent_spec_id: agent(1).id,
        agent_spec_revision: revision(1),
        external_ref: Some("opaque ref".into()),
        status: SessionStatus::Ready,
    }
}
fn assignment(name: &str, session: &Session) -> Assignment {
    Assignment {
        id: AssignmentId::new(name).unwrap(),
        task_id: task("travel", None, &[]).id().clone(),
        session_id: session.id.clone(),
        agent_spec_id: session.agent_spec_id.clone(),
        agent_spec_revision: session.agent_spec_revision,
        status: AssignmentStatus::Prepared,
    }
}
fn setup() -> SqliteStore {
    let mut store = SqliteStore::open_in_memory().unwrap();
    store.insert_resource(&Fixture::new().resource).unwrap();
    store.insert_task(&task("travel", None, &[])).unwrap();
    store.insert_agent_spec(&agent(1)).unwrap();
    store.insert_agent_spec(&agent(2)).unwrap();
    store
}
#[test]
fn migration_from_v1_preserves_all_existing_entities_and_legacy_capsules() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("v1.sqlite");
    let mut connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute_batch(include_str!("../migrations/001_initial.sql"))
        .unwrap();
    connection.pragma_update(None, "user_version", 1).unwrap();
    connection
        .pragma_update(None, "foreign_keys", true)
        .unwrap();
    let fixture = Fixture::new();
    connection.execute("INSERT INTO tasks VALUES ('legacy-parent','Parent','Group work',NULL,'pending'), ('legacy-dependency','Prerequisite','Prior work',NULL,'completed')", []).unwrap();
    connection
        .execute(
            "INSERT INTO tasks VALUES ('legacy','Title','Objective','legacy-parent','pending')",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO task_dependencies VALUES ('legacy','legacy-dependency')",
            [],
        )
        .unwrap();
    connection.execute("INSERT INTO resources VALUES ('legacy-resource','Name','Provider','manual','normal','available')",[]).unwrap();
    connection.execute("INSERT INTO capsules VALUES ('legacy-capsule','legacy','role','Objective','4096','[]','[]','[]','[]')",[]).unwrap();
    connection
        .execute(
            "INSERT INTO artifacts VALUES ('legacy-artifact','legacy','text','opaque',NULL,NULL)",
            [],
        )
        .unwrap();
    connection.execute("INSERT INTO receipts VALUES ('legacy-receipt','legacy','legacy-capsule','legacy-resource',NULL,'manual','{\"outcome\":\"completed\"}','{\"decision\":\"not_performed\"}','{\"input_tokens\":null,\"output_tokens\":null,\"cost\":null}')",[]).unwrap();
    connection
        .execute(
            "INSERT INTO receipt_artifacts VALUES ('legacy-receipt','legacy',0,'legacy-artifact')",
            [],
        )
        .unwrap();
    migrations::apply(&mut connection).unwrap();
    drop(connection);
    for _ in 0..2 {
        let store = SqliteStore::open(&path).unwrap();
        assert_eq!(store.schema_version().unwrap(), 2);
        let capsule = store
            .get_capsule(&braidwork_core::id::CapsuleId::new("legacy-capsule").unwrap())
            .unwrap();
        assert!(capsule.mission.is_empty());
        assert!(capsule.instructions.is_empty());
        let receipt = store
            .get_receipt(&braidwork_core::id::ReceiptId::new("legacy-receipt").unwrap())
            .unwrap();
        assert_eq!(receipt.artifacts.len(), 1);
        assert_eq!(receipt.usage, fixture.receipt.usage);
        assert_eq!(store.list_tasks().unwrap().len(), 3);
        let legacy = store
            .get_task(&braidwork_core::id::TaskId::new("legacy").unwrap())
            .unwrap();
        assert_eq!(legacy.title(), "Title");
        assert_eq!(legacy.objective(), "Objective");
        assert_eq!(legacy.parent().unwrap().as_str(), "legacy-parent");
        assert_eq!(
            legacy.dependencies().iter().next().unwrap().as_str(),
            "legacy-dependency"
        );
        assert_eq!(legacy.status(), TaskStatus::Pending);
        assert_eq!(store.list_resources().unwrap().len(), 1);
        let artifact = store.get_artifact(&receipt.artifacts[0]).unwrap();
        assert_eq!(artifact.content_ref, "opaque");
        assert_eq!(artifact.size_bytes, None);
        assert_eq!(artifact.media_type, None);
    }
}
#[test]
fn failed_v2_migration_rolls_back_altered_columns_and_preserves_v1_version() {
    let mut connection = rusqlite::Connection::open_in_memory().unwrap();
    connection
        .execute_batch(include_str!("../migrations/001_initial.sql"))
        .unwrap();
    connection.pragma_update(None, "user_version", 1).unwrap();
    connection
        .execute("CREATE TABLE sessions (sentinel TEXT)", [])
        .unwrap();
    assert!(migrations::apply(&mut connection).is_err());
    assert_eq!(
        connection
            .pragma_query_value::<u32, _>(None, "user_version", |row| row.get(0))
            .unwrap(),
        1
    );
    assert!(connection.prepare("SELECT mission FROM capsules").is_err());
    assert!(connection.prepare("SELECT * FROM agent_specs").is_err());
}
#[test]
fn definitions_revisions_sessions_and_assignments_round_trip_in_order() {
    let mut store = setup();
    assert_eq!(store.list_agent_specs().unwrap(), vec![agent(1), agent(2)]);
    assert!(store.list_sessions().unwrap().is_empty());
    assert!(store.list_assignments().unwrap().is_empty());
    assert!(store.list_delegations().unwrap().is_empty());
    let mut sessions = Vec::new();
    let mut assignments = Vec::new();
    for (name, status, external_ref) in [
        ("z", SessionStatus::Busy, None),
        ("a", SessionStatus::Dormant, Some("opaque".into())),
        ("m", SessionStatus::Ready, Some("manual".into())),
    ] {
        let session = Session {
            status,
            external_ref,
            ..session(name)
        };
        store.insert_session(&session).unwrap();
        let allocation = assignment(name, &session);
        store.insert_assignment(&allocation).unwrap();
        assert_eq!(store.get_session(&session.id).unwrap(), session);
        assert_eq!(store.get_assignment(&allocation.id).unwrap(), allocation);
        sessions.push(session);
        assignments.push(allocation);
    }
    sessions.sort_by(|a, b| a.id.cmp(&b.id));
    assignments.sort_by(|a, b| a.id.cmp(&b.id));
    assert_eq!(store.list_sessions().unwrap(), sessions);
    assert_eq!(store.list_assignments().unwrap(), assignments);
    assert!(matches!(
        store.insert_agent_spec(&agent(1)),
        Err(StoreError::AlreadyExists { .. })
    ));
    assert!(matches!(
        store.insert_session(&sessions[0]),
        Err(StoreError::AlreadyExists { .. })
    ));
    assert!(matches!(
        store.insert_assignment(&assignments[0]),
        Err(StoreError::AlreadyExists { .. })
    ));
}
#[test]
fn session_and_assignment_missing_references_are_contextual_and_no_rows_remain() {
    let mut store = setup();
    let valid = session("valid");
    store.insert_session(&valid).unwrap();
    let mut missing_resource = session("missing-resource");
    missing_resource.resource_id = braidwork_core::id::ResourceId::new("absent").unwrap();
    assert!(matches!(
        store.insert_session(&missing_resource),
        Err(StoreError::MissingReference {
            relation: "session resource",
            ..
        })
    ));
    let mut missing_revision = session("missing-revision");
    missing_revision.agent_spec_revision = revision(3);
    assert!(matches!(
        store.insert_session(&missing_revision),
        Err(StoreError::MissingReference {
            relation: "session agent revision",
            ..
        })
    ));
    for missing in ["task", "session", "agent"] {
        let mut allocation = assignment("candidate", &valid);
        match missing {
            "task" => allocation.task_id = braidwork_core::id::TaskId::new("absent").unwrap(),
            "session" => allocation.session_id = SessionId::new("absent").unwrap(),
            _ => allocation.agent_spec_revision = revision(3),
        }
        assert!(matches!(
            store.insert_assignment(&allocation),
            Err(StoreError::MissingReference { .. })
        ));
        assert!(store.list_assignments().unwrap().is_empty());
    }
    let mut mismatched = assignment("mismatch", &valid);
    mismatched.agent_spec_revision = revision(2);
    assert!(matches!(
        store.insert_assignment(&mismatched),
        Err(StoreError::AgentMismatch(_))
    ));
    // The SQL trigger independently enforces the same insertion-time rule.
    assert!(
        store
            .connection
            .execute(
                "INSERT INTO assignments VALUES ('raw','travel','valid','researcher',2,'prepared')",
                []
            )
            .is_err()
    );
}
#[test]
fn historical_revision_does_not_follow_later_session_configuration() {
    let mut store = setup();
    let session = session("chat");
    store.insert_session(&session).unwrap();
    let allocation = assignment("original", &session);
    store.insert_assignment(&allocation).unwrap();
    store
        .connection
        .execute(
            "UPDATE sessions SET agent_spec_revision = 2 WHERE id = 'chat'",
            [],
        )
        .unwrap();
    assert_eq!(
        store
            .get_assignment(&allocation.id)
            .unwrap()
            .agent_spec_revision,
        revision(1)
    );
    let future = assignment("future", &store.get_session(&session.id).unwrap());
    store.insert_assignment(&future).unwrap();
    assert_eq!(
        store
            .get_assignment(&future.id)
            .unwrap()
            .agent_spec_revision,
        revision(2)
    );
}
#[test]
fn delegation_edges_have_unbounded_depth_and_typed_missing_references() {
    let mut store = setup();
    let session = session("chat");
    store.insert_session(&session).unwrap();
    for name in ["coordinator", "expert", "worker"] {
        store
            .insert_assignment(&assignment(name, &session))
            .unwrap();
    }
    let edges = [
        Delegation::new(
            DelegationId::new("z").unwrap(),
            AssignmentId::new("coordinator").unwrap(),
            AssignmentId::new("expert").unwrap(),
        )
        .unwrap(),
        Delegation::new(
            DelegationId::new("a").unwrap(),
            AssignmentId::new("expert").unwrap(),
            AssignmentId::new("worker").unwrap(),
        )
        .unwrap(),
    ];
    for edge in &edges {
        store.insert_delegation(edge).unwrap();
        assert_eq!(store.get_delegation(edge.id()).unwrap(), *edge);
    }
    assert_eq!(
        store.list_delegations().unwrap(),
        vec![edges[1].clone(), edges[0].clone()]
    );
    assert!(matches!(
        store.insert_delegation(&edges[0]),
        Err(StoreError::AlreadyExists { .. })
    ));
    for (parent, child) in [("missing", "worker"), ("coordinator", "missing")] {
        let edge = Delegation::new(
            DelegationId::new("missing-edge").unwrap(),
            AssignmentId::new(parent).unwrap(),
            AssignmentId::new(child).unwrap(),
        )
        .unwrap();
        assert!(matches!(
            store.insert_delegation(&edge),
            Err(StoreError::MissingReference { .. })
        ));
    }
    assert!(
        store
            .connection
            .execute(
                "INSERT INTO delegations VALUES ('self','worker','worker')",
                []
            )
            .is_err()
    );
}
#[test]
fn delegation_policy_and_direct_child_limit_are_enforced() {
    let mut store = setup();
    let mut limited = agent(3);
    limited.delegation = DelegationPolicy {
        allowed: true,
        max_children: Some(1),
    };
    store.insert_agent_spec(&limited).unwrap();
    let session = Session {
        agent_spec_revision: revision(3),
        ..session("limited")
    };
    store.insert_session(&session).unwrap();
    for name in ["parent", "first", "second"] {
        store
            .insert_assignment(&assignment(name, &session))
            .unwrap();
    }
    let edge = |id, child| {
        Delegation::new(
            DelegationId::new(id).unwrap(),
            AssignmentId::new("parent").unwrap(),
            AssignmentId::new(child).unwrap(),
        )
        .unwrap()
    };
    store.insert_delegation(&edge("one", "first")).unwrap();
    assert!(matches!(
        store.insert_delegation(&edge("two", "second")),
        Err(StoreError::DelegationDenied(_))
    ));
    let mut denied = agent(4);
    denied.delegation.allowed = false;
    store.insert_agent_spec(&denied).unwrap();
    let session = Session {
        id: SessionId::new("denied").unwrap(),
        agent_spec_revision: revision(4),
        ..session
    };
    store.insert_session(&session).unwrap();
    store
        .insert_assignment(&assignment("denied-parent", &session))
        .unwrap();
    let edge = Delegation::new(
        DelegationId::new("denied-edge").unwrap(),
        AssignmentId::new("denied-parent").unwrap(),
        AssignmentId::new("first").unwrap(),
    )
    .unwrap();
    assert!(matches!(
        store.insert_delegation(&edge),
        Err(StoreError::DelegationDenied(_))
    ));
}
#[test]
fn manual_database_operations_are_atomic_and_provenance_checked() {
    let mut store = setup();
    let session = session("chat");
    store.insert_session(&session).unwrap();
    let allocation = assignment("work", &session);
    store.insert_assignment(&allocation).unwrap();
    assert!(store.mark_assignment_dispatched(&allocation.id).is_err());
    let prepared = capsule("capsule", &allocation.task_id);
    store
        .prepare_assignment_capsule(&allocation.id, &prepared)
        .unwrap();
    assert_eq!(
        store.get_assignment_capsule(&allocation.id).unwrap(),
        prepared
    );
    let other = capsule("orphan-capsule", &allocation.task_id);
    assert!(
        store
            .prepare_assignment_capsule(&allocation.id, &other)
            .is_err()
    );
    assert!(store.get_capsule(&other.id).is_err());
    let dispatched = store.mark_assignment_dispatched(&allocation.id).unwrap();
    assert_eq!(dispatched.status, AssignmentStatus::Dispatched);
    let produced = artifact("produced", &allocation.task_id);
    let mut receipt = Fixture::new().receipt;
    receipt.task_id = allocation.task_id.clone();
    receipt.capsule_id = prepared.id;
    receipt.artifacts = vec![produced.id.clone()];
    let wrong_resource = receipt.resource_id.clone();
    receipt.resource_id = braidwork_core::id::ResourceId::new("absent").unwrap();
    assert!(matches!(
        store.record_assignment_result(&allocation.id, &produced, &receipt),
        Err(StoreError::Provenance(_))
    ));
    assert!(store.get_artifact(&produced.id).is_err());
    receipt.resource_id = wrong_resource;
    // Fail after artifact insertion by colliding with an unrelated receipt ID.
    let mut blocker = receipt.clone();
    blocker.artifacts.clear();
    store.insert_receipt(&blocker).unwrap();
    assert!(matches!(
        store.record_assignment_result(&allocation.id, &produced, &receipt),
        Err(StoreError::AlreadyExists { .. })
    ));
    assert!(store.get_artifact(&produced.id).is_err());
    assert_eq!(
        store.get_assignment(&allocation.id).unwrap().status,
        AssignmentStatus::Dispatched
    );
    receipt.id = braidwork_core::id::ReceiptId::new("valid-result").unwrap();
    store
        .record_assignment_result(&allocation.id, &produced, &receipt)
        .unwrap();
    assert_eq!(
        store.get_assignment_receipt(&allocation.id).unwrap(),
        receipt
    );
    assert_eq!(
        store.get_assignment(&allocation.id).unwrap().status,
        AssignmentStatus::ResultReceived
    );
    assert!(
        store
            .record_assignment_result(&allocation.id, &produced, &receipt)
            .is_err()
    );
    assert!(store.mark_assignment_dispatched(&allocation.id).is_err());
}
#[test]
fn corrupted_json_revision_and_delegation_cannot_bypass_reconstruction() {
    let mut store = setup();
    store
        .connection
        .execute(
            "UPDATE agent_specs SET instructions = '{bad' WHERE revision = 1",
            [],
        )
        .unwrap();
    assert!(matches!(
        store.list_agent_specs(),
        Err(StoreError::Reconstruction(_))
    ));
    store
        .connection
        .execute(
            "UPDATE agent_specs SET instructions = '[]' WHERE revision = 1",
            [],
        )
        .unwrap();
    let session = session("chat");
    store.insert_session(&session).unwrap();
    store
        .insert_assignment(&assignment("work", &session))
        .unwrap();
    store
        .connection
        .pragma_update(None, "ignore_check_constraints", true)
        .unwrap();
    store
        .connection
        .execute(
            "INSERT INTO delegations VALUES ('invalid','work','work')",
            [],
        )
        .unwrap();
    store.connection.execute("INSERT INTO agent_specs VALUES ('corrupt',0,'Name','Role','Mission','[]','[]','{\"allowed\":false,\"max_children\":null}')", []).unwrap();
    store
        .connection
        .pragma_update(None, "ignore_check_constraints", false)
        .unwrap();
    assert!(matches!(
        store.list_delegations(),
        Err(StoreError::Reconstruction(ReconstructionError::Delegation(
            _
        )))
    ));
    assert!(matches!(
        store.list_agent_specs(),
        Err(StoreError::Reconstruction(ReconstructionError::Revision(0)))
    ));
}
