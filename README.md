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

## Status

Braidwork is in early development.

The first milestone focuses on manual multi-model coding workflows.

## License

Apache-2.0.
