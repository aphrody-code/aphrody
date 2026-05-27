// SPDX-License-Identifier: Apache-2.0
//! The event sink: where the engine publishes [`Event`]s for a surface to
//! consume.

use aphrody_agent_proto::{Event, EventMsg};
use tokio::sync::mpsc;

/// A wrapper over a [`tokio::sync::mpsc::UnboundedSender<Event>`].
///
/// The engine emits every [`EventMsg`] through an [`EventSink`]. The sink is
/// cheap to clone (it shares the underlying channel) so it can be handed to
/// concurrent helpers. Send failures (a dropped receiver) are tolerated: the
/// engine keeps running and finishes its bookkeeping rather than aborting,
/// because losing a consumer must not corrupt the rollout or the session
/// history.
#[derive(Debug, Clone)]
pub struct EventSink {
    tx: mpsc::UnboundedSender<Event>,
}

impl EventSink {
    /// Wrap an existing unbounded sender.
    #[must_use]
    pub fn new(tx: mpsc::UnboundedSender<Event>) -> Self {
        Self { tx }
    }

    /// Build a fresh unbounded channel, returning the sink and its receiver.
    #[must_use]
    pub fn unbounded() -> (Self, mpsc::UnboundedReceiver<Event>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (Self::new(tx), rx)
    }

    /// Emit a pre-built [`Event`].
    ///
    /// Returns `false` if the receiver has been dropped (the event is lost but
    /// the engine continues).
    pub fn send_event(&self, event: Event) -> bool {
        self.tx.send(event).is_ok()
    }

    /// Wrap `msg` in a fresh [`Event`] and emit it.
    ///
    /// Returns `false` if the receiver has been dropped.
    pub fn emit(&self, msg: EventMsg) -> bool {
        self.send_event(Event::new(msg))
    }

    /// Whether the consuming receiver is still alive.
    #[must_use]
    pub fn is_open(&self) -> bool {
        !self.tx.is_closed()
    }
}
