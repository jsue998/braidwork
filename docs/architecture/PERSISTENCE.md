# Canonical persistence

`braidwork-store` owns synchronous SQLite persistence of the existing core
entities. `SqliteStore` owns one private connection to either a caller-supplied
file or an in-memory database. SQLite provides local persistence, relational
constraints, and transactions without a server or asynchronous runtime. The
bundled SQLite build provides a consistent engine supporting STRICT tables.

The dependency remains `store → core`. Core contains no database concepts;
store returns actual core entities and accepts their typed IDs. Artifact content
remains external: only its metadata/reference is persisted.

```text
resources ──────────────────────────────────┐
                                           ▼
tasks ◄── parent                         receipts
  ▲                                        │ ▲
  ├── task_dependencies (task/prerequisite) │ │
  ├── capsules ◄───────────────────────────┘ │
  ├── artifacts ◄── receipt_artifacts ────────┘
  └── receipts.task_id

receipt capsule/produced-artifact task IDs must match receipts.task_id

agent_specs (id, revision) ◄── sessions ──► resources
           ▲                     ▲
           └──── assignments ─────┘
                    │ └──► tasks
                    ├── assignment_capsules ──► capsules
                    ├── assignment_receipts ──► receipts
                    └── delegations ──► assignments (parent/child)
```

## Initial schema and schema v2

The exact DDL lives in
[`001_initial.sql`](../../crates/braidwork-store/migrations/001_initial.sql).
It remains unchanged. Its seven tables are retained in v2. Entity IDs, labels,
enum names, content references,
and JSON use TEXT. Entity primary keys explicitly reject NULL.

| Table | Columns | Keys and relationships |
| --- | --- | --- |
| `resources` | `id`, `name`, `provider`, `access_mode`, `scarcity`, `status` | PK `id` |
| `tasks` | `id`, `title`, `objective`, nullable `parent`, `status` | PK `id`; parent FK to tasks; no self-parent |
| `task_dependencies` | `task_id`, `dependency_id` | Composite PK; both FKs to tasks; no self-dependency |
| `capsules` | `id`, `task_id`, `role`, `objective`, `max_estimated_tokens`, `inputs`, `constraints`, `acceptance_criteria`, `expected_outputs`; v2 adds `mission`, `instructions` | PK `id`; task FK; UNIQUE `(id, task_id)` |
| `artifacts` | `id`, `task_id`, `kind`, `content_ref`, nullable `media_type`, nullable `size_bytes` | PK `id`; task FK; UNIQUE `(id, task_id)` |
| `receipts` | `id`, `task_id`, `capsule_id`, `resource_id`, nullable `model_id`, `access_mode`, `execution`, `verification`, `usage` | PK `id`; task/resource FKs; composite capsule/task FK; UNIQUE `(id, task_id)` |
| `receipt_artifacts` | `receipt_id`, `task_id`, `position`, `artifact_id` | PK `(receipt_id, position)`; composite receipt/task and artifact/task FKs |

[`002_execution_model.sql`](../../crates/braidwork-store/migrations/002_execution_model.sql)
adds six STRICT tables, bringing the total to thirteen:

| Table | Exact columns | Keys and relationships |
| --- | --- | --- |
| `agent_specs` | `id`, `revision`, `name`, `role`, `mission`, `instructions`, `expertise`, `delegation` | PK `(id, revision)`; positive revision through `u32::MAX` |
| `sessions` | `id`, `label`, `resource_id`, `agent_spec_id`, `agent_spec_revision`, nullable `external_ref`, `status` | PK `id`; resource FK; composite agent/revision FK; UNIQUE `(id, resource_id)` |
| `assignments` | `id`, `task_id`, `session_id`, `agent_spec_id`, `agent_spec_revision`, `status` | PK `id`; task/session FKs; composite agent/revision FK; UNIQUE `(id, task_id)` and `(id, session_id, task_id)` |
| `delegations` | `id`, `parent_assignment`, `child_assignment` | PK `id`; both assignment FKs; UNIQUE parent/child pair; CHECK against self-delegation |
| `assignment_capsules` | `assignment_id`, `capsule_id`, `task_id` | PK assignment; UNIQUE capsule and `(assignment_id, capsule_id, task_id)`; composite assignment/task and capsule/task FKs |
| `assignment_receipts` | `assignment_id`, `receipt_id`, `task_id`, `capsule_id`, `session_id`, `resource_id` | PK assignment; UNIQUE receipt; composite assignment/session/task, assignment/capsule/task, session/resource, and receipt/task/capsule/resource FKs |

All new columns are NOT NULL except external_ref. Revisions are INTEGER; all other
new columns use TEXT. Session and assignment states have CHECK constraints using
core snake_case labels. Agent instructions/expertise/delegation and capsule
instructions use JSON. Legacy capsules receive empty mission and `[]` instructions.
The new unique receipt index covers `(id, task_id, capsule_id, resource_id)` for
the association FK. No resource-to-session uniqueness or cascading deletes exist.

An assignment INSERT trigger checks that its exact agent revision matches the
session configuration at that moment. It is deliberately not a permanent FK
to session configuration: historical allocations must survive future agent
configuration changes. Task, session, and agent definition FKs remain permanent.

`receipt_artifacts.position` is a nonnegative INTEGER. It preserves the domain
vector's order and repetitions; the domain imposes no uniqueness constraint on
produced artifact IDs. Task dependencies are instead a unique, sorted set.

`max_estimated_tokens` and nullable `size_bytes` use decimal TEXT, not INTEGER:
SQLite integers are signed 64-bit, while these core values permit the entire
`u64` range. Reads parse them with checked `u64` conversion. This avoids domain
changes, truncation, or floating-point loss. SQL NULL preserves unknown size,
and decimal `0` preserves known zero. No additional numeric budget policy exists.

