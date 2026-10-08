use super::*;
use braidwork_core::{
    agent::DelegationPolicy,
    artifact::ArtifactKind,
    capsule::ExpectedOutput,
    id::ModelId,
    receipt::Verification,
    resource::{AccessMode, ResourceStatus, Scarcity},
};
use tempfile::{TempDir, tempdir};

fn resource_input() -> ResourceInput {
    ResourceInput {
        name: "Primary AI Account".into(),
        provider: "Example".into(),
        access_mode: AccessMode::Api,
        scarcity: Scarcity::Normal,
        status: ResourceStatus::Available,
    }
}
fn agent_input(name: &str, allowed: bool) -> AgentInput {
    AgentInput {
        name: name.into(),
        role: name.into(),
        mission: "Prepare a rigorous Japan travel plan".into(),
        instructions: vec!["Distinguish facts from uncertainty".into()],
        expertise: vec!["Travel research".into()],
        delegation: DelegationPolicy {
            allowed,
            max_children: Some(1),
        },
    }
}
fn task_input() -> TaskInput {
    TaskInput {
        title: "Research transportation".into(),
        objective: "Compare rail and local travel options".into(),
        parent: None,
        dependencies: vec![],
    }
}
fn session_input(resource: &Resource, agent: &AgentSpec) -> SessionInput {
    SessionInput {
        label: format!("{} chat", agent.name),
        resource_id: resource.id.clone(),
        agent_spec_id: agent.id.clone(),
        agent_spec_revision: agent.revision,
        external_ref: Some("https://example.com/chat".into()),
    }
}
fn preparation() -> PreparationInput {
    PreparationInput {
        inputs: vec![ContextInput::Inline {
            label: "Travel needs".into(),
            text: "Two travelers, fourteen days".into(),
        }],
        context_files: vec![],
        constraints: vec!["State assumptions".into()],
        acceptance_criteria: vec!["Compare costs".into()],
        expected_outputs: vec![ExpectedOutput {
            kind: ArtifactKind::Analysis,
            description: "Structured comparison".into(),
        }],
        max_context_tokens: "4096".into(),
    }
}
struct Fixture {
    directory: TempDir,
    service: DesktopService,
    resource: Resource,
    coordinator: AgentSpec,
    researcher: AgentSpec,
    coordinator_session: Session,
    research_session: Session,
    task: Task,
    assignment: Assignment,
}
fn fixture() -> Fixture {
    let directory = tempdir().unwrap();
    let service = DesktopService::default();
    service
        .init_project(directory.path(), "Japan Trip")
        .unwrap();
    let resource = service.create_resource(resource_input()).unwrap();
    let coordinator = service
        .create_agent(agent_input("Coordinator", true))
        .unwrap();
    let researcher = service
        .create_agent(agent_input("Travel Researcher", false))
        .unwrap();
    let coordinator_session = service
        .create_session(session_input(&resource, &coordinator))
        .unwrap();
    let research_session = service
        .create_session(session_input(&resource, &researcher))
        .unwrap();
    let task = service.create_task(task_input()).unwrap();
    let assignment = service
        .create_assignment(task.id(), &research_session.id)
        .unwrap();
    Fixture {
        directory,
        service,
        resource,
        coordinator,
        researcher,
        coordinator_session,
        research_session,
        task,
        assignment,
    }
}
#[test]
fn no_project_is_structured_and_serializable() {
    let error = DesktopService::default().workspace_snapshot().unwrap_err();
    assert_eq!(error.code, "no_project");
    assert_eq!(serde_json::to_value(&error).unwrap()["code"], "no_project");
}
#[test]
fn init_empty_snapshot_open_close_and_failed_switch() {
    let directory = tempdir().unwrap();
    let other = tempdir().unwrap();
    let service = DesktopService::default();
    let info = service
        .init_project(directory.path(), "Japan Trip")
        .unwrap();
    assert_eq!(info.format_version, 1);
    assert_eq!(info.schema_version, 2);
    let snapshot = service.workspace_snapshot().unwrap();
    assert!(snapshot.resources.is_empty());
    assert!(snapshot.tasks.is_empty());
    assert_eq!(
        service.open_project(other.path()).unwrap_err().code,
        "project_not_found"
    );
    assert_eq!(
        service.workspace_snapshot().unwrap().project.name,
        "Japan Trip"
    );
    assert_eq!(
        service
            .init_project(directory.path(), "Other")
            .unwrap_err()
            .code,
        "already_initialized"
    );
    service.close_project().unwrap();
    assert_eq!(service.workspace_snapshot().unwrap_err().code, "no_project");
    service.open_project(directory.path()).unwrap();
    assert_eq!(
        service.workspace_snapshot().unwrap().project.name,
        "Japan Trip"
    );
}
#[test]
fn general_purpose_entities_and_multiple_sessions_share_resource() {
    let f = fixture();
    let snapshot = f.service.workspace_snapshot().unwrap();
    assert_eq!(snapshot.resources.len(), 1);
    assert_eq!(snapshot.agents.len(), 2);
    assert_eq!(snapshot.sessions.len(), 2);
    assert!(
        snapshot
            .sessions
            .iter()
            .all(|s| s.resource_id == f.resource.id)
    );
    assert_eq!(f.coordinator.revision, NonZeroU32::MIN);
    assert_eq!(f.researcher.revision, NonZeroU32::MIN);
    assert_eq!(f.research_session.status, SessionStatus::Ready);
    assert_eq!(f.task.status(), TaskStatus::Pending);
    assert_eq!(f.assignment.agent_spec_id, f.researcher.id);
    assert_eq!(f.assignment.status, AssignmentStatus::Prepared);
    assert!(snapshot.workflow[0].capsule_id.is_none());
    assert!(snapshot.workflow[0].result.is_none());
}
#[test]
fn manual_workflow_reopens_with_exact_content_and_honest_receipt() {
    let f = fixture();
    let rendered = f
        .service
        .prepare_capsule(&f.assignment.id, preparation())
        .unwrap();
    assert!(rendered.rendered.contains("Researcher"));
    assert!(rendered.rendered.contains("Compare costs"));
    assert_eq!(
        f.service
            .render_capsule(&rendered.capsule_id)
            .unwrap()
            .rendered,
        rendered.rendered
    );
    assert_eq!(
        f.service
            .assignment_detail(&f.assignment.id)
            .unwrap()
            .assignment
            .status,
        AssignmentStatus::Prepared
    );
    f.service.mark_dispatched(&f.assignment.id).unwrap();
    let content = "# Japan travel plan\n<script>untrusted content</script>\n鉄道";
    let result = f
        .service
        .ingest_text(&f.assignment.id, content, IngestInput::default())
        .unwrap();
    assert_eq!(result.receipt.access_mode, AccessMode::Manual);
    assert_eq!(result.receipt.verification, Verification::NotPerformed);
    assert!(result.receipt.model_id.is_none());
    assert!(result.receipt.usage.input_tokens.is_none());
    assert!(result.receipt.usage.cost.is_none());
    let snapshot = f.service.workspace_snapshot().unwrap();
    assert_eq!(snapshot.tasks[0].status(), TaskStatus::Pending);
    assert!(
        snapshot
            .sessions
            .iter()
            .all(|s| s.status == SessionStatus::Ready)
    );
    assert_eq!(
        snapshot.assignments[0].status,
        AssignmentStatus::ResultReceived
    );
    assert!(snapshot.workflow[0].result.is_some());
    f.service.close_project().unwrap();
    let reopened = DesktopService::default();
    reopened.open_project(f.directory.path()).unwrap();
    let detail = reopened.result_detail(&f.assignment.id).unwrap();
    assert_eq!(detail.artifacts[0].text.as_deref(), Some(content));
    assert_eq!(detail.artifacts[0].status, PreviewStatus::Text);
    assert_eq!(
        reopened
            .assignment_detail(&f.assignment.id)
            .unwrap()
            .assignment
            .status,
        AssignmentStatus::ResultReceived
    );
    assert_eq!(
        reopened
            .ingest_text(&f.assignment.id, "second", IngestInput::default())
            .unwrap_err()
            .code,
        "workflow"
    );
}
#[test]
fn explicit_task_status_is_independent_of_execution_and_verification() {
    let f = fixture();
    f.service
        .set_task_status(f.task.id(), TaskStatus::Completed)
        .unwrap();
    assert_eq!(
        f.service.workspace_snapshot().unwrap().tasks[0].status(),
        TaskStatus::Completed
    );
    assert_eq!(
        f.service
            .assignment_detail(&f.assignment.id)
            .unwrap()
            .assignment
            .status,
        AssignmentStatus::Prepared
    );
    f.service
        .set_task_status(f.task.id(), TaskStatus::Pending)
        .unwrap();
    assert_eq!(
        f.service
            .set_task_status(&TaskId::new("missing").unwrap(), TaskStatus::Completed)
            .unwrap_err()
            .code,
        "not_found"
    );
}
#[test]
fn delegation_uses_real_policy_and_survives_reopen() {
    let f = fixture();
    let parent = f
        .service
        .create_assignment(f.task.id(), &f.coordinator_session.id)
        .unwrap();
    let edge = f
        .service
        .create_delegation(parent.id.clone(), f.assignment.id.clone())
        .unwrap();
    assert_eq!(edge.parent_assignment(), &parent.id);
    let another = f
        .service
        .create_assignment(f.task.id(), &f.research_session.id)
        .unwrap();
    assert_eq!(
        f.service
            .create_delegation(parent.id.clone(), another.id.clone())
            .unwrap_err()
            .code,
        "delegation_denied"
    );
    assert_eq!(
        f.service
            .create_delegation(f.assignment.id.clone(), another.id)
            .unwrap_err()
            .code,
        "delegation_denied"
    );
    assert_eq!(
        f.service
            .create_delegation(parent.id.clone(), parent.id)
            .unwrap_err()
            .code,
        "self_delegation"
    );
    let service = DesktopService::default();
    service.open_project(f.directory.path()).unwrap();
    assert_eq!(service.workspace_snapshot().unwrap().delegations.len(), 1);
}
#[test]
fn context_files_are_utf8_inline_snapshots_without_absolute_paths() {
    let f = fixture();
    let path = f.directory.path().join("travel-notes.txt");
    fs::write(&path, "Travel budget €3000").unwrap();
    let mut input = preparation();
    input.context_files.push(path.clone());
    let result = f.service.prepare_capsule(&f.assignment.id, input).unwrap();
    fs::write(&path, "changed").unwrap();
    assert!(result.rendered.contains("Travel budget €3000"));
    assert!(
        !result
            .rendered
            .contains(&path.to_string_lossy().to_string())
    );
    assert!(
        f.service
            .render_capsule(&result.capsule_id)
            .unwrap()
            .rendered
            .contains("Travel budget €3000")
    );
}
#[test]
fn bad_context_and_budget_fail_without_preparing_a_capsule() {
    let f = fixture();
    let path = f.directory.path().join("binary.dat");
    fs::write(&path, [255]).unwrap();
    let mut input = preparation();
    input.context_files.push(path);
    assert!(f.service.prepare_capsule(&f.assignment.id, input).is_err());
    let mut input = preparation();
    input.max_context_tokens = "0".into();
    assert_eq!(
        f.service
            .prepare_capsule(&f.assignment.id, input)
            .unwrap_err()
            .code,
        "context_forbidden"
    );
    let mut input = preparation();
    input.max_context_tokens = "-1".into();
    assert_eq!(
        f.service
            .prepare_capsule(&f.assignment.id, input)
            .unwrap_err()
            .code,
        "invalid_input"
    );
    assert!(
        f.service
            .assignment_detail(&f.assignment.id)
            .unwrap()
            .instructions
            .is_none()
    );
}
#[test]
fn file_ingest_preserves_known_model_zero_and_full_u64_without_js_rounding() {
    let f = fixture();
    f.service
        .prepare_capsule(&f.assignment.id, preparation())
        .unwrap();
    let path = f.directory.path().join("response.md");
    fs::write(&path, "Transport comparison").unwrap();
    let result = f
        .service
        .ingest_file(
            &f.assignment.id,
            &path,
            IngestInput {
                model_id: Some(ModelId::new("known-model").unwrap()),
                input_tokens: Some(u64::MAX.to_string()),
                output_tokens: Some("0".into()),
                cost_micros: Some("0".into()),
                currency: Some("JPY".into()),
                ..IngestInput::default()
            },
        )
        .unwrap();
    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["receipt"]["model_id"], "known-model");
    assert_eq!(
        json["receipt"]["usage"]["input_tokens"],
        u64::MAX.to_string()
    );
    assert_eq!(json["receipt"]["usage"]["output_tokens"], "0");
    assert_eq!(json["receipt"]["usage"]["cost"]["amount_micros"], "0");
    assert_eq!(
        f.service.result_detail(&f.assignment.id).unwrap().artifacts[0]
            .text
            .as_deref(),
        Some("Transport comparison")
    );
}
#[test]
fn malformed_usage_is_rejected_before_artifact_write() {
    let f = fixture();
    f.service
        .prepare_capsule(&f.assignment.id, preparation())
        .unwrap();
    for input in [
        IngestInput {
            cost_micros: Some("0".into()),
            ..IngestInput::default()
        },
        IngestInput {
            input_tokens: Some("18446744073709551616".into()),
            ..IngestInput::default()
        },
        IngestInput {
            output_tokens: Some(" 0".into()),
            ..IngestInput::default()
        },
    ] {
        assert_eq!(
            f.service
                .ingest_text(&f.assignment.id, "result", input)
                .unwrap_err()
                .code,
            "invalid_input"
        );
    }
    assert!(!f.directory.path().join(".braidwork/artifacts").exists());
}
#[test]
fn text_preview_is_bounded_by_actual_content_without_truncation() {
    let f = fixture();
    f.service
        .prepare_capsule(&f.assignment.id, preparation())
        .unwrap();
    let text = "a".repeat(usize::try_from(PREVIEW_LIMIT + 1).unwrap());
    f.service
        .ingest_text(&f.assignment.id, &text, IngestInput::default())
        .unwrap();
    let detail = f.service.result_detail(&f.assignment.id).unwrap();
    assert_eq!(detail.artifacts[0].status, PreviewStatus::TooLarge);
    assert!(detail.artifacts[0].text.is_none());
    let project = Project::open(f.directory.path()).unwrap();
    let receipt = project
        .store()
        .get_assignment_receipt(&f.assignment.id)
        .unwrap();
    let artifact = project.store().get_artifact(&receipt.artifacts[0]).unwrap();
    assert_eq!(
        project
            .read_artifact_content(&artifact.content_ref)
            .unwrap(),
        text.as_bytes()
    );
}
#[test]
fn binary_and_invalid_utf8_content_are_not_rendered() {
    for (media, expected) in [
        ("application/octet-stream", PreviewStatus::Binary),
        ("text/plain", PreviewStatus::InvalidUtf8),
    ] {
        let f = fixture();
        f.service
            .prepare_capsule(&f.assignment.id, preparation())
            .unwrap();
        let path = f.directory.path().join("result.bin");
        fs::write(&path, [255, 0]).unwrap();
        f.service
            .ingest_file(
                &f.assignment.id,
                &path,
                IngestInput {
                    media_type: Some(media.into()),
                    ..IngestInput::default()
                },
            )
            .unwrap();
        let result = f.service.result_detail(&f.assignment.id).unwrap();
        assert_eq!(result.artifacts[0].status, expected);
        assert!(result.artifacts[0].text.is_none());
    }
}
#[test]
fn missing_artifact_returns_typed_error_without_repair() {
    let f = fixture();
    f.service
        .prepare_capsule(&f.assignment.id, preparation())
        .unwrap();
    f.service
        .ingest_text(&f.assignment.id, "result", IngestInput::default())
        .unwrap();
    let directory = f.directory.path().join(".braidwork/artifacts");
    let path = fs::read_dir(&directory)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    fs::remove_file(&path).unwrap();
    assert_eq!(
        f.service.result_detail(&f.assignment.id).unwrap_err().code,
        "content_missing"
    );
    assert!(!path.exists());
    assert!(
        f.service.workspace_snapshot().unwrap().workflow[0]
            .result
            .is_some()
    );
}
#[test]
fn external_reference_only_allows_http_https() {
    assert_eq!(
        http_url("https://example.com/chat").as_deref(),
        Some("https://example.com/chat")
    );
    assert!(http_url("http://example.com").is_some());
    for reference in [
        "javascript:alert(1)",
        "file:///etc/passwd",
        "shell:run",
        "mailto:test@example.com",
        "Research chat",
        "https://",
    ] {
        assert!(http_url(reference).is_none());
    }
    let f = fixture();
    assert!(
        f.service
            .external_url(&f.research_session.id)
            .unwrap()
            .starts_with("https://")
    );
}
#[test]
fn required_input_is_revalidated_without_silent_normalization() {
    let f = fixture();
    let mut input = resource_input();
    input.name = "  ".into();
    assert_eq!(
        f.service.create_resource(input).unwrap_err().code,
        "invalid_input"
    );
    let mut input = resource_input();
    input.name = " Human name ".into();
    assert_eq!(
        f.service.create_resource(input).unwrap().name,
        " Human name "
    );
    let mut input = session_input(&f.resource, &f.researcher);
    input.resource_id = ResourceId::new("missing").unwrap();
    assert_eq!(
        f.service.create_session(input).unwrap_err().code,
        "missing_reference"
    );
}
#[test]
fn snapshot_json_and_preparation_budget_remain_lossless() {
    let f = fixture();
    let mut input = preparation();
    input.max_context_tokens = u64::MAX.to_string();
    let prepared = f.service.prepare_capsule(&f.assignment.id, input).unwrap();
    let value = serde_json::to_value(prepared).unwrap();
    assert_eq!(value["max_estimated_tokens"], u64::MAX.to_string());
    let value = serde_json::to_value(f.service.workspace_snapshot().unwrap()).unwrap();
    assert!(value["workflow"][0]["result"].is_null());
    assert_eq!(value["project"]["schema_version"], 2);
    assert!(value["project"].get("database_path").is_none());
}

