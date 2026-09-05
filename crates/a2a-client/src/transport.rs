// SPDX-License-Identifier: Apache-2.0
// Copyright AGNTCY Contributors (https://github.com/agntcy)
use std::collections::HashMap;

use a2a::*;
use async_trait::async_trait;
use auto_impl::auto_impl;

use crate::BoxStream;

pub type ServiceParams = HashMap<String, Vec<String>>;

/// The extension point for all protocol bindings.
///
/// Each protocol binding (JSON-RPC, REST, gRPC, or custom) implements this trait.
/// The [`TransportFactory`] creates instances of `Transport` for a given agent interface.
///
/// A blanket implementation for `Box<dyn Transport>` is automatically derived via
/// [`auto_impl`], enabling `A2AClient<Box<dyn Transport>>` for runtime-selected transports.
///
/// On wasm32 targets the trait uses `?Send` because reqwest futures are `!Send`
/// (the JS runtime is single-threaded and does not support cross-thread futures).
#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
#[auto_impl(Box)]
pub trait Transport: Send + Sync {
    /// Stable identifier for the protocol binding (e.g. `"jsonrpc"`, `"http+json"`,
    /// `"grpc"`). Used by diagnostics, logging, and the [`TransportKind`] surface
    /// that exposes the cross-platform default transport at runtime.
    ///
    /// Default implementation returns `"unknown"`; concrete bindings override.
    fn protocol_name(&self) -> &'static str {
        "unknown"
    }

    async fn send_message(
        &self,
        params: &ServiceParams,
        req: &SendMessageRequest,
    ) -> Result<SendMessageResponse, A2AError>;

    async fn send_streaming_message(
        &self,
        params: &ServiceParams,
        req: &SendMessageRequest,
    ) -> Result<BoxStream<'static, Result<StreamResponse, A2AError>>, A2AError>;

    async fn get_task(
        &self,
        params: &ServiceParams,
        req: &GetTaskRequest,
    ) -> Result<Task, A2AError>;

    async fn list_tasks(
        &self,
        params: &ServiceParams,
        req: &ListTasksRequest,
    ) -> Result<ListTasksResponse, A2AError>;

    async fn cancel_task(
        &self,
        params: &ServiceParams,
        req: &CancelTaskRequest,
    ) -> Result<Task, A2AError>;

    async fn subscribe_to_task(
        &self,
        params: &ServiceParams,
        req: &SubscribeToTaskRequest,
    ) -> Result<BoxStream<'static, Result<StreamResponse, A2AError>>, A2AError>;

    async fn create_push_config(
        &self,
        params: &ServiceParams,
        req: &TaskPushNotificationConfig,
    ) -> Result<TaskPushNotificationConfig, A2AError>;

    async fn get_push_config(
        &self,
        params: &ServiceParams,
        req: &GetTaskPushNotificationConfigRequest,
    ) -> Result<TaskPushNotificationConfig, A2AError>;

    async fn list_push_configs(
        &self,
        params: &ServiceParams,
        req: &ListTaskPushNotificationConfigsRequest,
    ) -> Result<ListTaskPushNotificationConfigsResponse, A2AError>;

    async fn delete_push_config(
        &self,
        params: &ServiceParams,
        req: &DeleteTaskPushNotificationConfigRequest,
    ) -> Result<(), A2AError>;

    async fn get_extended_agent_card(
        &self,
        params: &ServiceParams,
        req: &GetExtendedAgentCardRequest,
    ) -> Result<AgentCard, A2AError>;

    async fn destroy(&self) -> Result<(), A2AError>;
}

/// Factory that creates [`Transport`] instances from agent card interface declarations.
///
/// Each protocol binding provides its own `TransportFactory` implementation.
/// Register factories with [`A2AClientFactory`](crate::A2AClientFactory) to enable
/// automatic protocol negotiation.
#[cfg_attr(not(target_family = "wasm"), async_trait)]
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
pub trait TransportFactory: Send + Sync {
    /// Returns the protocol identifier this factory handles (e.g., "JSONRPC", "GRPC").
    fn protocol(&self) -> &str;

    /// Create a transport for the given agent interface.
    async fn create(
        &self,
        card: &AgentCard,
        iface: &AgentInterface,
    ) -> Result<Box<dyn Transport>, A2AError>;
}

