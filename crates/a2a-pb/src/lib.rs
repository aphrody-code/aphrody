// Copyright AGNTCY Contributors (https://github.com/agntcy)
// SPDX-License-Identifier: Apache-2.0

// Platform note: `a2a-pb` depends on `tonic` which transitively requires
// `tokio::net` (via hyper/mio). These transitive dependencies are incompatible
// with `wasm32-unknown-unknown` and `wasm32-wasi`. If you need protobuf type
// definitions on WASM, use the `a2a` crate directly (plain Serde types).
#[cfg(target_family = "wasm")]
compile_error!(
    "a2a-pb does not support wasm32 targets: tonic/mio require OS-level networking. \
     Use the `a2a` crate (plain Serde types) for WASM consumers instead."
);

/// Generated protobuf types for the A2A v1 protocol.
#[allow(clippy::all, clippy::pedantic, clippy::nursery, clippy::restriction, warnings)]
pub mod proto {
    include!("gen/lf.a2a.v1.rs");
}

/// ProtoJSON-capable generated protobuf types for the A2A v1 protocol.
#[allow(clippy::all, clippy::pedantic, clippy::nursery, clippy::restriction, warnings)]
pub mod protojson {
    include!(concat!(env!("OUT_DIR"), "/lf.a2a.v1.rs"));
    include!(concat!(env!("OUT_DIR"), "/lf.a2a.v1.serde.rs"));
}

pub mod pbconv;
pub mod protojson_conv;

pub use proto::*;
