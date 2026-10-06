use crate::{
    EntityId, ReconstructionError, SqliteStore, StoreError,
    codec::{column, enum_column, enum_text, id, json, read},
    error::insertion_error,
};
use braidwork_core::{
    agent::AgentSpec,
    assignment::{Assignment, AssignmentStatus},
    delegation::Delegation,
    id::{AgentSpecId, AssignmentId, DelegationId, SessionId},
    session::Session,
};
use rusqlite::{Connection, OptionalExtension, Row, TransactionBehavior, params};
use std::num::NonZeroU32;

impl SqliteStore {
    /// Inserts a historical definition without replacing any existing revision.
    ///
    /// # Errors
    /// Returns typed duplicate, serialization, or database errors.
    pub fn insert_agent_spec(&mut self, agent: &AgentSpec) -> Result<(), StoreError> {
        self.connection.execute(
            "INSERT INTO agent_specs (id, revision, name, role, mission, instructions, expertise, delegation) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![agent.id.as_str(), agent.revision.get(), agent.name, agent.role, agent.mission,
                serde_json::to_string(&agent.instructions)?, serde_json::to_string(&agent.expertise)?, serde_json::to_string(&agent.delegation)?],
        ).map_err(|error| insertion_error(error, EntityId::AgentSpec(agent.id.clone(), agent.revision)))?;
        Ok(())
    }
    /// Reads one exact agent revision; never selects a latest revision implicitly.
    ///
    /// # Errors
    /// Returns typed not-found, database, or reconstruction errors.
    pub fn get_agent_spec(
        &self,
        id: &AgentSpecId,
        revision: NonZeroU32,
    ) -> Result<AgentSpec, StoreError> {
        agent_on(&self.connection, id, revision)
    }
    /// Lists historical definitions ordered by ID and then numeric revision.
    ///
    /// # Errors
    /// Returns database or reconstruction errors.
    pub fn list_agent_specs(&self) -> Result<Vec<AgentSpec>, StoreError> {
        list(
            &self.connection,
            "SELECT * FROM agent_specs ORDER BY id, revision",
            agent_row,
        )
    }
    /// Inserts a session; its resource and exact agent revision must exist.
    ///
    /// # Errors
    /// Returns context-bearing missing-reference, duplicate, or persistence errors.
    pub fn insert_session(&mut self, session: &Session) -> Result<(), StoreError> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        require(
            exists(
                &tx,
                "resources",
                session.resource_id.as_str(),
                EntityId::Resource(session.resource_id.clone()),
            ),
            "session resource",
        )?;
        require(
            agent_on(&tx, &session.agent_spec_id, session.agent_spec_revision),
            "session agent revision",
        )?;
        tx.execute(
            "INSERT INTO sessions (id, label, resource_id, agent_spec_id, agent_spec_revision, external_ref, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![session.id.as_str(), session.label, session.resource_id.as_str(), session.agent_spec_id.as_str(), session.agent_spec_revision.get(), session.external_ref, enum_text(session.status)?],
        ).map_err(|error| insertion_error(error, EntityId::Session(session.id.clone())))?;
        tx.commit()?;
        Ok(())
    }
    /// Reads a persistent execution instance by typed ID.
    ///
    /// # Errors
    /// Returns typed not-found, database, or reconstruction errors.
    pub fn get_session(&self, id: &SessionId) -> Result<Session, StoreError> {
        session_on(&self.connection, id)
    }
    /// Lists sessions in identifier order, allowing many per resource.
    ///
    /// # Errors
    /// Returns database or reconstruction errors.
    pub fn list_sessions(&self) -> Result<Vec<Session>, StoreError> {
        list(
            &self.connection,
            "SELECT * FROM sessions ORDER BY id",
            session_row,
        )
    }
    /// Inserts a prepared allocation with the session's exact configured agent revision.
    ///
    /// Checks configuration at insertion time, not through a historical FK to the
    /// mutable session configuration. No override, scheduling, or automatic work occurs.
    ///
    /// # Errors
    /// Returns missing-reference, agent mismatch, workflow, duplicate, or persistence errors.
    pub fn insert_assignment(&mut self, assignment: &Assignment) -> Result<(), StoreError> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        require(
            exists(
                &tx,
                "tasks",
                assignment.task_id.as_str(),
                EntityId::Task(assignment.task_id.clone()),
            ),
            "assignment task",
        )?;
        let session = require(
            session_on(&tx, &assignment.session_id),
            "assignment session",
        )?;
        require(
            agent_on(
                &tx,
                &assignment.agent_spec_id,
                assignment.agent_spec_revision,
            ),
            "assignment agent revision",
        )?;
        if session.agent_spec_id != assignment.agent_spec_id
            || session.agent_spec_revision != assignment.agent_spec_revision
        {
            return Err(StoreError::AgentMismatch(assignment.id.clone()));
        }
        if assignment.status != AssignmentStatus::Prepared {
            return Err(workflow(&assignment.id, "new assignments must be prepared"));
        }
        tx.execute("INSERT INTO assignments (id, task_id, session_id, agent_spec_id, agent_spec_revision, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![assignment.id.as_str(), assignment.task_id.as_str(), assignment.session_id.as_str(), assignment.agent_spec_id.as_str(), assignment.agent_spec_revision.get(), enum_text(assignment.status)?],
        ).map_err(|error| insertion_error(error, EntityId::Assignment(assignment.id.clone())))?;
        tx.commit()?;
        Ok(())
    }
    /// Reads a historical allocation without substituting the current session agent.
    ///
    /// # Errors
    /// Returns typed not-found, database, or reconstruction errors.
    pub fn get_assignment(&self, id: &AssignmentId) -> Result<Assignment, StoreError> {
        assignment_on(&self.connection, id)
    }
    /// Lists allocations in identifier order.
    ///
    /// # Errors
    /// Returns database or reconstruction errors.
    pub fn list_assignments(&self) -> Result<Vec<Assignment>, StoreError> {
        list(
            &self.connection,
            "SELECT * FROM assignments ORDER BY id",
            assignment_row,
        )
    }
    /// Records an explicit edge under the parent's historical delegation policy.
    ///
    /// Permission and direct-child count are checked in the insertion transaction.
    /// No graph-depth bound or global cycle detection is imposed.
    ///
    /// # Errors
    /// Returns missing-reference, duplicate, policy denial, or persistence errors.
    pub fn insert_delegation(&mut self, edge: &Delegation) -> Result<(), StoreError> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        // Identify duplicate IDs before checking a policy that may now be exhausted.
        let duplicate: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM delegations WHERE id = ?1)",
            [edge.id().as_str()],
            |row| row.get(0),
        )?;
        if duplicate {
            return Err(StoreError::AlreadyExists {
                entity: EntityId::Delegation(edge.id().clone()),
            });
        }
        let parent = require(
            assignment_on(&tx, edge.parent_assignment()),
            "delegation parent assignment",
        )?;
        require(
            assignment_on(&tx, edge.child_assignment()),
            "delegation child assignment",
        )?;
        let agent = agent_on(&tx, &parent.agent_spec_id, parent.agent_spec_revision)?;
        let count: i64 = tx.query_row(
            "SELECT count(*) FROM delegations WHERE parent_assignment = ?1",
            [parent.id.as_str()],
            |row| row.get(0),
        )?;
        let repeated: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM delegations WHERE parent_assignment = ?1 AND child_assignment = ?2)", params![parent.id.as_str(), edge.child_assignment().as_str()], |row| row.get(0))?;
        if repeated {
            return Err(StoreError::Provenance("delegation edge already recorded"));
        }
        if !agent.delegation.allowed
            || agent
                .delegation
                .max_children
                .is_some_and(|limit| count >= i64::from(limit))
        {
            return Err(StoreError::DelegationDenied(parent.id));
        }
        tx.execute(
            "INSERT INTO delegations (id, parent_assignment, child_assignment) VALUES (?1, ?2, ?3)",
            params![
                edge.id().as_str(),
                edge.parent_assignment().as_str(),
                edge.child_assignment().as_str()
            ],
        )
        .map_err(|error| insertion_error(error, EntityId::Delegation(edge.id().clone())))?;
        tx.commit()?;
        Ok(())
    }
    /// Reads an edge, applying core self-delegation validation.
    ///
    /// # Errors
    /// Returns typed not-found, database, or reconstruction errors.
    pub fn get_delegation(&self, id: &DelegationId) -> Result<Delegation, StoreError> {
        read(
            &self.connection,
            "SELECT * FROM delegations WHERE id = ?1",
            id.as_str(),
            EntityId::Delegation(id.clone()),
            delegation_row,
        )
    }
    /// Lists delegation acts in identifier order.
    ///
    /// # Errors
    /// Returns database or reconstruction errors.
    pub fn list_delegations(&self) -> Result<Vec<Delegation>, StoreError> {
        list(
            &self.connection,
            "SELECT * FROM delegations ORDER BY id",
            delegation_row,
        )
    }
}

