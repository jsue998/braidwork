# Architecture

Implemented in M1 — Manual Braidwork / Desktop foundation:

```text
React UI              locally bundled Desktop interface
     │ Tauri IPC
     ▼
braidwork-desktop      serialized Rust adapter, project root state
     │                 braidwork-cli also calls project directly
     │
     ▼
braidwork-project      lifecycle, discovery, artifact content, Manual Bridge
     │
     ▼
braidwork-store        synchronous SQLite persistence and integrity
     │
     ▼
braidwork-core         typed domain entities and local invariants
```

The CLI uses core types to construct domain entities and accesses persistence
through the opened project. Core has no filesystem, database, or UI dependencies.
Store accepts caller-supplied database paths without assuming a project layout.
Project owns `.braidwork/project.json`, `.braidwork/braidwork.db`, its store, and
the lazily created `.braidwork/artifacts/` content directory. Store owns schema
migrations and relational execution provenance; core contains Resource, AgentSpec,
Session, Task, Assignment, Delegation, TaskCapsule, Artifact, and Receipt, plus
typed identities including ModelId. There is no model entity.

The project state is the source of truth. Provider conversations are execution
surfaces, not project storage.

- [Domain model](DOMAIN_MODEL.md)
- [Canonical persistence](PERSISTENCE.md)
- [Project runtime and CLI](PROJECT_RUNTIME.md)
- [Execution model](EXECUTION_MODEL.md)
- [Manual Bridge](MANUAL_BRIDGE.md)
- [Desktop](DESKTOP.md)

Manual execution supports any divisible work, including research, writing,
planning, and software. Agent/session structure, task dependencies, and delegation
are separate data relationships. Many sessions may share one resource; different
sessions may also use heterogeneous resources without provider-specific logic.

The Context Compiler, verification engine, dynamic planner, scheduler, provider
connectors, and more advanced Desktop capabilities remain future components. No model calls or automatic web
chat integration are implemented.
