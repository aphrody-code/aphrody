<!-- SPDX-License-Identifier: Apache-2.0 -->

# a2a-server

## What is `a2a-server`?

`a2a-server` is the server-side counterpart to `a2a-client`. It accepts `ask`
/ `ping` / `fact` envelopes (plus the full A2A message / task / push-config
surface) from peer agents and dispatches them to user-supplied handler
functions. The crate is built on `axum`, `tower`, `hyper`, and `tokio`, and
ships routers for both the JSON-RPC and REST protocol bindings, middleware
hooks for cross-cutting concerns, an in-memory task store, and an
`AgentCardProducer` that publishes the canonical `.well-known/agent-card.json`
descriptor. Published as `a2a-server-lf` on crates.io; imported in Rust as
`a2a_server`.

## Install

```toml
[dependencies]
a2a-server = { package = "a2a-server-lf", version = "0.3" }
a2a = { package = "a2a-lf", version = "0.2" }
tokio = { version = "1", features = ["full"] }
axum = "0.7"
```

## Quick start

```rust
use std::sync::Arc;
use a2a_server::{DefaultRequestHandler, InMemoryTaskStore, jsonrpc::jsonrpc_router};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = Arc::new(InMemoryTaskStore::default());
    let handler = Arc::new(DefaultRequestHandler::new(store));
    let app = jsonrpc_router(handler);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8788").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
```

## Public API

Source of truth: `crates/a2a-server/src/lib.rs`. Public re-exports cover
`AgentCardProducer`, `StaticAgentCard`, `WELL_KNOWN_AGENT_CARD_PATH`,
`AgentExecutor`, `ExecutorContext`, `DefaultRequestHandler`, `RequestHandler`,
`CallContext`, `CallInterceptor`, `InterceptedHandler`, `ServiceParams`,
`User`, `HttpPushSender`, `InMemoryPushConfigStore`, `PushConfigStore`,
`InMemoryTaskStore`, and `TaskStore`. Submodules `agent_card`, `executor`,
`handler`, `jsonrpc`, `middleware`, `push`, `rest`, `sse`, and `task_store`
are public; `tls` is gated behind `feature = "rustls"`.

## Routing

Message-type dispatch is performed by the protocol routers. `jsonrpc_router`
mounts a single POST endpoint that routes on the JSON-RPC `method` field
(`message/send`, `message/stream`, `tasks/get`, `tasks/list`, `tasks/cancel`,
push-config CRUD, etc.). `rest_router` exposes the same operations as RESTful
resources (`/message:send`, `/message:stream`, `/extendedAgentCard`, ...).
`CallInterceptor` lets you hook authentication, tracing, and rate-limiting
into every request without rewriting the handler.

## Cross-platform

Linux (cible #1) and Windows are fully supported. The crate emits a
`compile_error!` on `wasm32` targets because `axum` / `hyper` / `tokio::net`
require OS-level TCP sockets — WASM agents should consume the API through
`a2a-client` with `RestTransport` instead.

## License

Apache-2.0. Copyright AGNTCY Contributors; aphrody-code packaging.

## Related

- `a2a-client` — client-side counterpart.
- `a2a-pb` — generated protobuf bindings shared by both peers.