Enum columns use the domain's snake_case labels and SQL CHECK constraints.
Capsule context/requirements and receipt execution/verification/usage use JSON;
the entities themselves are not opaque JSON blobs. Nested JSON preserves vector
order, optional measurements, explicit currency and integer monetary millionths.
Usage and model identity are never inferred: unknown remains distinct from zero
or a known model. There is no models table or model-ID foreign key.

## Opening and migrations

`SqliteStore::open(path)` and `open_in_memory()` enable `PRAGMA foreign_keys = ON`
on their own connection before migration. No directory layout is created.
`schema_version()` reads persisted `PRAGMA user_version`; `SCHEMA_VERSION` defines
the expected version once, in `src/migrations.rs`.

Migration acquires an IMMEDIATE transaction before reading the version. Version
0 applies migrations 001 and 002; version 1 applies only 002; version 2 needs no
DDL. `SCHEMA_VERSION = 2` is written after successful migration in that transaction.
Future unsupported versions fail explicitly. A failed
migration rolls back both schema changes and the version. There is no replacement
DDL, downgrade, or migration framework. Schema labels and JSON representations
belong to this version; incompatible format changes will need a migration.

## API, integrity, and atomicity

The public API exposes `insert_resource`, `insert_task`, `insert_capsule`,
`insert_artifact`, `insert_receipt`, and the corresponding `get_*` methods. `list_resources` and `list_tasks` return
validated core entities in deterministic ID order; compound task listing reads
all rows and dependencies within one database snapshot.
Insertions take domain entities by reference; getters take the matching typed ID
and return a domain entity or a typed error. Execution entities add
`insert_agent_spec`, `get_agent_spec(id, revision)`, `list_agent_specs`;
`insert/get/list_session`, `insert/get/list_assignment`, and
`insert/get/list_delegation` (list method names are plural). Lists are ordered by
ID; agent definitions additionally sort by numeric revision. No latest-revision
lookup, definition replacement, general update, or delete is provided.

Manual Bridge adds `prepare_assignment_capsule`, `get_assignment_capsule`,
`mark_assignment_dispatched`, `record_assignment_result`, and
`get_assignment_receipt`. These are specific workflow operations, including
assignment-status changes, rather than a general update interface.

`set_task_status(&TaskId, TaskStatus)` is the only additional task mutation.
It acquires an IMMEDIATE transaction, reconstructs the existing task through the
same validated reader, changes only the status column, and returns the task.
Missing tasks are typed NotFound errors; invalid persisted relationships fail
before writing. It records the caller's declaration without transition rules,
receipt verification changes, or a schema migration.

Parent and dependency tasks must exist before task insertion. Capsules and
artifacts require existing tasks. Receipts require an existing task, resource,
and capsule; their produced artifacts must exist. Composite foreign keys enforce
that capsule and produced artifacts belong to the receipt's task, including for
writers outside the Rust API that enable foreign keys.

Session insertion requires resource and exact agent revision. Assignment insertion
requires task/session/agent, matching session configuration, and Prepared status.
Delegations require both allocations and the parent's historical agent policy;
permission and direct-child count are checked with insertion in an IMMEDIATE
transaction. These policy/workflow checks belong to the Rust API, not a global
graph engine; external SQL writers still receive the relational constraints.

Preparation inserts the capsule and same-task allocation link atomically. A
result transaction inserts artifact metadata, receipt, receipt artifacts, allocation
receipt provenance link, and ResultReceived status together. It requires one
prepared capsule, matching task/resource, and the supplied artifact. Both link
tables allow only one capsule/result per assignment in this version. Filesystem
content is outside this database transaction; see [Manual Bridge](MANUAL_BRIDGE.md).

Task plus dependency rows, and receipt plus produced-artifact rows, commit in
single transactions. A failure after an earlier child insertion rolls back the
whole operation. Single-row insertions are atomic SQLite statements. Compound
task/receipt reads use read transactions to reconstruct one consistent snapshot.
Default SQLite journaling is retained; no connection pool or concurrency layer
is added.

Verification evidence remains inside JSON as allowed by this schema. Its IDs are
validated on reconstruction, but are not covered by relational foreign keys;
evidence may originate from a different verification task. The same-task rule
applies only to the capsule and produced-artifact relation, not verifier evidence.

## Reconstruction and errors

Every persisted ID passes through the core's validated constructor. Tasks are
rebuilt with `Task::new`, then their recorded status is restored. Enum and nested
JSON decoding uses the domain's Serde types. Invalid columns, IDs, numbers,
relationships, or JSON return errors without panics or fabricated fallback values.
Foreign keys prevent relational corruption through normal store writes; opening
a connection does not perform a global audit of externally corrupted databases.

`StoreError` distinguishes database errors, serialization failures, domain
reconstruction failures, duplicate entities, missing entities, integrity
violations, and unsupported schema versions. Duplicate/missing errors carry
`EntityId`, retaining the entity's typed ID. SQLite extended error codes classify
duplicates and constraints without matching messages. `ReconstructionError`
preserves specific ID, task, delegation, revision, JSON, unsigned-number, and column
failures. `MissingReference` identifies the relationship and typed missing entity;
agent mismatch, workflow state, delegation permission/limit, and result provenance
have dedicated variants. CLI messages use those variants, not SQLite message text.

## Deliberately deferred

Project initialization/layout and CLI behavior belong to the separate
[project runtime](PROJECT_RUNTIME.md). Artifact bytes belong to project filesystem
content APIs; store persists only metadata. General updates/deletes, hashing,
graph traversal/cycle detection, scheduling, automated execution, verification
runners, context compilation/retrieval, provider integrations, and asynchronous
or server infrastructure remain outside this milestone.
