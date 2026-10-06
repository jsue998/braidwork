ALTER TABLE capsules ADD COLUMN mission TEXT NOT NULL DEFAULT '';
ALTER TABLE capsules ADD COLUMN instructions TEXT NOT NULL DEFAULT '[]';

CREATE TABLE agent_specs (
    id TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK (revision BETWEEN 1 AND 4294967295),
    name TEXT NOT NULL,
    role TEXT NOT NULL,
    mission TEXT NOT NULL,
    instructions TEXT NOT NULL,
    expertise TEXT NOT NULL,
    delegation TEXT NOT NULL,
    PRIMARY KEY (id, revision)
) STRICT;

CREATE TABLE sessions (
    id TEXT PRIMARY KEY NOT NULL,
    label TEXT NOT NULL,
    resource_id TEXT NOT NULL REFERENCES resources(id),
    agent_spec_id TEXT NOT NULL,
    agent_spec_revision INTEGER NOT NULL,
    external_ref TEXT,
    status TEXT NOT NULL CHECK (status IN ('ready', 'busy', 'dormant')),
    UNIQUE (id, resource_id),
    FOREIGN KEY (agent_spec_id, agent_spec_revision) REFERENCES agent_specs(id, revision)
) STRICT;

CREATE TABLE assignments (
    id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL REFERENCES tasks(id),
    session_id TEXT NOT NULL REFERENCES sessions(id),
    agent_spec_id TEXT NOT NULL,
    agent_spec_revision INTEGER NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('prepared', 'dispatched', 'result_received')),
    UNIQUE (id, task_id),
    UNIQUE (id, session_id, task_id),
    FOREIGN KEY (agent_spec_id, agent_spec_revision) REFERENCES agent_specs(id, revision)
) STRICT;

-- Check the session configuration at insertion time, without tying history to
-- future configuration changes through a permanent composite foreign key.
CREATE TRIGGER assignment_agent_matches_session BEFORE INSERT ON assignments
WHEN EXISTS (SELECT 1 FROM sessions WHERE id = NEW.session_id)
AND NOT EXISTS (
    SELECT 1 FROM sessions WHERE id = NEW.session_id
    AND agent_spec_id = NEW.agent_spec_id AND agent_spec_revision = NEW.agent_spec_revision
)
BEGIN
    SELECT RAISE(ABORT, 'assignment agent revision differs from session');
END;

CREATE TABLE delegations (
    id TEXT PRIMARY KEY NOT NULL,
    parent_assignment TEXT NOT NULL REFERENCES assignments(id),
    child_assignment TEXT NOT NULL REFERENCES assignments(id),
    UNIQUE (parent_assignment, child_assignment),
    CHECK (parent_assignment <> child_assignment)
) STRICT;

CREATE TABLE assignment_capsules (
    assignment_id TEXT PRIMARY KEY NOT NULL,
    capsule_id TEXT NOT NULL UNIQUE,
    task_id TEXT NOT NULL,
    UNIQUE (assignment_id, capsule_id, task_id),
    FOREIGN KEY (assignment_id, task_id) REFERENCES assignments(id, task_id),
    FOREIGN KEY (capsule_id, task_id) REFERENCES capsules(id, task_id)
) STRICT;

CREATE UNIQUE INDEX receipt_execution_identity ON receipts (id, task_id, capsule_id, resource_id);
CREATE TABLE assignment_receipts (
    assignment_id TEXT PRIMARY KEY NOT NULL,
    receipt_id TEXT NOT NULL UNIQUE,
    task_id TEXT NOT NULL,
    capsule_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    resource_id TEXT NOT NULL,
    FOREIGN KEY (assignment_id, session_id, task_id) REFERENCES assignments(id, session_id, task_id),
    FOREIGN KEY (assignment_id, capsule_id, task_id) REFERENCES assignment_capsules(assignment_id, capsule_id, task_id),
    FOREIGN KEY (session_id, resource_id) REFERENCES sessions(id, resource_id),
    FOREIGN KEY (receipt_id, task_id, capsule_id, resource_id) REFERENCES receipts(id, task_id, capsule_id, resource_id)
) STRICT;
