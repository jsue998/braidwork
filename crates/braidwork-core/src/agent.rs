//! Versioned, provider-independent agent definitions.
use crate::id::AgentSpecId;
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;

/// Permission and optional bound on direct delegated assignments.
///
/// A disabled policy permits no delegation. With permission, `None` imposes no
/// direct-child limit and `Some(0)` permits none. No depth or budget policy exists.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DelegationPolicy {
    /// Whether this agent may delegate work.
    pub allowed: bool,
    /// Maximum direct children recorded for each parent assignment, when bounded.
    pub max_children: Option<u32>,
}

/// The role a capacity adopts, separate from a session, task, or model.
///
/// `(id, revision)` identifies an immutable historical definition. Revisions start
/// at one and may coexist. No provider, credentials, or mandatory model is present.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSpec {
    /// Identity shared by revisions of this definition.
    pub id: AgentSpecId,
    /// Positive integer revision; never resolved implicitly to the latest revision.
    pub revision: NonZeroU32,
    /// Human-readable name.
    pub name: String,
    /// Responsibility assumed by a worker.
    pub role: String,
    /// Purpose of this agent, for any domain of work.
    pub mission: String,
    /// Provider-independent working instructions.
    pub instructions: Vec<String>,
    /// Declared specialties, not measured capabilities or routing scores.
    pub expertise: Vec<String>,
    /// Permission to delegate explicit work.
    pub delegation: DelegationPolicy,
}
