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
```

## Schema v1

The exact DDL lives in
[`001_initial.sql`](../../crates/braidwork-store/migrations/001_initial.sql).
All seven tables are STRICT. Entity IDs, labels, enum names, content references,
and JSON use TEXT. Entity primary keys explicitly reject NULL.

| Table | Columns | Keys and relationships |
| --- | --- | --- |
| `resources` | `id`, `name`, `provider`, `access_mode`, `scarcity`, `status` | PK `id` |
| `tasks` | `id`, `title`, `objective`, nullable `parent`, `status` | PK `id`; parent FK to tasks; no self-parent |
| `task_dependencies` | `task_id`, `dependency_id` | Composite PK; both FKs to tasks; no self-dependency |
| `capsules` | `id`, `task_id`, `role`, `objective`, `max_estimated_tokens`, `inputs`, `constraints`, `acceptance_criteria`, `expected_outputs` | PK `id`; task FK; UNIQUE `(id, task_id)` |
| `artifacts` | `id`, `task_id`, `kind`, `content_ref`, nullable `media_type`, nullable `size_bytes` | PK `id`; task FK; UNIQUE `(id, task_id)` |
| `receipts` | `id`, `task_id`, `capsule_id`, `resource_id`, nullable `model_id`, `access_mode`, `execution`, `verification`, `usage` | PK `id`; task/resource FKs; composite capsule/task FK; UNIQUE `(id, task_id)` |
| `receipt_artifacts` | `receipt_id`, `task_id`, `position`, `artifact_id` | PK `(receipt_id, position)`; composite receipt/task and artifact/task FKs |

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
0 applies the initial SQL and writes the current version in the same transaction.
Version 1 is left unchanged; unsupported versions fail explicitly. A failed
migration rolls back both schema changes and the version. There is no replacement
DDL, downgrade, or migration framework. Schema labels and JSON representations
belong to this version; incompatible format changes will need a migration.

## API, integrity, and atomicity

The public API exposes `insert_resource`, `insert_task`, `insert_capsule`,
`insert_artifact`, `insert_receipt`, and the corresponding `get_*` methods.
Insertions take domain entities by reference; getters take the matching typed ID
and return a domain entity or a typed error. There are no updates or deletes.

Parent and dependency tasks must exist before task insertion. Capsules and
artifacts require existing tasks. Receipts require an existing task, resource,
and capsule; their produced artifacts must exist. Composite foreign keys enforce
that capsule and produced artifacts belong to the receipt's task, including for
writers outside the Rust API that enable foreign keys.

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
preserves specific ID, task, JSON, unsigned-number, and column failures.

## Deliberately deferred

Project initialization/layout, CLI commands, updates/deletes, artifact bytes,
hashing/filesystem stores, graph traversal/cycle detection, scheduling, execution,
verification runners, context compilation/retrieval, provider integrations, and
all asynchronous or server infrastructure remain outside this milestone.
