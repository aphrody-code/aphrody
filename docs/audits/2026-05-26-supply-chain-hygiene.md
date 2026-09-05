# Supply-chain hygiene pass --- deny.toml ignores + cargo-vet store

- Date: 2026-05-26
- Scope: `deny.toml` advisory `ignore` list audit + cargo-vet store investigation.
- Baseline commit: `79df9b265` (branch `main`).
- Tooling: cargo-deny (advisories/bans/licenses/sources), cargo-vet 0.10.2,
  `cargo tree -i` on the full workspace (`--workspace --all-features`).
- Result: `cargo deny check` stays green (exit 0); stale-ignore warnings
  eliminated; cargo-vet store finding documented (no destructive action taken).

## 1. deny.toml [advisories].ignore audit

`cargo deny check advisories` flagged four ignores as `advisory-not-detected`
(`no crate matched advisory criteria`). Each was confirmed absent (or
no-longer-vulnerable) via `cargo tree -i` and removed. The other five ignores
produced no warning and were verified still-present; they are kept verbatim.

### 1.1 REMOVED (4) --- stale, no crate matched

| Advisory | Crate (reason cited) | cargo tree -i verdict | Why stale |
|----------|----------------------|-----------------------|-----------|
| RUSTSEC-2025-0141 | bincode 1.3.3 (via syntect) | `bincode@1.3.3` did not match any packages; only `bincode v2.0.1` (bgw fork, via lindera/lance/yara-x) present | syntect/bincode-1 chain gone; advisory targets 1.x, not the v2 fork |
| RUSTSEC-2024-0320 | yaml-rust (via serde_yaml 0.9) | `yaml-rust` and `yaml-rust2` did not match any packages | crate absent from graph entirely |
| RUSTSEC-2025-0052 | async-std (via dark-light -> mui-rs) | `async-std`, `dark-light`, `mui-rs` all did not match any packages | dark-light + mui-rs extracted to sibling repo aphrody-ts on 2026-05-23 |
| RUSTSEC-2026-0114 | wasmtime (via mui-rs) | `wasmtime v43.0.2` present, but via yara-x -> aphrody-re (NOT mui-rs) | advisory fix is ">=43.0.2"; graph already on patched v43.0.2, so it no longer matches |

Proof excerpts (`cargo tree -i <crate> --workspace --all-features`):

```text
$ cargo tree -i bincode@1.3.3
error: package ID specification `bincode@1.3.3` did not match any packages
help: there are similar package ID specifications:
  bincode@2.0.1

$ cargo tree -i yaml-rust
error: package ID specification `yaml-rust` did not match any packages

$ cargo tree -i async-std
error: package ID specification `async-std` did not match any packages

$ cargo tree -i mui-rs
error: package ID specification `mui-rs` did not match any packages

$ cargo tree -i wasmtime
wasmtime v43.0.2
  └── yara-x v1.16.0
      └── aphrody-re v1.0.0-canary (crates/aphrody-re)
          ├── aphrody v1.0.0-canary (crates/cli)
          └── google_mcp v1.0.0-canary (crates/google_mcp)
```

Note on RUSTSEC-2026-0114: the old ignore comment claimed wasmtime was
"v42.0.2 pulled by mui-rs". Both facts are now wrong --- the live graph has
wasmtime v43.0.2 pulled by yara-x, and v43.0.2 is exactly the patched release
the advisory points to (">=36.0.8 or >=43.0.2"). cargo-deny therefore reports
no match, so the ignore is dead weight and was removed.

### 1.2 KEPT (5) --- still present in the graph, no warning

Each crate below was confirmed live via `cargo tree -i`. These are genuine
transitive unmaintained/advisory hits with no safe upgrade path and no runtime
exploit path in aphrody; they remain ignored with refreshed dep-chain comments.

| Advisory | Crate (version) | Live dependency chain (top of cargo tree -i) |
|----------|-----------------|----------------------------------------------|
| RUSTSEC-2025-0134 | rustls-pemfile v2.2.0 | rustls-pemfile -> a2a-grpc (optional TLS/test path) |
| RUSTSEC-2021-0153 | encoding v0.2.33 | encoding -> lindera-dictionary -> lindera -> lance-tokenizer -> lance -> lancedb -> aphrody-memory |
| RUSTSEC-2024-0436 | paste v1.0.15 | paste -> datafusion-common -> datafusion -> lance -> lancedb -> aphrody-memory |
| RUSTSEC-2023-0071 | rsa v0.9.10 | rsa -> jsonwebtoken -> octocrab -> aphrody-marketplace |
| RUSTSEC-2017-0008 | serial v0.4.0 | serial -> portable-pty -> aphrody-terminal-backend -> aphrody |

Comment corrections applied while keeping these entries (id and ignore
behaviour unchanged --- comment-only fixes to reflect the live chains):

- RUSTSEC-2021-0153 reason was "transitive via quick-xml"; the live chain is
  lindera/lance/lancedb. Comment updated.
- RUSTSEC-2023-0071 reason said "transitive via lancedb"; the live RSA path is
  octocrab -> jsonwebtoken. Comment updated.
- RUSTSEC-2017-0008 reason said "transitive via testing = 22 dev-dep"; the live
  path is portable-pty -> aphrody-terminal-backend. Comment updated.

### 1.3 Ignore count

