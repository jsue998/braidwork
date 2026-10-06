//! Historical allocation of a task to a session and a specific agent revision.
use crate::id::{AgentSpecId, AssignmentId, SessionId, TaskId};
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;

/// Minimal manual handoff progress, distinct from execution and verification.
///
/// `Prepared` includes allocation before capsule preparation. Explicit dispatch
/// can follow capsule preparation. Ingestion records `ResultReceived`, which does
/// not mean accepted or task completed. Failures belong to receipt outcomes;
/// verification/completion and retry policies are deferred.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssignmentStatus {
    /// Allocated; its portable description may still need preparation.
    Prepared,
    /// A caller explicitly recorded a manual handoff.
    Dispatched,
    /// A result and receipt have been recorded, without implied verification.
    ResultReceived,
}

/// One allocation of work; it is not the task or an act of delegation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Assignment {
    /// Identity of this allocation.
    pub id: AssignmentId,
    /// Work being assigned.
    pub task_id: TaskId,
    /// Execution instance receiving the work.
    pub session_id: SessionId,
    /// Agent used at assignment time, retained independently of future session edits.
    pub agent_spec_id: AgentSpecId,
    /// Exact historical revision used by this assignment.
    pub agent_spec_revision: NonZeroU32,
    /// Recorded manual handoff progress.
    pub status: AssignmentStatus,
}
