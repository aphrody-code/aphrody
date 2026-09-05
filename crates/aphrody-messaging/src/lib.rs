// SPDX-License-Identifier: Apache-2.0
//! # aphrody-messaging
//!
//! Unified messaging crate combining:
//!
//! - **Outbound connectors** (`connector`, `slack`, `telegram`, `email`) —
//!   minimal, object-safe [`Connector`] trait for alerting / notification.
//! - **Bidirectional channels** ([`channels`]) — full [`channels::MessagingChannel`]
//!   trait with attachments, inbox streaming, room listing (Slack, Telegram,
//!   Matrix, Discord, X). Merged from the former `aphrody-channels` crate.
//!
//! ## Layering
//!
//! ```text
//!  channels::MessagingChannel (rich, bidirectional)
//!      Discord / Slack / Telegram / Matrix / X
//!
//!  connector::Connector (minimal, outbound-only)
//!      TelegramConnector / SlackConnector / SmtpConnector
//! ```

#![deny(unsafe_code)]

pub mod connector;
pub mod slack;
pub mod telegram;

#[cfg(not(target_arch = "wasm32"))]
pub mod email;

// Bidirectional messaging channels — merged from the former `aphrody-channels` crate.
pub mod channels;

pub use connector::{Connector, Message, MessageError, MessageId, MessageResult, ParseMode};
pub use slack::SlackConnector;
pub use telegram::TelegramConnector;

#[cfg(not(target_arch = "wasm32"))]
pub use email::{SmtpConnector, TlsMode};
