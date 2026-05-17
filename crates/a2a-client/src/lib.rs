// Copyright AGNTCY Contributors (https://github.com/agntcy)
// SPDX-License-Identifier: Apache-2.0

// Platform support:
// - Native (Linux / Windows / macOS): full support. reqwest uses OS networking
//   (mio / hyper / rustls). The `rustls-tls` and `native-tls` features are
//   only meaningful on native targets.
// - wasm32 (unknown-unknown / wasip1): the JSON-RPC and REST transports work
//   via reqwest's browser `fetch` backend (no TLS crate needed). SSE streaming
//   via `response.bytes_stream()` is supported. gRPC transport is server-side
//   only and therefore excluded on wasm32.

pub mod agent_card;
pub mod auth;
pub mod client;
pub mod factory;
pub mod jsonrpc;
pub mod middleware;
mod push_config_compat;
pub mod rest;
pub mod transport;

pub use client::A2AClient;
pub use factory::A2AClientFactory;
pub use transport::{ServiceParams, Transport, TransportFactory};

// On native targets, `BoxStream` is `Pin<Box<dyn Stream + Send + 'static>>`.
// On wasm32, reqwest futures are `!Send` (JS single-thread model), so we use
// `LocalBoxStream` (`Pin<Box<dyn Stream + 'static>>`, no Send bound) instead.
// Both types have identical usage ergonomics for consumers that don't spawn.
#[cfg(not(target_family = "wasm"))]
pub use futures::stream::BoxStream;
#[cfg(target_family = "wasm")]
pub use futures::stream::LocalBoxStream as BoxStream;

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

// reqwest::Certificate and rustls are not available on wasm32.
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
