// Copyright AGNTCY Contributors (https://github.com/agntcy)
// SPDX-License-Identifier: Apache-2.0

// Platform support:
// - `proto` module and `pbconv` are NOT available on wasm32 because the generated gRPC stubs
//   (A2aServiceClient / A2aServiceServer) reference `tonic::transport::Channel` which transitively
//   requires mio / tokio::net.
// - `protojson` and `protojson_conv` ARE available on all targets including wasm32: they only use
//   prost message structs + pbjson serde impls, no networking primitives. WASM consumers should use
//   `protojson_conv` directly.

/// Generated gRPC stubs and prost message types for the A2A v1 protocol.
///
/// Not available on wasm32 targets (requires tonic transport / mio).
/// Use `protojson_conv` for serialization on wasm32.
#[cfg(not(target_family = "wasm"))]
#[allow(clippy::all, clippy::pedantic, clippy::nursery, clippy::restriction, warnings)]
pub mod proto {
    include!("gen/lf.a2a.v1.rs");
}

/// ProtoJSON-capable generated protobuf types for the A2A v1 protocol.
///
/// Available on all targets including wasm32. These are pure prost message
/// structs with pbjson serde derives — no tonic or networking dependencies.
#[allow(clippy::all, clippy::pedantic, clippy::nursery, clippy::restriction, warnings)]
pub mod protojson {
    include!(concat!(env!("OUT_DIR"), "/lf.a2a.v1.rs"));
    include!(concat!(env!("OUT_DIR"), "/lf.a2a.v1.serde.rs"));
}

/// Proto ↔ native `a2a` type conversion helpers.
///
/// Not available on wasm32 targets (depends on `proto` which requires tonic).
#[cfg(not(target_family = "wasm"))]
pub mod pbconv;

/// ProtoJSON serialisation/deserialisation utilities.
///
/// Available on all targets including wasm32.
pub mod protojson_conv;

#[cfg(not(target_family = "wasm"))] pub use proto::*;
