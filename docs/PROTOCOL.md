<!-- SPDX-License-Identifier: Apache-2.0 -->
# aphrody A2A v1 — Normative Protocol Specification

> Status: stable. Version: 1.0.0. Date: 2026-05-17. License: Apache-2.0.

This document is the normative reference for third-party implementers who want
their agent to interoperate with aphrody / winclean instances. The companion
dev journal at [`docs/posts/2026-05-ai-json.md`](./posts/2026-05-ai-json.md)
explains the rationale; this file defines the wire contract.

## 1. Scope

This specification defines a file-based agent-to-agent (A2A) coordination
protocol with an optional HTTP overlay. It is compatible with AGNTCY
[a2a/v0.4](https://github.com/agntcy/dir) manifest discovery (the
`.well-known/<manifest>` pattern) and extends it with three project
extensions that describe channel transports, honest delivery semantics, and
mandatory documentation fact-checking.

The protocol targets a constrained but extremely common deployment: two or
more autonomous agents that share a filesystem (same host, same user, same
disk) but cannot rely on a long-running RPC runtime. It is designed to
degrade gracefully: every channel except the live HTTP overlay survives
process restarts, partial outages, and asynchronous peer wakeups.

Conformance terms (`MUST`, `SHOULD`, `MAY`) follow RFC 2119.

## 2. Manifest (`ai.json`)

Each agent MUST publish a manifest at the root of its workspace (`ai.json`)
AND a thin discovery copy at `.well-known/ai.json`. Both files MUST be valid
JSON, UTF-8 without BOM, and SHOULD include a trailing newline.

Required top-level fields:

- `schema_version` (string): the constant `"1.0.0"`.
- `kind` (string): one of `"agent"` (single-agent self-declaration) or
  `"coord"` (aggregate manifest at a shared mailbox referencing N agents).
- `id` (string): stable identifier following the convention
  `<short-name>@<vcs-org>/<repo>` for agents.
- `spec` (string): a free-form list of declared extension URIs in the form
  `"a2a/v0.4 + file-transport/v1 + honest-delivery/v1 + context7-version-pinning/v1"`.
- `agent` (object): identity of the underlying AI runtime (`name`, `model`,
  `provider` at minimum).
- `coord` (object): the channels this agent exposes (referenced through
  `exposed_channels`) and any peer pointers under `peers`.

The full JSON Schema (draft 2020-12) is at
[`schemas/ai.json/v1.json`](../schemas/ai.json/v1.json). Implementations MUST
validate their published manifest against that schema before serving it.

## 3. Envelope (`*.jsonl`)

Every coordination message — whether appended to a JSONL inbox, POSTed to the
HTTP listener, or echoed in a markdown reply — uses the same envelope. One
envelope per JSONL line. Files MUST be UTF-8, no BOM, LF-terminated.

Required fields:

- `id` (string): unique message identifier matching the regex
  `^[a-z0-9-]{3,64}$`. Convention: `<agent-short-name>-<purpose>-<seq>`.
- `ts` (string): ISO-8601 UTC timestamp, e.g. `"2026-05-17T13:50:23Z"`.
- `from` (string): the sender's manifest `id`.
- `type` (string): one of `"ping"`, `"ask"`, `"fact"`, `"ack"`.
- `subject` (string): short label, at most 200 characters.
- `body` (string OR object): free text (markdown allowed) or a structured
  JSON object. See [`schemas/ai.json/v1.json`](../schemas/ai.json/v1.json)
  `$defs.envelope` for the canonical shape.

Optional fields:

- `to` (string): recipient `id`. Omit for broadcast within a coord space.
- `re` (string): the `id` of the message being answered. REQUIRED for `ack`.
- `channel_hint` (enum): one of `file_jsonl`, `http_jsonrpc`,
  `heartbeat_file`, `git_tag`, `named_pipe`, `process_inspect`,
  `markdown_doc`. Hints the preferred reply channel.

Append-only discipline: implementers MUST NOT edit or delete existing lines.
The JSONL files are the audit trail; history is the contract.

## 4. Channels (7)

| Channel | Transport | Durable | Latency | Failure mode |
|---|---|---|---|---|
| `file_jsonl` | filesystem append (`inbox-from-<self>.jsonl`) | yes | ms | partner reads on next poll |
| `http_jsonrpc` | HTTP POST `:8788/msg` | no (mirrors to file) | ms | partner falls back to `file_jsonl` |
| `heartbeat_file` | filesystem write (`heartbeat-<self>.txt`) | overwritten | seconds | proof-of-life only |
| `git_tag` | `git tag <self>-<purpose>-<seq>` push | yes | minutes | partner pulls tags |
| `named_pipe` | Windows named pipe `\\.\pipe\<self>-<peer>` | no | microseconds | Win-only fallback |
| `process_inspect` | `ps -ef` / `tasklist` matching documented pattern | n/a | seconds | best-effort liveness |
| `markdown_doc` | committed `.md` file (e.g. `COORD.md`) | yes | minutes | human-readable broadcast |

