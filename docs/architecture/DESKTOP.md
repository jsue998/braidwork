# Desktop foundation

M1 adds a general-purpose, local-first work interface. It organizes the AI team
and manual handoffs; it is not a chat client, a planner, or a provider integration.
Research, writing, travel planning, analysis, and software use the same entities.

```text
React UI (locally bundled)
   │ Tauri IPC
   ▼
braidwork-desktop (Rust adapter)
   │
   ▼
braidwork-project (lifecycle + Manual Bridge + artifact content)
   │
   ▼
braidwork-store (SQLite schema v2 + integrity)
   │
   ▼
braidwork-core (domain entities + invariants)
```

UI is not canonical. SQLite/project state is canonical. Desktop calls the existing
Rust APIs, never the CLI, SQL, or a frontend filesystem API. Core is unchanged.
Project format remains 1 and store schema remains 2; no new migration is needed.

## Host, build, and boundaries

The app uses Tauri 2, React, strict TypeScript, Vite, npm, and a checked-in lockfile.
All frontend assets and system fonts are local; there is no CDN, login, telemetry,
remote configuration, update checker, cloud state, or custom local HTTP server.
Vite's development server is development tooling only. Native window decorations
remain enabled, with initial 1200×760 and minimum 960×640 dimensions.
Small development-only window icons satisfy native build requirements; final
branding and installers remain deferred.

The Rust workspace member lives in `apps/braidwork-desktop/src-tauri`.
Its default library builds the adapter, DTOs, and service tests without a display
or WebKit development libraries. The `desktop` feature enables Tauri, native
commands, build integration, and official plugins. Tauri configuration explicitly
enables that feature for both `tauri dev` and `tauri build`. Workspace checks cover
the headless service; native checks additionally require the feature and platform
libraries. This is a build boundary, not an alternate mocked implementation.

`DesktopService` retains only a canonical project root inside a mutex. Opening,
closing, switching, reading, and writing are serialized. Each operation opens a
fresh `Project`; no `rusqlite::Connection` is shared across threads. Native IPC
commands run service operations on Tauri's blocking executor, keeping database
and file work off the UI thread. A failed project switch preserves the previous
root. Close only releases the active root; it never deletes data.

Open requires the exact selected project root and uses strict `Project::open`.
Create separates a human display name, a folder component, and an existing parent
directory selected through the native dialog. `create_project` validates names,
canonicalizes the parent, joins the single component, and exclusively creates the
new child before calling `Project::init`. The chosen parent is never initialized.
Existing destinations (including empty directories) are rejected; opening an
existing project remains a separate action. Unicode display names are preserved.
The UI suggests a deterministic folder name while the name is entered, but stops
changing it once the user edits Folder. The path preview is presentation only;
Rust constructs the actual root and selects it only after successful initialization.

Folder validation is authoritative in Rust: reject empty/whitespace-only names,
`.`/`..`, separators, control characters, `: * ? " < > |`, trailing spaces/dots,
and case-insensitive Windows reserved names (including their extensions).
No arbitrary parent creation or recursive cleanup is performed. If initialization
fails, `remove_dir` attempts to remove only the still-empty new child; partial or
externally added content remains, the original error survives, and the previous
active project is retained. `Project::init` keeps its existing exclusive directory /
database-first / final-marker-rename strategy. Missing databases and corrupt metadata are errors,
not triggers for repairs, replacement projects, or a new empty state.

## IPC and frontend state

Commands are explicit: `open_project`, `create_project`, `close_project`,
`workspace_snapshot`, `create_resource`, `create_agent`, `create_session`,
`create_task`, `set_task_status`, `create_assignment`, `create_delegation`,
`prepare_capsule`, `render_capsule`, `mark_dispatched`, `ingest_text`, `ingest_file`,
`assignment_detail`, `result_detail`, and `open_external_reference`.

JSON fields and command argument names use **snake_case**. Core entity Serde
shapes are reused for resources, exact agent revisions, sessions, tasks,
assignments, and delegations. Small boundary read models aggregate workflow links,
receipt observations, artifact metadata, and optional previews. They introduce no
new domain entities. Every `u64` crossing IPC (budget, token observations, monetary
millionths, sizes) is a decimal string: JavaScript must not round those values.
`null` remains unknown; `"0"` is a known zero. Agent revisions remain exact `u32`
JSON numbers. Rust validates required input and decimal formats again.

Errors are `{code, message, detail}`: a machine category, a human message, and
optional expandable causal diagnostics. Typed project/store variants determine
categories; SQLite error strings are never used to select behavior. There are
no Debug dumps or default stack traces.

