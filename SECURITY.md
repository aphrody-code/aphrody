<!-- SPDX-License-Identifier: Apache-2.0 -->
# Security Policy

## Supported versions

| Branch / line   | Status            | Notes                                                  |
|-----------------|-------------------|--------------------------------------------------------|
| `main`          | Yes               | Active development. All security fixes land here first.|
| `stable`        | Yes               | Latest tagged release line. Backports accepted.        |
| `v0.1.x`        | Best-effort       | Current canary minor. Patched while it remains current.|
| Older `v0.0.x`  | No                | Out of support. Upgrade to `v0.1.x` or `stable`.       |

## Reporting a vulnerability

**Do not open a public GitHub issue.** Use one of the following private channels,
in preference order:

1. **Preferred** — file a private advisory through
   [GitHub Security Advisories](https://github.com/aphrody-code/aphrody/security/advisories/new).
   This keeps the report attached to the affected repository and lets us
   collaborate on a fix without disclosure.
2. **Email** — write to `security@aphrody.dev`. Use the PGP key below if you
   need transport encryption.

Include a minimal reproducer, the affected commit hash, and the platform
(Linux / Windows / wasm) you observed the issue on.

## Expected response time

- **Initial acknowledgement**: within **48 hours** (best-effort within
  **1 week** for off-hours weekends or holidays).
- **Triage decision**: within **5 business days**.
- **Coordinated disclosure window**: **30 days** from acknowledgement, unless
  mutually extended (high-severity supply-chain issues may require longer).

## Scope

**In scope:**

- The published `aphrody` binary and the published crates under the
  `aphrody-code/*` namespace.
- The A2A protocol surface (`ai.json` manifest, `.coord` mailbox listener,
  envelope schema) shipped from this repository.
- Supply-chain integrity of declared dependencies (`Cargo.lock`, `bun.lock`).
- Credential or token leakage in repository content or release artifacts.

**Out of scope:**

- Vendored upstreams under `vendor/` (Bun fork, Electron prebuilt) — report
  directly to their respective upstream projects.
- Archived crates listed in `CLAUDE.md` (`google_os`, `bun_ffi`, `python_ffi`,
  `google_kv`, `n2b` legacy) — they are no longer built or distributed.
- Deprecated branches (anything not listed as `Yes` / `Best-effort` above).
- Social-engineering, physical access, or denial-of-service via brute force
  against third-party infrastructure.
- Self-hosted infrastructure (VPS, A2A peer at `C:\winclean\`) — those are
  operator-specific and out of repository scope.

## Safe harbor

We will not pursue legal action against good-faith security research that:

- Respects the disclosure window above.
- Does not access, modify, or exfiltrate data belonging to other users.
- Does not degrade service for other users.

If in doubt, ask first via `security@aphrody.dev` — we would rather clarify
scope than discourage a report.

## PGP key

A PGP key for `security@aphrody.dev` will be published at
`https://aphrody.dev/.well-known/pgp-key.txt` — _pending publication_.
Until then, GitHub Security Advisories provide an equivalent confidential
channel.

## Acknowledgement policy

Researchers who responsibly disclose are credited in
[`SECURITY-HALL-OF-FAME.md`](./SECURITY-HALL-OF-FAME.md) and in the release
notes of the patched version, unless they request to remain anonymous.
