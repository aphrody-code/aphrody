// SPDX-License-Identifier: Apache-2.0
//! Error taxonomy for the `aphrody-tui` event loop.

use thiserror::Error;

/// All fatal conditions that can bubble out of [`crate::run`].
#[derive(Debug, Error)]
pub enum TuiError {
    /// Wraps any I/O error coming from crossterm or the terminal backend
    /// (raw mode toggling, alternate screen entry/exit, draw flush, etc.).
    #[error("terminal I/O failure: {0}")]
    Io(#[from] std::io::Error),

    /// Catch-all for invariants violated by user code (e.g. a widget that
    /// requested an area larger than the frame).  Carries a static message so
    /// it can be matched on cheaply in tests.
    #[error("terminal protocol error: {0}")]
    Terminal(&'static str),
}
