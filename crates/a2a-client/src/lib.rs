// SPDX-License-Identifier: Apache-2.0
// Copyright AGNTCY Contributors (https://github.com/agntcy)

// Platform support:
//
// - Native (Linux / Windows / macOS): full support. reqwest uses OS networking (mio / hyper /
//   rustls). The `rustls-tls` and `native-tls` features are only meaningful on native targets.
//
// - wasm32-unknown-unknown (browser): JSON-RPC + REST transports via reqwest browser `fetch`
//   backend. No TLS crate needed. SSE streaming via `response.bytes_stream()`. gRPC (server-side
//   only) excluded. Futures are `!Send` — BoxStream is LocalBoxStream on this target.
//
// - wasm32-wasip1: reqwest 0.13 falls back to the native HTTP stack on WASI (target_os = "wasi"),
//   which requires socket2/mio — syscalls that WASI p1 does not expose. The reqwest-dependent
//   modules (jsonrpc, rest, agent_card, factory) are therefore cfg-stripped on WASI. The trait
//   definitions (Transport, TransportFactory), data types, middleware, and auth ARE available and
//   compile cleanly. A future upgrade to WASI p2 / wasi-http can lift this restriction.

// Modules available on all targets (trait definitions, data types, helpers).
pub mod auth;
pub mod client;
pub mod middleware;
pub mod transport;

// Modules that require reqwest: available on native + wasm32-unknown-unknown,
// excluded on wasm32-wasip1 (target_os = "wasi").
#[cfg(not(target_os = "wasi"))] pub mod agent_card;
#[cfg(not(target_os = "wasi"))] pub mod factory;
#[cfg(not(target_os = "wasi"))] pub mod jsonrpc;
#[cfg(not(target_os = "wasi"))] mod push_config_compat;
#[cfg(not(target_os = "wasi"))] pub mod rest;

pub use client::A2AClient;
#[cfg(not(target_os = "wasi"))] pub use factory::A2AClientFactory;
// On native targets, `BoxStream` is `Pin<Box<dyn Stream + Send + 'static>>`.
// On wasm32 (both unknown-unknown and wasip1), reqwest futures are `!Send`
// (single-thread JS / WASI model), so we use `LocalBoxStream` instead.
#[cfg(not(target_family = "wasm"))] pub use futures::stream::BoxStream;
#[cfg(target_family = "wasm")]
pub use futures::stream::LocalBoxStream as BoxStream;
pub use transport::{
    ServiceParams, Transport, TransportFactory, TransportKind, default_transport_kind,
};

// Used by jsonrpc, rest, push_config_compat — all excluded on wasm32-wasip1.
#[cfg(not(target_os = "wasi"))]
pub(crate) fn a2a_error_from_details(
    code: i32,
    message: String,
    details: Vec<a2a::TypedDetail>,
) -> a2a::A2AError {
    use a2a::{error_code, errordetails, reason_to_error_code};
    use serde_json::Value;

    let mut code = code;
    let mut message = message;

    for detail in &details {
        match detail.type_url.as_str() {
            errordetails::BAD_REQUEST_TYPE => {
                if let Some(Value::Array(violations)) = detail.value.get("fieldViolations") {
                    let violation_strs: Vec<String> = violations
                        .iter()
                        .filter_map(|v| {
                            let field = v.get("field")?.as_str()?;
                            let desc = v.get("description")?.as_str()?;
                            if field.is_empty() {
                                Some(desc.to_string())
                            } else {
                                Some(format!("{field}: {desc}"))
                            }
                        })
                        .collect();
                    if !violation_strs.is_empty() {
                        message = format!("{}: {}", message, violation_strs.join("; "));
                    }
                }
                if code == error_code::INTERNAL_ERROR {
                    code = error_code::INVALID_PARAMS;
                }
            },
            errordetails::ERROR_INFO_TYPE => {
                if let Some(Value::String(domain)) = detail.value.get("domain") {
                    if domain == errordetails::PROTOCOL_DOMAIN {
                        if let Some(Value::String(reason)) = detail.value.get("reason") {
                            if let Some(c) = reason_to_error_code(reason) {
                                code = c;
                            }
                        }
                    }
                }
            },
            _ => {},
        }
    }

    a2a::A2AError { code, message, details: (!details.is_empty()).then_some(details) }
}

// reqwest::Certificate and rustls are not available on wasm32 / WASI.
// The `rustls-tls` / `native-tls` features only activate on native targets.
#[cfg(all(not(target_family = "wasm"), any(feature = "rustls-tls", feature = "native-tls")))]
pub(crate) fn build_reqwest_client_with_root_pem(
    pem: &[u8],
) -> Result<reqwest::Client, a2a::A2AError> {
    let cert = reqwest::Certificate::from_pem(pem)
        .map_err(|e| a2a::A2AError::internal(format!("invalid PEM certificate: {e}")))?;
    reqwest::Client::builder()
        .add_root_certificate(cert)
        .build()
        .map_err(|e| a2a::A2AError::internal(format!("failed to build HTTP client: {e}")))
}

#[cfg(test)]
pub(crate) mod test_utils {
    // rcgen and rustls are native-only; this module is only reached on native.
    #[cfg(not(target_family = "wasm"))]
    pub(crate) fn rcgen_self_signed_ca_pem() -> Vec<u8> {
        let mut params = rcgen::CertificateParams::new(Vec::<String>::new()).unwrap();
        params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        params.distinguished_name.push(rcgen::DnType::CommonName, "Test CA");
        let key = rcgen::KeyPair::generate().unwrap();
        let cert = params.self_signed(&key).unwrap();
        cert.pem().into_bytes()
    }

    /// Install the rustls `ring` `CryptoProvider` exactly once for the whole
    /// test process. Required because `reqwest::Client::new()` and
    /// `reqwest::Client::builder().build()` instantiate a rustls
    /// `ClientConfig`, and rustls 0.23+ refuses to pick a default provider
    /// implicitly. Without this every transport test panics with
    /// `No provider set` (origin: reqwest/async_impl/client.rs).
    #[cfg(not(target_family = "wasm"))]
    pub(crate) fn install_rustls_provider() {
        use std::sync::Once;
        static ONCE: Once = Once::new();
        ONCE.call_once(|| {
            let _ = rustls::crypto::ring::default_provider().install_default();
        });
    }
}
