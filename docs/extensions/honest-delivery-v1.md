<!-- SPDX-License-Identifier: Apache-2.0 -->
# A2A Extension: `honest-delivery/v1`

- **Spec URL**: `https://aphrody.dev/a2a-extensions/honest-delivery/v1`
- **Version**: 1.0.0
- **Status**: stable
- **Date**: 2026-05-17
- **License**: Apache-2.0
- **Related**: [ai.json dev journal](../posts/2026-05-ai-json.md),
  [parallel YOLO grind loop](../posts/2026-05-yolo-grind-loop.md),
  [schema](../../schemas/ai.json/v1.json).

## Abstract

`honest-delivery/v1` enforces per-deliverable classification in every commit
message produced by autonomous loops. Each claimed deliverable carries one
of three tri-state labels — `FAIT`, `INCOMPLET`, or `NON_FAIT` — together
with the justification shape required for that state. Agents and humans
reading the log can immediately separate verified progress from optimistic
work-in-progress.

## Why

Long autonomous runs naturally inflate. After eight hours of grinding, a
loop summary that says "shipped 4 features" routinely covers one merged
crate, one placeholder behind a feature flag, one blocker the agent worked
around, and one item it forgot. The cost is paid the next morning when a
human re-reads the log and discovers half of it is aspirational. This
extension makes inflation structurally impossible by requiring proof.

## Format

Each deliverable claim is annotated with exactly one of three labels:

- **`FAIT`** (done) — A verifiable artifact MUST accompany the claim.
  Acceptable artifacts include: a commit SHA that introduced it, a file
  path that exists on disk, a `cargo check` (or equivalent) exit code
  captured in CI, a passing test name, or a fetched URL with a 200 status.
  The artifact MUST be checkable by a third party without re-running the
  agent.

- **`INCOMPLET`** (partial) — The concrete missing piece MUST be named.
  "Scaffolded the module but the `parse_header` function still returns
  `unimplemented!()` at `crates/foo/src/parser.rs:42`" is acceptable;
  "mostly done" is not. The named piece becomes the next loop's target.

- **`NON_FAIT`** (not done) — The blocker MUST be named, with enough
  context for a reader to act on it. "Waiting on upstream PR
  agntcy/slim#123 to merge" is acceptable; "blocked" is not. Where the
  blocker is human-arbitrated (scope decision, secret access), the
  required human action MUST be spelled out.

Implementations SHOULD attach the tri-state value to the matching
`a2a.Task` under `metadata.honest_status` so it surfaces in any A2A-aware
dashboard. The reference shape used in this repo is visible in the
`tasks[*].metadata.honest_status` field of [`ai.json`](../../ai.json).

## Adoption

In this repository:

- The `/aphrody-yolo-grind` skill enforces honest-delivery on every loop
  tick: each dispatched sub-agent reports tri-state per deliverable, and
  the orchestrator refuses to mark a PLAN.md item complete without a
  `FAIT` artifact attached.
- The `/start` skill recommends honest-delivery for any autonomous session
  the user launches manually.
- Human-authored commits are encouraged but not required to use the
  labels; the cost of mislabelling is much lower when a human is in the
  loop and the label has obvious value mostly for unattended runs.

## Conformance

An agent advertising `honest-delivery/v1` in its `AgentCard.extensions`
MUST produce one of the three labels for every deliverable it claims in
any autonomous run, MUST attach the required justification shape, and
MUST NOT promote a deliverable from `INCOMPLET` or `NON_FAIT` to `FAIT`
without producing the corresponding verifiable artifact.
