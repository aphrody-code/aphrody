<!-- SPDX-License-Identifier: Apache-2.0 -->
# A2A Extension: `context7-version-pinning/v1`

- **Spec URL**: `https://aphrody.dev/a2a-extensions/context7-version-pinning/v1`
- **Version**: 1.0.0
- **Status**: stable
- **Date**: 2026-05-17
- **License**: Apache-2.0
- **Related**: [ai.json dev journal](../posts/2026-05-ai-json.md),
  [parallel YOLO grind loop](../posts/2026-05-yolo-grind-loop.md),
  [schema](../../schemas/ai.json/v1.json).

## Abstract

`context7-version-pinning/v1` requires an agent to consult the
[`context7`](https://github.com/upstash/context7) MCP server — calling
`resolve-library-id` followed by `query-docs` — before adding any library
dependency or making a non-trivial library API decision. The extension
exists to counter training-data drift, which routinely causes LLMs to
confidently pin stale versions of fast-moving crates, packages, or SDKs.

## Why

LLM training cutoffs lag library release cadence by months or years. Asked
to add `wgpu` to a workspace, an agent will cheerfully write
`wgpu = "0.20"` based on what was current when its weights were frozen,
even though the upstream is on v26 with a different API surface and the
old version no longer compiles against current `wasm-bindgen`. The same
failure mode applies to `next.js`, `react`, `tonic`, `tokio`, `rustls`,
`reqwest`, `swc`, `turbopack`, and every other dependency that ships a
new minor every few weeks.

The fix is mechanical: before pinning, the agent fetches authoritative
docs from the live registry, reads what version is current, and which
API the current version expects.

## Rule

Any commit that adds or bumps a dependency in any of the following files
MUST reference a `context7` check in the commit body or the originating
PR description:

- `Cargo.toml` workspace `[workspace.dependencies]` or per-crate
  `[dependencies]`.
- `package.json` `dependencies` / `devDependencies` / `peerDependencies`.
- Equivalent manifests in other ecosystems (`*.csproj`, `pyproject.toml`,
  `go.mod`, `pubspec.yaml`, `Gemfile`, etc.).

A "non-trivial library usage" — defined as code that calls more than a
hello-world surface of an external library API, or that crosses a major
version boundary — is also in scope. Recording the check as a single
line such as `context7: wgpu -> v26.0.1 (resolved 2026-05-17)` is
sufficient.

The check is unnecessary for: refactoring local code, scripts written
from scratch against an internal library, debug-only changes to business
logic, and general programming concepts.

## Scope

This extension applies to **autonomous agents**. Humans are encouraged to
use the same workflow when it would catch a stale assumption, but are not
required to: the human cost of mislabelling is high relative to the
benefit, and humans typically open the registry page directly anyway.

## Conformance

An agent advertising `context7-version-pinning/v1` in its
`AgentCard.extensions` MUST perform the `resolve-library-id` +
`query-docs` lookup before any in-scope commit, MUST record the
verification in the commit body or PR description, and MUST NOT pin a
version that the current `query-docs` result contradicts without an
explicit override note explaining why the older pin is necessary
(transitive constraint, upstream regression, etc.).
