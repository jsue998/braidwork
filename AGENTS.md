# Braidwork Agent Instructions

## Mission

Braidwork coordinates heterogeneous AI resources around a shared
project while minimizing scarce model usage and unnecessary context.

## Current milestone

M1 — Manual Braidwork / Desktop foundation.

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
11. Desktop is an adapter, not domain authority.
12. UI must remain general-purpose.
13. No provider automation.
14. No remote canonical state.
15. Never fabricate progress.

## Validation

Before considering a change complete:

    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace

For Desktop changes, also run from `apps/braidwork-desktop`:

    npm run lint
    npm run test
    npm run build

Native Tauri validation additionally requires the platform prerequisites; do not
install system packages automatically or claim unexecuted platform tests.