Channels `file_jsonl`, `heartbeat_file`, `git_tag`, `markdown_doc` are
durable and survive across both peers being offline at different times.
`http_jsonrpc` and `named_pipe` are live overlays for low-latency push.
`process_inspect` is observation-only — agents cannot send through it, but
they can confirm a peer is alive by matching its documented command pattern.

An implementation MAY expose any subset; it MUST support `file_jsonl` as the
authoritative durable channel.

## 5. HTTP listener (`:8788`)

The HTTP overlay is OPTIONAL. When implemented it MUST bind to TCP port
`8788` on `127.0.0.1` and expose the following endpoints. All responses MUST
use `Content-Type: application/json` (or `text/plain` for `GET /ping` when
returning the literal string `pong`).

| Method | Path | Behavior |
|---|---|---|
| `POST` | `/msg` | Accept an envelope as JSON body, validate against the envelope schema, mirror to `inbox-from-<self>.jsonl`, return `{"ok": true, "mirrored_to": "<path>"}`. |
| `GET`  | `/ping` | Health check. Return either the literal `pong` (text/plain) or `{"ok": true, "ts": "<iso-8601>"}`. |
| `GET`  | `/inbox` | Return the JSONL contents of `inbox-from-<peer>.jsonl` as `application/x-ndjson`. |
| `GET`  | `/ai.json` | Return the canonical manifest. |
| `GET`  | `/.well-known/ai.json` | Return the thin discovery copy. |

Errors MUST return HTTP 4xx with a JSON body shaped as
`{"error": "<reason>"}`. Common cases: `400` (envelope schema mismatch),
`404` (path not served), `413` (body exceeds `etiquette.max_message_kb`).

Every POST MUST be mirrored to the durable JSONL file before the response is
sent, so that the file channel remains authoritative even if the listener
process dies between request and reply.

## 6. Handshake (3-deep)

The 3-deep handshake closes the loop and proves bidirectional reachability:

1. Agent A POSTs (or appends) a `ping` or `ask` envelope with `id=X`.
2. Agent B receives, processes, and replies with an `ack` envelope carrying
   `re=X` and its own `id=Y`.
3. Agent A confirms receipt with an `ack` envelope carrying `re=Y`.

Steps 1 and 2 prove A→B reachability. Step 3 proves B→A reachability through
the same loop without requiring a separate test. Implementations MUST treat
step 3 as the success condition; absence of step 3 within
`etiquette.rate_limit_per_minute * 5` minutes indicates a half-open
relationship that SHOULD be re-attempted on the next outbound batch.

## 7. Extensions

Three extensions are declared by aphrody. Each is versioned independently;
`v1` is the current revision. The URIs resolve to the in-tree specs once the
repository is published to GitHub Pages.

- **`file-transport/v1`** — Defines the seven channels in section 4 and the
  envelope contract.
  - URI: `https://aphrody.dev/a2a-extensions/file-transport/v1`
  - Spec: [`docs/extensions/file-transport-v1.md`](./extensions/file-transport-v1.md)

- **`honest-delivery/v1`** — Mandatory tri-state delivery classification
  (`FAIT` / `INCOMPLET` / `NON_FAIT`) attached to every claimed deliverable,
  with a 5-point UI gate for browser-facing artefacts.
  - URI: `https://aphrody.dev/a2a-extensions/honest-delivery/v1`
  - Spec: [`docs/extensions/honest-delivery-v1.md`](./extensions/honest-delivery-v1.md)

- **`context7-version-pinning/v1`** — Required `context7` MCP fact-check
  (`resolve-library-id` then `query-docs`) before adding any dependency or
  making a non-trivial library API decision.
  - URI: `https://aphrody.dev/a2a-extensions/context7-version-pinning/v1`
  - Spec: [`docs/extensions/context7-version-pinning-v1.md`](./extensions/context7-version-pinning-v1.md)

Implementers MAY declare additional extensions in the `agent.capabilities.extensions`
array of their manifest, using vendor-namespaced URIs to avoid collision.

## 8. Compliance checklist

An implementation claiming "compatible with aphrody a2a v1" MUST satisfy:

- [ ] `ai.json` validates against `schemas/ai.json/v1.json`.
- [ ] A thin discovery copy is published at `.well-known/ai.json`.
- [ ] Envelopes are appended (never edited or deleted) to the peer's
      `inbox-from-<self>.jsonl` file.
- [ ] `heartbeat-<self>.txt` is bumped at least every 10 minutes while the
      agent is active.
- [ ] At minimum the `ping` and `ack` envelope types are supported.
- [ ] The 3-deep handshake (section 6) completes against a reference peer.
- [ ] OPTIONAL: HTTP listener on `:8788` exposing the five endpoints in
      section 5.
- [ ] OPTIONAL: the three declared extensions are honored when their URIs
      appear in a peer's manifest.

## 9. References

- AGNTCY a2a directory spec: <https://github.com/agntcy/dir>
- This repository: <https://github.com/aphrody-code/aphrody>
- Manifest schema: [`schemas/ai.json/v1.json`](../schemas/ai.json/v1.json)
- Reference manifest: [`ai.json`](../ai.json)
- Dev journal narrative: [`docs/posts/2026-05-ai-json.md`](./posts/2026-05-ai-json.md)
- Extension index: [`docs/extensions/index.md`](./extensions/index.md)
