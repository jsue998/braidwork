use super::*;
use braidwork_core::{
    id::{ResourceId, TaskId},
    resource::{AccessMode, Resource, ResourceStatus, Scarcity},
    task::Task,
};
use braidwork_store::SCHEMA_VERSION;
use std::fs;

fn marker(root: &Path) -> PathBuf {
    root.join(STATE_DIRECTORY).join(PROJECT_FILE)
}
fn resource() -> Resource {
    Resource {
        id: ResourceId::new("manual-main").unwrap(),
        name: "Manual resource".into(),
        provider: "Example".into(),
        access_mode: AccessMode::Manual,
        scarcity: Scarcity::Normal,
        status: ResourceStatus::Available,
    }
}

#[test]
fn init_creates_only_the_canonical_layout_and_versions() {
    let directory = tempfile::tempdir().unwrap();
    let project = Project::init(directory.path(), Some("compiler-lab")).unwrap();
    assert_eq!(project.root(), directory.path().canonicalize().unwrap());
    assert_eq!(project.state_path(), project.root().join(STATE_DIRECTORY));
    assert_eq!(
        project.database_path(),
        project.state_path().join(DATABASE_FILE)
    );
    assert_eq!(project.format_version(), PROJECT_FORMAT_VERSION);
    assert_eq!(project.schema_version().unwrap(), SCHEMA_VERSION);
    let json: serde_json::Value =
        serde_json::from_slice(&fs::read(marker(directory.path())).unwrap()).unwrap();
    assert_eq!(
        json,
        serde_json::json!({"format_version":1,"name":"compiler-lab"})
    );
    let mut entries: Vec<_> = fs::read_dir(project.state_path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    entries.sort();
    assert_eq!(entries, [DATABASE_FILE, PROJECT_FILE]);
}

#[test]
fn implicit_name_uses_root_filename_and_human_names_are_not_normalized() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("Compiler Lab");
    fs::create_dir(&root).unwrap();
    let project = Project::init(&root, None).unwrap();
    assert_eq!(project.metadata().name(), "Compiler Lab");
    let other = directory.path().join("other");
    fs::create_dir(&other).unwrap();
    let project = Project::init(&other, Some("  Human Name  ")).unwrap();
    assert_eq!(project.metadata().name(), "  Human Name  ");
    drop(project);
    assert_eq!(
        Project::open(&other).unwrap().metadata().name(),
        "  Human Name  "
    );
}

#[test]
fn reopening_preserves_metadata_resources_and_task_relationships() {
    let directory = tempfile::tempdir().unwrap();
    let resource = resource();
    let prerequisite = Task::new(
        TaskId::new("requirements").unwrap(),
        "Requirements",
        "Understand requirements",
        None,
        [],
    )
    .unwrap();
    let task = Task::new(
        TaskId::new("implementation").unwrap(),
        "Implementation",
        "Accepted design",
        Some(prerequisite.id().clone()),
        [prerequisite.id().clone()],
    )
    .unwrap();
    {
        let mut project = Project::init(directory.path(), Some("demo")).unwrap();
        project.store_mut().insert_resource(&resource).unwrap();
        project.store_mut().insert_task(&prerequisite).unwrap();
        project.store_mut().insert_task(&task).unwrap();
    }
    for _ in 0..2 {
        let project = Project::open(directory.path()).unwrap();
        assert_eq!(project.metadata().name(), "demo");
        assert_eq!(
            project.store().get_resource(&resource.id).unwrap(),
            resource
        );
        assert_eq!(project.store().get_task(task.id()).unwrap(), task);
    }
}

#[test]
fn duplicate_init_preserves_marker_and_database_contents() {
    let directory = tempfile::tempdir().unwrap();
    let mut project = Project::init(directory.path(), Some("original")).unwrap();
    project.store_mut().insert_resource(&resource()).unwrap();
    drop(project);
    let original = fs::read(marker(directory.path())).unwrap();
    assert!(matches!(
        Project::init(directory.path(), Some("replacement")),
        Err(ProjectError::AlreadyInitialized(_))
    ));
    assert_eq!(fs::read(marker(directory.path())).unwrap(), original);
    let project = Project::open(directory.path()).unwrap();
    assert_eq!(
        project.store().get_resource(&resource().id).unwrap(),
        resource()
    );
}

#[test]
fn ambiguous_existing_state_is_preserved_without_publishing_a_marker() {
    let directory = tempfile::tempdir().unwrap();
    let state = directory.path().join(STATE_DIRECTORY);
    fs::create_dir(&state).unwrap();
    fs::write(state.join("sentinel"), b"preserve").unwrap();
    assert!(matches!(
        Project::init(directory.path(), Some("demo")),
        Err(ProjectError::ExistingState { .. })
    ));
    assert_eq!(fs::read(state.join("sentinel")).unwrap(), b"preserve");
    assert!(!marker(directory.path()).exists());
    assert!(!state.join(DATABASE_FILE).exists());
}

