//! Explicit delegation edges, separate from task dependencies and session membership.
use crate::id::{AssignmentId, DelegationId};
use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};

/// An assignment attempted to delegate work directly to itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelfDelegation(pub AssignmentId);
impl fmt::Display for SelfDelegation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "assignment {} cannot delegate to itself", self.0)
    }
}
impl Error for SelfDelegation {}

/// An auditable act of handing work from one assignment to another.
///
/// Arbitrary depth is representable. Local self-reference is rejected, including
/// on deserialization. Existence, permission, and direct-child limits require
/// canonical state; global cycle detection is intentionally deferred.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "DelegationData")]
pub struct Delegation {
    id: DelegationId,
    parent_assignment: AssignmentId,
    child_assignment: AssignmentId,
}
impl Delegation {
    /// Creates an explicit edge without interpreting task dependencies.
    ///
    /// # Errors
    /// Returns [`SelfDelegation`] if parent and child are the same assignment.
    pub fn new(
        id: DelegationId,
        parent_assignment: AssignmentId,
        child_assignment: AssignmentId,
    ) -> Result<Self, SelfDelegation> {
        if parent_assignment == child_assignment {
            return Err(SelfDelegation(parent_assignment));
        }
        Ok(Self {
            id,
            parent_assignment,
            child_assignment,
        })
    }
    /// Identity of this delegation act.
    #[must_use]
    pub fn id(&self) -> &DelegationId {
        &self.id
    }
    /// Assignment handing off work.
    #[must_use]
    pub fn parent_assignment(&self) -> &AssignmentId {
        &self.parent_assignment
    }
    /// Assignment receiving delegated work.
    #[must_use]
    pub fn child_assignment(&self) -> &AssignmentId {
        &self.child_assignment
    }
}
#[derive(Deserialize)]
struct DelegationData {
    id: DelegationId,
    parent_assignment: AssignmentId,
    child_assignment: AssignmentId,
}
impl TryFrom<DelegationData> for Delegation {
    type Error = SelfDelegation;
    fn try_from(data: DelegationData) -> Result<Self, Self::Error> {
        Self::new(data.id, data.parent_assignment, data.child_assignment)
    }
}
