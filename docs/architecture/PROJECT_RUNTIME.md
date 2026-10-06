# Project runtime and CLI

`braidwork-project` owns the lifecycle of a local Braidwork project: its root,
marker, database location, initialization, opening, and upward discovery. It
owns a `SqliteStore` and exposes domain reads and insertions through `store()`
and `store_mut()`. The CLI parses user input, constructs core entities, invokes
project/store operations, and presents results. It contains no layout management
or SQL. Core is unchanged and contains only domain semantics; store assumes no
`.braidwork/` directory and keeps its connection private.

```text
user
 │
 ▼
braidwork-cli
 │
 ▼
braidwork-project
 ├── project.json
 │
 └── braidwork-store
        │
        ▼
      SQLite
        │
        ▼
  braidwork-core entities
```

## Layout and metadata

```text
<project-root>/
└── .braidwork/
    ├── project.json
    └── braidwork.db
```

The three names are constants in `braidwork-project`. The explicit marker,
not a SQLite file alone, identifies the project. Its required fields are:

```json
{
  "format_version": 1,
  "name": "compiler-lab"
}
```

`PROJECT_FORMAT_VERSION` defines the supported marker version once. It is
independent of the store schema version, currently 1. Metadata has no project
UUID or timestamps. A human name must contain non-whitespace text; valid names
retain their exact representation, including exterior whitespace. Names are
not IDs and are not converted to slugs. The default name is the root directory's
UTF-8 filename; roots without a usable filename require an explicit name.

## Lifecycle and initialization atomicity

`Project::init(root, Option<&str>)` requires an existing directory and canonicalizes
it. After validating the name, it creates the state directory exclusively.
Existing valid state returns `AlreadyInitialized`; ambiguous or incompatible
state returns `ExistingState` with its underlying cause. Existing files are
preserved; there is no automatic replacement or cleanup.

Initialization opens and migrates the database first. It then exclusively writes
`project.json.tmp`, synchronizes and closes that file, and renames it to the final
marker within the same directory. Successful initialization leaves only the two
canonical files. The marker is published only after the essential database is
ready. A failed initialization may leave incomplete state or a temporary file,
but no earlier final marker. That state requires explicit inspection on retry.
This is reasonable filesystem atomicity, not a filesystem transaction or a
power-loss recovery protocol; no directory synchronization or crash recovery
engine is implemented.

`Project::open(root)` opens only that root. It requires a regular marker file,
parses its required fields, validates format and name, and requires an existing
regular database file before opening the store. Missing/corrupt metadata or a
missing database is never regenerated. SQLite migrations remain the store's
responsibility.

`Project::discover(start)` canonicalizes an existing directory and walks its
parents to filesystem root. It opens the first encountered `.braidwork` location.
A corrupt, incomplete, or incompatible nearest project produces an error;
discovery never skips it to select a different ancestor. Nested valid projects
therefore select the nearest root. Roots reported by the API are absolute and
canonicalized using standard filesystem APIs.

Accessors expose `root`, `state_path`, `database_path`, validated `metadata`,
`format_version`, `schema_version`, `store`, and `store_mut`. There is no generic
repository trait, application crate, or public database connection.

## CLI contract

```text
braidwork [--project ROOT] [--json]
 ├── init [PATH] [--name NAME]
 ├── status
 ├── resource
 │    ├── add ID --name NAME --provider PROVIDER
 │    │         [--access-mode MODE] [--scarcity SCARCITY] [--status STATUS]
 │    ├── list
 │    └── show ID
 └── task
      ├── add ID --title TITLE --objective OBJECTIVE
      │         [--parent TASK_ID] [--depends-on TASK_ID]...
      ├── list
      └── show ID
```

`--help` and `--version` require no project. Global flags can also follow
subcommands. `init` uses its positional path or the current directory and does
not discover an existing project. Combining `init` with `--project` is rejected
before initialization: use `init <PATH>` to choose the target. Other commands
use upward discovery from the current directory unless `--project` supplies an
exact root; an explicit descendant root never falls back to discovery.

Resource options map exactly to core enums. Defaults are `manual`, `normal`,
and `available`, shown in help. Tasks are constructed through `Task::new` as
`Pending`; repeated prerequisite flags are accepted and core deduplicates them.
Store validates referenced parents/prerequisites and inserts atomically.
Neither command edits existing entities; duplicates fail without replacement.
Lists are ordered by ID; task dependencies remain sorted. Empty lists succeed.

Human output uses concise field displays and plain aligned tables. Status reports
project name/root, marker/store versions, resource count, total task count, and
counts for pending/in_progress/completed. Store list reconstruction applies the
same checks as getters, with a single snapshot for all listed tasks and dependencies.
The resource and task totals in status are separate reads, not a transaction
spanning the entire status report.

In JSON mode, list results are arrays; add/show results use domain Serde data.
Init returns `name`, `root`, `state_path`, `database_path`, `project_format_version`,
and `store_schema_version`. Status returns that same structure under `project`,
plus `resource_count` and `tasks` containing `total`, `pending`, `in_progress`, and
`completed`. JSON is the only successful stdout content in this mode. This format
is defined for this version and carries no 1.0 compatibility guarantee.

## Errors and scope

`ProjectError` distinguishes not-found discovery, duplicate initialization,
preserved incompatible state, invalid roots, missing/invalid markers, malformed
metadata JSON, unsupported marker format, invalid names, missing databases,
path-aware I/O failures, and typed store failures. Metadata accessors expose
validated immutable fields. Library errors retain their sources, without
converting them to strings or using `anyhow`.

The CLI reports errors on stderr, exits nonzero, and prints no default backtrace.
ID parsing reuses core validation, and missing/duplicate entities retain store
semantics. Integrity errors explain that parent and prerequisite tasks must
already exist. Normal human and JSON results go to stdout.

Project, store, and real-binary integration tests use temporary directories.
There are no interactive prompts, provider checks, or network operations.
Updates/deletes, task transitions, capsule/artifact/receipt commands, dispatch,
manual bridge, context compilation, verification runners, artifact content,
Git integration, scheduling, providers, plugins, servers, and async infrastructure
remain deliberately deferred. No placeholder commands are introduced.
