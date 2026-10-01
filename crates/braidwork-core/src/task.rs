//! Units of project work with local dependency invariants, without a graph engine.

use std::{collections::BTreeSet, error::Error, fmt};

use serde::{Deserialize, Serialize};

use crate::id::TaskId;

/// The minimal task lifecycle needed to track manual work.
///
/// Pending work can start, and work in progress can complete after acceptance.
/// Failed or rejected attempts belong in receipts; the task can remain open for
/// another attempt. Readiness, scheduling, and transition enforcement are deferred.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Work has not started or is awaiting another attempt.
    Pending,
    /// Work is underway, including any verification still required.
    InProgress,
    /// The task's result has been accepted.
    Completed,
}

/// A task relationship would refer directly to the task itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaskError {
    /// A task cannot depend directly on itself.
    SelfDependency(TaskId),
    /// A task cannot be its own parent.
    SelfParent(TaskId),
}

impl fmt::Display for TaskError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SelfDependency(id) => write!(formatter, "task {id} cannot depend on itself"),
            Self::SelfParent(id) => write!(formatter, "task {id} cannot be its own parent"),
        }
    }
}

impl Error for TaskError {}

/// A unit of work owned by the project, with typed references to other tasks.
///
/// Dependencies are a sorted set: repeated references are collapsed and their
/// serialization is deterministic. Construction and deserialization reject
/// direct self-references. Cross-task cycles and reference existence require
/// project-wide knowledge and are intentionally not checked here.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "TaskData")]
pub struct Task {
    id: TaskId,
    title: String,
    objective: String,
    parent: Option<TaskId>,
    dependencies: BTreeSet<TaskId>,
    status: TaskStatus,
}

impl Task {
    /// Creates a pending task, collapsing duplicate dependencies.
    ///
    /// # Errors
    /// Returns [`TaskError`] if the task depends on itself or is its own parent.
    pub fn new(
        id: TaskId,
        title: impl Into<String>,
        objective: impl Into<String>,
        parent: Option<TaskId>,
        dependencies: impl IntoIterator<Item = TaskId>,
    ) -> Result<Self, TaskError> {
        let dependencies: BTreeSet<_> = dependencies.into_iter().collect();
        if dependencies.contains(&id) {
            return Err(TaskError::SelfDependency(id));
        }
        if parent.as_ref() == Some(&id) {
            return Err(TaskError::SelfParent(id));
        }
        Ok(Self {
            id,
            title: title.into(),
            objective: objective.into(),
            parent,
            dependencies,
            status: TaskStatus::Pending,
        })
    }

    /// Returns this task's immutable identity.
    #[must_use]
    pub fn id(&self) -> &TaskId {
        &self.id
    }

    /// Returns the human-readable title.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Returns the intended result of the work.
    #[must_use]
    pub fn objective(&self) -> &str {
        &self.objective
    }

    /// Returns the optional enclosing task; parenthood does not imply a dependency.
    #[must_use]
    pub fn parent(&self) -> Option<&TaskId> {
        self.parent.as_ref()
    }

    /// Returns unique prerequisites in identifier order.
    #[must_use]
    pub fn dependencies(&self) -> &BTreeSet<TaskId> {
        &self.dependencies
    }

    /// Returns the project's declared task status.
    #[must_use]
    pub fn status(&self) -> TaskStatus {
        self.status
    }

    /// Records a status chosen by the caller; this does not execute or verify work.
    pub fn set_status(&mut self, status: TaskStatus) {
        self.status = status;
    }
}

// Rehydration must pass through the same relationship checks as construction.
#[derive(Deserialize)]
struct TaskData {
    id: TaskId,
    title: String,
    objective: String,
    parent: Option<TaskId>,
    dependencies: BTreeSet<TaskId>,
    status: TaskStatus,
}

impl TryFrom<TaskData> for Task {
    type Error = TaskError;

    fn try_from(data: TaskData) -> Result<Self, Self::Error> {
        let mut task = Self::new(
            data.id,
            data.title,
            data.objective,
            data.parent,
            data.dependencies,
        )?;
        task.set_status(data.status);
        Ok(task)
    }
}
