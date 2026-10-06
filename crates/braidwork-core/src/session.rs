//! Persistent execution instances, independent of the external conversation state.
use crate::id::{AgentSpecId, ResourceId, SessionId};
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;

/// Declared availability of an execution instance, not task progress.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    /// Can receive work.
    Ready,
    /// Declared occupied; this is not an automatic concurrency lock.
    Busy,
    /// Retained but currently inactive.
    Dormant,
}

/// A persistent conversation or execution instance using one agent definition.
///
/// Many sessions may share a resource. The opaque external reference identifies
/// an execution surface, never canonical state. Model choice can vary by attempt
/// and belongs in receipts, not permanently in this configuration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    /// Identity of this instance.
    pub id: SessionId,
    /// Human label for locating the instance manually.
    pub label: String,
    /// Capacity through which it is accessed.
    pub resource_id: ResourceId,
    /// Agent definition currently configured for this session.
    pub agent_spec_id: AgentSpecId,
    /// Exact configured revision.
    pub agent_spec_revision: NonZeroU32,
    /// Uninterpreted URL, label, thread identifier, or manual handle.
    pub external_ref: Option<String>,
    /// Declared session availability.
    pub status: SessionStatus,
}
