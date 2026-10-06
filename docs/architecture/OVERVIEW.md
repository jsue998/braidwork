# Architecture

Implemented in M0 — Foundation:

```text
braidwork-cli          user commands, help, human/JSON output
     │
     ▼
braidwork-project      local lifecycle, marker, paths, discovery
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
Project owns `.braidwork/project.json`, `.braidwork/braidwork.db`, and its store.

The project state is the source of truth. Provider conversations are execution
surfaces, not project storage.

- [Domain model](DOMAIN_MODEL.md)
- [Canonical persistence](PERSISTENCE.md)
- [Project runtime and CLI](PROJECT_RUNTIME.md)

The Context Compiler, Manual Bridge, verifier, artifact content store, and
orchestration are future roadmap components, not implemented layers or APIs.
