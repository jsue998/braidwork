use crate::Project;
use braidwork_core::id::ArtifactId;
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};
use thiserror::Error;

/// Lazily created directory for local artifact bytes.
pub const ARTIFACT_DIRECTORY: &str = "artifacts";
const HEX: &[u8; 16] = b"0123456789abcdef";
const PREFIX: &str = "artifact:";

/// Local content failures, independent of database metadata persistence.
#[derive(Debug, Error)]
pub enum ArtifactContentError {
    /// The reference is not a supported local filename; absolute/traversal paths fail.
    #[error("invalid local artifact reference: {0}")]
    InvalidReference(String),
    /// Referenced bytes do not exist; no fallback content is invented.
    #[error("artifact content missing: {0}")]
    Missing(String),
    /// Writing must never replace a file already occupying this identity.
    #[error("artifact content already exists: {0}")]
    AlreadyExists(ArtifactId),
    /// Content I/O failure retains the location and original error.
    #[error("artifact content I/O at {path}: {source}")]
    Io {
        /// Local path involved.
        path: PathBuf,
        /// Original filesystem failure.
        #[source]
        source: std::io::Error,
    },
}
impl Project {
    /// Writes new bytes exclusively and returns a portable local content reference.
    ///
    /// The directory is created lazily. Filenames encode ID bytes as lowercase hex,
    /// so arbitrary valid domain IDs cannot become paths. No hashing is performed.
    /// Metadata insertion remains a separate responsibility; ingestion composes it.
    ///
    /// # Errors
    /// Returns [`ArtifactContentError`] for existing content or filesystem failure.
    pub fn write_artifact_content(
        &self,
        id: &ArtifactId,
        bytes: &[u8],
    ) -> Result<String, ArtifactContentError> {
        let directory = self.state_path().join(ARTIFACT_DIRECTORY);
        match fs::create_dir(&directory) {
            Ok(()) => {}
            Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(source) => return Err(io_error(&directory, source)),
        }
        let mut key = String::new();
        for byte in id.as_str().bytes() {
            key.push(char::from(HEX[usize::from(byte >> 4)]));
            key.push(char::from(HEX[usize::from(byte & 15)]));
        }
        let reference = format!("{PREFIX}{key}.bin");
        let path = self.content_path(&reference)?;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|source| {
                if source.kind() == std::io::ErrorKind::AlreadyExists {
                    ArtifactContentError::AlreadyExists(id.clone())
                } else {
                    io_error(&path, source)
                }
            })?;
        let write = file.write_all(bytes).and_then(|()| file.sync_all());
        drop(file);
        if let Err(source) = write {
            // Only our just-created file is eligible for best-effort cleanup.
            let _ = fs::remove_file(&path);
            return Err(io_error(&path, source));
        }
        Ok(reference)
    }
    /// Reads bytes using a portable reference, never an arbitrary filesystem path.
    ///
    /// # Errors
    /// Returns invalid-reference, missing-content, or path-aware I/O errors.
    pub fn read_artifact_content(&self, reference: &str) -> Result<Vec<u8>, ArtifactContentError> {
        let path = self.content_path(reference)?;
        fs::read(&path).map_err(|source| {
            if source.kind() == std::io::ErrorKind::NotFound {
                ArtifactContentError::Missing(reference.to_owned())
            } else {
                io_error(&path, source)
            }
        })
    }
    /// Reads a bounded preview without modifying or truncating canonical content.
    ///
    /// Returns `None` when the actual file exceeds `limit` bytes, independently of
    /// metadata. At most `limit + 1` bytes are read; an empty file is `Some(vec![])`.
    ///
    /// # Errors
    /// Returns invalid-reference, missing-content, or path-aware I/O errors.
    pub fn read_artifact_content_up_to(
        &self,
        reference: &str,
        limit: u64,
    ) -> Result<Option<Vec<u8>>, ArtifactContentError> {
        let path = self.content_path(reference)?;
        let file = fs::File::open(&path).map_err(|source| {
            if source.kind() == std::io::ErrorKind::NotFound {
                ArtifactContentError::Missing(reference.to_owned())
            } else {
                io_error(&path, source)
            }
        })?;
        let mut bytes = Vec::new();
        file.take(limit.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|source| io_error(&path, source))?;
        Ok((bytes.len() as u64 <= limit).then_some(bytes))
    }
    pub(crate) fn remove_new_content(&self, reference: &str) -> Result<(), ArtifactContentError> {
        let path = self.content_path(reference)?;
        fs::remove_file(&path).map_err(|source| io_error(&path, source))
    }
    fn content_path(&self, reference: &str) -> Result<PathBuf, ArtifactContentError> {
        let key = reference
            .strip_prefix(PREFIX)
            .and_then(|value| value.strip_suffix(".bin"));
        let Some(key) = key.filter(|key| {
            !key.is_empty()
                && key.len() % 2 == 0
                && key
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        }) else {
            return Err(ArtifactContentError::InvalidReference(reference.to_owned()));
        };
        Ok(self
            .state_path()
            .join(ARTIFACT_DIRECTORY)
            .join(format!("{key}.bin")))
    }
}
fn io_error(path: &Path, source: std::io::Error) -> ArtifactContentError {
    ArtifactContentError::Io {
        path: path.to_owned(),
        source,
    }
}
