use braidwork_core::{
    assignment::AssignmentStatus,
    id::{ArtifactId, AssignmentId, ReceiptId},
    receipt::Verification,
};
use braidwork_project::Project;
use serde_json::Value;
use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

fn success(root: &Path, args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_braidwork"))
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    String::from_utf8(output.stdout).unwrap()
}
fn json(root: &Path, args: &[&str]) -> Value {
    let mut flags = vec!["--json"];
    flags.extend_from_slice(args);
    serde_json::from_str(&success(root, &flags)).unwrap()
}
fn failure(root: &Path, args: &[&str], expected: &str) {
    let output = Command::new(env!("CARGO_BIN_EXE_braidwork"))
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains(expected), "{args:?}: {stderr}");
}
fn agent(root: &Path, id: &str, revision: &str, delegate: bool) -> Value {
    let mission = format!("Produce rigorous travel research revision {revision}");
    let mut args = vec![
        "agent",
        "add",
        id,
        "--revision",
        revision,
        "--name",
        id,
        "--role",
        id,
        "--mission",
        &mission,
        "--instruction",
        "Distinguish facts from inference",
        "--expertise",
        "research",
    ];
    if delegate {
        args.push("--allow-delegation");
    }
    json(root, &args)
}
fn session(root: &Path, id: &str, agent: &str, resource: &str, revision: &str) -> Value {
    json(
        root,
        &[
            "session",
            "add",
            id,
            "--label",
            id,
            "--resource",
            resource,
            "--agent",
            agent,
            "--agent-revision",
            revision,
            "--external-ref",
            "manual chat handle",
        ],
    )
}
fn bootstrap(root: &Path) {
    json(root, &["init", "--name", "travel-planning"]);
    json(
        root,
        &[
            "resource",
            "add",
            "ai-main",
            "--name",
            "Primary account",
            "--provider",
            "Example",
        ],
    );
    agent(root, "researcher", "1", false);
    agent(root, "researcher", "2", false);
    agent(root, "coordinator", "1", true);
    session(root, "research-chat", "researcher", "ai-main", "1");
    session(root, "coordinator-chat", "coordinator", "ai-main", "1");
    json(
        root,
        &[
            "task",
            "add",
            "travel",
            "--title",
            "Travel plan",
            "--objective",
            "Research and prepare a structured travel plan",
        ],
    );
}
fn allocation(root: &Path, task: &str, session: &str) -> String {
    json(root, &["assignment", "create", task, "--session", session])["id"]
        .as_str()
        .unwrap()
        .to_owned()
}

#[test]
fn real_binary_manual_bridge_keeps_context_result_receipt_and_revision_across_processes() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    bootstrap(root);
    let id = allocation(root, "travel", "research-chat");
    let capsule = prepare_manual_context(root, &id);
    let capsule_id = capsule["id"].as_str().unwrap();
    fs::write(root.join("context.txt"), "Changed after preparation").unwrap();
    let rendered = success(root, &["capsule", "render", capsule_id]);
    assert!(rendered.contains("Two travelers"));
    assert!(!rendered.contains("Changed after preparation"));
    assert_eq!(
        json(root, &["capsule", "render", capsule_id])["rendered"],
        rendered
    );
    let dispatch = json(root, &["dispatch", &id]);
    assert_eq!(dispatch["assignment"]["status"], "prepared");
    assert_eq!(dispatch["agent"]["revision"], 1);
    let human = success(root, &["dispatch", &id]);
    assert!(human.contains("manual chat handle"));
    assert!(human.contains("researcher@1"));
    assert_eq!(
        json(root, &["assignment", "show", &id])["assignment"]["status"],
        "prepared"
    );
    let marked = json(root, &["dispatch", &id, "--mark-dispatched"]);
    assert_eq!(marked["assignment"]["status"], "dispatched");
    let bytes = b"# Braidwork Result\n\n## Output\nStructured travel plan\n";
    fs::write(root.join("result.md"), bytes).unwrap();
    let result = json(
        root,
        &["ingest", &id, "--file", "result.md", "--kind", "analysis"],
    );
    assert_eq!(result["assignment"]["status"], "result_received");
    assert_eq!(
        result["receipt"]["verification"]["decision"],
        "not_performed"
    );
    assert_eq!(result["receipt"]["execution"]["outcome"], "completed");
    assert_eq!(result["receipt"]["model_id"], Value::Null);
    assert_eq!(
        result["receipt"]["usage"],
        serde_json::json!({"input_tokens":null,"output_tokens":null,"cost":null})
    );
    let detail = json(root, &["assignment", "show", &id]);
    assert_eq!(detail["receipt"], result["receipt"]);
    assert_eq!(detail["capsule"], capsule);
    assert_eq!(json(root, &["task", "show", "travel"])["status"], "pending");
    let deep = root.join("src/deep");
    fs::create_dir_all(&deep).unwrap();
    assert_eq!(json(&deep, &["assignment", "show", &id]), detail);
    let project = Project::open(root).unwrap();
    let artifact = project
        .store()
        .get_artifact(&ArtifactId::new(result["artifact"]["id"].as_str().unwrap()).unwrap())
        .unwrap();
    assert_eq!(
        project
            .read_artifact_content(&artifact.content_ref)
            .unwrap(),
        bytes
    );
    let receipt = project
        .store()
        .get_receipt(&ReceiptId::new(result["receipt"]["id"].as_str().unwrap()).unwrap())
        .unwrap();
    assert_eq!(receipt.verification, Verification::NotPerformed);
    assert_eq!(
        project
            .store()
            .get_assignment(&AssignmentId::new(&id).unwrap())
            .unwrap()
            .status,
        AssignmentStatus::ResultReceived
    );
    failure(
        root,
        &["--json", "ingest", &id, "--file", "result.md"],
        "already been received",
    );
    assert_eq!(
        fs::read_dir(root.join(".braidwork/artifacts"))
            .unwrap()
            .count(),
        1
    );
    assert_registry_outputs(root, &id);
}

