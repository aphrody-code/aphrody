<!-- SPDX-License-Identifier: Apache-2.0 -->
# aphrody Security Model

Threat model complementing the disclosure process in
[`SECURITY.md`](../SECURITY.md). It enumerates assets, trust boundaries,
attacker classes, and a STRIDE matrix mapping mitigations to threats. Protocol
references resolve to [`docs/extensions/file-transport-v1.md`](extensions/file-transport-v1.md)
and the forthcoming `docs/PROTOCOL.md` (tracked in `docs/PLAN.md`).

## 1. Scope

In scope:

- The `aphrody` CLI binary (Linux x86_64, Windows x86_64, future arm64).
- The `aphrody-wasm` browser library targeting `wasm32-unknown-unknown`.
- The A2A file + HTTP coordination protocol (`ai.json` manifest, `.coord/`
  JSONL mailbox, listener on `:8788`).
- The published crates.io artifacts under the `aphrody-code/*` namespace.

Out of scope:

- The host operating system (Linux kernel, Windows kernel, browser engine).
- Third-party LLM APIs called by user-supplied scripts or skills.
- The user's clipboard, screen contents, or input devices.
- Vendored upstreams under `vendor/` (Bun fork, Electron prebuilt) — report
  to their respective projects.

## 2. Assets

What an attacker might target:

- **A1** — User home directory. The CLI inherits full filesystem access from
  the invoking shell.
- **A2** — Outbound network. The CLI performs DNS resolution, scrape fetches,
  and A2A HTTP peering.
- **A3** — Local A2A inbox JSONL files under `.coord/` (contain coordination
  history, peer envelopes, heartbeat traces).
- **A4** — `Cargo.lock` and `Cargo.toml` integrity (supply chain root of trust
  for every reproducible build).
- **A5** — WASM module integrity (browser users execute the binary directly
  in their tab).
- **A6** — `aphrody doctor --json` output (diagnostic surface that may carry
  peer state hints, hostnames, version strings).
- **A7** — Adjacent `.env` / credential files. aphrody **never** reads them by
  design, but they sit on disk in directories aphrody can walk.

## 3. Trust boundaries

Four boundaries are crossed at runtime:

- **TB1** — User to aphrody binary. The user trusts the compiled binary they
  installed (signature / checksum at install time).
- **TB2** — aphrody binary to filesystem. The binary runs with the user's UID
  and inherits its FS rights; no privilege escalation occurs.
- **TB3** — aphrody binary to peer agent over A2A. Trust is mediated by the
  signed manifest plus heartbeat verification described in
  [`docs/extensions/file-transport-v1.md`](extensions/file-transport-v1.md).
- **TB4** — aphrody binary to third-party HTTP (DNS resolvers, scrape targets,
  registry mirrors). No trust assumed; all output is validated against schema
  before use.

## 4. Attacker classes

- **AC1** — Local unprivileged user on the same machine. Can read the user's
  files (subject to FS permissions) and invoke binaries.
- **AC2** — Remote network attacker. Can attempt MITM if the user pulls data
  from a non-TLS source or if certificate validation is disabled.
- **AC3** — Malicious peer agent. Speaks the A2A protocol correctly but lies
  about identity, capabilities, or work state.
- **AC4** — Supply-chain attacker. Pushes a compromised release to an upstream
  dependency we transitively pull (direct or indirect).
- **AC5** — Malicious crates.io / npm publisher. Typosquats the `aphrody`
  name (e.g. `aprody`, `aphroddy`) to capture mis-typed installs.

## 5. STRIDE matrix

The table below applies the STRIDE classification (Spoofing, Tampering,
Repudiation, Information disclosure, Denial of service, Elevation of
privilege) to representative asset by attacker pairs.

