// SPDX-License-Identifier: Apache-2.0
// Copyright AGNTCY Contributors (https://github.com/agntcy)

// Platform note: `a2a-server` depends on `axum`, `tokio::net`, and `hyper`,
// which require OS-level TCP sockets. These are incompatible with
// `wasm32-unknown-unknown` and `wasm32-wasi`. Server-side HTTP/gRPC cannot
// run in a browser WASM context. Use `a2a-client` with the REST transport
// for WASM consumer scenarios.
#[cfg(target_family = "wasm")]
compile_error!(
    "a2a-server does not support wasm32 targets: axum/hyper/tokio::net require OS networking. \
     WASM agents should use a2a-client with RestTransport instead."
);

pub mod agent_card;
pub mod executor;
pub mod handler;
pub mod jsonrpc;
pub mod middleware;
pub mod push;
pub mod rest;
pub mod sse;
pub mod task_store;
#[cfg(feature = "rustls")] pub mod tls;

pub use agent_card::{AgentCardProducer, StaticAgentCard, WELL_KNOWN_AGENT_CARD_PATH};
pub use executor::{AgentExecutor, ExecutorContext};
pub use handler::{DefaultRequestHandler, RequestHandler};
pub use middleware::{CallContext, CallInterceptor, InterceptedHandler, ServiceParams, User};
pub use push::{HttpPushSender, InMemoryPushConfigStore, PushConfigStore};
pub use task_store::{InMemoryTaskStore, TaskStore};

#[cfg(test)]
pub(crate) mod test_util {
    pub(crate) fn install_crypto_provider() {
        #[cfg(feature = "rustls-tls")]
        {
            let _ = rustls::crypto::ring::default_provider().install_default();
        }
    }
}
