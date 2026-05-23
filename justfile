# SPDX-License-Identifier: Apache-2.0
# ============================================================================
#  justfile — task runner for the aphrody Rust monorepo.
#
#  As of 2026-05-23 the non-Rust toolchains were extracted into SIBLING repos:
#    * Go      → C:\src\aphrody-go   (go.work)
#    * Python  → C:\src\aphrody-py   (uv / pyproject.toml)
#    * TS/Bun  → C:\src\aphrody-ts   (apps/* + packages/* + package.json)
#  Run `just build|test|lint|fmt` in those repos for their respective phases.
#  This justfile now drives the Rust Cargo workspace (crates/*) only.
#
#  Toolchain version is pinned in rust-toolchain.toml.
#  Run `just` with no args to list every recipe.
#
#  Recipe lines are single program invocations (or one-line `&&` chains) so
#  they run identically under POSIX sh and Windows cmd.exe — no shell-specific
#  syntax, keeping Linux #1 / Windows #2 parity per CLAUDE.md §0.
# ============================================================================

# Default recipe: show the catalogue.
default:
    @just --list

# ---------------------------------------------------------------------------
# Aggregate recipes — Rust phases (go/python/ts live in sibling repos).
# ---------------------------------------------------------------------------

# Build the Rust workspace.
build: build-rust

# Test the Rust workspace.
test: test-rust

# Lint the Rust workspace (clippy -D warnings).
lint: lint-rust

# Format the Rust workspace in place.
fmt: fmt-rust

# CI gate: lint then test, no formatting side effects.
ci: lint test

# Fetch Rust dependencies. (go/python/ts: see C:\src\aphrody-{go,py,ts}.)
install:
    cargo fetch --locked

# ---------------------------------------------------------------------------
# Rust — Cargo workspace (crates/*). Source of truth: Cargo.toml.
# ---------------------------------------------------------------------------

build-rust:
    cargo build --workspace --locked

test-rust:
    cargo nextest run --workspace --locked

lint-rust:
    cargo clippy --workspace --all-targets --locked -- -D warnings

fmt-rust:
    cargo fmt --all

# ---------------------------------------------------------------------------
# Go / Bun / Python were extracted to sibling repos on 2026-05-23.
# Run their phases there:
#   Go     → C:\src\aphrody-go   : just build|test|lint|fmt  (go.work)
#   Python → C:\src\aphrody-py   : uv sync / uv run pytest / uv run ruff
#   TS/Bun → C:\src\aphrody-ts   : bun test / oxlint / oxfmt  (apps/* + packages/*)
# ---------------------------------------------------------------------------

# ---------------------------------------------------------------------------
# Cross-platform target verification (Rust priority targets, CLAUDE.md §0).
# ---------------------------------------------------------------------------

check-targets:
    cargo check -p aphrody --target x86_64-unknown-linux-gnu --locked
    cargo check -p aphrody --target x86_64-pc-windows-msvc --locked
    cargo check -p aphrody --target wasm32-unknown-unknown --locked

# Supply-chain gate (Rust): CVEs, licences, bans, unused deps.
audit:
    cargo deny check
    cargo machete

# ---------------------------------------------------------------------------
# packages/* UI forks moved to C:\src\aphrody-ts on 2026-05-23.
# Their oxlint/oxfmt/n2b sync recipes (sync-packages*) now live there.
# ---------------------------------------------------------------------------