The snapshot includes project name/root/format/schema, resources, agent revisions,
sessions, tasks, assignments, delegations, and per-assignment capsule/result
metadata. It never loads artifact content. Domain getters validate reconstruction.
The desktop mutex serializes its own operations, but a snapshot is an aggregation
of existing store reads, not a single transaction spanning all tables: concurrent
CLI processes can change the database between reads. Mutations still use real
store integrity/transaction rules, and users can refresh after external changes.

React state/context holds only the current view, selection, forms, feedback, and
latest read model. A startup lookup recovers any Rust-selected project after
a webview reload; only a typed no-project outcome is treated as an empty start.
Refresh occurs on opening, an explicit refresh action, and
successful mutations. There is no polling or optimistic canonical data. Requests
ignore stale results after selection/project changes. Errors, loading, empty
states, and a render error boundary prevent silent blank screens.

## Workspace and terminology

- **Overview:** real task/session/assignment/result counts, declared task completion,
  and needs-attention derived from workflow links and recorded states. No tasks
  means no completion percentage. No agent percentages or invented timestamps.
- **Team:** agents define role/mission/instructions/expertise/policy; sessions bind
  an exact agent revision to a resource. Multiple sessions can share one resource.
  A session never implies a permanently fixed model.
- **Resources:** declared provider/access/scarcity/status and actual session counts.
  No invented quotas, credentials, or provider enums.
- **Work:** separate task, assigned-work, and delegation views. Graph nodes are
  assignments, with task title, agent role, session, and recorded state. Edges are
  real delegations, not inferred dependencies. A deterministic grid supports cycles
  without topological sorting; visual positions are not persisted.
- **Results:** receipt-derived results, actual model known/unknown, verification,
  usage, and artifact metadata. Content is fetched only for a selected result.

The inspector displays the selected resource, exact agent revision, session, task,
assignment workflow, or result. IDs remain available under Technical details;
forms generate IDs in Rust. User-facing “Agent”, “Assigned work”, “Instructions”,
“Result”, and “History” do not collapse AgentSpec, Assignment, TaskCapsule, Artifact,
or Receipt. The compact CSS token system defaults to explicit Dark, with
visible focus, labeled controls, native modal keyboard focus, and reduced-motion
support. At narrower desktop widths the inspector overlays the workspace.

## UI language, preferences, and visual identity

Typed local dictionaries provide English and Spanish with exactly matching keys.
The initial language follows `navigator.language` (`es*` selects Spanish, otherwise
English) unless an explicit preference exists. Compact EN/ES and Dark/Light/System
controls appear on the start screen and topbar; switching requires no project
mutation. All interface labels, statuses, forms, feedback, accessibility text, and
frontend-started native dialog titles use this layer. User names, objectives,
paths, model IDs, and result content are never translated automatically.
Common IPC failures are localized by `error.code`; unknown codes retain the
backend message. Expandable details preserve the original backend wording and cause.

Only `braidwork.ui.language` and `braidwork.ui.theme` are stored in localStorage.
These are noncanonical UI preferences, not project metadata or cached project data.
Dark is the first-run default even on a light OS. The root has an explicit
`data-theme` attribute; Light has its own sober palette, and only System subscribes
to OS color-scheme changes. Unavailable preference storage does not prevent use.

The visual language is a retro technical workstation: near-black surfaces,
restrained phosphor/olive accents, amber warnings, visible fine borders, and 2–4px
corners. Local monospace stacks mark branding, navigation, labels, headings,
metrics, and technical metadata; prose and long output remain readable. There are
no CRT effects, neon shadows, terminal UI, remote fonts, or animated decoration.
The shell structure remains unchanged. Project names lead; paths are smaller,
ellipsized secondary text with full-path tooltips. Task creation reads “New task” /
“Nueva tarea”. Status text and distinct symbols complement color, without adding
domain states or fabricated progress. The delegation view loads on demand and its
keyboard instructions describe actual selection/movement, never unsupported deletion.

## Manual workflow

```text
Create/Open project → Resource → Agents → Sessions → Task
  → Assign to a Session → Prepare instructions → Copy
  → External AI (manual) → Mark dispatched → Paste/import result
  → Artifact + Receipt → Inspect → explicitly declare Task status if appropriate
```

Desktop creates revision-one agents and Ready sessions. Agent selection always
includes the exact revision. Task construction uses `Task::new`; allocation uses
`Project::create_assignment`. Delegation uses the parent's historical policy,
including allowed/direct-child limits; failures do not change frontend data.
The second workflow records Coordinator Assignment → Research Assignment and
shows that real edge in the graph. No team or delegation is generated by AI.