#[test]
fn invalid_root_and_names_fail_before_creating_state() {
    let directory = tempfile::tempdir().unwrap();
    for name in ["", "   ", "\t\n", "\u{2003}"] {
        assert!(matches!(
            Project::init(directory.path(), Some(name)),
            Err(ProjectError::InvalidName)
        ));
        assert!(!directory.path().join(STATE_DIRECTORY).exists());
    }
    let file = directory.path().join("file");
    fs::write(&file, b"preserve").unwrap();
    assert!(matches!(
        Project::init(&file, Some("demo")),
        Err(ProjectError::InvalidRoot(_))
    ));
    assert!(matches!(
        Project::init(directory.path().join("absent"), Some("demo")),
        Err(ProjectError::Io { .. })
    ));
}

#[test]
fn corrupt_or_incomplete_json_is_rejected_and_not_rewritten() {
    let directory = tempfile::tempdir().unwrap();
    drop(Project::init(directory.path(), Some("demo")).unwrap());
    for text in [
        "{invalid",
        "{}",
        r#"{"format_version":1}"#,
        r#"{"format_version":"1","name":"demo"}"#,
    ] {
        fs::write(marker(directory.path()), text).unwrap();
        assert!(matches!(
            Project::open(directory.path()),
            Err(ProjectError::MetadataJson { .. })
        ));
        assert_eq!(fs::read_to_string(marker(directory.path())).unwrap(), text);
        assert!(matches!(
            Project::init(directory.path(), Some("new")),
            Err(ProjectError::ExistingState { .. })
        ));
        assert_eq!(fs::read_to_string(marker(directory.path())).unwrap(), text);
    }
}

#[test]
fn unsupported_format_and_invalid_persisted_name_fail() {
    let directory = tempfile::tempdir().unwrap();
    drop(Project::init(directory.path(), Some("demo")).unwrap());
    for version in [0, PROJECT_FORMAT_VERSION + 1] {
        fs::write(
            marker(directory.path()),
            serde_json::to_vec(&serde_json::json!({"format_version":version,"name":"demo"}))
                .unwrap(),
        )
        .unwrap();
        assert!(
            matches!(Project::open(directory.path()), Err(ProjectError::UnsupportedFormat { found }) if found == version)
        );
    }
    for name in ["", " \t\n"] {
        fs::write(
            marker(directory.path()),
            serde_json::to_vec(&serde_json::json!({"format_version":1,"name":name})).unwrap(),
        )
        .unwrap();
        assert!(matches!(
            Project::open(directory.path()),
            Err(ProjectError::InvalidName)
        ));
    }
}

#[test]
fn open_requires_a_real_marker_and_does_not_recreate_missing_database() {
    let directory = tempfile::tempdir().unwrap();
    drop(Project::init(directory.path(), Some("demo")).unwrap());
    let database = directory.path().join(STATE_DIRECTORY).join(DATABASE_FILE);
    fs::remove_file(&database).unwrap();
    assert!(matches!(
        Project::open(directory.path()),
        Err(ProjectError::DatabaseMissing(_))
    ));
    assert!(!database.exists());
    fs::remove_file(marker(directory.path())).unwrap();
    assert!(matches!(
        Project::open(directory.path()),
        Err(ProjectError::MarkerMissing(_))
    ));
    fs::create_dir(marker(directory.path())).unwrap();
    assert!(matches!(
        Project::open(directory.path()),
        Err(ProjectError::InvalidMarker(_))
    ));
}

#[test]
fn discovery_finds_root_from_several_levels_and_selects_nearest_project() {
    let directory = tempfile::tempdir().unwrap();
    drop(Project::init(directory.path(), Some("outer")).unwrap());
    let nested = directory.path().join("src/deep/module");
    fs::create_dir_all(&nested).unwrap();
    for start in [directory.path(), &directory.path().join("src"), &nested] {
        let project = Project::discover(start).unwrap();
        assert_eq!(project.root(), directory.path().canonicalize().unwrap());
    }
    let inner = directory.path().join("src");
    drop(Project::init(&inner, Some("inner")).unwrap());
    assert_eq!(
        Project::discover(&nested).unwrap().metadata().name(),
        "inner"
    );
    assert_eq!(
        Project::open(directory.path()).unwrap().metadata().name(),
        "outer"
    );
}

#[test]
fn discovery_outside_a_project_returns_not_found() {
    let directory = tempfile::tempdir().unwrap();
    assert!(matches!(
        Project::discover(directory.path()),
        Err(ProjectError::NotFound(_))
    ));
}