#[test]
fn native_configuration_enables_real_host_with_narrow_local_permissions() {
    let config: serde_json::Value =
        serde_json::from_str(include_str!("../../tauri.conf.json")).unwrap();
    assert_eq!(config["build"]["features"], serde_json::json!(["desktop"]));
    let csp = config["app"]["security"]["csp"].as_str().unwrap();
    assert!(!csp.contains("unsafe-eval"));
    assert!(!csp.contains("script-src 'unsafe-inline'"));
    assert_eq!(config["app"]["windows"][0]["decorations"], true);
    let capability: serde_json::Value =
        serde_json::from_str(include_str!("../../capabilities/main.json")).unwrap();
    assert_eq!(capability["windows"], serde_json::json!(["main"]));
    let permissions = capability["permissions"].as_array().unwrap();
    assert_eq!(permissions.len(), 21);
    assert!(permissions.iter().all(|p| {
        let name = p.as_str().unwrap();
        name.starts_with("allow-")
            || matches!(
                name,
                "dialog:allow-open" | "clipboard-manager:allow-write-text"
            )
    }));
    assert!(capability.get("remote").is_none());
    assert!(include_bytes!("../../icons/icon.png").starts_with(b"\x89PNG"));
}

#[test]
fn new_project_creates_only_a_direct_child_and_preserves_human_name() {
    let parent = tempdir().unwrap();
    let sentinel = parent.path().join("existing.txt");
    fs::write(&sentinel, "keep me").unwrap();
    let service = DesktopService::default();
    let info = service
        .create_project(parent.path(), "investigacion-vlsi", "Investigación VLSI")
        .unwrap();
    assert_eq!(
        info.root,
        parent
            .path()
            .join("investigacion-vlsi")
            .canonicalize()
            .unwrap()
    );
    assert_eq!(info.name, "Investigación VLSI");
    assert!(!parent.path().join(".braidwork").exists());
    assert_eq!(fs::read_to_string(sentinel).unwrap(), "keep me");
    service.close_project().unwrap();
    let reopened = service.open_project(&info.root).unwrap();
    assert_eq!(reopened.name, info.name);
    service.close_project().unwrap();
    let unicode = service
        .create_project(parent.path(), "日本旅行", "日本旅行")
        .unwrap();
    assert_eq!(unicode.name, "日本旅行");
}

