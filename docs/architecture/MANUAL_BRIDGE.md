# Manual Bridge v1

The bridge is a local, general-purpose prepare/copy/ingest workflow. Consumer
web chats remain manual until an authorized integration exists. There is no
browser opening, scraping, cookies, private tokens, reverse-engineered endpoint,
or provider automation. External conversations never become canonical storage.

```text
Task + Session + historical AgentSpec revision
              │
              ▼
         Assignment
              │ prepare (one immutable association)
              ▼
         TaskCapsule
              │ render / dispatch inspection
              ▼
       manual copy to external AI
              │ manual copy of returned result
              ▼
            ingest
              ├── artifact bytes → project filesystem
              ├── Artifact metadata → SQLite
              └── Receipt + allocation link/status → SQLite
```

## Prepare and render

```bash
braidwork assignment create travel-research --session research-chat
braidwork capsule prepare <ASSIGNMENT_ID> \
  --context-file requirements.txt \
  --context-text "Prefer rail travel" \
  --constraint "Do not invent prices" \
  --accept "Explain budget assumptions" \
  --output analysis:"Structured itinerary" \
  --output text:"Budget summary" \
  --max-context-tokens 4096
braidwork capsule render <CAPSULE_ID>
```

Prepare captures the task objective and historical agent role/mission/instructions.
Task is not extended with acceptance criteria; criteria, constraints, and expected
outputs are explicit preparation inputs. Each flag is repeatable except budget.
`--output` uses `KIND:DESCRIPTION`, splitting only on the first colon. Kinds are
patch, analysis, finding, test_result, documentation, question, text, and data.
Omitting outputs requests generic text. Expertise stays agent metadata.

Context files must be UTF-8 and are snapshotted as inline text with their filename
as label. Relative input paths resolve from the CLI's current directory, not the
project root. Context-text values are also inline. Later file changes cannot alter
the persisted capsule. No searching, ranking, token estimation, or automatic
reference resolution occurs. Default budget is 4096 declared estimated tokens;
zero requires no selected context. Nonzero bounds are recorded, not measured or
enforced against a tokenizer/provider limit.

Renderer consumes only TaskCapsule and produces portable Markdown sections:
Role, Mission, Objective, Context, Instructions, Constraints, Acceptance Criteria,
Expected Output, and Response Contract. Opaque reference inputs remain labeled
external references. No chat-message or provider-specific request syntax exists.
The suggested response format is human-readable:

```markdown
# Braidwork Result

## Summary
What was done.

## Output
The substantive work.

## Notes
Uncertainty, evidence, limitations, or questions.
```

These headings are guidance, not a required parser contract. Ingest preserves the
exact input bytes, including results with other headings or no Markdown structure.
It does not require model-produced JSON, parse quality, or split outputs automatically.

## Dispatch

```bash
braidwork dispatch <ASSIGNMENT_ID>
braidwork dispatch <ASSIGNMENT_ID> --mark-dispatched
```

Both show session, resource, opaque external reference, historical agent revision,
task, and rendered instructions. Display alone changes no status. The explicit
flag records Dispatched only after preparation. No browser or clipboard is touched.
A person chooses the model externally; Session stores no permanent ModelId.

## Ingest and receipt

```bash
braidwork ingest <ASSIGNMENT_ID> --file result.md
cat result.md | braidwork ingest <ASSIGNMENT_ID> --stdin
braidwork ingest <ASSIGNMENT_ID> --file result.md \
  --model actual-model --input-tokens 120 --output-tokens 0 \
  --cost-micros 0 --currency USD --kind analysis
```

File and stdin are mutually exclusive and one is required. Assignment/capsule
lookup occurs before input consumption. Optional model is constructed with ModelId;
it is never inferred from account/provider. Optional token counts remain `None`
when omitted. Cost and currency must be supplied together; known zero cost is
preserved as zero millionths of the explicit currency, never treated as unknown.
Artifact kind defaults to text and media type to `text/markdown`, both configurable.

Project ingestion creates UUID v4 ArtifactId/ReceiptId and records actual byte
size. Receipt references the assigned task, prepared capsule, session resource,
and new artifact. Its access mode is Manual because this bridge transports the
result manually, regardless of the resource's declared access mode. Execution is
Completed (a result was supplied), verification is NotPerformed, and usage retains
only supplied observations. These facts do not certify result quality or acceptance.
The assignment becomes ResultReceived; the task and session statuses are unchanged.

Successful JSON ingestion returns `assignment`, `artifact`, and `receipt` objects.
`assignment show` includes the allocation plus optional capsule and receipt, so
organizational provenance can be inspected after reopening. No separate artifact
or receipt command groups are introduced in this version.

## Content layout and portability

```text
.braidwork/
├── project.json                 # format remains 1
├── braidwork.db                 # schema 2
└── artifacts/                   # created lazily on first content write
    └── <hex-encoded-artifact-id>.bin
```

Core treats `content_ref` as opaque. Project produces/interprets
`artifact:<hex-encoded-id>.bin`, relative to its artifact directory. It stores no
absolute machine-specific paths. Hex encodes the UTF-8 ID bytes; arbitrary valid
IDs cannot become separators or traversal paths. Reads accept only this local
reference format. UUID v4 is provided by the `uuid` library; no timestamps-only
identity or custom randomness is used. Content is not hashed/content-addressed.

`Project::write_artifact_content` writes exclusively and synchronizes the new file;
existing content is never replaced. `read_artifact_content` returns exact bytes
or typed missing/invalid-reference/I/O errors. Metadata remains in SQLite. The
low-level write API does not itself insert metadata; Project::ingest composes both.

## Atomicity and failures

The content file is written before the SQLite transaction. Store then inserts
Artifact, Receipt, receipt_artifacts, assignment_receipts, and the assignment's
ResultReceived status in one IMMEDIATE transaction, validating matching task,
capsule, session, resource, and supplied artifact. A database failure rolls back
all rows and status. Project removes only the file just created for that failed
result. A cleanup failure preserves both original and cleanup errors. Existing
content is untouched. Ordinary write failure attempts to remove its partial file.

SQLite and filesystem are not one transaction. Process failure after content write
but before database commit can leave an orphan file; directory synchronization,
crash recovery, orphan scanning, and garbage collection are not implemented. The
write-before-commit sequence avoids committing metadata before content is ready;
external deletion and hardware failures are outside this guarantee.

Workflow errors prevent replacing capsules or ingesting a second successful result
into the same assignment. A new assignment represents another attempt; no automatic
retry or model call occurs. JSON results stay on stdout; errors go to stderr with
nonzero exit status, typed relationship/workflow context, and no default backtrace.

Verification runners, provider connectors, context compilation/retrieval, clipboard
support, response parsing, multiple artifact extraction, GUI, planners, and schedulers
remain deliberately deferred.
