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
