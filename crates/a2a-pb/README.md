<!-- SPDX-License-Identifier: Apache-2.0 -->

# a2a-pb

Generated protobuf bindings and conversion utilities for the AGNTCY A2A v1
protocol, used throughout the Aphrody agent-to-agent stack.

## What is `a2a-pb`?

`a2a-pb` provides the prost-generated message types and tonic gRPC stubs for
the [AGNTCY a2a v0.4](https://github.com/agntcy/a2a-spec) specification. The
pre-generated source in `src/gen/` is the authoritative copy committed to the
repository. Codegen is gated behind the `A2A_PB_REGEN=1` environment variable
so that ordinary builds are hermetic and do not require `protoc` or network
access.

Two module surfaces are exposed:

- `proto` — prost message structs plus tonic `A2aServiceClient` /
  `A2aServiceServer`. Not available on `wasm32` because tonic transport
  transitively requires `mio`/`tokio::net`.
- `protojson` / `protojson_conv` — pure prost message structs with pbjson
  serde derives. Available on all targets, including `wasm32`. WASM consumers
  must use `protojson_conv` for serialisation.

## Install

Add to `Cargo.toml`:

```toml
[dependencies]
a2a-pb = "0.1.8"
```

Or use a git path dependency during development:

```toml
a2a-pb = { path = "../a2a-pb" }
```

## Generated types

All types live under the `a2a_pb::proto` module on native targets and under
`a2a_pb::protojson` on all targets (including wasm32). The five most important
public types are:

| Type | Proto origin | Role |
|---|---|---|
| `Task` | `lf/a2a/v1/task.proto` | Core unit of agent work; carries `status`, `artifacts`, and `history`. |
| `Message` | `lf/a2a/v1/message.proto` | Single communication unit; holds `role`, `parts`, and optional metadata. |
| `Part` | `lf/a2a/v1/part.proto` | Content fragment inside a `Message`; oneof `text`, `raw`, `url`, or `data`. |
| `Artifact` | `lf/a2a/v1/artifact.proto` | Task output container; one or more `Part` values plus an artifact ID. |
| `TaskStatus` | `lf/a2a/v1/task.proto` | Current state snapshot for a `Task`; wraps `TaskState` enum and timestamp. |

The tonic service traits `A2aServiceClient<T>` and `A2aServiceServer<T>` are
generated in `proto::a2a_service_client` and `proto::a2a_service_server`.

The `pbconv` module exposes `to_proto_struct`, `from_proto_struct`,
`json_value_to_proto_value`, and `proto_value_to_json_value` for lossless
round-trips between `serde_json::Value` maps and `prost_types::Struct`.

The `protojson_conv` module exposes the `ProtoJsonPayload` trait along with
`to_value`, `from_value`, and `from_str` helpers for serialising native `a2a`
crate types via the pbjson ProtoJSON wire format.

## Regeneration

To regenerate from the `.proto` sources (requires `protoc`):

```bash
A2A_PB_REGEN=1 cargo build -p a2a-pb --locked
```

Without `A2A_PB_REGEN=1`, `build.rs` skips codegen entirely so that published
builds are hermetic. This gating is required because crates.io rejects build
scripts that write files outside `$OUT_DIR`; the committed `src/gen/` files
serve as the canonical generated output for all downstream consumers.

## Cross-platform

| Target | `proto` + tonic | `protojson` + `protojson_conv` |
|---|---|---|
| Linux (x86_64-unknown-linux-gnu) | Supported | Supported |
| Windows (x86_64-pc-windows-msvc) | Supported | Supported |
| WASM (wasm32-unknown-unknown) | Not available | Supported |

## License

Apache-2.0. See [LICENSE](../../LICENSE).

## Related crates

- [`a2a-client`](../a2a-client) — async client built on `a2a-pb` types.
- [`a2a-server`](../a2a-server) — tonic server implementation.
- [`a2a-grpc`](../a2a-grpc) — gRPC transport layer integrating client and server.
