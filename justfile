# SPDX-License-Identifier: Apache-2.0
# ============================================================================
#  justfile — task runner for the unified aphrody polyglot monorepo.
#
#  This justfile unifies the tasks for all 3 toolchains inside the monorepo:
#    * Rust   (crates/*)
#    * Python (py/)
#    * Bun/TS (apps/*)
#
#  Runs identically under POSIX sh (Linux) and Windows PowerShell.
# ============================================================================

# Use powershell on Windows to avoid "sh not found" issues
set windows-shell := ["powershell", "-NoProfile", "-Command"]

# Default recipe: show the catalogue.
default:
    @just --list

# ---------------------------------------------------------------------------
# Aggregate recipes — Unified Polyglot Monorepo
# ---------------------------------------------------------------------------

# Build all compilable workspaces (Rust).
build: build-rust

# Test all workspaces (Rust, Python, TS/JS).
test: test-rust test-py test-ts

# Lint all workspaces (Clippy, Ruff, Oxlint).
lint: lint-rust lint-py lint-ts

# Format all workspaces in place (rustfmt, ruff format, oxfmt).
fmt: fmt-rust fmt-py fmt-ts

# CI gate: run all lints and tests.
ci: lint test check-targets audit

# Install/sync dependencies for all workspaces.
install: install-rust install-py install-ts

# ---------------------------------------------------------------------------
# Rust Crate Workspace (crates/*)
# ---------------------------------------------------------------------------

build-rust:
    cargo build --workspace --locked

test-rust:
    cargo nextest run --workspace --locked

lint-rust:
    cargo clippy --workspace --all-targets --locked -- -D warnings

fmt-rust:
    cargo fmt --all

install-rust:
    cargo fetch --locked

# Cross-platform target verification (Rust priority targets, CLAUDE.md §0).
check-targets:
    cargo check -p aphrody --target x86_64-unknown-linux-gnu --locked
    cargo check -p aphrody --target x86_64-pc-windows-msvc --locked
    cargo check -p aphrody --target wasm32-unknown-unknown --locked

# Supply-chain gate (Rust): CVEs, licences, bans, unused deps.
audit:
    cargo deny check
    cargo machete

# ---------------------------------------------------------------------------
# Python Workspace (py/)
# ---------------------------------------------------------------------------

test-py:
    uv --directory py run pytest

lint-py:
    uv --directory py run --with ruff ruff check .

fmt-py:
    uv --directory py run --with ruff ruff format .

install-py:
    uv --directory py sync --all-extras

# ---------------------------------------------------------------------------
# TypeScript/JS Bun Workspace (root & apps/*)
# ---------------------------------------------------------------------------

test-ts:
    bun test apps/

lint-ts:
    bun run lint

fmt-ts:
    bun run fmt

install-ts:
    bun install