/// Cross-platform identifier for the underlying HTTP backend the [`Transport`]
/// implementations use at runtime.
///
/// Returned by [`default_transport_kind`]. The variant is selected at compile
/// time from `cfg(target_arch)` / `cfg(target_os)`:
///
/// | Target                       | Backend          | Variant      |
/// |------------------------------|------------------|--------------|
/// | `x86_64-unknown-linux-gnu`   | tokio + hyper + rustls (HTTP/2) | `NativeHyper` |
/// | `x86_64-pc-windows-msvc`     | tokio + hyper + rustls (IOCP)   | `NativeHyper` |
/// | `aarch64-apple-darwin`       | tokio + hyper + rustls          | `NativeHyper` |
/// | `wasm32-unknown-unknown`     | browser `fetch` API             | `BrowserFetch` |
/// | `wasm32-wasip1`              | none — modules stripped         | `Unsupported` |
///
/// The `NativeHyper` backend is the same object on all native targets (reqwest
/// 0.13 → hyper 1.x), so platform-specific OS layers (epoll, IOCP, kqueue) are
/// hidden behind the tokio runtime. The `BrowserFetch` backend uses the host
/// browser's `Window.fetch` via the reqwest 0.13 wasm shim — no TLS crate is
/// involved (TLS is delegated to the browser).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransportKind {
    /// Native reqwest backend: tokio + hyper + rustls. Used on Linux, Windows,
    /// macOS, Android, iOS.
    NativeHyper,
    /// Browser `fetch` API backend via reqwest's wasm shim. Used on
    /// `wasm32-unknown-unknown`.
    BrowserFetch,
    /// No HTTP transport available at this build target. Currently
    /// `wasm32-wasip1` falls in this bucket because reqwest 0.13 still requires
    /// socket2/mio on `target_os = "wasi"`. Lift once WASI p2 / wasi-http is
    /// the default.
    Unsupported,
}

impl TransportKind {
    /// Human-readable name, suitable for logs and CLI output.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            TransportKind::NativeHyper => "native-hyper",
            TransportKind::BrowserFetch => "browser-fetch",
            TransportKind::Unsupported => "unsupported",
        }
    }
}

impl core::fmt::Display for TransportKind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Returns the [`TransportKind`] that this build of `a2a-client` will use for
/// HTTP I/O.
///
/// This is a `const fn` so it can be referenced from feature flags and
/// `static` initialisers. It performs no allocation, no I/O, no FFI.
///
/// ```
/// use a2a_client::transport::{TransportKind, default_transport_kind};
///
/// let kind = default_transport_kind();
/// assert_ne!(kind, TransportKind::Unsupported); // unless on wasi
/// println!("a2a-client backend: {kind}");
/// ```
#[must_use]
pub const fn default_transport_kind() -> TransportKind {
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    {
        TransportKind::BrowserFetch
    }
    #[cfg(target_os = "wasi")]
    {
        TransportKind::Unsupported
    }
    #[cfg(not(target_family = "wasm"))]
    {
        TransportKind::NativeHyper
    }
}

#[cfg(test)]
mod kind_tests {
    use super::{TransportKind, default_transport_kind};

    #[test]
    fn kind_str_is_stable() {
        assert_eq!(TransportKind::NativeHyper.as_str(), "native-hyper");
        assert_eq!(TransportKind::BrowserFetch.as_str(), "browser-fetch");
        assert_eq!(TransportKind::Unsupported.as_str(), "unsupported");
    }

    #[test]
    fn kind_display_matches_as_str() {
        for k in
            [TransportKind::NativeHyper, TransportKind::BrowserFetch, TransportKind::Unsupported]
        {
            assert_eq!(k.to_string(), k.as_str());
        }
    }

    #[cfg(not(target_family = "wasm"))]
    #[test]
    fn default_is_native_on_native_targets() {
        assert_eq!(default_transport_kind(), TransportKind::NativeHyper);
    }

    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    #[test]
    fn default_is_browser_fetch_on_wasm_browser() {
        assert_eq!(default_transport_kind(), TransportKind::BrowserFetch);
    }

    #[cfg(target_os = "wasi")]
    #[test]
    fn default_is_unsupported_on_wasi() {
        assert_eq!(default_transport_kind(), TransportKind::Unsupported);
    }
}
