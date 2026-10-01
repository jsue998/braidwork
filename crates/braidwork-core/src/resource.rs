//! Provider-independent descriptions of usable AI capacity.

use serde::{Deserialize, Serialize};

use crate::id::ResourceId;

/// How a worker accesses a resource, without prescribing any integration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessMode {
    /// A person transfers the capsule and result, for example via copy/paste.
    Manual,
    /// An officially supported programmatic API.
    Api,
    /// An officially supported CLI or execution harness.
    Harness,
    /// A locally available inference resource.
    Local,
}

/// A declared scarcity category, not a measured quota or scheduler score.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scarcity {
    /// Capacity can be used freely for routine work.
    Abundant,
    /// Ordinary capacity with no exceptional scarcity.
    Normal,
    /// Capacity should be conserved for valuable work.
    Scarce,
    /// Capacity is extremely limited.
    Critical,
}

/// The resource's last declared availability; the core does not refresh it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceStatus {
    /// The resource can currently receive work.
    Available,
    /// The resource cannot currently be used.
    Unavailable,
    /// The resource's usable allowance has been exhausted.
    Exhausted,
}

/// Access to usable AI capacity, such as an account, subscription, or local runtime.
///
/// A resource is not a model: it describes access and availability, and may expose
/// different models over time. Provider names are labels, not integration types.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resource {
    /// Identity of this particular resource.
    pub id: ResourceId,
    /// Human-readable name.
    pub name: String,
    /// Provider name or identifier, interpreted only as a label by the core.
    pub provider: String,
    /// Declared means of accessing the resource.
    pub access_mode: AccessMode,
    /// Declared scarcity of its usable capacity.
    pub scarcity: Scarcity,
    /// Last declared availability.
    pub status: ResourceStatus,
}