#[test]
fn portable_folder_validation_rejects_paths_control_and_reserved_names() {
    let parent = tempdir().unwrap();
    let service = DesktopService::default();
    let invalid = [
        "",
        " ",
        ".",
        "..",
        "a/b",
        "a\\b",
        "/root",
        "a\nb",
        "a\0b",
        "a:b",
        "a*b",
        "a?b",
        "a\"b",
        "a<b",
        "a>b",
        "a|b",
        "trailing ",
        "trailing.",
        "CON",
        "pRn",
        "aux",
        "NUL",
        "con.txt",
        "com1",
        "COM9",
        "lpt1",
        "LpT9",
    ];
    for folder in invalid {
        let error = service
            .create_project(parent.path(), folder, "Project")
            .unwrap_err();
        assert_eq!(error.code, "invalid_folder", "{folder:?}");
    }
    assert_eq!(fs::read_dir(parent.path()).unwrap().count(), 0);
    for prefix in ["COM", "LPT"] {
        for number in 1..=9 {
            assert_eq!(
                service
                    .create_project(parent.path(), &format!("{prefix}{number}"), "Project")
                    .unwrap_err()
                    .code,
                "invalid_folder"
            );
        }
    }
    assert!(
        service
            .create_project(parent.path(), "COM10", "Project")
            .is_ok()
    );
}

