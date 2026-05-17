# aphrody-terminal-llm

LLM event bus and multiplexer for the aphrody-terminal stack.

## What this crate owns

- `EventBus` — tokio broadcast channel typed on `LlmEvent`; fan-out to N subscribers.
- `OscParser` — decodes `\e]aphrody-{md,json,sub-agent,mcp,hook,skill,task};<payload>\a`
  sequences into `LlmEvent` values; invalid base64 or JSON is logged and dropped.
- `SubAgentRegistry` — thread-safe id→info map; live-stream snapshot via channel.
- `McpStatusRegistry` — thread-safe server→status map.
- `HookEventLog` — fixed-capacity ring buffer (1000 entries) of hook firings.
- `SkillSlot` — per-skill activation state machine (idle/active/error).
- `TaskTree` — parent/child task graph keyed by string id.

## Scope

Native-only (`std` + tokio). The WASM renderer subscribes via a separate adapter
(`aphrody-terminal-wasm`) that bridges the broadcast channel over `wasm-bindgen-futures`.
No web-sys, no wasm targets here.

## Usage

```rust
let bus = EventBus::new(256);
let mut rx = bus.subscribe();

// Publish from any thread
bus.publish(LlmEvent::Markdown { body: "# Hello".into() }).ok();

// Receive
let ev = rx.recv().await.unwrap();
```

## OSC format

`\x1b]aphrody-<op>;<payload>\x07`  (BEL terminator) or `\x1b]aphrody-<op>;<payload>\x1b\\` (ST).

| op | payload |
|----|---------|
| `md` | base64(markdown UTF-8) |
| `json` | base64(JSON) |
| `sub-agent` | `<id>;<status>;<text>` |
| `mcp` | `<server>;<state>[;<rpc>]` |
| `hook` | base64(JSON `{event, payload}`) |
| `skill` | base64(JSON `{name, phase, payload}`) |
| `task` | `<id>;<status>;<subject>` |

## License

Apache-2.0 — see root `LICENSE`.
