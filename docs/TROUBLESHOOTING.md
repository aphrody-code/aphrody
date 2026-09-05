<!-- SPDX-License-Identifier: Apache-2.0 -->

# Troubleshooting

Common pitfalls hit by engineers working on `aphrody`. Each entry follows a
**Cause** / **Fix** structure. Most of these have already been wired with
mitigations; this page exists to short-circuit the rediscovery loop.

If your issue is not here, see [`FAQ.md`](./FAQ.md) or open a GitHub issue
using `.github/ISSUE_TEMPLATE/bug_report.yml`.

## 1. `aphrody --version` panics with "No provider set"

- **Cause**: rustls 0.23 requires an explicit `CryptoProvider` install before
  any `reqwest::Client::new()` call. Without it the TLS stack panics on first
  use.
- **Fix**: This should NOT happen in the shipped binary — it is already wired
  in `crates/cli/src/main.rs:177` via
  `rustls::crypto::ring::default_provider().install_default()`. If you observe
  the panic you are likely running an outdated binary. Confirm with
  `aphrody doctor` (rustls CryptoProvider section).

## 2. `cargo build` fails with `--icf=all rejected`

- **Cause**: `cargo-zigbuild` and the LLD `--icf=all` flag are incompatible.
  `zigcc` rejects the flag outright.
- **Fix**: Already removed from `.cargo/config.toml` for
  `x86_64-unknown-linux-gnu`. If you re-introduced it locally, drop it.
  `--gc-sections` (still enabled) covers most dead-code stripping.

## 3. `cargo machete` reports unused deps (false positive)

- **Cause**: `cfg`-gated transitive deps that machete cannot see (typical for
  WASM targets, `getrandom` feature plumbing, prost build helpers).
- **Fix**: Add an ignore list to the crate's `Cargo.toml`:
  ```toml
  [package.metadata.cargo-machete]
  ignored = ["wasm-bindgen", "..."]
  ```
  Live examples: `crates/aphrody-wasm/Cargo.toml:45-46`,
  `crates/base/Cargo.toml:25-26`, `crates/a2a-pb/Cargo.toml:47-48`.

## 4. `cargo vet --locked` fails with formatting errors

- **Cause**: `supply-chain/audits.toml` or `supply-chain/config.toml` drifted
  from the canonical form (whitespace, key order, trailing newlines).
- **Fix**: `cargo vet fmt` reformats in place. Caveat: it may strip
  comments — commit (or stash) before running, and review the diff.

## 5. `mrx scan` reports `file_count=0 bytes=0` on Windows

- **Cause** (pre-2026-05-17 build): `workspace_key()` returned back-slash keys
  on Windows, so accumulation under forward-slash bucket lookups silently
  zeroed out.
- **Fix**: Upgrade to a commit that includes the YOLO #51 fix. The
  normaliser now lives in the unified `crates/mrx/` crate (the former
  `mrx-{core,detect,audit,watch,cli}` were merged into it) and is
  cross-tested at `workspace_key_normalises_windows_paths`.

## 6. `wasm-pack build` fails with "Cannot find module run.js"

- **Cause**: Bun's global install layout for `wasm-pack` can desync between
  Windows and Linux, leaving the shim pointing at a missing entry file.
- **Fix**: Direct `cargo build --target wasm32-unknown-unknown --release`
  works in all known cases and is what CI uses. To reinstall the tool:
  `bun install -g wasm-pack`.

## 7. `aphrody doctor` reports "peer winclean: offline" or "DEGRADED"

- **Cause**: The A2A peer Claude is not currently running in `C:\winclean`,
  so heartbeat / inbox checks return stale data.
- **Fix**: By design DEGRADED is non-fatal — `aphrody` operates standalone.
  If you want the peer up, start the listener from the peer repo:
  `bun run C:/winclean/.coord/listener.ts`.

## 8. `docs/SUMMARY.md` is out of date

- **Cause**: Hand edits or a doc was added/removed without regenerating.
- **Fix**: DO NOT hand-edit `docs/SUMMARY.md`. Re-run
  `cargo run -p aphrody-summary` after any doc add, remove, or rename. The
  script is the single source of truth.

## 9. `a2a-pb` build fails with "build scripts must only write to $OUT_DIR"

- **Cause**: `crates/a2a-pb/src/gen/` is the authoritative pre-generated
  source for crates.io publish. The codegen path
  (`tonic_prost_build`) writes outside `$OUT_DIR`, which crates.io rejects;
  it is therefore gated behind the env var `A2A_PB_REGEN=1`.
- **Fix**: Do not set the env var unless you are regenerating protos. Plain
  `cargo build -p a2a-pb` uses the committed `src/gen/` files and works
  everywhere.

## 10. `tokio` will not compile on `wasm32-unknown-unknown`

- **Cause**: tokio's full feature set is not WASM compatible (epoll, mio,
  thread parking, etc.).
- **Fix**: Use selective features. The proven combo is `tokio-stream` +
  `js-sys` + `wasm-bindgen-futures`. See `crates/aphrody-wasm/Cargo.toml` for
  the working pattern.

## 11. GTK3 CVE warnings (RUSTSEC-2024-04xx)

- **Cause**: `tao` / `wry` on Linux pull GTK3 transitively for the desktop
  GUI surface.
- **Fix**: Ignored in `deny.toml` with justification — the `cli` binary is
  NOT linked to GTK. Only `crates/gui` is, and `gui` is excluded from the
  CLI release pipeline. Track GTK4 migration upstream in `tao`.

## 12. `bun install` vs `npm install`

- **Cause**: Repo policy: bun mandatory, node and npm forbidden across all
  JS/TS surfaces (scripts, vendor, MCP, build tooling).
- **Fix**: Always `bun install`, `bun run`, `bun x`, `bun build`. If a
  vendored `package.json` hardcodes `npm`, patch the vendor file rather than
  installing node.

## 13. `cargo check` succeeds but the editor still shows errors

- **Cause**: rust-analyzer keeps a cached diagnostics snapshot from the
  previous session, so fixes applied in this session may not be reflected
  until the LSP re-indexes.
- **Fix**: Run `cargo check --workspace --locked` and then trigger a
  workspace reload in the editor. The shell result is the source of truth,
  not the cached squiggles.

## 14. Got more?

Skim [`FAQ.md`](./FAQ.md) first. If still stuck, open a GitHub issue using
`.github/ISSUE_TEMPLATE/bug_report.yml` and attach the output of
`aphrody doctor`.
