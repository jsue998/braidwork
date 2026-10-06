use braidwork_core::{
    artifact::Artifact,
    capsule::{ContextBudget, TaskCapsule},
    id::{ArtifactId, CapsuleId, InvalidId, ModelId, ReceiptId, ResourceId, TaskId},
    receipt::Receipt,
    resource::Resource,
    task::Task,
};
use rusqlite::{Connection, Row, params};

use crate::{
    EntityId, ReconstructionError, SqliteStore, StoreError,
    codec::{
        column, enum_column, enum_text, id, json, optional_id, optional_unsigned, read, unsigned,
    },
    error::{insertion_error, integrity_error},
};

impl SqliteStore {
    /// Inserts an AI resource without replacing an existing record.
    ///
    /// # Errors
    /// Returns [`StoreError::AlreadyExists`] for a duplicate ID, or a database,
    /// integrity, or serialization error if persistence fails.
    pub fn insert_resource(&mut self, resource: &Resource) -> Result<(), StoreError> {
        self.connection
            .execute(
                "INSERT INTO resources (id, name, provider, access_mode, scarcity, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    resource.id.as_str(),
                    resource.name,
                    resource.provider,
                    enum_text(resource.access_mode)?,
                    enum_text(resource.scarcity)?,
                    enum_text(resource.status)?
                ],
            )
            .map_err(|error| insertion_error(error, EntityId::Resource(resource.id.clone())))?;
        Ok(())
    }

    /// Reads a resource by its typed identity.
    ///
    /// # Errors
    /// Returns [`StoreError::NotFound`] if absent, or a database/reconstruction
    /// error if querying or validating persisted data fails.
    pub fn get_resource(&self, resource_id: &ResourceId) -> Result<Resource, StoreError> {
        read(
            &self.connection,
            "SELECT id, name, provider, access_mode, scarcity, status FROM resources WHERE id = ?1",
            resource_id.as_str(),
            EntityId::Resource(resource_id.clone()),
            resource_from_row,
        )
    }

    /// Lists resources in deterministic identifier order.
    ///
    /// # Errors
    /// Returns a database or reconstruction error, using the same validation as
    /// [`Self::get_resource`]. An empty store returns an empty vector.
    pub fn list_resources(&self) -> Result<Vec<Resource>, StoreError> {
        let mut statement = self.connection.prepare(
            "SELECT id, name, provider, access_mode, scarcity, status FROM resources ORDER BY id",
        )?;
        let mut rows = statement.query([])?;
        let mut resources = Vec::new();
        while let Some(row) = rows.next()? {
            resources.push(resource_from_row(row)?);
        }
        Ok(resources)
    }

    /// Lists tasks and their dependencies in one consistent snapshot, by ID.
    ///
    /// # Errors
    /// Returns a database or reconstruction error, using the same validation as
    /// [`Self::get_task`]. An empty store returns an empty vector.
    pub fn list_tasks(&self) -> Result<Vec<Task>, StoreError> {
        let transaction = self.connection.unchecked_transaction()?;
        let tasks = {
            let mut statement = transaction
                .prepare("SELECT id, title, objective, parent, status FROM tasks ORDER BY id")?;
            let mut rows = statement.query([])?;
            let mut tasks = Vec::new();
            while let Some(row) = rows.next()? {
                tasks.push(task_from_row(&transaction, row)?);
            }
            tasks
        };
        transaction.commit()?;
        Ok(tasks)
    }

    /// Atomically inserts a task and its prerequisites.
    ///
    /// Parent and dependency tasks must already exist. Parentage does not imply
    /// a dependency. No global graph traversal or cycle detection is performed.
    ///
    /// # Errors
    /// Returns [`StoreError::AlreadyExists`] for a duplicate ID,
    /// [`StoreError::Integrity`] for missing references, or a database/serialization
    /// error. Every failure rolls back the task and all dependency rows.
    pub fn insert_task(&mut self, task: &Task) -> Result<(), StoreError> {
        let transaction = self.connection.transaction()?;
        transaction.execute(
            "INSERT INTO tasks (id, title, objective, parent, status) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![task.id().as_str(), task.title(), task.objective(), task.parent().map(TaskId::as_str), enum_text(task.status())?],
        ).map_err(|error| insertion_error(error, EntityId::Task(task.id().clone())))?;
        for dependency in task.dependencies() {
            transaction
                .execute(
                    "INSERT INTO task_dependencies (task_id, dependency_id) VALUES (?1, ?2)",
                    params![task.id().as_str(), dependency.as_str()],
                )
                .map_err(integrity_error)?;
        }
        transaction.commit()?;
        Ok(())
    }

    /// Reads a task and its dependencies in a single database snapshot.
    ///
    /// Reconstruction passes through [`Task::new`], then restores recorded status.
    ///
    /// # Errors
    /// Returns [`StoreError::NotFound`] if absent, or a database/reconstruction
    /// error, including invalid persisted identifiers or self-references.
    pub fn get_task(&self, task_id: &TaskId) -> Result<Task, StoreError> {
        let transaction = self.connection.unchecked_transaction()?;
        let task = read(
            &transaction,
            "SELECT id, title, objective, parent, status FROM tasks WHERE id = ?1",
            task_id.as_str(),
            EntityId::Task(task_id.clone()),
            |row| task_from_row(&transaction, row),
        )?;
        transaction.commit()?;
        Ok(task)
    }

    /// Inserts a capsule whose task already exists.
    ///
    /// Selected context and requirements are stored as JSON; the estimated token
    /// budget is stored as decimal text to preserve the domain's full `u64` range.
    ///
    /// # Errors
    /// Returns [`StoreError::AlreadyExists`] for a duplicate ID,
    /// [`StoreError::Integrity`] for a missing task, or a database/serialization error.
    pub fn insert_capsule(&mut self, capsule: &TaskCapsule) -> Result<(), StoreError> {
        insert_capsule_row(&self.connection, capsule)
    }

    /// Reads a portable capsule, validating its IDs and decoding nested domain data.
    ///
    /// # Errors
    /// Returns [`StoreError::NotFound`] if absent, or a database/reconstruction error.
    pub fn get_capsule(&self, capsule_id: &CapsuleId) -> Result<TaskCapsule, StoreError> {
        read(
            &self.connection,
            "SELECT id, task_id, role, objective, max_estimated_tokens, inputs, constraints,
                acceptance_criteria, expected_outputs, mission, instructions FROM capsules WHERE id = ?1",
            capsule_id.as_str(),
            EntityId::Capsule(capsule_id.clone()),
            |row| {
                Ok(TaskCapsule {
                    id: id(row, "id")?,
                    task_id: id(row, "task_id")?,
                    role: column(row, "role")?,
                    mission: column(row, "mission")?,
                    instructions: json(row, "instructions")?,
                    objective: column(row, "objective")?,
                    inputs: json(row, "inputs")?,
                    constraints: json(row, "constraints")?,
                    acceptance_criteria: json(row, "acceptance_criteria")?,
                    expected_outputs: json(row, "expected_outputs")?,
                    context_budget: ContextBudget {
                        max_estimated_tokens: unsigned(row, "max_estimated_tokens")?,
                    },
                })
            },
        )
    }

    /// Inserts artifact metadata whose originating task already exists.
    ///
    /// Content remains an opaque external reference; no bytes or hashes are stored.
    ///
    /// # Errors
    /// Returns [`StoreError::AlreadyExists`] for a duplicate ID,
    /// [`StoreError::Integrity`] for a missing task, or a database/serialization error.
    pub fn insert_artifact(&mut self, artifact: &Artifact) -> Result<(), StoreError> {
        insert_artifact_row(&self.connection, artifact)
    }

    /// Reads artifact metadata, preserving unknown size separately from known zero.
    ///
    /// # Errors
    /// Returns [`StoreError::NotFound`] if absent, or a database/reconstruction error.
    pub fn get_artifact(&self, artifact_id: &ArtifactId) -> Result<Artifact, StoreError> {
        read(
            &self.connection,
            "SELECT id, task_id, kind, content_ref, media_type, size_bytes FROM artifacts WHERE id = ?1",
            artifact_id.as_str(),
            EntityId::Artifact(artifact_id.clone()),
            |row| {
                Ok(Artifact {
                    id: id(row, "id")?,
                    task_id: id(row, "task_id")?,
                    kind: enum_column(row, "kind")?,
                    content_ref: column(row, "content_ref")?,
                    media_type: column(row, "media_type")?,
                    size_bytes: optional_unsigned(row, "size_bytes")?,
                })
            },
        )
    }

    /// Atomically inserts a receipt and its ordered artifact references.
    ///
    /// Task, capsule, resource, and produced artifacts must exist. The capsule and
    /// produced artifacts must belong to the receipt's task. Artifact order and
    /// repetitions are preserved; verification and usage retain their JSON data.
    /// A missing model identity stays unknown and is never inferred from a resource.
    ///
    /// # Errors
    /// Returns [`StoreError::AlreadyExists`] for a duplicate ID,
    /// [`StoreError::Integrity`] for invalid relational references/provenance, or
    /// a database/serialization error. Every failure rolls back the whole receipt.
    pub fn insert_receipt(&mut self, receipt: &Receipt) -> Result<(), StoreError> {
        let transaction = self.connection.transaction()?;
        insert_receipt_row(&transaction, receipt)?;
        transaction.commit()?;
        Ok(())
    }

    /// Reads a receipt and its ordered artifacts in a single database snapshot.
    ///
    /// # Errors
    /// Returns [`StoreError::NotFound`] if absent, or a database/reconstruction
    /// error, including malformed JSON or an invalid optional model ID.
    pub fn get_receipt(&self, receipt_id: &ReceiptId) -> Result<Receipt, StoreError> {
        let transaction = self.connection.unchecked_transaction()?;
        let receipt = read(
            &transaction,
            "SELECT id, task_id, capsule_id, resource_id, model_id, access_mode,
                execution, verification, usage FROM receipts WHERE id = ?1",
            receipt_id.as_str(),
            EntityId::Receipt(receipt_id.clone()),
            |row| {
                let stored_id: ReceiptId = id(row, "id")?;
                let artifacts = related_ids(
                    &transaction,
                    "SELECT artifact_id FROM receipt_artifacts WHERE receipt_id = ?1 ORDER BY position",
                    stored_id.as_str(),
                    "artifact_id",
                )?;
                Ok(Receipt {
                    id: stored_id,
                    task_id: id(row, "task_id")?,
                    capsule_id: id(row, "capsule_id")?,
                    resource_id: id(row, "resource_id")?,
                    model_id: optional_id(row, "model_id")?,
                    access_mode: enum_column(row, "access_mode")?,
                    artifacts,
                    execution: json(row, "execution")?,
                    verification: json(row, "verification")?,
                    usage: json(row, "usage")?,
                })
            },
        )?;
        transaction.commit()?;
        Ok(receipt)
    }
}

