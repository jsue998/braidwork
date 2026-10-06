//! Lifecycle and discovery of local Braidwork projects, independent of CLI UX.
//!
//! A versioned marker identifies a project. Its sibling database is owned by
//! `braidwork-store`; core entities remain unaware of this filesystem layout.

use braidwork_store::{SqliteStore, StoreError};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use thiserror::Error;

/// Directory containing canonical project state.
pub const STATE_DIRECTORY: &str = ".braidwork";
/// Explicit project marker and human metadata.
pub const PROJECT_FILE: &str = "project.json";
/// Canonical `SQLite` database filename.
pub const DATABASE_FILE: &str = "braidwork.db";
/// The only project metadata format currently supported.
pub const PROJECT_FORMAT_VERSION: u32 = 1;

/// Validated human metadata, preserved without trimming or slug conversion.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ProjectMetadata {
    format_version: u32,
    name: String,
}

impl ProjectMetadata {
    /// Returns the supported marker format version.
    #[must_use]
    pub fn format_version(&self) -> u32 {
        self.format_version
    }
    /// Returns the original human name, which is neither empty nor whitespace-only.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Deserialize)]
struct Marker {
    format_version: u32,
    name: String,
}

/// Typed lifecycle failures; existing state is never destroyed or repaired.
#[derive(Debug, Error)]
pub enum ProjectError {
    /// No marker directory was found during discovery.
    #[error("no Braidwork project found from {0}; run braidwork init first")]
    NotFound(PathBuf),
    /// A valid project already occupies this root.
    #[error("project already initialized at {0}")]
    AlreadyInitialized(PathBuf),
    /// Existing state is ambiguous or incompatible and has been preserved.
    #[error("existing state at {path} was preserved: {source}")]
    ExistingState {
        /// Existing state directory.
        path: PathBuf,
        /// Reason the existing project could not be opened.
        #[source]
        source: Box<ProjectError>,
    },
    /// The supplied root is not a directory.
    #[error("project root is not a directory: {0}")]
    InvalidRoot(PathBuf),
    /// State exists but its explicit marker does not.
    #[error("project marker missing: {0}")]
    MarkerMissing(PathBuf),
    /// The marker path exists but is not a file.
    #[error("project marker is not a file: {0}")]
    InvalidMarker(PathBuf),
    /// An existing marker has no usable database file.
    #[error("project database missing or not a file: {0}")]
    DatabaseMissing(PathBuf),
    /// Metadata cannot be parsed, including missing or mistyped required fields.
    #[error("invalid project metadata at {path}: {source}")]
    MetadataJson {
        /// Marker being read or written.
        path: PathBuf,
        /// Original JSON error.
        #[source]
        source: serde_json::Error,
    },
    /// No marker migrations or fallback versions are supported yet.
    #[error("unsupported project format {found}; expected {PROJECT_FORMAT_VERSION}")]
    UnsupportedFormat {
        /// Persisted format found in the marker.
        found: u32,
    },
    /// A human name must contain some non-whitespace text.
    #[error("project name must contain non-whitespace text; supply --name if it cannot be derived")]
    InvalidName,
    /// Filesystem failure retains the path and original error.
    #[error("filesystem error at {path}: {source}")]
    Io {
        /// Path involved in the failed operation.
        path: PathBuf,
        /// Original filesystem error.
        #[source]
        source: std::io::Error,
    },
    /// Persistence failure retains the store's typed semantics.
    #[error(transparent)]
    Store(#[from] StoreError),
}

/// An opened project owning its store and canonical absolute paths.
pub struct Project {
    root: PathBuf,
    state_path: PathBuf,
    database_path: PathBuf,
    metadata: ProjectMetadata,
    store: SqliteStore,
}

impl Project {
    /// Initializes an existing directory with metadata and a migrated database.
    ///
    /// Without a name, the root directory's UTF-8 filename is used. The state
    /// directory is created exclusively, then the database, then a temporary
    /// marker which is renamed into place. Failure may leave incomplete state,
    /// but never publishes a marker before essential components are ready.
    /// Existing or incomplete state is preserved for explicit inspection.
    ///
    /// # Errors
    /// Returns typed root/name, existing-state, filesystem, JSON, or store errors.
    pub fn init(root: impl AsRef<Path>, name: Option<&str>) -> Result<Self, ProjectError> {
        let root = canonical_root(root.as_ref())?;
        let name = name
            .or_else(|| root.file_name().and_then(|value| value.to_str()))
            .ok_or(ProjectError::InvalidName)?;
        validate_name(name)?;
        let metadata = ProjectMetadata {
            format_version: PROJECT_FORMAT_VERSION,
            name: name.to_owned(),
        };
        let state_path = root.join(STATE_DIRECTORY);
        if let Err(source) = fs::create_dir(&state_path) {
            if source.kind() == std::io::ErrorKind::AlreadyExists {
                return match Self::open(&root) {
                    Ok(_) => Err(ProjectError::AlreadyInitialized(root)),
                    Err(source) => Err(ProjectError::ExistingState {
                        path: state_path,
                        source: Box::new(source),
                    }),
                };
            }
            return Err(io_error(&state_path, source));
        }
        let database_path = state_path.join(DATABASE_FILE);
        let store = SqliteStore::open(&database_path)?;
        let marker_path = state_path.join(PROJECT_FILE);
        let bytes =
            serde_json::to_vec_pretty(&metadata).map_err(|source| ProjectError::MetadataJson {
                path: marker_path.clone(),
                source,
            })?;
        let temporary = state_path.join(format!("{PROJECT_FILE}.tmp"));
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|source| io_error(&temporary, source))?;
        file.write_all(&bytes)
            .map_err(|source| io_error(&temporary, source))?;
        file.sync_all()
            .map_err(|source| io_error(&temporary, source))?;
        drop(file);
        fs::rename(&temporary, &marker_path).map_err(|source| io_error(&marker_path, source))?;
        Ok(Self {
            root,
            state_path,
            database_path,
            metadata,
            store,
        })
    }

