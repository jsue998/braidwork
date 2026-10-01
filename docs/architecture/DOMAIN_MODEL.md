# Domain model

`braidwork-core` represents the manual workflow's data and local invariants.
The project owns canonical state. Workers receive task-specific capsules and
return results that can become artifacts; their conversations are not storage.
This milestone defines no project aggregate or execution engine.

```text
Task --parent/dependencies--> Task
 │
 ▼
TaskCapsule             Resource
 │                        │
 └───────────┬────────────┘
             ▼
       Execution (external to core)
             │
             ├──► Artifact(s) ──► external content reference
             │
             └──► Receipt ◄── verification decision/evidence
                   │
                   └── task + capsule + resource + optional model + artifact IDs
```

## Entities and relationships

- **Resource** describes usable AI capacity: its identity, display name,
  provider label, access mode, declared scarcity, and declared availability.
  It differs from a model because an account, subscription, harness, or local
  runtime provides access whose limits and availability are independent of
  whichever model it exposes. No provider-specific configuration is included.
- **Task** describes project work through a title, objective, optional parent,
  unique prerequisites, and status. Parentage groups work and does not imply a
  dependency. `Pending`, `InProgress`, and `Completed` are sufficient to record
  manual progress. Completed means the result was accepted; failed or rejected
  attempts are recorded in receipts, leaving the task open for another attempt.
  Status updates record the caller's decision and do not enforce transitions.
- **TaskCapsule** references one task and describes the worker's role, objective,
  selected context, constraints, acceptance criteria, and expected artifact
  kinds/descriptions. Inline text and opaque external references represent
  already-selected context. `ContextBudget.max_estimated_tokens` bounds that
  context in provider-independent estimated tokens, including resolved references.
  The core does not count tokens; the future Context Compiler will estimate them.
  Actual provider/model limits, including rendering overhead, must be checked
  outside the domain. This budget excludes response size; zero permits no
  selected context. No particular tokenizer is prescribed by the model.
  No provider, chat messages, prompt syntax, or request parameters are embedded.
  The same capsule can be delivered using any resource or access mode. Issue a
  new capsule ID when revising a description already used by a receipt.
- **Artifact** describes a meaningful task result: a patch, analysis, finding,
  test result, documentation, question, text, or data. It is not a model message:
  it has task provenance and an opaque reference to separately stored content.
  Its entity ID is distinct from its content reference, which can be an
  algorithm-qualified digest in a future content-addressed store. Media type
  and byte size are optional. The core stores no artifact bytes and does not
  resolve references or compute hashes.
- **Receipt** records an attempt against a particular task, capsule, and resource,
  its resulting artifact IDs, execution outcome, verification, and observed
  usage. It also records the access mode used and an optional concrete `ModelId`,
  so later resource edits cannot alter those historical details. `Some(ModelId)`
  records the model known to have been used; `None` means it could not be
  determined, including in manual execution. Resource identity never implies
  model identity. There is no model entity or registry in this milestone.
  Execution completion and acceptance are distinct:
  completed work may be unverified or rejected, and failed work may have partial
  artifacts. Verification records acceptance or rejection with an explanation
  and optional artifact evidence, including evidence from separate verifier
  tasks. A receipt demonstrates what was recorded and the evidence cited; it
  does not itself execute checks or certify their truth. Manual review can use
  an explanation without evidence artifacts.

## Invariants and representation

`ResourceId`, `ModelId`, `TaskId`, `CapsuleId`, `ArtifactId`, and `ReceiptId` are distinct,
immutable string newtypes. Consumers supply IDs; no IDs are generated. Empty,
whitespace-only IDs and IDs with leading or trailing whitespace fail with
`InvalidId`. Valid strings are preserved exactly, without silent normalization;
internal whitespace remains permitted. IDs support display, comparison, ordering,
hashing, and parsing. Construction, conversion, parsing, and deserialization all
apply the same validation.

Task relationships are private and validated by `Task::new`. A task cannot depend
on itself or be its own parent; either case returns a typed `TaskError`.
Dependencies use a `BTreeSet`, collapsing duplicates and producing deterministic
ordering. Task deserialization goes through the same checks, and every ID is
validated during deserialization. Other entity fields are public metadata: they
do not introduce speculative validation or mutable access to ID strings.

Usage observations are independently optional. Unknown input/output tokens or
cost remain `None`, including after serialization. A known zero is `Some(0)` or
a recorded zero monetary amount. Money uses integer millionths of a named
currency unit, avoiding floating-point ambiguity. Token measurements are supplied
by callers, never computed by the core.

Public domain data supports Serde; JSON is only a test format and the wire format
is not yet stable. Entity links use typed IDs, not nested provider responses.
Checking referenced entity existence and agreement between task, capsule, and
artifact provenance requires canonical project state and belongs to a future
application/project layer. Capsules and receipts must be retained there to make
their references auditable.

## Deliberately deferred

Persistence, hashing, project initialization, graph cycle detection, lifecycle
transition policies, scheduling, execution, dispatch, response ingestion,
verification runners, prompt rendering, context retrieval/budget enforcement,
token counting, quotas, provider integrations, and CLI changes are outside this
implementation. The model introduces no interfaces or infrastructure for them.

`serde` supplies boundary serialization in core; `serde_json` is a test-only
dependency. Small domain errors implement the standard error traits directly,
so no error framework or application-level error dependency is needed.
