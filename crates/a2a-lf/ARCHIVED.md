<!-- SPDX-License-Identifier: Apache-2.0 -->

# crates/a2a-lf/ — Status: documentation-only landing directory

**Archived / pending-decision flag date:** 2026-05-18

## What this directory is

This directory contains a single file (`README.md`) that serves as an
alternate documentation landing page for the crates.io published name
`a2a-lf`. It is **not** a Cargo package: there is no `Cargo.toml`, no `src/`,
and no entry in the workspace `[workspace.members]` list of the repository
root `Cargo.toml`.

The actual source for the published crate `a2a-lf` lives in
[`crates/a2a/`](../a2a/) (note: `crates/a2a/Cargo.toml` declares
`name = "a2a-lf"`; the workspace alias `a2a = { package = "a2a-lf",
path = "crates/a2a" }` is defined at root `Cargo.toml`).

The `lf` suffix is inherited from the upstream `a2a-rs` workspace naming
convention (paired with `a2a-client-lf`, `a2a-server-lf`). It is **not** a
lock-free runtime layer.

## Why the directory still exists

- The README is referenced from sibling crate documentation
  (`crates/a2a/README.md` "Related" section, line 80: `a2a-lf — same crate,
  alternate doc landing page for the published name`).
- The prior repository dedup + cohesion audit
  (`docs/audits/2026-05-18-dedup-cohesion-sweep.md` row 62, action P0-1)
  flagged this directory as a candidate for deletion (`rm -rf
  crates/a2a-lf/`) because no workspace member depends on it as a path
  dependency and `cargo metadata --no-deps` returns no entry for the
  directory.

These two signals are not contradictory but they require a human decision:
keep the alternate landing page (useful for users who search by the
published name) or remove it (less filesystem confusion, fewer "is this a
missing-Cargo.toml package?" tickets).

## Decision recipe — if a maintainer chooses to delete later

```bash
cd C:/src/aphrody

# 1. Remove cross-reference from sibling README before deleting,
#    otherwise crates/a2a/README.md:80 will point at a 404.
#    Edit crates/a2a/README.md and drop the "- `a2a-lf` — same crate, ..."
#    bullet from the "Related" section.

# 2. Git-aware removal (preserves history).
git rm -rf crates/a2a-lf/

# 3. Verify nothing in the workspace resolves a path dep to crates/a2a-lf/.
cargo metadata --no-deps --format-version 1 \
  | grep -F '"crates/a2a-lf' \
  && echo "STILL REFERENCED — abort" \
  || echo "no path-dep references — safe"

# 4. Workspace must still build (offline-friendly).
cargo check --workspace --locked --offline
```

## What to NOT touch

- `crates/a2a/` — that **is** the published `a2a-lf` package. Never delete
  or rename it without an upstream coordination plan; the crates.io name is
  load-bearing for downstream consumers.
- Cargo.lock entries for `a2a-lf` — those refer to the package name, not to
  this directory.
- `supply-chain/config.toml` policies and exemptions named `a2a-lf` — same
  reason, those are package-name keyed.

## License

Apache-2.0. Copyright AGNTCY Contributors; aphrody-code packaging.