#[test]
fn stdin_ingest_preserves_known_model_zero_measurements_and_exclusive_input_flags() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    bootstrap(root);
    let id = allocation(root, "travel", "research-chat");
    json(root, &["capsule", "prepare", &id]);
    let mut child = Command::new(env!("CARGO_BIN_EXE_braidwork"))
        .current_dir(root)
        .args([
            "--json",
            "ingest",
            &id,
            "--stdin",
            "--model",
            "actual-model",
            "--input-tokens",
            "0",
            "--output-tokens",
            "0",
            "--cost-micros",
            "0",
            "--currency",
            "USD",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"Raw manual result, no required headings")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["receipt"]["model_id"], "actual-model");
    assert_eq!(result["receipt"]["usage"]["input_tokens"], 0);
    assert_eq!(result["receipt"]["usage"]["cost"]["amount_micros"], 0);
    assert_eq!(result["receipt"]["usage"]["cost"]["currency"], "USD");
    failure(
        root,
        &["ingest", &id, "--file", "missing.md", "--stdin"],
        "cannot be used",
    );
    failure(root, &["ingest", &id], "required arguments");
    failure(
        root,
        &["ingest", &id, "--stdin", "--cost-micros", "0"],
        "--currency",
    );
    failure(
        root,
        &["ingest", &id, "--stdin", "--currency", "USD"],
        "--cost-micros",
    );
}

#[test]
fn hierarchical_delegation_and_heterogeneous_sessions_reopen_through_the_binary() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    bootstrap(root);
    for (resource, provider) in [("ai-b", "Provider B"), ("ai-c", "Provider C")] {
        json(
            root,
            &[
                "resource",
                "add",
                resource,
                "--name",
                resource,
                "--provider",
                provider,
            ],
        );
    }
    agent(root, "expert", "1", true);
    agent(root, "worker", "1", false);
    session(root, "expert-chat", "expert", "ai-b", "1");
    session(root, "worker-chat", "worker", "ai-c", "1");
    let ids = [
        allocation(root, "travel", "coordinator-chat"),
        allocation(root, "travel", "expert-chat"),
        allocation(root, "travel", "worker-chat"),
    ];
    let first = json(
        root,
        &["delegation", "add", "--from", &ids[0], "--to", &ids[1]],
    );
    let second = json(
        root,
        &["delegation", "add", "--from", &ids[1], "--to", &ids[2]],
    );
    let listed = json(root, &["delegation", "list"]);
    assert_eq!(listed.as_array().unwrap().len(), 2);
    for edge in [first, second] {
        assert_eq!(
            json(root, &["delegation", "show", edge["id"].as_str().unwrap()]),
            edge
        );
    }
    for (id, session) in ids
        .iter()
        .zip(["coordinator-chat", "expert-chat", "worker-chat"])
    {
        assert_eq!(
            json(root, &["assignment", "show", id])["assignment"]["session_id"],
            session
        );
    }
    failure(
        root,
        &["delegation", "add", "--from", &ids[0], "--to", &ids[0]],
        "cannot delegate to itself",
    );
    failure(
        root,
        &["delegation", "add", "--from", &ids[2], "--to", &ids[0]],
        "historical agent policy",
    );
    failure(
        root,
        &["delegation", "add", "--from", "absent", "--to", &ids[0]],
        "missing delegation parent assignment",
    );
    failure(
        root,
        &["delegation", "add", "--from", &ids[0], "--to", "absent"],
        "missing delegation child assignment",
    );
    let project = Project::open(root).unwrap();
    assert_eq!(project.store().list_delegations().unwrap().len(), 2);
}

