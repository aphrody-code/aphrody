<!-- SPDX-License-Identifier: Apache-2.0 -->

# a2a

## What is `a2a`?

`a2a` is the top-level Rust facade for the AGNTCY a2a/v0.4 agent-to-agent
protocol inside Aphrody. It bundles the wire-compatible type system (messages,
tasks, artifacts, parts, JSON-RPC envelopes, streaming events, error details)
that every other crate in the `a2a-*` family consumes. The crate is published
on crates.io as `a2a-lf` (the upstream a2a-rs convention) and imported in Rust
under the short name `a2a`, so a typical downstream consumer writes
`use a2a::*;` and gets every protocol type plus its serde derives ready to
go. There is no MSRV trickery and no runtime dependency on tokio, axum, or
reqwest — those land in the companion `a2a-client` and `a2a-server` crates.

## Install

```toml
[dependencies]
a2a = { package = "a2a-lf", version = "0.3" }
```

For end-to-end agent loops you usually want the full stack:

```toml
a2a-client = { package = "a2a-client-lf", version = "0.1" }
a2a-server = { package = "a2a-server-lf", version = "0.3" }
a2a-grpc   = "0.2"
a2a-pb     = "0.1"
```

## Quick start

```rust
use a2a::{Message, Part, Role};

let message = Message::new(Role::User, vec![Part::text("hello peer")]);
assert_eq!(message.text(), Some("hello peer"));
```

## Re-exports

Source of truth: `crates/a2a/src/lib.rs`. The crate re-exports every public
item from six submodules through `pub use <mod>::*;`:

- `agent_card` — `AgentCard` and the well-known descriptor types served at
  `.well-known/agent-card.json`.
- `types` — `Message`, `Task`, `Artifact`, `Part`, `PartContent`, `Role`,
  `TaskState`, `TaskStatus`, `SendMessageRequest`, `SendMessageResponse`,
  `SendMessageConfiguration`, `GetTaskRequest`, `ListTasksRequest`,
  `ListTasksResponse`, plus ID constructors `new_task_id`, `new_context_id`,
  `new_message_id`, `new_artifact_id`.
- `event` — streaming envelopes `StreamResponse`, `TaskStatusUpdateEvent`,
  `TaskArtifactUpdateEvent` for SSE / gRPC streams.
- `jsonrpc` — `JsonRpcRequest`, `JsonRpcResponse`, `JsonRpcError`, `JsonRpcId`.
- `errors` — `A2AError` plus protocol error code helpers.
- `errordetails` — typed error payload structures for richer client surfaces.

Top-level constants: `VERSION = "1.0"`, `SVC_PARAM_VERSION = "A2A-Version"`,
`SVC_PARAM_EXTENSIONS = "A2A-Extensions"`.

## Cross-platform

Linux (cible #1), Windows 11 Canary, and WASM are all supported. The
`wasm32-unknown-unknown` target activates the `uuid/js` feature to delegate
v4/v7 entropy to `Math.random` / browser CSPRNG, so the same `Message::new`
flow runs unchanged inside a browser-hosted agent loop.

## License

Apache-2.0. Copyright AGNTCY Contributors; aphrody-code packaging.

## Related

- `a2a-pb` — prost-generated protobuf bindings and pbjson conversion helpers.
- `a2a-client` — async client with JSON-RPC / REST / gRPC transports.
- `a2a-server` — `axum`-based server framework and task store.
- `a2a-grpc` — gRPC binding layer connecting client and server over tonic.
- `a2a-lf` — same crate, alternate doc landing page for the published name.
