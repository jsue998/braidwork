use crate::error::IpcError;
use std::{
    path::PathBuf,
    sync::{Mutex, MutexGuard},
};

#[derive(Default)]
pub(crate) struct ProjectState(Mutex<Option<PathBuf>>);
impl ProjectState {
    pub(crate) fn lock(&self) -> Result<MutexGuard<'_, Option<PathBuf>>, IpcError> {
        self.0.lock().map_err(|_| {
            IpcError::new(
                "state_unavailable",
                "Project access was interrupted. Restart Braidwork before continuing.",
            )
        })
    }
}