#[test]
fn revision_two_is_explicit_and_does_not_reinterpret_revision_one_assignment() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    bootstrap(root);
    let first = allocation(root, "travel", "research-chat");
    session(root, "new-chat", "researcher", "ai-main", "2");
    let second = allocation(root, "travel", "new-chat");
    assert_eq!(
        json(root, &["assignment", "show", &first])["assignment"]["agent_spec_revision"],
        1
    );
    assert_eq!(
        json(root, &["assignment", "show", &second])["assignment"]["agent_spec_revision"],
        2
    );
    assert_eq!(
        json(root, &["capsule", "prepare", &first])["mission"],
        "Produce rigorous travel research revision 1"
    );
    assert_eq!(
        json(root, &["capsule", "prepare", &second])["mission"],
        "Produce rigorous travel research revision 2"
    );
    failure(
        root,
        &[
            "agent",
            "add",
            "researcher",
            "--name",
            "Changed",
            "--role",
            "Changed",
            "--mission",
            "Changed",
        ],
        "entity already exists: agent researcher@1",
    );
}

#[test]
fn missing_references_context_files_and_workflow_errors_are_actionable_without_partial_state() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    bootstrap(root);
    failure(
        root,
        &[
            "session",
            "add",
            "missing",
            "--label",
            "Missing",
            "--resource",
            "absent",
            "--agent",
            "researcher",
        ],
        "missing session resource",
    );
    failure(
        root,
        &[
            "session",
            "add",
            "missing",
            "--label",
            "Missing",
            "--resource",
            "ai-main",
            "--agent",
            "researcher",
            "--agent-revision",
            "3",
        ],
        "missing session agent revision",
    );
    failure(
        root,
        &[
            "assignment",
            "create",
            "absent",
            "--session",
            "research-chat",
        ],
        "entity not found: task absent",
    );
    failure(
        root,
        &["assignment", "create", "travel", "--session", "absent"],
        "entity not found: session absent",
    );
    let id = allocation(root, "travel", "research-chat");
    failure(root, &["dispatch", &id], "no capsule prepared");
    failure(
        root,
        &["capsule", "prepare", &id, "--context-file", "absent.txt"],
        "cannot read input file",
    );
    failure(
        root,
        &[
            "capsule",
            "prepare",
            &id,
            "--context-text",
            "Context",
            "--max-context-tokens",
            "0",
        ],
        "zero context budget",
    );
    failure(
        root,
        &["capsule", "prepare", &id, "--output", "unknown:Description"],
        "invalid value",
    );
    json(root, &["capsule", "prepare", &id]);
    failure(root, &["capsule", "prepare", &id], "already prepared");
    failure(
        root,
        &["ingest", &id, "--file", "absent.md"],
        "cannot read input file",
    );
    assert_eq!(
        json(root, &["assignment", "show", &id])["assignment"]["status"],
        "prepared"
    );
    assert!(!root.join(".braidwork/artifacts").exists());
    failure(
        root,
        &["agent", "show", "researcher", "--revision", "0"],
        "invalid value",
    );
}

#[test]
fn new_help_and_empty_json_lists_work_without_simulated_data() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    for command in [
        "agent",
        "session",
        "assignment",
        "delegation",
        "capsule",
        "dispatch",
        "ingest",
    ] {
        assert!(success(root, &[command, "--help"]).contains("Usage:"));
    }
    json(root, &["init"]);
    for command in ["agent", "session", "assignment", "delegation"] {
        assert_eq!(json(root, &[command, "list"]), serde_json::json!([]));
    }
}

fn prepare_manual_context(root: &Path, id: &str) -> Value {
    fs::write(
        root.join("context.txt"),
        "Two travelers; five days; accessible transport.",
    )
    .unwrap();
    let capsule = json(
        root,
        &[
            "capsule",
            "prepare",
            id,
            "--context-file",
            "context.txt",
            "--context-text",
            "Prefer rail travel",
            "--constraint",
            "Do not fabricate prices",
            "--accept",
            "Explain budget assumptions",
            "--output",
            "analysis:Structured itinerary",
            "--output",
            "text:Budget summary",
            "--max-context-tokens",
            "2048",
        ],
    );
    assert_eq!(capsule["context_budget"]["max_estimated_tokens"], 2048);
    assert_eq!(
        capsule["instructions"][0],
        "Distinguish facts from inference"
    );
    assert_eq!(capsule["expected_outputs"].as_array().unwrap().len(), 2);
    capsule
}

fn assert_registry_outputs(root: &Path, id: &str) {
    assert_eq!(
        json(root, &["agent", "show", "researcher", "--revision", "1"])["revision"],
        1
    );
    assert_eq!(
        json(root, &["agent", "show", "researcher", "--revision", "2"])["revision"],
        2
    );
    assert_eq!(
        json(root, &["session", "list"]).as_array().unwrap().len(),
        2
    );
    assert_eq!(
        json(root, &["session", "show", "research-chat"])["resource_id"],
        "ai-main"
    );
    assert_eq!(json(root, &["assignment", "list"])[0]["id"], id);
    let human = success(root, &["assignment", "show", id]);
    assert!(human.contains(&format!("ID: {id}")));
    assert!(human.contains("Agent: researcher@1"));
    assert!(human.contains("Status: result_received"));
    let list = success(root, &["assignment", "list"]);
    assert!(list.contains("AGENT@REVISION"));
    assert!(list.contains("researcher@1"));
    assert_eq!(json(root, &["agent", "list"]).as_array().unwrap().len(), 3);
}
