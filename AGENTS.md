# Braidwork Agent Instructions

## Mission

Braidwork coordinates heterogeneous AI resources around a shared software
project while minimizing scarce model usage and unnecessary context.

## Current milestone

M0 — Foundation.

Do not implement future roadmap features unless explicitly requested.

## Architectural rules

1. `braidwork-core` contains domain logic and must not depend on UI concerns.
2. `braidwork-store` owns persistence.
3. `braidwork-cli` owns user-facing CLI behavior.
4. Project state is authoritative; model conversations are not.
5. No provider-specific concepts belong in core primitives unless unavoidable.
6. Manual workflows are first-class, not temporary hacks.
7. Do not add browser scraping or private-session-token integrations.
8. Prefer deterministic mechanisms over LLM calls whenever possible.
9. Do not add heavyweight infrastructure without a demonstrated requirement.
10. Unsafe Rust is forbidden unless this policy is deliberately changed.

## Validation

Before considering a change complete:

    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace
