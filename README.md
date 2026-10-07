# Braidwork

**Coordinate the AI you already have.**

Braidwork is an open-source, local-first and free-first orchestration runtime
for coordinating multiple AI resources around a shared project: research,
writing, planning, software, or other work divided among specialists.

Its purpose is simple:

- use free and abundant models for routine work;
- preserve scarce or expensive models for high-value reasoning;
- give every model only the context required for its task;
- keep project state independent from any AI provider;
- verify work whenever deterministic evidence is available;
- support manual copy/paste workflows as a first-class execution mode.

## Core primitives

Braidwork is built around two fundamental objects:

- **Task Capsule** — exactly what a worker needs to know and produce.
- **Receipt** — exactly what was executed, by which resource, with what
  context, and what evidence validates the result.

## Principles

1. Free-first.
2. Local-first.
3. Evidence-first.
4. Intelligence is a resource.
5. Context is a budget.
6. Manual-first, automation when officially supported.

## Current

M1 — Manual Braidwork / Desktop foundation includes a local desktop interface,
a CLI for scripting and debugging, canonical SQLite persistence, versioned
agent definitions, independent sessions, assignments, and explicit delegations.
Manual Bridge prepares portable instructions and ingests external results as
filesystem artifacts and auditable receipts. Commands also work from project
subdirectories through upward discovery.

## Braidwork Desktop

The desktop foundation is a general-purpose work interface, not a chat client.
Open or create a real project, register resources, define agents and sessions,
create and assign work, and prepare portable instructions. Copy them into your
external AI, explicitly record dispatch, then paste/import the response and inspect
its artifacts and unverified receipt. Team, Work, Resources, Results, and a
selection inspector show canonical project data, including explicit delegations.
Task completion is a separate user action; no progress or provider usage is invented.

Desktop is an early development build, not a finished packaged release.
Development requires Rust, Node.js 22.12+ and npm, plus Tauri's native platform
prerequisites. On Linux, GTK 3 / WebKitGTK 4.1 development packages are required.
See [Desktop architecture and prerequisites](docs/architecture/DESKTOP.md).

```bash
cd apps/braidwork-desktop
npm install
npm run tauri -- dev
```

Frontend-only `npm run dev` serves local assets for development; it does not
provide a fake runtime. Use the Tauri command for project operations and native dialogs.

```bash
# From the repository root:
make check          # Rust domain/runtime and headless desktop service
make desktop-check # ESLint, Vitest, strict TypeScript and local asset build
make desktop-build # Native debug application, no installers
```

No account, credentials, network backend, telemetry, or automatic provider access
is needed. Consumer web chats remain manual. Windows/macOS architecture is
configured; actual native builds must be validated on those hosts.

## Braidwork CLI — Quick Start

Install the binary from this checkout:

```bash
cargo install --path crates/braidwork-cli
```

Then, in an existing project directory:

```bash
braidwork init --name travel-research

braidwork resource add chatgpt-plus-main \
  --name "ChatGPT Plus" --provider OpenAI \
  --access-mode manual --scarcity scarce

braidwork agent add researcher \
  --name "Researcher" --role "Research specialist" \
  --mission "Prepare a rigorous travel plan" \
  --instruction "Distinguish facts from assumptions"

braidwork session add research-chat \
  --label "Travel research chat" --resource chatgpt-plus-main \
  --agent researcher --external-ref "My research conversation"

braidwork task add itinerary \
  --title "Research itinerary" --objective "Prepare a structured travel plan"

braidwork assignment create itinerary --session research-chat
# Use the assignment ID printed above:
braidwork capsule prepare <ASSIGNMENT_ID> \
  --constraint "Do not invent prices" --accept "Explain budget assumptions"
braidwork capsule render <CAPSULE_ID>
# Manually copy instructions to the external AI, then save its result as result.md.
braidwork ingest <ASSIGNMENT_ID> --file result.md
braidwork assignment show <ASSIGNMENT_ID>
braidwork status
# Declare progress explicitly when appropriate (does not verify the receipt):
braidwork task status itinerary completed
```

State lives in `.braidwork/project.json` and `.braidwork/braidwork.db`; returned
content lives in the lazily created `.braidwork/artifacts/`. Everything survives
process restarts. Many sessions can share one resource. Agent revisions coexist
without replacing historical definitions. Ingestion records unverified work,
not automatic acceptance or task completion.

`init [PATH]` initializes an existing directory; omitting
`--name` uses its directory name. `--project <ROOT>` opens an exact project root
for other commands, while `--json` emits script-friendly JSON results. For example:

```bash
braidwork --json --project /path/to/travel-research status
```

`--project` cannot be combined with `init`; use `init <PATH>` instead.
Use `braidwork --help` and each command group's `--help` for the complete interface.
Resource defaults are manual access, normal scarcity,
and available status; resource name and provider are required.

## Roadmap

Dynamic planning, scheduling, Context Compiler, verification runners, advanced Desktop, and
official provider integrations remain future work. External model execution is
manual; this version does not contact providers or automate consumer web chats.
See [ROADMAP](docs/ROADMAP.md), [Execution model](docs/architecture/EXECUTION_MODEL.md),
and [Manual Bridge](docs/architecture/MANUAL_BRIDGE.md).

## License

Apache-2.0.
