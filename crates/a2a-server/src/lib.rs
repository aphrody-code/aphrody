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
pub mod auth;
pub mod commands;
pub mod env;
pub mod executor;
pub mod ext_routes;
pub mod handler;
pub mod jsonrpc;
pub mod middleware;
pub mod push;
pub mod rest;
pub mod sse;
pub mod task_store;
#[cfg(feature = "rustls")] pub mod tls;
pub mod workspace;

pub use agent_card::{AgentCardProducer, StaticAgentCard, WELL_KNOWN_AGENT_CARD_PATH};
pub use auth::{
    AnonymousUserBuilder, AuthUserBuilder, BasicValidator, BearerValidator,
    StaticBasicValidator, StaticBearerValidator, UserBuilder,
};
pub use commands::{
    Command, CommandArgument, CommandDescriptor, CommandExecutionResponse, CommandRegistry,
};
pub use env::{GEMINI_DIR, find_env_file, load_environment, parse_dotenv};
pub use executor::{AgentExecutor, ExecutorContext};
pub use ext_routes::ext_router;
pub use handler::{DefaultRequestHandler, RequestHandler};
pub use middleware::{CallContext, CallInterceptor, InterceptedHandler, ServiceParams, User};
pub use push::{HttpPushSender, InMemoryPushConfigStore, PushConfigStore};
pub use task_store::{FsTaskStore, InMemoryTaskStore, TaskStore};
pub use workspace::{WORKSPACE_PATH_ENV, resolve_workspace, workspace_override_configured};

#[cfg(test)]
pub(crate) mod test_util {
    pub(crate) fn install_crypto_provider() {
        #[cfg(feature = "rustls-tls")]
        {
            let _ = rustls::crypto::ring::default_provider().install_default();
        }
    }
}