| # | Threat | Asset | Attacker | Mitigation |
|---|---|---|---|---|
| T01 | Spoofing | A3 (inbox) | AC3 | Per-agent write allocation enforced in the A2A protocol; envelopes carry an agent id checked against the manifest. |
| T02 | Spoofing | A1 (binary identity) | AC5 | Crates.io owner lock on `aphrody-*` names; GitHub release artifacts ship cosign signatures. |
| T03 | Tampering | A4 (`Cargo.lock`) | AC4 | `cargo-vet` audits + `cargo-deny advisories` + `--locked` enforced across every workflow and local alias. |
| T04 | Tampering | A5 (WASM module) | AC4 | Subresource Integrity (SRI) hashes in `pkg.json` (TODO, tracked in section 8). |
| T05 | Tampering | A2 (HTTP body) | AC2 | rustls 0.23 with the `ring` provider; no `accept-invalid-certs` flag is exposed. |
| T06 | Repudiation | A3 (inbox) | AC3 | JSONL append-only with monotonic envelope sequence numbers; heartbeat file gives an out-of-band timeline anchor. |
| T07 | Information disclosure | A6 (doctor output) | AC1 | `--json` omits secrets by design (no env values, no token bytes); users review before sharing. |
| T08 | Information disclosure | A7 (.env adjacent) | AC1 | aphrody walkers honour `.gitignore`-style allowlists and never open files matching `*.env`, `*.pem`, `id_*`. |
| T09 | Denial of service | A3 (inbox) | AC3 | Append-rate-limit at the HTTP listener (TODO: hard-cap 100 envelopes/sec); manifest declares a max inbox size. |
| T10 | Denial of service | A2 (network) | AC2 | Per-host concurrent connection cap, exponential backoff on 5xx, hard timeout per request. |
| T11 | Elevation of privilege | TB2 (FS) | AC1 | aphrody runs as the invoking user; no setuid, no helper daemon, no sudo prompts. |
| T12 | Elevation of privilege | A5 (WASM) | AC4 | Browser sandbox owns the WASM execution context; aphrody-wasm requests no extra capabilities. |

## 6. Out-of-band trust (the A2A handshake)

The 3-deep ack handshake documented in
[`docs/extensions/file-transport-v1.md`](extensions/file-transport-v1.md)
proves **liveness** (the peer is alive and observing the shared mailbox) but
does **not** prove identity. Future work: sign manifests with a long-lived
agent key (rotation policy TBD, target every 90 days) and verify that
signature on the first `ask` exchange. Current state is trust-on-first-use
(TOFU): the manifest fingerprint observed on first contact is pinned for the
lifetime of the local `.coord/` directory; any later manifest change requires
explicit operator acknowledgement.

## 7. Mitigations currently in place

- **Supply chain**: `cargo-vet` (Google/Mozilla/Fuchsia/ISRG imports),
  `cargo-deny` (advisories, bans, licenses, sources), `cargo-machete` and
  `cargo audit-machete` in CI.
- **Build integrity**: `cargo-auditable` embeds an SBOM in the binary;
  `dtolnay/rust-toolchain` action pinned by SHA in every workflow.
- **Web**: rustls 0.23 with the `ring` provider initialised before the first
  `reqwest::Client::new()` call (see `crates/cli/src/main.rs`).
- **Crypto**: AES-GCM via the `aes-gcm` crate (RFC 5116); no hand-rolled
  primitives.
- **Sandboxing**: Flatpak manifest declares minimal `finish-args`
  (`--share=network --filesystem=home` only); no session bus, no device passthrough.

## 8. Known gaps and roadmap

- No SRI hash on the published WASM bundle. Planned Q3 2026 with the first
  signed `aphrody-wasm` npm release.
- No manifest signing. Planned Q4 2026; long-lived per-agent Ed25519 key
  with annotated rotation log.
- Inbox JSONL has no rate limit on file-based writes (only the HTTP overlay
  benefits from OS-level connection limits). A file-watcher rate limiter is
  tracked in `docs/PLAN.md`.
- No production fuzzing of the envelope parser. A `cargo-fuzz` skeleton is
  checked in; targets for `envelope.parse`, `manifest.load`, `coord.append`
  are pending.

## 9. Reporting a finding

The disclosure process, supported branches, and PGP key live in
[`SECURITY.md`](../SECURITY.md). Please use the private channels listed
there; do not file public issues for unfixed vulnerabilities.
