use serde_json::Value;
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn run(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_braidwork"))
        .current_dir(root)
        .args(args)
        .output()
        .unwrap()
}
fn success(root: &Path, args: &[&str]) -> String {
    let output = run(root, args);
    assert!(
        output.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "unexpected stderr: {:?}",
        output.stderr
    );
    String::from_utf8(output.stdout).unwrap()
}
fn json(root: &Path, args: &[&str]) -> Value {
    serde_json::from_str(&success(root, args)).unwrap()
}
fn failure(root: &Path, args: &[&str], expected: &str) {
    let output = run(root, args);
    assert!(!output.status.success(), "unexpected success: {args:?}");
    assert!(output.stdout.is_empty(), "errors must not reach stdout");
    assert!(
        String::from_utf8(output.stderr).unwrap().contains(expected),
        "expected {expected}"
    );
}
fn initialized() -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    success(directory.path(), &["init", "--name", "demo"]);
    directory
}

#[test]
fn binary_end_to_end_persists_resources_and_tasks_across_processes_and_discovery() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let init = json(root, &["--json", "init", "--name", "compiler-lab"]);
    assert_eq!(init["name"], "compiler-lab");
    assert_eq!(init["project_format_version"], 1);
    assert_eq!(init["store_schema_version"], 2);
    assert!(root.join(".braidwork/project.json").is_file());
    assert!(root.join(".braidwork/braidwork.db").is_file());
    let status = json(root, &["--json", "status"]);
    assert_eq!(status["resource_count"], 0);
    assert_eq!(status["tasks"]["total"], 0);
    let resource = json(
        root,
        &[
            "--json",
            "resource",
            "add",
            "chatgpt-plus-main",
            "--name",
            "ChatGPT Plus",
            "--provider",
            "OpenAI",
            "--scarcity",
            "scarce",
        ],
    );
    assert_eq!(resource["access_mode"], "manual");
    assert_eq!(resource["status"], "available");
    assert_eq!(resource["scarcity"], "scarce");
    assert_eq!(
        json(root, &["--json", "resource", "list"]),
        serde_json::json!([resource])
    );
    assert_eq!(
        json(root, &["--json", "resource", "show", "chatgpt-plus-main"]),
        resource
    );
    let prerequisite = json(
        root,
        &[
            "--json",
            "task",
            "add",
            "requirements",
            "--title",
            "Understand requirements",
            "--objective",
            "Precise plan",
        ],
    );
    assert_eq!(prerequisite["status"], "pending");
    let child = json(
        root,
        &[
            "--json",
            "task",
            "add",
            "implementation",
            "--title",
            "Implement feature",
            "--objective",
            "Accepted design",
            "--parent",
            "requirements",
            "--depends-on",
            "requirements",
            "--depends-on",
            "requirements",
        ],
    );
    assert_eq!(child["parent"], "requirements");
    assert_eq!(child["dependencies"], serde_json::json!(["requirements"]));
    assert_eq!(child["status"], "pending");
    assert_eq!(
        json(root, &["--json", "task", "list"]),
        serde_json::json!([child, prerequisite])
    );
    assert_eq!(
        json(root, &["--json", "task", "show", "implementation"]),
        child
    );
    assert_nested_discovery(root, &child);
}

fn assert_nested_discovery(root: &Path, child: &Value) {
    let deep = root.join("src/deep/module");
    fs::create_dir_all(&deep).unwrap();
    let status = json(&deep, &["--json", "status"]);
    assert_eq!(
        status["project"]["root"],
        root.canonicalize().unwrap().to_str().unwrap()
    );
    assert_eq!(status["resource_count"], 1);
    assert_eq!(
        status["tasks"],
        serde_json::json!({"total":2,"pending":2,"in_progress":0,"completed":0})
    );
    assert_eq!(
        json(&deep, &["task", "show", "implementation", "--json"]),
        *child
    );
    assert!(success(&deep, &["resource", "list"]).contains("chatgpt-plus-main"));
    assert!(success(&deep, &["task", "list"]).contains("implementation"));
    assert!(
        success(&deep, &["task", "show", "implementation"]).contains("Dependencies: requirements")
    );
}

#[test]
fn human_output_is_useful_and_empty_lists_are_valid_json_arrays() {
    let directory = initialized();
    let root = directory.path();
    assert!(success(root, &["status"]).contains("Project: demo"));
    assert_eq!(
        json(root, &["--json", "resource", "list"]),
        serde_json::json!([])
    );
    assert_eq!(
        json(root, &["--json", "task", "list"]),
        serde_json::json!([])
    );
    assert!(success(root, &["resource", "list"]).contains("PROVIDER"));
    assert!(success(root, &["task", "list"]).contains("TITLE"));
    let output = success(
        root,
        &[
            "resource",
            "add",
            "local",
            "--name",
            "Local",
            "--provider",
            "Example",
            "--access-mode",
            "local",
            "--scarcity",
            "abundant",
            "--status",
            "unavailable",
        ],
    );
    assert!(output.contains("Access: local"));
    assert!(success(root, &["resource", "show", "local"]).contains("Status: unavailable"));
    success(
        root,
        &[
            "task",
            "add",
            "task",
            "--title",
            "Title",
            "--objective",
            "Objective",
        ],
    );
    let output = success(root, &["task", "show", "task"]);
    for text in [
        "ID: task",
        "Title: Title",
        "Objective: Objective",
        "Status: pending",
        "Parent: (none)",
        "Dependencies: (none)",
    ] {
        assert!(output.contains(text));
    }
}

