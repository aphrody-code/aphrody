// SPDX-License-Identifier: Apache-2.0
//! # aphrody-messaging
//!
//! Tier-1 messaging connectors derived from the
//! [`docs/audits/2026-05-19-hermes-agent-vs-aphrody.md`](../../../docs/audits/2026-05-19-hermes-agent-vs-aphrody.md)
//! audit:
//!
//! - **Telegram** — Bot API `sendMessage` (`telegram` module).
//! - **Slack**    — Web API `chat.postMessage` (`slack` module).
//! - **SMTP**     — native RFC 5321 client with STARTTLS / implicit TLS support
//!                  (`email` module; non-wasm only).
//!
//! Each provider implements the object-safe
//! [`connector::Connector`] trait, which lets callers route a single
//! [`connector::Message`] through an arbitrary dyn dispatch table.
//!
//! Distinct from the heavier [`aphrody-channels`] crate (full
//! `MessagingChannel` trait with attachments, inbox streaming, room listing),
//! `aphrody-messaging` is the deliberately minimal outbound-only surface used
//! by alerting / notification pipelines.
//!
//! ## Layering
//!
//! ```text
//!         Connector trait (connector.rs)
//!        ╱           │              ╲
//! TelegramConnector  SlackConnector  SmtpConnector
//! ```
//!
//! Adding a new provider is a single-file change: implement
//! `async fn send` + `async fn health` + `fn id` and expose the struct
//! through `lib.rs`.

#![deny(unsafe_code)]

pub mod connector;
pub mod slack;
pub mod telegram;

#[cfg(not(target_arch = "wasm32"))]
pub mod email;

pub use connector::{Connector, Message, MessageError, MessageId, MessageResult, ParseMode};
pub use slack::SlackConnector;
pub use telegram::TelegramConnector;

#[cfg(not(target_arch = "wasm32"))]
pub use email::{SmtpConnector, TlsMode};
