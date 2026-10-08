//! Desktop-only new-folder creation, separate from initializing an existing root.
use crate::error::IpcError;
use std::{
    fs,
    io::ErrorKind,
    path::{Component, Path, PathBuf},
};

pub(crate) fn destination(parent: &Path, folder: &str) -> Result<PathBuf, IpcError> {
    validate_folder(folder)?;
    let parent = parent.canonicalize().map_err(|error| IpcError {
        code: "invalid_parent",
        message: "Choose an existing parent folder.".into(),
        detail: Some(error.to_string()),
    })?;
    if !parent.is_dir() {
        return Err(IpcError::new(
            "invalid_parent",
            "Choose an existing parent folder.",
        ));
    }
    let destination = parent.join(folder);
    if destination.parent() != Some(parent.as_path()) {
        return Err(IpcError::new(
            "invalid_folder",
            "Folder must be a single directory name.",
        ));
    }
    Ok(destination)
}

fn validate_folder(folder: &str) -> Result<(), IpcError> {
    let mut components = Path::new(folder).components();
    let single =
        matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none();
    let stem = folder
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ["COM", "LPT"].iter().any(|prefix| {
            stem.strip_prefix(prefix).is_some_and(|suffix| {
                suffix.len() == 1 && matches!(suffix.as_bytes()[0], b'1'..=b'9')
            })
        });
    if !single
        || folder.trim().is_empty()
        || folder.ends_with([' ', '.'])
        || folder
            .chars()
            .any(|c| c.is_control() || "/\\:*?\"<>|".contains(c))
        || reserved
    {
        return Err(IpcError::new(
            "invalid_folder",
            "Use a single portable folder name without reserved names, separators, or trailing spaces/dots.",
        ));
    }
    Ok(())
}

pub(crate) fn create_directory(path: &Path) -> Result<(), IpcError> {
    fs::create_dir(path).map_err(|error| IpcError {
        code: if error.kind() == ErrorKind::AlreadyExists {
            "destination_exists"
        } else {
            "create_directory"
        },
        message: if error.kind() == ErrorKind::AlreadyExists {
            "That folder already exists. Choose another folder name or open it instead."
        } else {
            "The project folder could not be created. Check its location and permissions."
        }
        .into(),
        detail: Some(error.to_string()),
    })
}
