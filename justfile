# SPDX-License-Identifier: Apache-2.0
# ============================================================================
#  justfile — unified task runner for the aphrody polyglot monorepo.
#
#  Four toolchains, one entrypoint:
#    * Rust   → cargo / cargo-nextest / clippy / rustfmt   (Cargo workspace)
#    * Go      → go build/test/vet/fmt                       (go.work)
#    * Bun     → bun install/test + tsc                      (package.json)
#    * Python  → uv + pytest + ruff                          (pyproject.toml)
#
#  Toolchain versions are pinned in mise.toml (+ rust-toolchain.toml for Rust).
#  Run `just` with no args to list every recipe.
#
#  Recipe lines are single program invocations (or one-line `&&` chains) so
#  they run identically under POSIX sh and Windows cmd.exe — no shell-specific
#  syntax, keeping Linux #1 / Windows #2 parity per CLAUDE.md §0.
# ============================================================================

# Path to the sole Go module (extend the list as more modules land in go.work).
go_module := "go/aphrody-tokenizer-go"

# Default recipe: show the catalogue.
default:
    @just --list

# ---------------------------------------------------------------------------
# Aggregate recipes — run a phase across every language.
# ---------------------------------------------------------------------------

# Build all four toolchains.
build: build-rust build-go build-bun build-py

# Test all four toolchains.
test: test-rust test-go test-bun test-py

# Lint all four toolchains (clippy -D warnings, go vet, tsc, ruff).
lint: lint-rust lint-go lint-bun lint-py

# Format all four toolchains in place.
fmt: fmt-rust fmt-go fmt-bun fmt-py

# CI gate: lint then test, everything, no formatting side effects.
ci: lint test

# Install/sync every toolchain's dependencies.
install: install-bun install-py
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
# Go — go.work workspace. Source of truth: go.work + each module's go.mod.
# ---------------------------------------------------------------------------

build-go:
    cd {{go_module}} && go build ./...

test-go:
    cd {{go_module}} && go test ./...

lint-go:
    cd {{go_module}} && go vet ./...

fmt-go:
    cd {{go_module}} && go fmt ./...

# ---------------------------------------------------------------------------
# Bun — workspace root (package.json). Members: apps/photoshop-*, packages/*.
# ---------------------------------------------------------------------------

install-bun:
    bun install

# Typecheck across the bun workspace (no-op until apps/* ship a typecheck script).
build-bun: install-bun

# Run the bun test runner across the workspace.
test-bun:
    bun test

# Lint JS/TS with oxlint (oxc) — config: .oxlintrc.json. Global binary (bun add -g).
lint-bun:
    oxlint

# Format JS/TS/JSON with oxfmt (oxc formatter) — config: .oxfmtrc.json. Global binary.
fmt-bun:
    oxfmt apps/

# Check formatting without writing (CI gate).
fmt-bun-check:
    oxfmt --check apps/

# ---------------------------------------------------------------------------
# Python — uv workspace (apps/* + libs/*). Source of truth: pyproject.toml.
# ---------------------------------------------------------------------------

install-py:
    uv sync

# "Build" for Python = materialise the locked uv environment.
build-py: install-py

test-py:
    uv run pytest

lint-py:
    uv run ruff check .

fmt-py:
    uv run ruff format .

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
# packages/* UI forks — synced oxc/bun toolchain (gts = Google-style reference).
# Each fork carries its own .oxlintrc.json / .oxfmtrc.json / bunfig.toml.
# `-` prefix: keep going across forks even when a fork reports findings.
# ---------------------------------------------------------------------------

# Lint the 4 packages with oxlint (per-fork config).
sync-packages-lint:
    -cd packages/gts && oxlint
    -cd packages/material-web && oxlint
    -cd packages/ui && oxlint
    -cd packages/tailwindcss && oxlint
    -cd packages/lit && oxlint

# Check formatting across the 4 packages with oxfmt.
sync-packages-fmt:
    -cd packages/gts && oxfmt --check .
    -cd packages/material-web && oxfmt --check .
    -cd packages/ui && oxfmt --check .
    -cd packages/tailwindcss && oxfmt --check .
    -cd packages/lit && oxfmt --check .

# Bun→Node migration scan (n2b, via WSL on Windows) — report-only, NDJSON to var/n2b/.
sync-packages-n2b:
    -WSL_UTF8=1 wsl.exe ./bin/n2b packages/gts --report jsonl > var/n2b/gts.jsonl
    -WSL_UTF8=1 wsl.exe ./bin/n2b packages/material-web --report jsonl > var/n2b/material-web.jsonl
    -WSL_UTF8=1 wsl.exe ./bin/n2b packages/ui --report jsonl > var/n2b/ui.jsonl
    -WSL_UTF8=1 wsl.exe ./bin/n2b packages/tailwindcss --report jsonl > var/n2b/tailwindcss.jsonl
    -WSL_UTF8=1 wsl.exe ./bin/n2b packages/lit --report jsonl > var/n2b/lit.jsonl

# Full sync sweep across the 4 packages: lint + format-check + n2b report.
sync-packages: sync-packages-lint sync-packages-fmt sync-packages-n2b