#[test]
fn failed_creation_keeps_active_project_and_preexisting_content() {
    let parent = tempdir().unwrap();
    let service = DesktopService::default();
    let original = service
        .create_project(parent.path(), "active", "Active")
        .unwrap();
    let occupied = parent.path().join("occupied");
    fs::create_dir(&occupied).unwrap();
    fs::write(occupied.join("important.txt"), "original").unwrap();
    assert_eq!(
        service
            .create_project(parent.path(), "occupied", "Other")
            .unwrap_err()
            .code,
        "destination_exists"
    );
    assert_eq!(
        service
            .create_project(parent.path(), "active", "Other")
            .unwrap_err()
            .code,
        "destination_exists"
    );
    assert_eq!(
        service
            .create_project(parent.path(), "unused", "  ")
            .unwrap_err()
            .code,
        "invalid_name"
    );
    assert_eq!(
        service
            .create_project(&parent.path().join("missing"), "child", "Other")
            .unwrap_err()
            .code,
        "invalid_parent"
    );
    assert_eq!(
        service
            .create_project(&occupied.join("important.txt"), "child", "Other")
            .unwrap_err()
            .code,
        "invalid_parent"
    );
    assert_eq!(
        service.workspace_snapshot().unwrap().project.root,
        original.root
    );
    assert_eq!(
        fs::read_to_string(occupied.join("important.txt")).unwrap(),
        "original"
    );
    assert!(!occupied.join(".braidwork").exists());
    assert!(!parent.path().join("unused").exists());
    assert!(!parent.path().join("missing").exists());
}
