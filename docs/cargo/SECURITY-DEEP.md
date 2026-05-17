<!-- SPDX-License-Identifier: Apache-2.0 -->

# Supply-chain Security -- Deep Dive

> Reference files: supply-chain/config.toml, supply-chain/audits.toml,
> deny.toml, Cargo.lock, .github/workflows/release.yml.

## 1. Overview

Aphrody applies a five-tool defense-in-depth model to its Cargo dependency
tree. cargo-vet tracks per-crate audit attestations sourced from trusted
vendor feeds (Google, Mozilla, Fuchsia, ChromeOS, Bytecode Alliance, Embark
Studios, Zcash). cargo-deny enforces CVE advisories, license compatibility,
banned crates, and trusted source registries in a single gate. cargo-machete
flags dependencies listed in Cargo.toml that are never actually imported,
preventing dead weight from accumulating. cargo-auditable embeds a binary SBOM into every release artifact. All CI workflows pin dtolnay/rust-toolchain to a full commit SHA, blocking tag-hijack attacks.

---

## 2. cargo-vet -- Audited Dependency Tree

cargo-vet requires every crate to carry a signed audit or a documented exemption. Attestations come from vendor feeds or first-party reviews.

Seven feeds are imported in supply-chain/config.toml:

| Import key | Source |
|---|---|
| google | github.com/google/rust-crate-audits |
| mozilla | github.com/mozilla/supply-chain |
| fuchsia | fuchsia.googlesource.com third-party rust crates |
| chromeos | chromium.googlesource.com ChromeOS rust crates |
| bytecode-alliance | github.com/bytecodealliance/wasmtime supply-chain |
| embark-studios | github.com/EmbarkStudios/rust-ecosystem |
| zcash | github.com/zcash/rust-ecosystem supply-chain |

Snapshots are pinned in supply-chain/imports.lock against feed tampering.

### Workflow

Every new dependency must satisfy one of: (1) a matching audit in a trusted feed, (2) a first-party audit via `cargo vet certify`, or (3) a documented exemption in supply-chain/config.toml tracked for later conversion to a real audit.

Refresh snapshots: `cargo vet fetch-imports`. Surface gaps: `cargo vet suggest`.

### First-Party Audits (supply-chain/audits.toml)

Five crates attested by aphrody-code:

| Crate | Version | Criteria | Summary |
|---|---|---|---|
| block-buffer | 0.9.0 | safe-to-run | Two unsafe sub-slice reads; no I/O; no_std; RustCrypto. |
| opaque-debug | 0.3.0 | safe-to-run | Zero unsafe; no_std declarative macro; RustCrypto. |
| subtle | 2.6.1 | safe-to-run | One read_volatile barrier on stack local (sound); no_std; Dalek crypto. |
| wasm-bindgen-futures | 0.4.71 | safe-to-run | Re-export shim; zero own unsafe; rustwasm WG. |
| wasm-bindgen-macro | 0.2.121 | safe-to-run | Compile-time only; no file writes or network; rustwasm WG. |

Custom criteria (crypto-safe, ub-risk-0 through ub-risk-3) follow Fuchsia taxonomy.

Run: `cargo vet --locked`

---

## 3. cargo-deny -- Advisories, Bans, Licenses, Sources

cargo-deny enforces four policy axes over the full resolved graph across all seven target triples in deny.toml.

### Advisories (CVE / Yanked / Unmaintained)

Database: github.com/rustsec/advisory-db. yanked = deny. Schema v2.

Justified CVE ignores currently in deny.toml:

| Advisory | Reason |
|---|---|
| RUSTSEC-2024-0411 through RUSTSEC-2024-0420 (GTK3) | GTK3 deprecation; tao/wry transitive. CLI binary is GTK-free (crates/gui only, excluded from CLI). Tracking wry 1.0 GTK4 migration. |
| RUSTSEC-2024-0370 (proc-macro-error) | Unmaintained; transitive via rmcp. No runtime impact; migrate when alternatives ship. |
| RUSTSEC-2025-0134 (rustls-pemfile) | Unmaintained; optional TLS test dep in a2a-grpc only. Tracking tonic-tls migration upstream. |

### Allowed Licenses

Allowed (deny.toml [licenses]):
MIT, Apache-2.0, Apache-2.0 WITH LLVM-exception, BSD-2-Clause, BSD-3-Clause, BSL-1.0, CC0-1.0, ISC, MPL-2.0, Unicode-3.0, Unicode-DFS-2016, Zlib, OpenSSL, 0BSD, CDLA-Permissive-2.0.

