# Aphrody A2A (Agent2Agent)

Community implementation aligned with [A2A Protocol 1.0](https://a2a-protocol.org/latest/specification/) and the [a2aproject](https://github.com/orgs/a2aproject/repositories) SDK ecosystem.

## Layers

| Layer | Crate / artifact | Role |
| --- | --- | --- |
| Data model | `a2a`, `a2a-pb` | Tasks, messages, parts, agent card, JSON-RPC types (proto-backed) |
| Server framework | `a2a-server` | JSON-RPC, REST, SSE, task store, `AgentExecutor` |
| Coordination | `a2a-coord` | `ai.json`, JSONL mailboxes, HTTP bridge, native peer dispatch |
| Client / duel | `a2a-client`, `a2a_duel_loop` | Remote calls and file-based duel ticks |

## Quick start

```bash
# HTTP listener: agent card + JSON-RPC + file /msg
aphrody a2a serve --bind 127.0.0.1:8788

# One JSONL envelope (duel / coord loop)
aphrody a2a tick --iteration 1 --side aphrody --peer winclean

# Headless native peer (Claude Code, Grok, agy, bxc)
aphrody a2a invoke "refactor the auth module" --peer grok
aphrody a2a invoke "summarize PLAN.md" --peer agy --dry-run
```

Dry-run for the server executor: `APHRODY_A2A_DRY_RUN=1 aphrody a2a serve`.

## `ai.json`

Repo root [`ai.json`](../../ai.json) declares:

- A2A **1.0** (`a2a_protocol_version`, `spec: a2a/v1.0`)
- Peers: `claude`, `grok`, `agy`, `bxc` with documented CLI invocations
- Coord: `.coord/inbox-from-<peer>.jsonl`, default bind `127.0.0.1:8788`

## Native peers

| Peer | CLI | Notes |
| --- | --- | --- |
| `grok` | `grok --prompt-file … --always-approve --permission-mode bypassPermissions` | Do **not** pass `--effort` with `grok-build` (HTTP 400) |
| `agy` | `agy -p "…" --dangerously-skip-permissions --add-dir <repo>` | Antigravity / Gemini |
| `claude` | `claude -p "…" --output-format text` | Claude Code |
| `bxc` | `bxc search … --json` | Web search / recon |

Override binaries: `GROK_BIN`, `APHRODY_AGY_BIN`, `CLAUDE_BIN`, `BXC_BIN`.

## Protocol bindings (A2A 1.0)

On `aphrody a2a serve`:

| Binding | Entry |
| --- | --- |
| Agent Card | `GET /.well-known/agent-card.json` |
| JSON-RPC 2.0 | `POST /` — `SendMessage`, `GetTask`, `ListTasks`, … |
| HTTP+REST | merged from `a2a-server` REST router (same handler) |
| File bridge | `POST /msg` — JSONL envelopes |

Service headers: `A2A-Version: 1.0`, optional `A2A-Extensions`.

## JSON-RPC

`SendMessage` routes through `CoordPeerExecutor`:

- Metadata `aphrody_peer` or `peer` selects the CLI
- Message prefix `@grok:` or `/peer grok` also selects the peer
- Completed tasks include an artifact with peer stdout

Well-known agent card: `GET /.well-known/agent-card.json`.

## File channel

Envelopes (`ping`, `ask`, `fact`, `ack`, `error`) append to `.coord/inbox-from-<short-id>.jsonl`.  
`POST /msg` accepts the same JSON for HTTP peers.

## References

- Spec: https://a2a-protocol.org/latest/specification/
- Org SDKs: https://github.com/a2aproject/a2a-rs (Rust), `a2a-python`, `a2a-js`, `a2a-go`
- Peer MCP doc: [peer-a2a-mcp-csharp.md](../peer-a2a-mcp-csharp.md)