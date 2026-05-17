<!-- SPDX-License-Identifier: Apache-2.0 -->
# aphrody A2A Extensions

This directory holds the canonical specifications for the three custom
[AGNTCY A2A](https://github.com/agntcy) `AgentExtension`s declared by the
aphrody agent card ([`ai.json`](../../ai.json) at the repo root, schema in
[`schemas/ai.json/v1.json`](../../schemas/ai.json/v1.json)).

When this repository is published, each spec resolves at its GitHub Pages URL
(`https://aphrody.dev/a2a-extensions/<name>/v1`) so remote peers can fetch
the extension definition by following the `uri` field they discover in the
agent card. The same files are committed in-tree so the contract travels with
the source.

Context on why these extensions exist:

- [dev journal — ai.json file-based A2A handshake](../posts/2026-05-ai-json.md)
- [dev journal — parallel YOLO grind loop](../posts/2026-05-yolo-grind-loop.md)

## Extensions

- **[`file-transport/v1`](./file-transport-v1.md)** — Append-only JSONL
  inbox protocol with an optional HTTP listener overlay. Defines how two
  agents that share a filesystem (but cannot rely on live RPC) exchange
  envelopes through per-direction `inbox-from-<sender>.jsonl` files, with
  heartbeats, git tags, and process inspection as out-of-band channels.

- **[`honest-delivery/v1`](./honest-delivery-v1.md)** — Tri-state delivery
  classification (`FAIT` / `INCOMPLET` / `NON_FAIT`) that autonomous agents
  must attach to every claimed deliverable. Each state requires a specific
  shape of justification, preventing "shipped N features" inflation when
  half of them are placeholders or blockers.

- **[`context7-version-pinning/v1`](./context7-version-pinning-v1.md)** —
  Required `context7` MCP lookup (`resolve-library-id` then `query-docs`)
  before adding any library dependency or making a non-trivial library API
  decision. Counters the training-data drift that makes LLMs confidently
  pin stale versions of fast-moving crates.

All three are versioned independently; `v1` is the current revision.