- Before: 9 ignores. After: 5 ignores.
- CLAUDE.md target is "CVE ignores < 5". Five is the verified minimum that does
  not regress `cargo deny check`: all five still match a live crate and cannot
  be dropped without an upstream upgrade or removing the consuming feature.
  Driving below five requires upstream movement (tracked):
  - rustls-pemfile: blocked until tonic-tls migrates to rustls-pki-types.
  - encoding / paste: blocked on lance/lancedb/datafusion upgrades.
  - rsa: blocked on octocrab/jsonwebtoken dropping the rsa backend.
  - serial: blocked on portable-pty dropping the serial crate.

## 2. cargo-vet store investigation

### 2.1 Symptom

`cargo vet suggest` fails with `store not found at C:\src\aphrody\supply-chain`.
The canonical root store directory `supply-chain/` does not exist on disk.

### 2.2 What var/tauri/supply-chain/ actually is (NOT the aphrody store)

A cargo-vet store exists under `var/tauri/supply-chain/` (audits.toml,
config.toml, imports.lock). It is a STRAY artifact, not a misplaced copy of
aphrody's store:

- config.toml is the Tauri upstream project's own vet store: it carries
  `[policy.tauri]`, `[policy.tauri-build]`, `[policy.tauri-codegen]`, ... and
  Tauri-specific exemptions (gtk, gtk-sys, webkit2gtk, tao, muda, tray-icon,
  window-vibrancy, javascriptcore-rs, soup3, ...).
- Its pins are ancient and unrelated to aphrody's real graph: tauri
  2.0.0-beta.18, reqwest 0.11.24, windows 0.48, rustls 0.21.10. The aphrody
  graph today is lance 6.0 / datafusion 53 / octocrab 0.50 / yara-x 1.16 /
  rustls 0.23 --- no overlap.
- `var/` is gitignored (.gitignore line 62: `var/`), so this store is untracked
  scratch from a Tauri vendoring/build under var/tauri/.

Conclusion: copying or promoting var/tauri/supply-chain to the repo root would
import the WRONG policy and a foreign project's exemptions. It must not be used
as aphrody's store. No action taken on it (var/ is out of scope and explicitly
off-limits for deletion).

### 2.3 The real root store WAS tracked and was deleted (likely collateral)

The canonical root supply-chain/ store did exist and was tracked in git, and
was actively maintained:

- `b1ef8f04b chore(supply-chain): fix cargo vet formatting drift in audits.toml and config.toml`
- e787e15c3, b22850981, 96ae82e73, 3100fcc5a, efa4477e7 --- all touched supply-chain/.

It was deleted on 2026-05-19 by:

```text
ad0946691 chore(docs): clean hallucinations and obsolete files
  supply-chain/audits.toml    |  182 -
  supply-chain/config.toml    | 3527 ---
  supply-chain/imports.lock   | 2117 --
```

That commit's stated intent was removing docs/bun-docs/google-os-plan and other
obsolete files; the supply-chain store deletion looks like collateral damage in
that sweep. Strong corroborating evidence: .gitignore line 17 still reads
`# NOTE: supply-chain/imports.lock IS tracked (cargo-vet feed pins).` --- a now
dangling note that the root store is supposed to exist and be tracked.

### 2.4 Recommendation (no action taken --- out of scope + needs validation)

Re-establishing the store is plausibly the right fix but is NOT trivially safe
and is outside this pass's file scope (only deny.toml + this doc were in scope).
Recommended follow-up, to be done deliberately in a dedicated PR:

1. Restore the canonical store from just before deletion:
   `git checkout ad0946691^ -- supply-chain/`
   (brings back audits.toml / config.toml / imports.lock at last-good state).
2. Re-run `cargo vet` against today's graph; the graph changed massively since
   2026-05-19 (UI crates extracted, lance/datafusion/octocrab added), so expect
   many new unaudited crates. Triage with `cargo vet suggest` and either import
   audits (Google/Mozilla/Fuchsia/ISRG/Embark/Zcash feeds, already wired in the
   old config) or add justified exemptions.
3. Keep var/tauri/supply-chain/ untouched (foreign Tauri store; gitignored). A
   later cleanup PR could remove it, but that is a var/ deletion forbidden here.
4. If the project decides NOT to restore cargo-vet, then .gitignore line 17 (the
   dangling note) and the audit-vet alias should be cleaned up so the tooling
   story is consistent. That is a separate decision.

Until one of those is done, `cargo deny check` remains the active supply-chain
gate (green), and `cargo vet` is effectively offline (no store). cargo-deny
covers CVEs + licenses + bans + sources, so there is no security regression from
the missing vet store --- only the loss of signed-audit attestations.

## 3. Final verification --- cargo deny check exit 0

```text
$ cargo deny check
# 44 non-fatal warnings: multiple-versions=warn duplicates + 2 unmatched
# license allowances (OpenSSL, Unicode-DFS-2016) --- all pre-existing,
# unrelated to this change.
advisories ok, bans ok, licenses ok, sources ok
exit code: 0
```

- advisory-not-detected warnings after the edit: 0 (was 4).
- error lines: 0.
- Sections: advisories ok, bans ok, licenses ok, sources ok.

## 4. Files changed

- deny.toml --- removed 4 stale [advisories].ignore entries (RUSTSEC-2025-0141,
  RUSTSEC-2024-0320, RUSTSEC-2025-0052, RUSTSEC-2026-0114); refreshed dep-chain
  comments on 3 kept entries (RUSTSEC-2021-0153, RUSTSEC-2023-0071,
  RUSTSEC-2017-0008); added a dated rationale block.
- docs/audits/2026-05-26-supply-chain-hygiene.md --- this report.

No .rs, no Cargo.toml, no var/ files were modified. No commit, no branch switch.
