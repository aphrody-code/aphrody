<!-- SPDX-License-Identifier: Apache-2.0 -->

# a2a-client

## What is `a2a-client`?

`a2a-client` is the high-level Rust client for the AGNTCY a2a/v0.4 protocol. It
sends `ask` / `ping` / `fact` envelopes (and the full A2A message / task /
push-config surface) to a peer agent and returns typed responses. The crate
provides a `Transport` trait, three first-class implementations (JSON-RPC,
REST, gRPC via the companion `a2a-grpc` crate), middleware hooks, and an
`AgentCard` resolver that auto-selects the correct transport for a remote
agent. The published name on crates.io is `a2a-client-lf`; the Rust import
path remains `a2a_client`.

## Install

```toml
[dependencies]
a2a-client = { package = "a2a-client-lf", version = "0.1" }
a2a = { package = "a2a-lf", version = "0.2" }
```

## Quick start

```rust
use a2a_client::{A2AClient, rest::RestTransport};
use reqwest::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let http = Client::builder().build()?;
    let transport = RestTransport::new(http, "http://localhost:8788/msg".into());
    let client = A2AClient::new(transport);
    // client.send_message(&req).await? ...
    Ok(())
}
```

## Public API

Source of truth: `crates/a2a-client/src/lib.rs`. The crate exports
`A2AClient`, the `Transport` and `TransportFactory` traits, `ServiceParams`,
and the `BoxStream` alias (which becomes `LocalBoxStream` on `wasm32`).
Submodules `auth`, `client`, `middleware`, and `transport` are available on
every target; `agent_card`, `factory`, `jsonrpc`, and `rest` are gated off
`target_os = "wasi"` because they pull in `reqwest`. `A2AClientFactory` is the
re-exported entry point for agent-card-driven transport selection on native
and `wasm32-unknown-unknown` targets.

## Transports

- HTTP via `RestTransport` (RESTful endpoints, default) and
  `JsonRpcTransport` (JSON-RPC 2.0 envelope).
- gRPC via `GrpcTransport` from the companion `a2a-grpc` crate.
- File-based JSONL inbox fallback: implement `Transport` against an `.coord`
  mailbox directory for offline peer-to-peer scenarios.

## Cross-platform

Linux (cible #1) and Windows are fully supported through `reqwest` over
`tokio` / `mio`. `wasm32-unknown-unknown` uses the browser `fetch` backend,
SSE streaming via `bytes_stream`, and `LocalBoxStream` because reqwest
futures are `!Send` in the single-threaded JS runtime. `wasm32-wasip1` builds
the trait definitions and data types only — the HTTP modules are
`cfg`-stripped until `wasi-http` (WASI Preview 2) lands.

## License

Apache-2.0. Copyright AGNTCY Contributors; aphrody-code packaging.

## Related

- `a2a-server` — server-side counterpart.
- `a2a-pb` — generated protobuf bindings.
- `a2a-grpc` — gRPC transport binding.
