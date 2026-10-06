use crate::{
    agent::{AgentSpec, DelegationPolicy},
    assignment::{Assignment, AssignmentStatus},
    delegation::Delegation,
    id::{AgentSpecId, AssignmentId, DelegationId, ResourceId, SessionId, TaskId},
    session::{Session, SessionStatus},
};
use serde::{Serialize, de::DeserializeOwned};
use std::{collections::HashSet, fmt::Debug, num::NonZeroU32};
fn round_trip<T: Serialize + DeserializeOwned + PartialEq + Debug>(value: &T) {
    assert_eq!(
        &serde_json::from_slice::<T>(&serde_json::to_vec(value).unwrap()).unwrap(),
        value
    );
}
fn agent(revision: u32) -> AgentSpec {
    AgentSpec {
        id: AgentSpecId::new("researcher").unwrap(),
        revision: NonZeroU32::new(revision).unwrap(),
        name: "Researcher".into(),
        role: "Research".into(),
        mission: "Research a travel plan".into(),
        instructions: vec!["Cite evidence".into()],
        expertise: vec!["research".into()],
        delegation: DelegationPolicy {
            allowed: true,
            max_children: Some(2),
        },
    }
}
#[test]
fn new_ids_support_display_parsing_ordering_hashing_and_round_trip() {
    macro_rules! check {
        ($type:ty) => {{
            let first = <$type>::new("a identity").unwrap();
            let second = <$type>::new("b").unwrap();
            assert_eq!(first.to_string(), "a identity");
            assert_eq!("a identity".parse::<$type>().unwrap(), first);
            assert!(first < second);
            assert_eq!(
                HashSet::from([first.clone(), first.clone(), second]).len(),
                2
            );
            round_trip(&first);
        }};
    }
    check!(AgentSpecId);
    check!(SessionId);
    check!(AssignmentId);
    check!(DelegationId);
}
#[test]
fn agent_revisions_and_policies_are_explicit_and_serializable() {
    assert_ne!(agent(1), agent(2));
    round_trip(&agent(1));
    round_trip(&agent(u32::MAX));
    for policy in [
        DelegationPolicy::default(),
        DelegationPolicy {
            allowed: true,
            max_children: None,
        },
        DelegationPolicy {
            allowed: true,
            max_children: Some(0),
        },
    ] {
        round_trip(&policy);
    }
    let mut invalid = serde_json::to_value(agent(1)).unwrap();
    invalid["revision"] = serde_json::json!(0);
    assert!(serde_json::from_value::<AgentSpec>(invalid).is_err());
}
#[test]
fn sessions_preserve_opaque_refs_statuses_and_exact_agent_revision() {
    for status in [
        SessionStatus::Ready,
        SessionStatus::Busy,
        SessionStatus::Dormant,
    ] {
        for external_ref in [None, Some("opaque chat handle".into())] {
            let session = Session {
                id: SessionId::new("chat").unwrap(),
                label: "Research chat".into(),
                resource_id: ResourceId::new("account").unwrap(),
                agent_spec_id: agent(2).id,
                agent_spec_revision: agent(2).revision,
                external_ref,
                status,
            };
            round_trip(&session);
            let mut invalid = serde_json::to_value(&session).unwrap();
            invalid["agent_spec_revision"] = serde_json::json!(0);
            assert!(serde_json::from_value::<Session>(invalid).is_err());
        }
    }
}
#[test]
fn assignments_record_all_handoff_states_and_historical_revision() {
    for status in [
        AssignmentStatus::Prepared,
        AssignmentStatus::Dispatched,
        AssignmentStatus::ResultReceived,
    ] {
        let allocation = Assignment {
            id: AssignmentId::new("allocation").unwrap(),
            task_id: TaskId::new("travel-plan").unwrap(),
            session_id: SessionId::new("chat").unwrap(),
            agent_spec_id: agent(1).id,
            agent_spec_revision: agent(1).revision,
            status,
        };
        round_trip(&allocation);
        assert_ne!(allocation.agent_spec_revision, agent(2).revision);
        let mut invalid = serde_json::to_value(&allocation).unwrap();
        invalid["agent_spec_revision"] = serde_json::json!(0);
        assert!(serde_json::from_value::<Assignment>(invalid).is_err());
    }
}
#[test]
fn delegation_rejects_self_reference_on_construction_and_deserialization() {
    let parent = AssignmentId::new("parent").unwrap();
    let child = AssignmentId::new("child").unwrap();
    let edge = Delegation::new(DelegationId::new("edge").unwrap(), parent.clone(), child).unwrap();
    round_trip(&edge);
    assert!(Delegation::new(edge.id().clone(), parent.clone(), parent).is_err());
    let mut invalid = serde_json::to_value(&edge).unwrap();
    invalid["child_assignment"] = invalid["parent_assignment"].clone();
    assert!(serde_json::from_value::<Delegation>(invalid).is_err());
}