fn related_ids<T: TryFrom<String, Error = InvalidId>>(
    connection: &Connection,
    sql: &str,
    parent_id: &str,
    column_name: &str,
) -> Result<Vec<T>, StoreError> {
    let mut statement = connection.prepare(sql)?;
    let mut rows = statement.query([parent_id])?;
    let mut ids = Vec::new();
    while let Some(row) = rows.next()? {
        ids.push(id(row, column_name)?);
    }
    Ok(ids)
}

fn resource_from_row(row: &Row<'_>) -> Result<Resource, StoreError> {
    Ok(Resource {
        id: id(row, "id")?,
        name: column(row, "name")?,
        provider: column(row, "provider")?,
        access_mode: enum_column(row, "access_mode")?,
        scarcity: enum_column(row, "scarcity")?,
        status: enum_column(row, "status")?,
    })
}

fn task_from_row(connection: &Connection, row: &Row<'_>) -> Result<Task, StoreError> {
    let stored_id: TaskId = id(row, "id")?;
    let dependencies = related_ids(
        connection,
        "SELECT dependency_id FROM task_dependencies WHERE task_id = ?1 ORDER BY dependency_id",
        stored_id.as_str(),
        "dependency_id",
    )?;
    let mut task = Task::new(
        stored_id,
        column::<String>(row, "title")?,
        column::<String>(row, "objective")?,
        optional_id(row, "parent")?,
        dependencies,
    )
    .map_err(ReconstructionError::Task)?;
    task.set_status(enum_column(row, "status")?);
    Ok(task)
}

