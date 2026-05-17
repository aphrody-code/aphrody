// Copyright AGNTCY Contributors (https://github.com/agntcy)
// SPDX-License-Identifier: Apache-2.0

// Platform note: `a2a-grpc` depends on `tonic` (gRPC over HTTP/2) which
// requires `tokio::net` / `mio` OS-level sockets. These are fundamentally
// incompatible with `wasm32-unknown-unknown` and `wasm32-wasi`.
// gRPC-Web-over-WASM is not yet supported; use REST transport via
// `a2a-client` with `RestTransportFactory` for WASM consumers.
#[cfg(target_family = "wasm")]
compile_error!(
    "a2a-grpc does not support wasm32 targets: tonic/mio require OS-level TCP networking. For \
     WASM consumers use a2a-client with RestTransportFactory."
);

pub mod client;
pub mod errors;
pub mod server;

pub use client::{GrpcTransport, GrpcTransportFactory};
pub use server::GrpcHandler;
#[cfg(feature = "rustls")] pub use tokio_rustls::rustls;
