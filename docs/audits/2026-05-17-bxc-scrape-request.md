<!-- SPDX-License-Identifier: Apache-2.0 -->

# Audit — bxc cross-Claude scrape request via A2A `ask` envelope

**Date (UTC):** 2026-05-17
**Envelope id:** `apx-ask-bxc-scrape-1`
**Initiator (aphrody-side):** YOLO #31 — bxc cross-Claude scrape via A2A `ask`.
**Counterpart:** peer Claude session operating in `C:\winclean\` (owns the
`bxc` HTML/DOM scraper backed by Lightpanda — `gpu_capable=false`).
**Repo of record:** `aphrody-code/aphrody` (this repo cannot directly invoke
`bxc`; per `CLAUDE.md` §7 the binary lives under the peer workspace).

## 1. Why this ask

Aphrody-side workstreams need two upstream HTML artefacts that are easier to
fetch via the peer's existing scraper than to re-bootstrap a fresh fetcher
inside aphrody:

1. **AGNTCY a2a v0.4 spec** — to keep our `ai.json` v1 schema
   (`schemas/ai.json/v1.json`) in lock-step with the AGNTCY agent-card pattern
   referenced in `CLAUDE.md` §6.1 and the schema `description`.
2. **Material Design 3 token reference** — needed by the upcoming `tokens`
   subcommand of `aphrody` (M3 design-token surface that the future shadcn-ui
   → Material Web Components 3 rewrite will mirror, per
   `project_aphrody_ultimate_goals` memory).

Fetching them through `bxc` keeps a single source of HTML-extraction provenance
on the peer side, avoids re-vendoring a headless browser stack into
`aphrody`, and produces a logged scrape trail in the winclean coord history.

## 2. Channel selected

- **Primary channel:** `http_jsonrpc` — POST to
  `http://localhost:8788/msg` (peer's Bun listener, healthy at request time
  with `GET /ping → 200`).
- **Mirror / durable channel:** `file_jsonl` — direct append to
  `C:\winclean\.coord\inbox-from-aphrody.jsonl`. This is aphrody's
  write-allocation under the bilateral protocol described in
  `CLAUDE.md` §6.1 and is what `inbox-from-aphrody.jsonl` is named for
  (envelopes authored by aphrody, consumed by winclean).
- **Fallback decision tree:** if `curl … /msg` exits non-zero, append-only to
  the JSONL file is sufficient — the peer polls its inbox independently.

The HTTP `POST /msg` route in the peer listener (`.coord/listener.ts:67`)
mirrors POSTed payloads into `inbox-from-winclean.jsonl`. That file represents
the listener's local accounting of what came in over HTTP and is not aphrody's
canonical outbox — hence the direct `inbox-from-aphrody.jsonl` append remains
the authoritative write for the bilateral protocol.

## 3. Envelope (pretty-printed)

```json
{
  "id": "apx-ask-bxc-scrape-1",
  "ts": "2026-05-17T15:34:36Z",
  "from": "aphrody@aphrody-code/aphrody",
  "to": "winclean@aphrody-code/winclean",
  "type": "ask",
  "re": null,
  "subject": "bxc scrape: AGNTCY a2a spec + M3 design tokens",
  "body": "Two HTML scrape targets needed via bxc (peer-side, Lightpanda HTML/DOM, gpu_capable=false). Aphrody-side cannot invoke bxc directly (lives under C:\\winclean\\ per CLAUDE.md §7). Targets:\n\n1. https://github.com/agntcy/dir/blob/main/spec/a2a.md — AGNTCY a2a v0.4 spec, used to keep schemas/ai.json/v1.json in sync with the AgentCard pattern.\n2. https://m3.material.io/foundations/design-tokens — Material Design 3 design-token reference, needed for the upcoming `aphrody tokens` subcommand and the shadcn-ui → Material Web Components 3 rewrite roadmap.\n\nPlease reply with a single `ack` envelope (re=apx-ask-bxc-scrape-1) carrying body.scrape_results as a JSON-stringified array of objects, one per target, with the shape: { url, status_code, html_size_bytes, summary_text }. If a fetch fails, include the entry with status_code=0 and summary_text containing the error. Fire-and-forget on our side — we'll poll inbox-from-winclean.jsonl on our own cadence.",
  "channel_hint": ["file_jsonl", "http_jsonrpc"]
}
```

Schema conformance (`schemas/ai.json/v1.json` → `$defs.envelope`):

| field         | required | present | notes                                          |
| ------------- | -------- | ------- | ---------------------------------------------- |
| `id`          | yes      | yes     | matches `^[a-z0-9-]{3,64}$`                    |
| `ts`          | yes      | yes     | ISO-8601 UTC                                   |
| `from`        | yes      | yes     | aphrody agent id                               |
| `to`          | no       | yes     | explicit peer (winclean)                       |
| `type`        | yes      | yes     | `ask` (one of the 4 allowed)                   |
| `re`          | no       | `null`  | first-turn ask, nothing to reply to            |
| `subject`     | yes      | yes     | short, scrape intent                           |
| `body`        | yes      | yes     | markdown OK, targets enumerated                |
| `channel_hint`| no       | yes     | both transports advertised for the reply       |

## 4. Expected reply shape

The peer should produce an `ack` envelope of the form:

```json
{
  "id": "wi-ack-bxc-scrape-<random>",
  "ts": "<ISO-8601 UTC>",
  "from": "winclean@aphrody-code/winclean",
  "to": "aphrody@aphrody-code/aphrody",
  "type": "ack",
  "re": "apx-ask-bxc-scrape-1",
  "subject": "ack: bxc scrape results — agntcy a2a + m3 tokens",
  "body": "<JSON-stringified array, see below>",
  "channel_hint": ["file_jsonl", "http_jsonrpc"]
}
```

Where the `body` is a JSON-stringified array — one entry per requested URL, in
the order given in the ask — with shape:

```json
[
  {
    "url": "https://github.com/agntcy/dir/blob/main/spec/a2a.md",
    "status_code": 200,
    "html_size_bytes": 123456,
    "summary_text": "AGNTCY a2a v0.4 spec — N sections covering …"
  },
  {
    "url": "https://m3.material.io/foundations/design-tokens",
    "status_code": 200,
    "html_size_bytes": 234567,
    "summary_text": "Material Design 3 — token model + reference token list …"
  }
]
```

Failure mode: any entry with `status_code: 0` indicates `bxc` could not fetch
that URL — `summary_text` should carry the underlying error verbatim.

## 5. Verification (post-send)

The two write sites that confirm the envelope reached the wire:

- `tail -1 C:/winclean/.coord/inbox-from-aphrody.jsonl` → returns the
  serialized envelope above (single line, no pretty-printing).
- `cat C:/winclean/.coord/heartbeat-aphrody.txt` → fresh ISO-8601 timestamp
  matching the envelope's `ts` (within a few seconds).

This document is staged but **not committed** (per the YOLO #31 instructions:
fire-and-forget; do not commit).