pub(crate) fn insert_capsule_row(
    connection: &Connection,
    capsule: &TaskCapsule,
) -> Result<(), StoreError> {
    connection
        .execute(
            "INSERT INTO capsules (id, task_id, role, objective, max_estimated_tokens,
                inputs, constraints, acceptance_criteria, expected_outputs, mission, instructions)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                capsule.id.as_str(),
                capsule.task_id.as_str(),
                capsule.role,
                capsule.objective,
                capsule.context_budget.max_estimated_tokens.to_string(),
                serde_json::to_string(&capsule.inputs)?,
                serde_json::to_string(&capsule.constraints)?,
                serde_json::to_string(&capsule.acceptance_criteria)?,
                serde_json::to_string(&capsule.expected_outputs)?,
                capsule.mission,
                serde_json::to_string(&capsule.instructions)?
            ],
        )
        .map_err(|error| insertion_error(error, EntityId::Capsule(capsule.id.clone())))?;
    Ok(())
}

pub(crate) fn insert_artifact_row(
    connection: &Connection,
    artifact: &Artifact,
) -> Result<(), StoreError> {
    connection
        .execute(
            "INSERT INTO artifacts (id, task_id, kind, content_ref, media_type, size_bytes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                artifact.id.as_str(),
                artifact.task_id.as_str(),
                enum_text(artifact.kind)?,
                artifact.content_ref,
                artifact.media_type,
                artifact.size_bytes.map(|value| value.to_string())
            ],
        )
        .map_err(|error| insertion_error(error, EntityId::Artifact(artifact.id.clone())))?;
    Ok(())
}

pub(crate) fn insert_receipt_row(
    connection: &Connection,
    receipt: &Receipt,
) -> Result<(), StoreError> {
    connection
        .execute(
            "INSERT INTO receipts (id, task_id, capsule_id, resource_id, model_id,
                access_mode, execution, verification, usage)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                receipt.id.as_str(),
                receipt.task_id.as_str(),
                receipt.capsule_id.as_str(),
                receipt.resource_id.as_str(),
                receipt.model_id.as_ref().map(ModelId::as_str),
                enum_text(receipt.access_mode)?,
                serde_json::to_string(&receipt.execution)?,
                serde_json::to_string(&receipt.verification)?,
                serde_json::to_string(&receipt.usage)?
            ],
        )
        .map_err(|error| insertion_error(error, EntityId::Receipt(receipt.id.clone())))?;
    for (position, artifact) in (0_i64..).zip(&receipt.artifacts) {
        connection
            .execute(
                "INSERT INTO receipt_artifacts (receipt_id, task_id, position, artifact_id)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    receipt.id.as_str(),
                    receipt.task_id.as_str(),
                    position,
                    artifact.as_str()
                ],
            )
            .map_err(integrity_error)?;
    }
    Ok(())
}
