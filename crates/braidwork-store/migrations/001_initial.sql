CREATE TABLE resources (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    provider TEXT NOT NULL,
    access_mode TEXT NOT NULL CHECK (access_mode IN ('manual', 'api', 'harness', 'local')),
    scarcity TEXT NOT NULL CHECK (scarcity IN ('abundant', 'normal', 'scarce', 'critical')),
    status TEXT NOT NULL CHECK (status IN ('available', 'unavailable', 'exhausted'))
) STRICT;

CREATE TABLE tasks (
    id TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL,
    objective TEXT NOT NULL,
    parent TEXT REFERENCES tasks(id),
    status TEXT NOT NULL CHECK (status IN ('pending', 'in_progress', 'completed')),
    CHECK (parent IS NULL OR parent <> id)
) STRICT;

CREATE TABLE task_dependencies (
    task_id TEXT NOT NULL REFERENCES tasks(id),
    dependency_id TEXT NOT NULL REFERENCES tasks(id),
    PRIMARY KEY (task_id, dependency_id),
    CHECK (task_id <> dependency_id)
) STRICT;

CREATE TABLE capsules (
    id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL REFERENCES tasks(id),
    role TEXT NOT NULL,
    objective TEXT NOT NULL,
    max_estimated_tokens TEXT NOT NULL,
    inputs TEXT NOT NULL,
    constraints TEXT NOT NULL,
    acceptance_criteria TEXT NOT NULL,
    expected_outputs TEXT NOT NULL,
    UNIQUE (id, task_id)
) STRICT;

CREATE TABLE artifacts (
    id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL REFERENCES tasks(id),
    kind TEXT NOT NULL CHECK (kind IN ('patch', 'analysis', 'finding', 'test_result', 'documentation', 'question', 'text', 'data')),
    content_ref TEXT NOT NULL,
    media_type TEXT,
    size_bytes TEXT,
    UNIQUE (id, task_id)
) STRICT;

CREATE TABLE receipts (
    id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL REFERENCES tasks(id),
    capsule_id TEXT NOT NULL,
    resource_id TEXT NOT NULL REFERENCES resources(id),
    model_id TEXT,
    access_mode TEXT NOT NULL CHECK (access_mode IN ('manual', 'api', 'harness', 'local')),
    execution TEXT NOT NULL,
    verification TEXT NOT NULL,
    usage TEXT NOT NULL,
    UNIQUE (id, task_id),
    FOREIGN KEY (capsule_id, task_id) REFERENCES capsules(id, task_id)
) STRICT;

CREATE TABLE receipt_artifacts (
    receipt_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    position INTEGER NOT NULL CHECK (position >= 0),
    artifact_id TEXT NOT NULL,
    PRIMARY KEY (receipt_id, position),
    FOREIGN KEY (receipt_id, task_id) REFERENCES receipts(id, task_id),
    FOREIGN KEY (artifact_id, task_id) REFERENCES artifacts(id, task_id)
) STRICT;
