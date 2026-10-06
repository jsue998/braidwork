# Execution model

Braidwork coordinates the AI capacity the user already has around canonical
project state. The same primitives support research, writing, planning, study,
design, analysis, software work, or any goal divided among specialists. No role
name, provider, or organization shape is hardcoded.

## Distinct identities and responsibilities

| Concept | Meaning |
| --- | --- |
| Resource | Access capacity: account, subscription, API, harness, or local runtime. Declared availability/scarcity are not measured quotas. |
| Model | Concrete execution model identity, represented only by `ModelId`; no model entity or registry exists. |
| AgentSpec | Provider-independent agent definition: name, role, mission, instructions, expertise, and delegation policy. |
| Session | Persistent conversation/execution instance using a resource and a configured agent revision; external reference is opaque. |
| Task | Work objective, parent, dependencies, and declared project progress. |
| Assignment | Allocation of a task to a session using an exact historical agent revision. |
| Delegation | Explicit act linking parent and child assignments; not allocation or a task prerequisite. |
| TaskCapsule | Portable snapshot of role, mission, instructions, objective, selected context, requirements, expected outputs, and declared context budget. |
| Artifact | Meaningful result metadata/provenance and an external content reference, distinct from an external chat message. |
| Receipt | Recorded execution outcome, resource, optional actual model, resulting artifacts, verification, and optional usage. |

`AgentSpecId`, `SessionId`, `AssignmentId`, and `DelegationId` use exactly the
existing typed-ID guarantees: no empty/whitespace-only/exterior-whitespace IDs,
no silent normalization, internal whitespace allowed, and validated Serde
rehydration. Core generates no IDs; project operations use UUID v4 for allocation,
capsule, artifact, receipt, and delegation identities.

## Revisions and provenance

Agent identity is `(AgentSpecId, revision)`. Revisions use `NonZeroU32`, serialized
as positive integers, and start at 1. `researcher@1` and `researcher@2` coexist.
Insertion of an existing pair fails; no definition update API exists. Explicit
revision lookup never resolves “latest.” The CLI supports additive `agent add
--revision N`; omitting revision chooses 1.

A session references a resource and one exact agent revision. `external_ref` may
be a URL, label, thread ID, provider conversation ID, or any manual handle; core
never interprets it. `Ready`, `Busy`, and `Dormant` describe declared instance
availability. They are not task statuses or automatic occupancy locks. No session
configuration/status update API or scheduler is provided yet.

Assignment creation requires an existing task, session, and agent revision. Its
revision must match the session's configuration at insertion; no override exists.
An insertion-time SQL trigger independently enforces that match. Assignment stores
its own historical pair instead of following future session agent edits. Capsule
preparation and delegation policy read that historical pair. No model is fixed in
Session: model choice can change by execution, and unknown stays unknown in Receipt.

Many sessions can share a resource without a uniqueness restriction:

```text
Resource: ai-main
 ├── coordinator-chat ── AgentSpec coordinator@1
 ├── research-chat    ── AgentSpec researcher@1
 └── review-chat      ── AgentSpec reviewer@1
```

The same structure may instead use different resources/providers for each session.
There are no provider-specific branches. External conversations are execution
surfaces; Braidwork retains the authoritative entities and provenance.

## Three separate graphs

```text
Task graph:                Organization / session structure:
Task A ─prerequisite─► B    resources + agent revisions + registered sessions
                           (membership/configuration, not work dependencies)

Delegation graph:
Coordinator Assignment ─► Expert Assignment ─► Worker Assignment
      session C                session E            session W
```

Task dependencies order work. Session configuration says which instances and
specialties exist. Delegation records who handed work to whom. Task parentage
neither creates delegation nor implies a prerequisite. None of these graphs
creates tasks, sessions, or agents automatically.

A Delegation rejects self-reference in core construction and deserialization and
through SQL CHECK. Its assignments must exist. Store checks the parent's exact
historical agent policy: `allowed = false` denies delegation; an allowed policy
with `None` permits unlimited direct children, `Some(0)` permits none, and
`Some(n)` bounds direct outgoing edges per parent assignment. Counts and insertion
share an IMMEDIATE transaction. Repeated parent/child edges are rejected. A child
can have multiple parents; arbitrary depth is representable. Global cycles are
not detected, and there is no global depth limit or automatic delegation.

## Manual allocation lifecycle

```text
Prepared ── explicit mark-dispatched ──► Dispatched
   │                                        │
   └────────────── ingest ───────────────────┘
                       │
                       ▼
                ResultReceived
```

Prepared includes allocation before a capsule exists. Preparation associates one
immutable capsule without changing status. Render and ordinary dispatch inspection
have no state-changing effect. Marking dispatch requires a capsule and is idempotent
while dispatched. Ingestion accepts prepared or dispatched allocations, records
one result, and moves to ResultReceived atomically with its database records.
Further ingestion/preparation requires a new assignment; failed commits leave the
original allocation usable, without automatic retries.

There is no Assignment `Failed` duplicating `Receipt.execution`, nor `Completed`
claiming acceptance before verification exists. SessionStatus, AssignmentStatus,
TaskStatus, ExecutionOutcome, and Verification answer different questions.
Manual ingestion records `Completed` execution and `NotPerformed` verification.
It does not change TaskStatus, mark accepted, or execute any checks.

## Capsule and receipt associations

`assignment_capsules` stores a single assignment/capsule association, enforcing
matching task IDs. `TaskCapsule` has no SessionId or AssignmentId and remains
self-contained for render/copy. Mission and instructions are captured directly
in portable fields; legacy capsules migrate to empty mission/instructions and
Serde defaults support old serialized capsules. Task itself is unchanged.

`assignment_receipts` stores one result receipt per assignment and enforces task,
prepared capsule, assigned session, and session resource provenance through
composite foreign keys. Receipt itself retains its existing general representation,
including optional model and honest optional usage. Joining the assignment link
recovers session and exact historical agent revision. Verification evidence keeps
its existing JSON semantics and may originate from other tasks.

See [Canonical persistence](PERSISTENCE.md) for exact schema and APIs and
[Manual Bridge](MANUAL_BRIDGE.md) for transport, content, and atomicity.

## Deliberately deferred

OrganizationPlan, dynamic topology, autonomous planning/spawning/delegation,
scheduling, retries, budget propagation, graph cycle detection, session editing,
verification/completion policies, provider integrations, browser automation,
Context Compiler, token counting, and GUI remain future work. The organization
is data ready for future consumers, not an implemented autonomous planner.
