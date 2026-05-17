---
name: a2a-duel-loop
description: Continuous 2-Claude coordination duel over the A2A file-based protocol (ai.json + .coord JSONL mailbox + HTTP listener). Each iteration writes one envelope into the peer's inbox, runs a code-reviewer pass, and schedules the next tick. Mission target — sustain the duel across hours/days without human handhold.
when_to_use: User types "/a2a-duel-loop", says "lance le duel", "duel loop", "run the A2A duel", "tick the duel", or asks to keep two Claudes coordinating in a sustained back-and-forth on a shared project. Pair with `loop` skill (`/loop /a2a-duel-loop`) to make it self-pacing under the dynamic loop runtime.
---

# A2A duel loop — sustained two-Claude coordination

You are tick-driving a back-and-forth between this Claude (aphrody side, `C:\src\aphrody`) and the peer Claude (winclean side, `C:\winclean`). The protocol lives in `ai.json` + `.coord/` and is documented in `docs/posts/2026-05-ai-json.md`.

This skill formalises one **iteration** of the duel: read what the peer last said, do work, write a reply into the peer's inbox, optionally have a `code-reviewer` agent audit the exchange, and schedule the next tick.

## Operating contract

- **Each tick is one iteration**, identified by an integer `--iteration N` passed to `scripts/duel-cycle.ts`.
- **One envelope per tick** is appended to the peer's inbox (`C:\winclean\.coord\inbox-from-aphrody.jsonl` when this side speaks, `inbox-from-winclean.jsonl` when simulating the peer for testing).
- **No live IPC**, only files: `ai.json`, `.coord/inbox-from-*.jsonl`, `.coord/heartbeat-*.txt`, `.coord/http-log.jsonl` (via the Bun listener on `:8788`).
- **Honest delivery extension** (`https://aphrody.dev/a2a-extensions/honest-delivery/v1`) applies: tri-state status `FAIT` / `INCOMPLET` / `NON_FAIT` on every claimed deliverable. No "shipped" without the 5-point UI gate when UI is in scope.
- **Heartbeat is mandatory**: bump `C:\winclean\.coord\heartbeat-aphrody.txt` on every tick (ISO-8601 UTC). If the peer's `heartbeat-winclean.txt` is older than 600 s, surface the staleness in the next envelope.

## The loop, one iteration

```
1.  bun .claude/skills/a2a-duel-loop/scripts/duel-cycle.ts --iteration N [--side aphrody|winclean]
        → reads `C:\winclean\.coord\inbox-from-{peer}.jsonl` last line
        → reads peer ai.json for fresh asset inventory / open_asks
        → composes a reply envelope (id apx-duel-N-<hex>, from/to inferred from --side)
        → appends to the correct inbox JSONL
        → POSTs to localhost:8788/msg (if listener responds) for mirroring
        → bumps the matching heartbeat-*.txt

2.  Optional: Agent({ subagent_type: "code-reviewer", description: "Review duel tick N",
        prompt: "Read the envelope just appended (...) and the peer's last 3 envelopes. Flag
                 (a) contradictions vs the canonical ai.json, (b) over-claim language vs the
                 honest-delivery extension, (c) missed open_asks. Report in <120 words."
    })
    Hold the code-reviewer report; surface a one-line summary in the next tick's envelope body.

3.  ScheduleWakeup({ delaySeconds: 60, prompt: "/a2a-duel-loop continue tick N+1",
                     reason: "next duel tick" })  -- only available when invoked under /loop.
    If ScheduleWakeup is not available (skill invoked directly), end with a one-line
    instruction telling the user to either re-invoke /a2a-duel-loop or to wrap with /loop.
```

## Picking the envelope body

The body should advance one of these explicitly, in priority order:

1. **Resolve a peer `ask`** by attaching an `ack` envelope referencing `re: <peer-ask-id>`, with the artifact link (commit SHA, file path) inline.
2. **Surface a fact** that closes a known gap (e.g., a CI run result, a benchmark number, a security audit finding).
3. **Open a new `ask`** when forward motion needs the peer's owned territory (e.g., a Cargo.toml edit in the peer's workspace).
4. **Heartbeat-only** as last resort: a `ping` envelope when there's literally nothing to advance — but treat this as a smell. If three consecutive ticks are pings, raise the issue out-of-band.

Keep envelopes **under 64 KB** (per the etiquette block in both ai.json mirrors). Markdown body is fine; no base64 blobs, link to files instead.

## When to break the loop

- The peer's heartbeat is older than 1800 s (30 min) and three consecutive `ping` envelopes were not answered — assume peer offline, post one final `fact` envelope summarising state, end the loop.
- The shared project enters a state that requires user policy (e.g., publishing public, merging breaking PRs, force-pushing) — surface to user via the standard ask-for-input pattern, do not act.
- `ai.json` schema drift between mirrors crosses a major version — block the loop, post a `fact` envelope flagging the drift with both versions, and stop until reconciliation is committed.

## Files this skill touches

| Path | Direction | Role |
|---|---|---|
| `C:\winclean\.coord\inbox-from-aphrody.jsonl` | aphrody → winclean | one envelope per tick, append-only |
| `C:\winclean\.coord\inbox-from-winclean.jsonl` | winclean → aphrody | what we *read* each tick (peer's last reply) |
| `C:\winclean\.coord\heartbeat-aphrody.txt` | aphrody | ISO-8601 UTC, rewritten each tick |
| `C:\winclean\.coord\http-log.jsonl` | listener | hits on `localhost:8788`, append-only |
| `C:\src\aphrody\ai.json` | aphrody (canonical) | A2A v0.4 CollaborationManifest — read fresh each tick |
| `C:\winclean\ai.json` | winclean (canonical) | peer's mirror — read fresh each tick |

## Pairing with the `loop` skill

The natural composition is:

```
/loop 60s /a2a-duel-loop
```

The `loop` runtime invokes this skill on a dynamic 60 s cadence; the skill returns after one tick; the runtime re-fires. Use `ScheduleWakeup` inside this skill only when running under `/loop` (it errors otherwise).

For a one-shot test of the protocol (without sustained loop), invoke `/a2a-duel-loop` directly: it runs one tick and exits, suggesting the user wrap with `/loop`.

## Related skills and references

- **`/start`** — broader autonomous execution. Use when the goal is project-wide forward motion rather than the narrow A2A coordination.
- **`/loop`** — pacing runtime. Pair as `/loop 60s /a2a-duel-loop`.
- **`code-reviewer` agent** — invoked optionally per tick for envelope auditing.
- **`docs/posts/2026-05-ai-json.md`** — the dev-journal post explaining the protocol, channels, and the first 3-deep handshake.
- **`schemas/ai.json/v1.json`** — JSON Schema for the channel layer + envelope.
- **`ai.json`** (root) — AGNTCY a2a/v0.4 CollaborationManifest, source of truth for AgentCards / tasks / shared resources / mirrors.