#[test]
fn corrupt_or_missing_nearest_marker_stops_discovery_at_that_root() {
    let directory = tempfile::tempdir().unwrap();
    drop(Project::init(directory.path(), Some("outer")).unwrap());
    let child = directory.path().join("child");
    fs::create_dir(&child).unwrap();
    let state = child.join(STATE_DIRECTORY);
    fs::create_dir(&state).unwrap();
    assert!(matches!(
        Project::discover(&child),
        Err(ProjectError::MarkerMissing(_))
    ));
    fs::write(marker(&child), b"{broken").unwrap();
    assert!(matches!(
        Project::discover(&child),
        Err(ProjectError::MetadataJson { .. })
    ));
}

#[test]
fn explicit_open_does_not_discover_a_parent_project() {
    let directory = tempfile::tempdir().unwrap();
    drop(Project::init(directory.path(), Some("outer")).unwrap());
    let child = directory.path().join("child");
    fs::create_dir(&child).unwrap();
    assert!(matches!(
        Project::open(&child),
        Err(ProjectError::MarkerMissing(_))
    ));
    assert_eq!(
        Project::discover(&child).unwrap().metadata().name(),
        "outer"
    );
}

#[test]
fn unusable_database_returns_store_error_without_changing_marker() {
    let directory = tempfile::tempdir().unwrap();
    drop(Project::init(directory.path(), Some("demo")).unwrap());
    let original = fs::read(marker(directory.path())).unwrap();
    fs::write(
        directory.path().join(STATE_DIRECTORY).join(DATABASE_FILE),
        b"not SQLite",
    )
    .unwrap();
    assert!(matches!(
        Project::open(directory.path()),
        Err(ProjectError::Store(_))
    ));
    assert_eq!(fs::read(marker(directory.path())).unwrap(), original);
}

#[test]
fn state_path_occupied_by_a_file_is_preserved() {
    let directory = tempfile::tempdir().unwrap();
    let state = directory.path().join(STATE_DIRECTORY);
    fs::write(&state, b"not a project directory").unwrap();
    assert!(matches!(
        Project::init(directory.path(), Some("demo")),
        Err(ProjectError::ExistingState { .. })
    ));
    assert_eq!(fs::read(&state).unwrap(), b"not a project directory");
    assert!(Project::discover(directory.path()).is_err());
}

#[test]
fn a_database_alone_is_not_a_project_marker() {
    let directory = tempfile::tempdir().unwrap();
    let state = directory.path().join(STATE_DIRECTORY);
    fs::create_dir(&state).unwrap();
    drop(SqliteStore::open(state.join(DATABASE_FILE)).unwrap());
    assert!(matches!(
        Project::open(directory.path()),
        Err(ProjectError::MarkerMissing(_))
    ));
    assert!(matches!(
        Project::discover(directory.path()),
        Err(ProjectError::MarkerMissing(_))
    ));
    assert!(!state.join(PROJECT_FILE).exists());
}

#[test]
fn declared_status_and_bounded_content_survive_reopen_without_changing_bytes() {
    use braidwork_core::{id::ArtifactId, task::TaskStatus};
    let directory = tempfile::tempdir().unwrap();
    let mut project = Project::init(directory.path(), Some("Japan Trip")).unwrap();
    let task = Task::new(
        TaskId::new("travel").unwrap(),
        "Transport",
        "Compare travel options",
        None,
        [],
    )
    .unwrap();
    project.store_mut().insert_task(&task).unwrap();
    project
        .set_task_status(task.id(), TaskStatus::Completed)
        .unwrap();
    let reference = project
        .write_artifact_content(&ArtifactId::new("preview").unwrap(), b"exact content")
        .unwrap();
    assert_eq!(
        project.read_artifact_content_up_to(&reference, 12).unwrap(),
        None
    );
    assert_eq!(
        project
            .read_artifact_content_up_to(&reference, 13)
            .unwrap()
            .unwrap(),
        b"exact content"
    );
    assert_eq!(
        project.read_artifact_content_up_to(&reference, 0).unwrap(),
        None
    );
    let empty = project
        .write_artifact_content(&ArtifactId::new("empty-preview").unwrap(), b"")
        .unwrap();
    assert_eq!(
        project.read_artifact_content_up_to(&empty, 0).unwrap(),
        Some(vec![])
    );
    assert!(matches!(
        project.read_artifact_content_up_to("artifact:aa.bin", 20),
        Err(crate::ArtifactContentError::Missing(_))
    ));
    drop(project);
    let reopened = Project::open(directory.path()).unwrap();
    assert_eq!(
        reopened.store().get_task(task.id()).unwrap().status(),
        TaskStatus::Completed
    );
    assert_eq!(
        reopened.read_artifact_content(&reference).unwrap(),
        b"exact content"
    );
}
