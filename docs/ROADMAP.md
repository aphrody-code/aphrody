<!-- SPDX-License-Identifier: Apache-2.0 -->

# Roadmap

A 12-month outlook for aphrody, broken down by quarter. These are targets,
not promises — release dates slip, scope changes, and an unexpected CVE can
reshuffle a quarter. We publish the plan anyway because the alternative is
worse. For background on what aphrody is and why it exists, start at
[`SOURCE_OF_TRUTH.md`](./SOURCE_OF_TRUTH.md) and
[`COMPARISON.md`](./COMPARISON.md).

## Q2 2026 (current — May to June)

Stabilise `v1.0.0-canary` and execute the crates.io publish ladder. Publish
order: `base` first (no_std primitives, smallest blast radius), then
`a2a-pb` (the protobuf generated code, frozen API surface), then
`aphrody` itself (the user-facing CLI). Show HN launch lands in D+15;
the public read-out targets one thousand GitHub stars and the start of
an organic contributor pipeline. Internal milestone: the YOLO grind loop
runs unattended across overnight cycles without manual intervention (the
mode is implemented as the `/aphrody-yolo-grind` skill under `.claude/skills/`).

## Q3 2026 (July to September)

Ship `v1.0.0` stable. Cut a release line, freeze the public API surface,
and commit to backporting security fixes for at least two minor versions.
Distribution surface widens: a Snap package for Ubuntu, an AUR entry for
Arch, and a Homebrew tap for macOS users who want a best-effort build.
The devcontainer image goes GA. Star target: ten thousand. Contributor
target: fifty distinct authors with merged PRs. The stable line also
unlocks the formal third-party audit window we have been deferring.

## Q4 2026 (October to December)

Adopt the AGNTCY `a2a/v0.5` spec once it lands, with backward compatibility
for `v0.4` peers during the migration window. WebGPU support arrives in
`aphrody-wasm` via the `wgpu` crate, enabling browser-side render workloads
driven from the CLI. The MRX (Monorepo Real-time X-platform mapper) daemon
goes GA — `aphrody mrx watch` will run as a long-lived process emitting
filesystem-change deltas over the A2A channel. Star target: fifty
thousand. This is also the quarter we expect to be invited (or rejected)
into the AGNTCY consortium.

## Q1 2027 (January to March)

AGNTCY consortium membership formalised. Multi-language agent support lands
behind a stable FFI: Python, Go, and TypeScript bindings expose the A2A
protocol without requiring a Rust toolchain. The official MCP (Model
Context Protocol) bridge ships, replacing the current third-party adapter
we vendor. Star target: one hundred thousand. Quarter-end check: enough
external usage and feedback to plan a `v2.0` cycle with confidence.

## Cross-cutting commitments

These hold across every quarter and are non-negotiable:

- Linux Ubuntu 26.04 remains build target #1. A regression on Linux blocks
  the release; Windows or WASM regressions do not.
- Zero telemetry. Period. See [`FAQ.md`](./FAQ.md) for the rationale.
- Supply-chain hygiene: `cargo deny check` and `cargo vet` must stay green
  on `main` at all times. See [`SECURITY.md`](../SECURITY.md) for the
  vulnerability disclosure policy.
- All public docs follow the cross-link style established here — answers
  point to the source-of-truth file rather than duplicating prose.

Pull requests proposing changes to this roadmap are welcome — open an
issue first to discuss.
