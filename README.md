# Braidwork

**Coordinate the AI you already have.**

Braidwork is an open-source, local-first and free-first orchestration runtime
for coordinating multiple AI resources around a shared software project.

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

M0 — Foundation now includes a working local CLI, typed domain entities, and
canonical SQLite persistence. You can initialize a project, register resources
and pending tasks, and inspect their persisted state. Commands also work from
subdirectories through upward project discovery.

## Quick Start

Install the binary from this checkout:

```bash
cargo install --path crates/braidwork-cli
```

Then, in an existing project directory:

```bash
braidwork init --name compiler-lab

braidwork resource add chatgpt-plus-main \
  --name "ChatGPT Plus" --provider OpenAI \
  --access-mode manual --scarcity scarce

braidwork task add requirements \
  --title "Understand requirements" --objective "Produce a precise plan"

braidwork task add implementation \
  --title "Implement feature" --objective "Implement the accepted design" \
  --depends-on requirements

braidwork status
braidwork resource list
braidwork task list
braidwork task show implementation
```

State lives in `.braidwork/project.json` and `.braidwork/braidwork.db` and survives
process restarts. `init [PATH]` initializes an existing directory; omitting
`--name` uses its directory name. `--project <ROOT>` opens an exact project root
for other commands, while `--json` emits script-friendly JSON results. For example:

```bash
braidwork --json --project /path/to/compiler-lab status
```

`--project` cannot be combined with `init`; use `init <PATH>` instead.
Use `braidwork --help`, `braidwork resource --help`, or `braidwork task --help`
for the complete interface. Resource defaults are manual access, normal scarcity,
and available status; resource name and provider are required.

## Roadmap

Manual capsule dispatch, result ingestion, verification, scheduling, and official
provider integrations remain future work. This version records project state;
it does not execute models or contact providers. See [ROADMAP](docs/ROADMAP.md)
and [Project runtime architecture](docs/architecture/PROJECT_RUNTIME.md).

## License

Apache-2.0.
