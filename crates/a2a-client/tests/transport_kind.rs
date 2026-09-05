// SPDX-License-Identifier: Apache-2.0
// Copyright AGNTCY Contributors (https://github.com/agntcy)

//! Integration test asserting the cross-platform [`Transport`] surface is
//! publicly reachable and that [`default_transport_kind`] resolves to the
//! correct backend for the build target.
//!
//! This test is the public consumer contract for PLAN item "Cross-platform:
//! trait `Transport` portable (HTTP/2 Linux, IOCP Win, fetch wasm)". On
//! native targets it instantiates a real [`JsonRpcTransport`] +
//! [`RestTransport`] through their public constructors and checks the
//! `protocol_name()` discriminator. On `wasm32-wasip1` the reqwest-backed
//! constructors are cfg-stripped (PLAN P-Wasm) so only the trait + kind
//! surface is exercised.

use a2a_client::{Transport, TransportKind, default_transport_kind};

#[test]
fn default_transport_kind_is_in_range() {
    let kind = default_transport_kind();
    // Whatever the target, the resolution function must return one of the
    // documented variants — never a panic / never undefined behaviour.
    assert!(matches!(
        kind,
        TransportKind::NativeHyper | TransportKind::BrowserFetch | TransportKind::Unsupported
    ));
}

#[cfg(not(target_family = "wasm"))]
#[test]
fn native_target_resolves_to_hyper() {
    assert_eq!(default_transport_kind(), TransportKind::NativeHyper);
    assert_eq!(default_transport_kind().as_str(), "native-hyper");
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
#[test]
fn browser_wasm_resolves_to_fetch() {
    assert_eq!(default_transport_kind(), TransportKind::BrowserFetch);
    assert_eq!(default_transport_kind().as_str(), "browser-fetch");
}

#[cfg(target_os = "wasi")]
#[test]
fn wasi_resolves_to_unsupported() {
    // reqwest 0.13 still requires socket2/mio on target_os = "wasi", so the
    // HTTP transports are cfg-stripped at this target. The trait + kind
    // surface remain available.
    assert_eq!(default_transport_kind(), TransportKind::Unsupported);
}

// Compile-time-only contract for wasm32-unknown-unknown: assert the public
// `JsonRpcTransport` constructor signature is reachable via the public
// `reqwest::Client` re-export when the target is the browser. We cannot
// actually run wasm tests here (no wasm-bindgen-test runner wired), but
// `cargo check --target wasm32-unknown-unknown --tests` flips this into a
// type-check that catches any regression that hides the constructor behind
// a feature gate that's off by default on wasm.
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
#[allow(dead_code)]
fn browser_jsonrpc_transport_compiles() {
    use a2a_client::jsonrpc::JsonRpcTransport;
    let client = reqwest::Client::new();
    let _transport: Box<dyn Transport> =
        Box::new(JsonRpcTransport::new(client, "https://example.invalid/jsonrpc".to_string()));
}

#[cfg(all(not(target_family = "wasm"), feature = "rustls-tls"))]
#[test]
fn jsonrpc_transport_advertises_protocol_name() {
    use a2a_client::jsonrpc::JsonRpcTransport;

    // The rustls 0.23 CryptoProvider must be installed before any
    // reqwest::Client::new() on native rustls builds; otherwise the call
    // panics with `No provider set`. Install once for the test process.
    let _ = rustls::crypto::ring::default_provider().install_default();

    let client = reqwest::Client::new();
    let transport: Box<dyn Transport> =
        Box::new(JsonRpcTransport::new(client, "http://127.0.0.1:0/jsonrpc".to_string()));
    assert_eq!(transport.protocol_name(), "jsonrpc");
}

#[cfg(all(not(target_family = "wasm"), feature = "rustls-tls"))]
#[test]
fn rest_transport_advertises_protocol_name() {
    use a2a_client::rest::RestTransport;

    let _ = rustls::crypto::ring::default_provider().install_default();

    let client = reqwest::Client::new();
    let transport: Box<dyn Transport> =
        Box::new(RestTransport::new(client, "http://127.0.0.1:0".to_string()));
    assert_eq!(transport.protocol_name(), "http+json");
}
