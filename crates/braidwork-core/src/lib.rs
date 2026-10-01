//! Core domain model for Braidwork.
//!
//! Braidwork coordinates heterogeneous AI resources around a shared project
//! while minimizing the context and scarce intelligence consumed per task.
//! The project owns canonical state; capsules are portable views of that state,
//! artifacts reference meaningful outputs, and receipts record execution evidence.
//!
//! Entity identifiers are distinct types, so references cannot be interchanged:
//!
//! ```compile_fail
//! use braidwork_core::id::{ResourceId, TaskId};
//!
//! fn select_resource(_id: ResourceId) {}
//! select_resource(TaskId::new("task-1").unwrap());
//! ```
//!
//! Model identity is distinct from the resource providing access, in both directions:
//!
//! ```compile_fail
//! use braidwork_core::id::{ModelId, ResourceId};
//!
//! fn record_model(_id: ModelId) {}
//! record_model(ResourceId::new("resource-1").unwrap());
//! ```
//!
//! ```compile_fail
//! use braidwork_core::id::{ModelId, ResourceId};
//!
//! fn select_resource(_id: ResourceId) {}
//! select_resource(ModelId::new("model-1").unwrap());
//! ```

pub mod artifact;
pub mod capsule;
pub mod id;
pub mod receipt;
pub mod resource;
pub mod task;

#[cfg(test)]
mod tests;
