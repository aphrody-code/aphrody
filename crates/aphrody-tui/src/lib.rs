// SPDX-License-Identifier: Apache-2.0
//! Ratatui-based full-screen agent surface for aphrody.
//!
//! `aphrody-tui` is a thin client over the SQ/EQ agent protocol
//! (`aphrody-agent-proto`). It does not depend on the agent engine: it consumes
//! a stream of [`aphrody_agent_proto::Event`]s and produces
//! [`aphrody_agent_proto::Submission`]s over `tokio` mpsc channels, so it can be
//! unit-tested in isolation and wired to the real engine later without any
//! coupling.
//!
//! # Architecture
//!
//! - [`AppState`] is the pure, terminal-free state machine. [`AppState::apply_event`]
//!   folds protocol events into a scrollable transcript; [`AppState::on_key`]
//!   turns a [`KeyPress`] into an [`Action`]. Both are deterministic and carry
//!   the test suite.
//! - [`draw`] renders an [`AppState`] into a Ratatui [`ratatui::Frame`] and is a
//!   pure function of the state (testable against `TestBackend`).
//! - [`run`] owns the real terminal and multiplexes engine events, key input,
//!   and a redraw tick. It is the only async / I/O-bearing entry point.
//!
//! # Example wiring
//!
//! ```no_run
//! use tokio::sync::mpsc::unbounded_channel;
//! use aphrody_agent_proto::{Event, Submission};
//!
//! # async fn demo() -> std::io::Result<()> {
//! let (_event_tx, event_rx) = unbounded_channel::<Event>();
//! let (sub_tx, _sub_rx) = unbounded_channel::<Submission>();
//! // The engine drives `_event_tx` and reads `_sub_rx`; the TUI owns the
//! // other ends.
//! aphrody_tui::run(event_rx, sub_tx).await
//! # }
//! ```

#![doc(html_no_source)]

mod app;
mod state;
mod ui;

pub use app::run;
pub use state::Action;
pub use state::AppState;
pub use state::KeyPress;
pub use state::PendingApproval;
pub use state::TranscriptCell;
pub use ui::draw;
