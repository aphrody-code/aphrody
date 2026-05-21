// SPDX-License-Identifier: Apache-2.0
//! `NotebookClient` — high-level façade composing [`Auth`], session tokens
//! and [`HttpTransport`] behind a stable RPC surface.

use crate::auth::Auth;
use crate::error::Result;
use crate::transport::{HttpTransport, SessionTokens};

/// Top-level handle every public function in `notebooklm::{notebooks, sources,
/// chat, artifacts, research}` is built on.
#[derive(Debug, Clone)]
pub struct NotebookClient {
    pub(crate) transport: HttpTransport,
}

impl NotebookClient {
    /// Construct a client from a pre-resolved auth + token pair.
    pub fn new(auth: Auth, tokens: SessionTokens) -> Result<Self> {
        Ok(Self {
            transport: HttpTransport::new(auth, tokens)?,
        })
    }

    /// Bootstrap from environment (`NOTEBOOKLM_OAUTH_TOKEN`/`NOTEBOOKLM_COOKIES`)
    /// + caller-supplied tokens. Useful for CLI flows that want one-liner
    /// construction without manual auth juggling.
    pub fn from_env(tokens: SessionTokens) -> Result<Self> {
        let auth = Auth::from_env()?;
        Self::new(auth, tokens)
    }

    /// Borrow the transport (escape hatch for advanced low-level callers).
    pub fn transport(&self) -> &HttpTransport {
        &self.transport
    }

    /// Mutable accessor (for token refresh).
    pub fn transport_mut(&mut self) -> &mut HttpTransport {
        &mut self.transport
    }
}