    /// Opens exactly the supplied root; does not discover parents or invent files.
    ///
    /// # Errors
    /// Returns explicit missing/invalid marker, format/name, filesystem, JSON,
    /// missing database, or store errors. No corrupt metadata is regenerated.
    pub fn open(root: impl AsRef<Path>) -> Result<Self, ProjectError> {
        let root = canonical_root(root.as_ref())?;
        let state_path = root.join(STATE_DIRECTORY);
        let marker_path = state_path.join(PROJECT_FILE);
        let marker_info = match fs::metadata(&marker_path) {
            Ok(info) => info,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                return Err(ProjectError::MarkerMissing(marker_path));
            }
            Err(source) => return Err(io_error(&marker_path, source)),
        };
        if !marker_info.is_file() {
            return Err(ProjectError::InvalidMarker(marker_path));
        }
        let bytes = fs::read(&marker_path).map_err(|source| io_error(&marker_path, source))?;
        let marker: Marker =
            serde_json::from_slice(&bytes).map_err(|source| ProjectError::MetadataJson {
                path: marker_path,
                source,
            })?;
        if marker.format_version != PROJECT_FORMAT_VERSION {
            return Err(ProjectError::UnsupportedFormat {
                found: marker.format_version,
            });
        }
        validate_name(&marker.name)?;
        let database_path = state_path.join(DATABASE_FILE);
        match fs::metadata(&database_path) {
            Ok(info) if info.is_file() => {}
            Ok(_) => return Err(ProjectError::DatabaseMissing(database_path)),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                return Err(ProjectError::DatabaseMissing(database_path));
            }
            Err(source) => return Err(io_error(&database_path, source)),
        }
        let store = SqliteStore::open(&database_path)?;
        Ok(Self {
            root,
            state_path,
            database_path,
            metadata: ProjectMetadata {
                format_version: marker.format_version,
                name: marker.name,
            },
            store,
        })
    }

    /// Discovers the nearest project by walking an existing directory's parents.
    ///
    /// Any encountered state directory is opened immediately. An invalid marker
    /// stops discovery instead of silently selecting a different parent project.
    ///
    /// # Errors
    /// Returns [`ProjectError::NotFound`] if none exists, or the first encountered
    /// project's open error. Filesystem and invalid start-directory errors propagate.
    pub fn discover(start: impl AsRef<Path>) -> Result<Self, ProjectError> {
        let start = canonical_root(start.as_ref())?;
        for root in start.ancestors() {
            let state = root.join(STATE_DIRECTORY);
            match fs::symlink_metadata(&state) {
                Ok(_) => return Self::open(root),
                Err(source) if source.kind() == std::io::ErrorKind::NotFound => {}
                Err(source) => return Err(io_error(&state, source)),
            }
        }
        Err(ProjectError::NotFound(start))
    }

    /// Canonical absolute project root.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }
    /// Canonical state location under the root.
    #[must_use]
    pub fn state_path(&self) -> &Path {
        &self.state_path
    }
    /// Database location; callers need not know `SQLite` details.
    #[must_use]
    pub fn database_path(&self) -> &Path {
        &self.database_path
    }
    /// Validated marker metadata.
    #[must_use]
    pub fn metadata(&self) -> &ProjectMetadata {
        &self.metadata
    }
    /// Project marker format, distinct from the database schema version.
    #[must_use]
    pub fn format_version(&self) -> u32 {
        self.metadata.format_version()
    }
    /// Reads the associated store's persisted schema version.
    ///
    /// # Errors
    /// Returns the store error if the schema version cannot be queried.
    pub fn schema_version(&self) -> Result<u32, ProjectError> {
        Ok(self.store.schema_version()?)
    }
    /// Access to domain reads, without exposing a database connection.
    #[must_use]
    pub fn store(&self) -> &SqliteStore {
        &self.store
    }
    /// Access to explicit domain insertions owned by this project.
    pub fn store_mut(&mut self) -> &mut SqliteStore {
        &mut self.store
    }
}

fn validate_name(name: &str) -> Result<(), ProjectError> {
    if name.trim().is_empty() {
        Err(ProjectError::InvalidName)
    } else {
        Ok(())
    }
}
fn io_error(path: &Path, source: std::io::Error) -> ProjectError {
    ProjectError::Io {
        path: path.to_owned(),
        source,
    }
}
fn canonical_root(path: &Path) -> Result<PathBuf, ProjectError> {
    let root = fs::canonicalize(path).map_err(|source| io_error(path, source))?;
    if !root.is_dir() {
        return Err(ProjectError::InvalidRoot(root));
    }
    Ok(root)
}

#[cfg(test)]
mod tests;
