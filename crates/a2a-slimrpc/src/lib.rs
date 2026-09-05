// SPDX-License-Identifier: Apache-2.0
// Copyright AGNTCY Contributors (https://github.com/agntcy)
pub mod client;
mod common;
pub mod errors;
pub mod server;

pub use client::{SlimRpcTransport, SlimRpcTransportFactory, parse_slimrpc_target};
pub use server::SlimRpcHandler;
