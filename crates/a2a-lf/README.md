<!-- SPDX-License-Identifier: Apache-2.0 -->

# a2a-lf

## What is `a2a-lf`?

`a2a-lf` is the published name on crates.io for the Aphrody A2A protocol type
system. The crate ships the wire-compatible message, task, artifact, and event
types for the AGNTCY a2a/v0.4 specification, the JSON-RPC envelope models, and
the typed error surface used by every other crate in the `a2a-*` family. The
`lf` suffix is inherited from the upstream `a2a-rs` workspace naming
convention (paired with `a2a-client-lf`, `a2a-server-lf`); it is **not** a
lock-free runtime layer. The library is imported in Rust under the short name
`a2a`, which is why the workspace `Cargo.toml` aliases the dependency:

```toml
a2a = { package = "a2a-lf", path = "crates/a2a", version = "0.3" }
```

The source tree lives at `crates/a2a/` inside the Aphrody monorepo.

## Install

```toml
[dependencies]
a2a-lf = "0.3"
```

Or, more idiomatically, alias on import:

```toml
a2a = { package = "a2a-lf", version = "0.3" }
```

## Quick start

```rust
use a2a::{Message, Part, Role, new_message_id};

let mut msg = Message::new(Role::Agent, vec![
    Part::text("task accepted"),
]);
msg.message_id = new_message_id();
```

## Public API

Source of truth: `crates/a2a/src/lib.rs` (the lib that this `a2a-lf` package
publishes). Six submodules are exported in their entirety:

- `agent_card` — `AgentCard` describing a peer's capabilities and transports.
- `types` — `Message`, `Task`, `Artifact`, `Part`, `PartContent`, `Role`,
  `TaskState`, `TaskStatus`, plus the request / response pairs
  `SendMessageRequest`, `SendMessageResponse`, `SendMessageConfiguration`,
  `GetTaskRequest`, `ListTasksRequest`, `ListTasksResponse`.
- `event` — streaming envelopes `StreamResponse`, `TaskStatusUpdateEvent`,
  `TaskArtifactUpdateEvent` for SSE and gRPC server-streaming.
- `jsonrpc` — `JsonRpcRequest`, `JsonRpcResponse`, `JsonRpcError`, `JsonRpcId`.
- `errors` — `A2AError` and protocol error code constants.
- `errordetails` — strongly typed error payload variants.

ID constructors: `new_task_id`, `new_context_id`, `new_message_id`,
`new_artifact_id`. Protocol constants: `VERSION`, `SVC_PARAM_VERSION`,
`SVC_PARAM_EXTENSIONS`.

## Concurrency model

`a2a-lf` is a pure data-model crate. There are no `Mutex`, no `RwLock`, and no
background tasks: every published struct is `Send + Sync` as soon as its
fields are. Lock-free composition is the responsibility of downstream layers
(`a2a-server` mailbox dispatch, the `.coord` JSONL file-based protocol)
because they own the contention surface. Treat `a2a-lf` as the wire schema and
serialize it with `serde_json` (or `pbjson` via `a2a-pb`) on every send.

## Cross-platform

Linux (cible #1), Windows 11 Canary, and WASM all build cleanly. On
`wasm32-unknown-unknown` and `wasm32-wasi` the `uuid` dependency picks up the
`js` feature so v4/v7 generators use `Math.random` / browser CSPRNG instead of
calling into the OS.

## License

Apache-2.0. Copyright AGNTCY Contributors; aphrody-code packaging.

## Related

- `a2a` — short import alias for this same crate inside the Aphrody workspace.
- `a2a-pb` — prost-generated protobuf bindings for the wire format.
- `a2a-client` / `a2a-server` / `a2a-grpc` — transport layers built on top.