#[test]
fn duplicate_and_not_found_errors_use_stderr_and_fail_without_overwriting() {
    let directory = initialized();
    let root = directory.path();
    success(
        root,
        &[
            "resource",
            "add",
            "main",
            "--name",
            "Original",
            "--provider",
            "Example",
        ],
    );
    failure(
        root,
        &[
            "--json",
            "resource",
            "add",
            "main",
            "--name",
            "Changed",
            "--provider",
            "Other",
        ],
        "entity already exists: resource main",
    );
    assert_eq!(
        json(root, &["--json", "resource", "show", "main"])["name"],
        "Original"
    );
    success(
        root,
        &[
            "task",
            "add",
            "task",
            "--title",
            "Original",
            "--objective",
            "Objective",
        ],
    );
    failure(
        root,
        &[
            "task",
            "add",
            "task",
            "--title",
            "Changed",
            "--objective",
            "Other",
        ],
        "entity already exists: task task",
    );
    assert_eq!(
        json(root, &["--json", "task", "show", "task"])["title"],
        "Original"
    );
    failure(
        root,
        &["resource", "show", "missing"],
        "entity not found: resource missing",
    );
    failure(
        root,
        &["--json", "task", "show", "missing"],
        "entity not found: task missing",
    );
    failure(root, &["init", "--name", "new"], "already initialized");
}

#[test]
fn invalid_task_references_and_ids_do_not_leave_partial_entities() {
    let directory = initialized();
    let root = directory.path();
    for relation in ["--parent", "--depends-on"] {
        failure(
            root,
            &[
                "--json",
                "task",
                "add",
                "candidate",
                "--title",
                "Title",
                "--objective",
                "Objective",
                relation,
                "missing",
            ],
            "Check that the parent and prerequisite tasks exist",
        );
        assert_eq!(
            json(root, &["--json", "task", "list"]),
            serde_json::json!([])
        );
    }
    failure(
        root,
        &[
            "task",
            "add",
            "self",
            "--title",
            "Title",
            "--objective",
            "Objective",
            "--depends-on",
            "self",
        ],
        "cannot depend on itself",
    );
    failure(
        root,
        &["resource", "show", " leading"],
        "no leading or trailing whitespace",
    );
    failure(
        root,
        &["task", "show", "trailing "],
        "no leading or trailing whitespace",
    );
    failure(
        root,
        &[
            "resource",
            "add",
            "api",
            "--name",
            "API",
            "--provider",
            "Example",
            "--access-mode",
            "unknown",
        ],
        "invalid value",
    );
}

#[test]
fn explicit_project_opens_exact_root_instead_of_discovering_current_project() {
    let first = initialized();
    let second = initialized();
    let second_path = second.path().to_str().unwrap();
    let status = json(
        first.path(),
        &["--json", "--project", second_path, "status"],
    );
    assert_eq!(
        status["project"]["root"],
        second.path().canonicalize().unwrap().to_str().unwrap()
    );
    let deep = first.path().join("subdir");
    fs::create_dir(&deep).unwrap();
    failure(
        first.path(),
        &["--project", deep.to_str().unwrap(), "status"],
        "project marker missing",
    );
    failure(
        first.path(),
        &["--project", second_path, "init"],
        "use init <PATH>",
    );
    assert_eq!(
        json(first.path(), &["--json", "status"])["project"]["name"],
        "demo"
    );
}

#[test]
fn commands_outside_a_project_fail_and_explicit_init_path_works() {
    let directory = tempfile::tempdir().unwrap();
    failure(directory.path(), &["status"], "no Braidwork project found");
    failure(
        directory.path(),
        &["--json", "resource", "list"],
        "no Braidwork project found",
    );
    let child = directory.path().join("compiler-lab");
    fs::create_dir(&child).unwrap();
    let result = json(
        directory.path(),
        &["--json", "init", child.to_str().unwrap()],
    );
    assert_eq!(result["name"], "compiler-lab");
    assert_eq!(
        json(
            directory.path(),
            &["--json", "--project", child.to_str().unwrap(), "status"]
        )["project"]["name"],
        "compiler-lab"
    );
    assert!(!directory.path().join(".braidwork").exists());
}

#[test]
fn discovery_errors_on_nearest_corrupt_marker_and_does_not_use_parent() {
    let directory = initialized();
    let child = directory.path().join("child");
    fs::create_dir_all(child.join(".braidwork")).unwrap();
    fs::write(child.join(".braidwork/project.json"), b"{broken").unwrap();
    failure(&child, &["--json", "status"], "invalid project metadata");
    assert!(!child.join(".braidwork/braidwork.db").exists());
}

#[test]
fn help_version_and_required_arguments_work_without_a_project() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    for args in [
        vec!["--help"],
        vec!["resource", "--help"],
        vec!["task", "--help"],
        vec!["init", "--help"],
        vec!["resource", "add", "--help"],
        vec!["task", "add", "--help"],
    ] {
        assert!(success(root, &args).contains("Usage:"));
    }
    let help = success(root, &["resource", "add", "--help"]);
    for default in [
        "[default: manual]",
        "[default: normal]",
        "[default: available]",
    ] {
        assert!(help.contains(default));
    }
    assert_eq!(
        success(root, &["--version"]).trim(),
        concat!("braidwork ", env!("CARGO_PKG_VERSION"))
    );
    failure(root, &["resource", "add", "id"], "required arguments");
    failure(root, &["task", "add", "id"], "required arguments");
}