Preparation uses explicit text, native-selected UTF-8 files, constraints,
acceptance criteria, expected output descriptions, and a declared estimated-token
budget (default 4096). Rust snapshots file text inline, labeled by filename;
absolute context paths are not persisted. No retrieval or token counting occurs.
Instructions use the existing provider-independent Markdown renderer and readable
response contract. Copy and browser opening are explicit gestures. Merely viewing
or copying instructions never dispatches work.

Pasted text and selected file bytes both call `Project::ingest`. Optional model,
kind/media type, token observations, cost micros, and currency remain honest;
cost and currency must be supplied together. Receipt transport is Manual,
execution is Completed, verification is NotPerformed, assignment becomes
ResultReceived, and neither Task nor Session automatically changes. Content is
written exclusively before the database transaction; on DB failure only newly
written unreferenced content is removed. The existing filesystem/SQLite crash
boundary may leave an orphan; Desktop does not claim a distributed transaction.

`SqliteStore::set_task_status` and `Project::set_task_status` provide a narrow,
transactional user declaration, preserving task references and all receipt data.
There are no new transitions or verification claims. CLI parity is
`braidwork task status ID pending|in-progress|completed`.

## Security and preview

The main local window has only official dialog-open and clipboard-write
permissions, plus the explicit application command allowlist generated by
`tauri-build::AppManifest`. There is no clipboard read, shell, process, general filesystem,
HTTP-client, remote-webview, or frontend opener permission. File reads happen in
specific Rust preparation/ingestion APIs after native file selection.
Automatic JavaScript link opening is disabled. The Rust-only opener plugin is called by `open_external_reference`, which resolves
the stored session reference and validates HTTP/HTTPS with a URL parser. Opaque
references can be copied; file/javascript/custom schemes are rejected.

CSP permits bundled scripts only, no eval, no remote frames/styles/scripts, and
IPC connections only. Inline styles are permitted for React Flow node positioning;
inline scripts remain disallowed in production. Development CSP additionally
permits the explicit loopback Vite/HMR origin and the exact SHA-256 hash of the
locked React refresh preamble. A frontend test checks that hash against the
installed plugin, so upgrades cannot silently break development or require
general inline-script permission. Capabilities apply to the main local window.

Model output is untrusted. React displays it as escaped preformatted text, with
no HTML/Markdown execution. Text-like content previews read at most 1 MiB + one
sentinel byte through Project's bounded content API. Large results show metadata
and an explanation; canonical files remain complete. Binary/unknown media types
and invalid UTF-8 get no text preview. The backend checks actual content size,
not merely artifact metadata. Missing content returns an error without repair.

## Development and validation

Use Rust and Node 22.12+ with npm (no global npm packages). Install dependencies
locally with `npm install` in `apps/braidwork-desktop`. Commands:

```bash
# Repository root, no display required:
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
make desktop-check

# Desktop directory:
npm run typecheck
npm run tauri -- info
npm run tauri -- dev
npm run tauri -- build --debug --no-bundle

# Native Rust checks, platform prerequisites required:
cargo clippy -p braidwork-desktop --features desktop --all-targets -- -D warnings
```

See [official Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for
Linux, Windows, and macOS. Debian development typically needs the following
user-run installation; Braidwork tooling never performs it automatically:

```bash
sudo apt install build-essential pkg-config libgtk-3-dev libwebkit2gtk-4.1-dev libxdo-dev libssl-dev librsvg2-dev
```

Windows uses WebView2 and the platform toolchain; macOS uses its native WebKit
and development toolchain. Configuration avoids Unix commands and hardcoded
platform paths. Cross-platform execution must be validated on those actual hosts.

Service tests use temporary projects and the non-software Japan Trip fixture.
They cover multi-session/shared-resource organization, real delegation policy,
manual workflow and reopening, explicit task status, exact usage/zero/unknown,
UTF-8 context snapshots, safe URLs, preview limits, binary text handling, and
missing content. Frontend tests cover deterministic presentation and escaped
untrusted text. Workspace tests continue checking the real CLI binary.

## Deliberately deferred

No planner, decomposition, dynamic agents, scheduler, verification engine,
Context Compiler, retrieval, model calls, APIs, browser automation, remote canonical
state, GUI chat, search, plugins, deletes, agent editing, recent projects, persisted
graph positions, rich Markdown/image/video preview, or installer branding.
Session reconfiguration is specifically deferred: Assignment freezes the agent
revision but does not independently freeze ResourceId. Future session mutation
must design historical resource provenance before introducing `session update`.
