use braidwork_core::task::TaskError;
use braidwork_project::ProjectError;
use braidwork_store::StoreError;
use std::io;
use thiserror::Error;
#[derive(Debug, Error)]
pub enum CliError {
    #[error("cannot read input file {path}: {source}")]
    InputFile {
        path: std::path::PathBuf,
        #[source]
        source: io::Error,
    },
    #[error(transparent)]
    Project(#[from] ProjectError),
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Task(#[from] TaskError),
    #[error("cannot create task. Check that the parent and prerequisite tasks exist: {0}")]
    TaskReferences(StoreError),
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("JSON output error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("output formatting error: {0}")]
    Format(#[from] std::fmt::Error),
    #[error(
        "--project selects an existing project; use init <PATH> to choose an initialization directory"
    )]
    InitProjectConflict,
    #[error("--cost-micros and --currency must be supplied together")]
    IncompleteCost,
}