The ring crate carries a clarified expression (MIT AND ISC AND OpenSSL) verified via license-file hash. Unpublished workspace members are exempt from the license check.

### Banned Crates

Hard-banned crates (build fails on entry):

- git2: pulls libgit2 and OpenSSL. Use gix instead.
- wee_alloc: archived 2023-02-28; unbounded memory leak (pages never returned to host). Use the system allocator and wasm-opt -Oz for size.

wildcards = deny in published deps; allow-wildcard-paths = true for workspace path deps.

### Sources

Allowed registry: crates.io only. Allowed git source: github.com/modelcontextprotocol/rust-sdk.git. Any other origin fails.

Run: `cargo deny check`
Expected output: advisories ok | bans ok | licenses ok | sources ok

---

## 4. cargo-machete -- Unused Dependencies

cargo-machete flags declared dependencies that are never imported, preventing bloat and shrinking the auditable surface.

### Known False Positives

cfg-gated and codegen-only deps appear unused to static analysis and are suppressed via [package.metadata.cargo-machete] ignored = [...]:

| Crate | Suppressed dependencies |
|---|---|
| aphrody-wasm | wasm-bindgen, js-sys, web-sys, console_error_panic_hook, console_log, log |
| base | getrandom |
| a2a-pb | pbjson, pbjson-build, protoc-bin-vendored, tonic-prost-build |

Run: `cargo machete --with-metadata`

---

## 5. cargo-auditable -- Embedded SBOM

cargo-auditable embeds the dependency tree as a compressed JSON blob in the binary. Any consumer can inspect it without access to the build environment.

### Building

```bash
cargo auditable build --release -p aphrody --locked
```

For cross-compiled targets:

```bash
cargo auditable zigbuild --release -p aphrody --target x86_64-unknown-linux-gnu --locked
```

### Reading the SBOM

```bash
auditable info target/release/aphrody | jq
auditable info target/release/aphrody | jq '.packages | length'
```

### CI Integration

release.yml installs cargo-auditable via cargo-binstall and wraps each build matrix leg with the auditable subcommand, wired at commit 96ae82e73. SHA-256 checksums are generated alongside each artifact for independent verification.

---

## 6. Pinned Action SHAs

release.yml, codeql.yml, coverage.yml, and cross-platform.yml all pin dtolnay/rust-toolchain to the full commit SHA 5b842231ba77f5c045dba54ac5560fed2db780e2. Tag-based references are deliberately avoided: if @nightly is redirected to a malicious commit, pinned SHAs continue to fetch the reviewed version.

To re-pin after a toolchain update:

```bash
gh api repos/dtolnay/rust-toolchain/branches/nightly --jq .commit.sha
```

Update all four workflow files in a single build(ci): commit to keep the change auditable.

---

## 7. Verifying Yourself

Reproduce the full check from a fresh clone:

```bash
git clone https://github.com/aphrody-code/aphrody
cd aphrody
cargo vet --locked            # expect: unvetted backlog only, no formatting errors
cargo deny check              # expect: advisories ok, bans ok, licenses ok, sources ok
cargo machete --with-metadata # expect: no unused deps outside known false-positives
cargo build --release -p aphrody --locked
auditable info target/release/aphrody | jq '.packages | length'
# expect: 300+ deps embedded in binary SBOM
```

cargo vet output will list pending exemptions (expected); it must not report formatting errors or unknown criteria.

---

## 8. Known Gaps and Roadmap

| Gap | Status | Target |
|---|---|---|
| Unvetted exemptions backlog (865 entries from cargo vet init baseline) | Active -- reducing one per week via dedicated audit(vet): PRs | Ongoing |
| SLSA Level 3+ provenance attestation | Not started | Q3 2026 -- release-please + cosign signing |
| Reproducible builds proof | Not started | Q4 2026 -- deterministic --remap-path-prefix + binary diffing |
| First-party audit feed | Planned | Publish aphrody/rust-crate-audits when backlog drops below 100 exemptions |

---

## 9. Reporting a Supply-Chain Issue

Supply-chain vulnerabilities -- including compromised upstream crates,
malicious transitive dependencies, and advisories not yet surfaced by
cargo deny -- follow the same responsible disclosure process as all security
issues. See SECURITY.md at the repository root for the reporting address,
expected response times, and embargo policy.
