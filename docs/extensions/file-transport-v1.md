<!-- SPDX-License-Identifier: Apache-2.0 -->
# A2A Extension: `file-transport/v1`

- **Spec URL**: `https://aphrody.dev/a2a-extensions/file-transport/v1`
- **Version**: 1.0.0
- **Status**: stable
- **Date**: 2026-05-17
- **License**: Apache-2.0
- **Related**: [ai.json dev journal](../posts/2026-05-ai-json.md),
  [parallel YOLO grind loop](../posts/2026-05-yolo-grind-loop.md),
  [schema](../../schemas/ai.json/v1.json) (definitions `Channel`, `Envelope`).

## Abstract

`file-transport/v1` defines an append-only JSONL inbox protocol, optionally
overlaid by a minimal HTTP listener, for agent-to-agent communication when
the participating agents share a filesystem but cannot rely on a live RPC
runtime. It is the bilateral transport that two Claude Code sessions used to
coordinate the work captured in this repository.

## Channels

An implementation MAY expose any subset of the following seven channels.
Each `Channel` entry in the agent's `ai.json` declares its `name`, `kind`,
optional `path` or `url`, and `direction` (`in`, `out`, `both`).

1. **`file_jsonl`** (primary) — Append-only JSONL mailbox files. Each side
   writes only into its own outbound file (`inbox-from-<self>.jsonl`); the
   peer tails it. One envelope per line, UTF-8, LF-terminated.
2. **`http_jsonrpc`** (overlay) — Optional HTTP listener (see below) that
   mirrors the JSONL mailbox for low-latency push.
3. **`heartbeat_file`** (proof-of-life) — A monotonically-rewritten text
   file (`heartbeat-<agent>.txt`) holding an ISO-8601 UTC timestamp.
4. **`git_tag`** (out-of-band) — Lightweight tags prefixed with the
   sender's agent id signal milestones to peers that fetch the repo
   (e.g. `aphrody-handshake-ack-3`).
5. **`named_pipe`** (Windows) — Optional Win32 named pipe
   (`\\.\pipe\<agent>-<peer>`) for sub-millisecond same-host delivery.
   Falls back to `file_jsonl` on Linux/macOS/WASM.
6. **`process_inspect`** (passive) — Peer-detection via process listing
   (`ps -ef`, `Get-Process`) matching the documented command pattern.
7. **`markdown_doc`** (slow path) — A shared `COORD.md` updated through
   commits when neither agent is online; sections timestamped and signed.

## Envelope schema

All channels carry the same JSON envelope, defined in
[`schemas/ai.json/v1.json`](../../schemas/ai.json/v1.json) under the
`Envelope` definition. Required: `envelope_version` (`"1"`), `message_id`
(ULID/UUID), `ts` (ISO-8601 UTC), `from`, `to`, `type` (e.g. `fact`,
`ask`, `ack`), `body` (any JSON). Optional: `in_reply_to`, `ttl_sec`,
`signature`.

## HTTP listener spec

Reference port: **8788** (TCP, loopback by default). Endpoints:

| Path        | Method | Body                | Response                              |
|-------------|--------|---------------------|---------------------------------------|
| `/ping`     | GET    | none                | `{ "agent": "<id>", "ts": "..." }`    |
| `/msg`      | POST   | one envelope (JSON) | `{ "ok": true, "stored_at": "..." }`  |
| `/inbox`    | GET    | optional `?since=`  | newline-delimited envelopes (JSONL)   |
| `/ai.json`  | GET    | none                | the agent's current manifest          |

The reference implementation is approximately forty lines of `Bun.serve`
and lives at `C:/winclean/.coord/listener.ts` in the peer repository.

## Failure modes

When the HTTP listener is down (crashed, port collision, agent offline),
senders MUST fall back to direct file append on the JSONL mailbox they own
per the bilateral write-allocation rule: each agent writes only its own
`inbox-from-<self>.jsonl`, never the peer's. The peer picks up new lines
on the next poll. Heartbeat staleness ( > 5 minutes by default) signals the
peer is offline; senders SHOULD still write so the peer catches up on resume.

## Conformance

An agent advertising `file-transport/v1` in its `AgentCard.extensions` MUST
implement at minimum `file_jsonl` (in and out), MUST publish an
`heartbeat_file` channel, and SHOULD expose `http_jsonrpc` on the documented
port. All other channels are OPTIONAL.
