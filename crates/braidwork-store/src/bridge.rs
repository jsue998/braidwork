use crate::{
    SqliteStore, StoreError,
    codec::id,
    entities::{insert_artifact_row, insert_capsule_row, insert_receipt_row},
    error::integrity_error,
    execution::{assignment_on, session_on, workflow},
};
use braidwork_core::{
    artifact::Artifact,
    assignment::{Assignment, AssignmentStatus},
    capsule::TaskCapsule,
    id::{AssignmentId, CapsuleId, ReceiptId},
    receipt::Receipt,
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

impl SqliteStore {
    /// Atomically inserts and associates the single capsule of a prepared assignment.
    ///
    /// The capsule remains portable and carries no assignment or session identifier.
    /// Preparation cannot replace a previously associated description.
    ///
    /// # Errors
    /// Returns workflow, provenance, duplicate, reconstruction, or database errors.
    pub fn prepare_assignment_capsule(
        &mut self,
        assignment_id: &AssignmentId,
        capsule: &TaskCapsule,
    ) -> Result<(), StoreError> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let assignment = assignment_on(&tx, assignment_id)?;
        if assignment.status != AssignmentStatus::Prepared {
            return Err(workflow(
                assignment_id,
                "capsule preparation requires a prepared assignment",
            ));
        }
        if capsule_id_on(&tx, assignment_id)?.is_some() {
            return Err(workflow(
                assignment_id,
                "a capsule is already prepared; create a new assignment for another attempt",
            ));
        }
        if assignment.task_id != capsule.task_id {
            return Err(StoreError::Provenance(
                "capsule and assignment tasks differ",
            ));
        }
        insert_capsule_row(&tx, capsule)?;
        tx.execute("INSERT INTO assignment_capsules (assignment_id, capsule_id, task_id) VALUES (?1, ?2, ?3)", params![assignment_id.as_str(), capsule.id.as_str(), capsule.task_id.as_str()]).map_err(integrity_error)?;
        tx.commit()?;
        Ok(())
    }
    /// Retrieves the portable description associated with an allocation.
    ///
    /// # Errors
    /// Returns not-found, unprepared-workflow, database, or reconstruction errors.
    pub fn get_assignment_capsule(
        &self,
        assignment_id: &AssignmentId,
    ) -> Result<TaskCapsule, StoreError> {
        let tx = self.connection.unchecked_transaction()?;
        assignment_on(&tx, assignment_id)?;
        let id = capsule_id_on(&tx, assignment_id)?.ok_or_else(|| {
            workflow(
                assignment_id,
                "no capsule prepared; run capsule prepare first",
            )
        })?;
        let capsule = self.get_capsule(&id)?;
        tx.commit()?;
        Ok(capsule)
    }
    /// Explicitly records dispatch after capsule preparation; repeated marks are idempotent.
    ///
    /// Merely rendering or inspecting a dispatch never invokes this operation.
    ///
    /// # Errors
    /// Returns missing-assignment/capsule, invalid workflow, or database errors.
    pub fn mark_assignment_dispatched(
        &mut self,
        assignment_id: &AssignmentId,
    ) -> Result<Assignment, StoreError> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut assignment = assignment_on(&tx, assignment_id)?;
        if assignment.status == AssignmentStatus::ResultReceived {
            return Err(workflow(
                assignment_id,
                "a result has already been received",
            ));
        }
        if capsule_id_on(&tx, assignment_id)?.is_none() {
            return Err(workflow(
                assignment_id,
                "no capsule prepared; run capsule prepare first",
            ));
        }
        tx.execute(
            "UPDATE assignments SET status = 'dispatched' WHERE id = ?1",
            [assignment_id.as_str()],
        )?;
        tx.commit()?;
        assignment.status = AssignmentStatus::Dispatched;
        Ok(assignment)
    }
    /// Atomically records one artifact, receipt, allocation link, and received status.
    ///
    /// The result must match the allocation's task, prepared capsule, and session
    /// resource. Verification and execution outcome are separate recorded facts.
    /// Filesystem content must already exist; this operation only owns database rows.
    ///
    /// # Errors
    /// Returns workflow/provenance, duplicate, serialization, reconstruction, or
    /// database errors. Every database write rolls back together on failure.
    pub fn record_assignment_result(
        &mut self,
        assignment_id: &AssignmentId,
        artifact: &Artifact,
        receipt: &Receipt,
    ) -> Result<Assignment, StoreError> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut assignment = assignment_on(&tx, assignment_id)?;
        if assignment.status == AssignmentStatus::ResultReceived {
            return Err(workflow(
                assignment_id,
                "a result has already been received; create a new assignment for another attempt",
            ));
        }
        let capsule_id = capsule_id_on(&tx, assignment_id)?.ok_or_else(|| {
            workflow(
                assignment_id,
                "no capsule prepared; run capsule prepare first",
            )
        })?;
        let session = session_on(&tx, &assignment.session_id)?;
        if artifact.task_id != assignment.task_id
            || receipt.task_id != assignment.task_id
            || receipt.capsule_id != capsule_id
            || receipt.resource_id != session.resource_id
            || receipt.artifacts.as_slice() != std::slice::from_ref(&artifact.id)
        {
            return Err(StoreError::Provenance(
                "result must match assignment task, capsule, session resource, and supplied artifact",
            ));
        }
        insert_artifact_row(&tx, artifact)?;
        insert_receipt_row(&tx, receipt)?;
        tx.execute("INSERT INTO assignment_receipts (assignment_id, receipt_id, task_id, capsule_id, session_id, resource_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![assignment_id.as_str(), receipt.id.as_str(), receipt.task_id.as_str(), receipt.capsule_id.as_str(), assignment.session_id.as_str(), receipt.resource_id.as_str()]).map_err(integrity_error)?;
        tx.execute(
            "UPDATE assignments SET status = 'result_received' WHERE id = ?1",
            [assignment_id.as_str()],
        )?;
        tx.commit()?;
        assignment.status = AssignmentStatus::ResultReceived;
        Ok(assignment)
    }
    /// Reads the receipt recorded for this allocation, retaining organizational provenance.
    ///
    /// # Errors
    /// Returns missing-assignment, no-result-workflow, or persistence/reconstruction errors.
    pub fn get_assignment_receipt(
        &self,
        assignment_id: &AssignmentId,
    ) -> Result<Receipt, StoreError> {
        self.get_assignment(assignment_id)?;
        let id: Option<ReceiptId> = self
            .connection
            .query_row(
                "SELECT receipt_id FROM assignment_receipts WHERE assignment_id = ?1",
                [assignment_id.as_str()],
                |row| Ok(id(row, "receipt_id")),
            )
            .optional()?
            .transpose()?;
        let id = id.ok_or_else(|| workflow(assignment_id, "no result receipt recorded"))?;
        self.get_receipt(&id)
    }
}
fn capsule_id_on(
    connection: &Connection,
    assignment_id: &AssignmentId,
) -> Result<Option<CapsuleId>, StoreError> {
    connection
        .query_row(
            "SELECT capsule_id FROM assignment_capsules WHERE assignment_id = ?1",
            [assignment_id.as_str()],
            |row| Ok(id(row, "capsule_id")),
        )
        .optional()?
        .transpose()
}