pub(crate) fn require<T>(
    value: Result<T, StoreError>,
    relation: &'static str,
) -> Result<T, StoreError> {
    value.map_err(|error| match error {
        StoreError::NotFound { entity } => StoreError::MissingReference { relation, entity },
        other => other,
    })
}
pub(crate) fn workflow(id: &AssignmentId, reason: &'static str) -> StoreError {
    StoreError::Workflow {
        assignment: id.clone(),
        reason,
    }
}
fn exists(
    connection: &Connection,
    table: &str,
    id: &str,
    entity: EntityId,
) -> Result<(), StoreError> {
    // Table names come exclusively from fixed internal call sites, never user input.
    read(
        connection,
        &format!("SELECT id FROM {table} WHERE id = ?1"),
        id,
        entity,
        |_| Ok(()),
    )
}
fn revision(row: &Row<'_>, name: &str) -> Result<NonZeroU32, StoreError> {
    let value = column(row, name)?;
    NonZeroU32::new(value).ok_or_else(|| ReconstructionError::Revision(value).into())
}
pub(crate) fn agent_on(
    connection: &Connection,
    id: &AgentSpecId,
    revision: NonZeroU32,
) -> Result<AgentSpec, StoreError> {
    connection
        .query_row(
            "SELECT * FROM agent_specs WHERE id = ?1 AND revision = ?2",
            params![id.as_str(), revision.get()],
            |row| Ok(agent_row(row)),
        )
        .optional()?
        .ok_or_else(|| StoreError::NotFound {
            entity: EntityId::AgentSpec(id.clone(), revision),
        })?
}
pub(crate) fn session_on(connection: &Connection, id: &SessionId) -> Result<Session, StoreError> {
    read(
        connection,
        "SELECT * FROM sessions WHERE id = ?1",
        id.as_str(),
        EntityId::Session(id.clone()),
        session_row,
    )
}
pub(crate) fn assignment_on(
    connection: &Connection,
    id: &AssignmentId,
) -> Result<Assignment, StoreError> {
    read(
        connection,
        "SELECT * FROM assignments WHERE id = ?1",
        id.as_str(),
        EntityId::Assignment(id.clone()),
        assignment_row,
    )
}
fn agent_row(row: &Row<'_>) -> Result<AgentSpec, StoreError> {
    Ok(AgentSpec {
        id: id(row, "id")?,
        revision: revision(row, "revision")?,
        name: column(row, "name")?,
        role: column(row, "role")?,
        mission: column(row, "mission")?,
        instructions: json(row, "instructions")?,
        expertise: json(row, "expertise")?,
        delegation: json(row, "delegation")?,
    })
}
fn session_row(row: &Row<'_>) -> Result<Session, StoreError> {
    Ok(Session {
        id: id(row, "id")?,
        label: column(row, "label")?,
        resource_id: id(row, "resource_id")?,
        agent_spec_id: id(row, "agent_spec_id")?,
        agent_spec_revision: revision(row, "agent_spec_revision")?,
        external_ref: column(row, "external_ref")?,
        status: enum_column(row, "status")?,
    })
}
fn assignment_row(row: &Row<'_>) -> Result<Assignment, StoreError> {
    Ok(Assignment {
        id: id(row, "id")?,
        task_id: id(row, "task_id")?,
        session_id: id(row, "session_id")?,
        agent_spec_id: id(row, "agent_spec_id")?,
        agent_spec_revision: revision(row, "agent_spec_revision")?,
        status: enum_column(row, "status")?,
    })
}
fn delegation_row(row: &Row<'_>) -> Result<Delegation, StoreError> {
    Delegation::new(
        id(row, "id")?,
        id(row, "parent_assignment")?,
        id(row, "child_assignment")?,
    )
    .map_err(|error| ReconstructionError::Delegation(error).into())
}
fn list<T>(
    connection: &Connection,
    sql: &str,
    decode: fn(&Row<'_>) -> Result<T, StoreError>,
) -> Result<Vec<T>, StoreError> {
    let mut statement = connection.prepare(sql)?;
    let mut rows = statement.query([])?;
    let mut result = Vec::new();
    while let Some(row) = rows.next()? {
        result.push(decode(row)?);
    }
    Ok(result)
}
