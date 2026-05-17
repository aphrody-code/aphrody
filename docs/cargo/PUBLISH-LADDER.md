<!-- SPDX-License-Identifier: Apache-2.0 -->

# Publish Ladder Runbook

Operational guide the maintainer follows for every release of the eight
publishable aphrody crates. Read top-to-bottom; do not skip rungs.

Cross-references:

- [`.github/workflows/release-please.yml`](../../.github/workflows/release-please.yml) — Conventional-Commit driven bump PRs.
- [`.github/workflows/release.yml`](../../.github/workflows/release.yml) — binary artifact build on `v*.*.*` tags.
- [`docs/cargo/SECURITY-DEEP.md`](SECURITY-DEEP.md) — supply-chain checklist (pending — see PLAN).
- [`docs/cargo/SUPPLY_CHAIN.md`](SUPPLY_CHAIN.md) — cargo-deny / cargo-vet policy.

## 1. Why a ladder

crates.io publishes are immutable. Once `foo 1.2.3` is uploaded it stays
forever. If `foo` depends on `bar` and `bar` is not yet on crates.io,
`cargo publish foo` rejects the upload because the registry resolver
cannot find the dep. The ladder fixes this by ordering uploads: leaf
crates first, then upstream consumers, one rung at a time.

## 2. The 8-rung ladder (topological)

Package names below are the cargo package name, not the directory name
(the `crates/a2a/`, `crates/a2a-client/`, `crates/a2a-server/`, and
`crates/cli/` dirs publish as `a2a-lf`, `a2a-client-lf`, `a2a-server-lf`,
and `aphrody` respectively).

1. `base` — `publish = true`, leaf, no aphrody-code internal deps.
2. `a2a-lf` — leaf, no internal deps (external `a2a-rs` ecosystem only).
3. `a2a-pb` — leaf for the gRPC stack; ships pre-generated `src/gen/`.
4. `a2a-client-lf` — depends on `a2a-pb`.
5. `a2a-server-lf` — depends on `a2a-pb`.
6. `a2a-grpc` — depends on `a2a-pb`, `a2a-client-lf`, `a2a-server-lf`.
7. `backend` — depends on `base` (currently `publish = false`; flip
   before first crates.io upload).
8. `aphrody-translate` — no internal deps; can publish any time.
9. `aphrody` — top of ladder; depends on `base` + `backend` (and
   transitively on the a2a stack). Currently `publish = false`; flip
   only once every rung above is on crates.io.

`aphrody-wasm` and the `mrx-*` family are independent ladders covered by
separate runbooks; do not mix them with the eight rungs above.

## 3. Pre-publish gates (run for EACH crate before `cargo publish`)

```bash
cargo build --release -p <name> --locked --offline
cargo clippy -p <name> --all-targets --locked -- -D warnings
cargo doc -p <name> --no-deps --locked
cargo package -p <name> --list --allow-dirty
cargo package -p <name> --allow-dirty
cargo publish -p <name> --dry-run --allow-dirty --locked
```

Inspect dry-run output for warnings: missing `description`, missing
`license`, unresolved path-only deps, oversized tarball.

## 4. Publish workflow (per rung, in order)

```bash
cargo login <token-from-crates.io-account-settings>
cargo publish -p <name> --locked
sleep 30
curl -s "https://crates.io/api/v1/crates/<name>" | jq '.crate.max_version'
```

The printed `max_version` must equal the version just pushed before
proceeding to the next rung.

## 5. Yank policy

Only yank for: critical CVE without a same-day fix, or an accidental
broken upload caught within one hour.

```bash
cargo yank --vers <version> -p <name>
cargo yank --vers <version> --undo -p <name>
```

## 6. Re-publish bumped versions

crates.io is immutable; to fix a release, bump the version and re-publish.
Use Conventional Commits with release-please
(`.github/workflows/release-please.yml`):

- `fix:` triggers a patch bump.
- `feat:` triggers a minor bump.
- `feat!:` or a `BREAKING CHANGE:` footer triggers a major bump.

## 7. crates.io API rate limits

- Authenticated publish: 5 per minute per crate, 10 per minute global.
- Unauthenticated metadata API: 100 per minute per source IP.

## 8. Automation status

Currently MANUAL. The maintainer runs each rung by hand. Tracked in
PLAN.md: `cargo-publish.yml` workflow that orchestrates the ladder
from a release-please bump PR merge.

## 9. Known per-crate quirks

- `base` — Windows-gated features pull `windows-rs`. Run
  `cargo check -p base --target x86_64-pc-windows-msvc --locked`
  separately before publishing.
- `a2a-pb` — DO NOT export `A2A_PB_REGEN=1` during `cargo publish`.
  crates.io rejects any build script that writes outside `$OUT_DIR`.
  Ship the committed `src/gen/` as authoritative.
- `a2a-server-lf` — depends only on `a2a-pb` (not on `a2a-client-lf`);
  publish rungs 4 and 5 in either order if `a2a-pb` is already live.
- `a2a-grpc` — features `native-tls` / `rustls-tls` reference both
  `a2a-client-lf` and `a2a-server-lf` features; both must be live first.
- `backend` — manifest currently has `publish = false`. Flip to `true`,
  ensure the `base` dep declares both `path` and `version`, commit, then
  publish.
- `aphrody` — bottom of the ladder. Verify every transitive internal
  dep is on crates.io before flipping `publish = true` in
  `crates/cli/Cargo.toml`.

## 10. Rollback

A failed `cargo publish` leaves no state on crates.io; the upload is
atomic per crate. Fix the manifest or source, re-run the gates in
section 3, then retry the publish.
