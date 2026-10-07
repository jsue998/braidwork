.PHONY: build check test fmt clippy run desktop-dev desktop-build desktop-check

build:
	cargo build --workspace

fmt:
	cargo fmt --all

clippy:
	cargo clippy --workspace --all-targets -- -D warnings

test:
	cargo test --workspace

check:
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets -- -D warnings
	cargo test --workspace

run:
	cargo run -p braidwork-cli

# Native development requires the documented platform prerequisites.
desktop-dev:
	cd apps/braidwork-desktop && npm run tauri -- dev

desktop-build:
	cd apps/braidwork-desktop && npm run tauri -- build --debug --no-bundle

desktop-check:
	cd apps/braidwork-desktop && npm run lint && npm run test && npm run build
