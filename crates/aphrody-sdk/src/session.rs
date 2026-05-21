// SPDX-License-Identifier: Apache-2.0
//! Session management — re-exports + a `SessionManager` facade.
//!
//! The TS SDK keeps per-call session state inside the `GeminiCliSession`
//! object. Our Rust port splits the responsibilities:
//!
//! * [`aphrody_session::Session`] owns the *in-memory* event log.
//! * [`aphrody_session::JsonlFileStore`] owns the *on-disk* persistence.
//! * [`SessionManager`] (this module) is the SDK-facing facade that combines
//!   them: it creates new sessions, persists turns, and exposes a tiny
//!   read-side API for resume / list / delete.
//!
//! TS provenance: `packages/gemini-cli/packages/sdk/src/session.ts`.

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::Mutex;

pub use aphrody_session::{
    JsonlFileStore, Session, SessionError, SessionEvent, SessionId, SessionStore, ToolCall, Turn,
    TurnRole,
};

use crate::types::{SdkError, SdkResult};

// ---------------------------------------------------------------------------
// SessionManager
// ---------------------------------------------------------------------------

/// In-process session manager.
///
/// Holds the *currently active* session in memory and an optional persistent
/// store on disk. Every `add_turn` / `add_tool_call` call is mirrored to the
/// store synchronously (when persistence is configured) so a crash never
/// loses more than the in-flight turn.
///
/// PUBLIC API — semver stable from v1.0.
#[derive(Clone)]
pub struct SessionManager {
    inner: Arc<Mutex<Session>>,
    store: Option<JsonlFileStore>,
}

impl std::fmt::Debug for SessionManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionManager")
            .field("store_dir", &self.store.as_ref().map(JsonlFileStore::dir))
            .finish_non_exhaustive()
    }
}

impl SessionManager {
    /// Spin up a fresh session. When `store_dir` is `Some(dir)`, every
    /// mutation is persisted to `{dir}/{session_id}.jsonl`.
    ///
    /// PUBLIC API — semver stable from v1.0.
    #[must_use]
    pub fn new(store_dir: Option<PathBuf>, metadata: serde_json::Value) -> Self {
        let id = SessionId::generate();
        let session = Session::new(id, metadata);
        let store = store_dir.map(JsonlFileStore::new);
        Self {
            inner: Arc::new(Mutex::new(session)),
            store,
        }
    }

    /// Spin up a manager around a pre-existing session id (used when the
    /// caller wants to re-use an external id space, e.g. matching a
    /// request-id flowing through HTTP / gRPC).
    ///
    /// # Errors
    /// Returns [`SdkError::Session`] if `id` is invalid.
    pub fn with_session_id(
        id: impl Into<String>,
        store_dir: Option<PathBuf>,
        metadata: serde_json::Value,
    ) -> SdkResult<Self> {
        let id = SessionId::new(id.into())?;
        let session = Session::new(id, metadata);
        Ok(Self {
            inner: Arc::new(Mutex::new(session)),
            store: store_dir.map(JsonlFileStore::new),
        })
    }

    /// Borrow the current session id.
    pub async fn current_id(&self) -> SessionId {
        self.inner.lock().await.id.clone()
    }

    /// Append a turn to the active session. Persists immediately when
    /// `store_dir` was set.
    ///
    /// # Errors
    /// Propagates IO / JSON failures from the underlying [`SessionStore`].
    pub async fn add_turn(&self, turn: Turn) -> SdkResult<()> {
        let mut guard = self.inner.lock().await;
        guard.add_turn(turn);
        if let Some(store) = &self.store {
            store.save_session(&guard).await?;
        }
        Ok(())
    }

    /// Append a tool call to the active session.
    ///
    /// # Errors
    /// Propagates IO / JSON failures from the underlying [`SessionStore`].
    pub async fn add_tool_call(&self, call: ToolCall) -> SdkResult<()> {
        let mut guard = self.inner.lock().await;
        guard.add_tool_call(call);
        if let Some(store) = &self.store {
            store.save_session(&guard).await?;
        }
        Ok(())
    }

    /// Finalize the active session with a `SessionEnd` event and persist.
    ///
    /// # Errors
    /// Propagates IO / JSON failures from the underlying [`SessionStore`].
    pub async fn end(&self) -> SdkResult<()> {
        let mut guard = self.inner.lock().await;
        guard.end();
        if let Some(store) = &self.store {
            store.save_session(&guard).await?;
        }
        Ok(())
    }

    /// Snapshot the active session (clone of the in-memory aggregate).
    pub async fn snapshot(&self) -> Session {
        self.inner.lock().await.clone()
    }

    /// Resume a previously persisted session by id. Replaces the in-memory
    /// session with the loaded one. Requires `store_dir` to have been set.
    ///
    /// # Errors
    /// Returns [`SdkError::InvalidConfig`] when no store is configured,
    /// [`SdkError::Session`] when the id is not found on disk.
    pub async fn resume(&self, id: &SessionId) -> SdkResult<()> {
        let Some(store) = &self.store else {
            return Err(SdkError::InvalidConfig(
                "resume requires a configured session store_dir".into(),
            ));
        };
        let loaded = store.load_session(id).await?;
        *self.inner.lock().await = loaded;
        Ok(())
    }

    /// List every session id known to the configured store. Returns an empty
    /// vec when no store is configured.
    ///
    /// # Errors
    /// Propagates IO failures from the underlying [`SessionStore`].
    pub async fn list(&self) -> SdkResult<Vec<SessionId>> {
        match &self.store {
            Some(store) => Ok(store.list_sessions().await?),
            None => Ok(Vec::new()),
        }
    }

    /// Delete a persisted session from disk.
    ///
    /// # Errors
    /// Returns [`SdkError::InvalidConfig`] when no store is configured,
    /// [`SdkError::Session`] when the id is not found.
    pub async fn delete(&self, id: &SessionId) -> SdkResult<()> {
        let Some(store) = &self.store else {
            return Err(SdkError::InvalidConfig(
                "delete requires a configured session store_dir".into(),
            ));
        };
        store.delete_session(id).await?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tempfile::TempDir;

    fn sample_turn(idx: u32) -> Turn {
        Turn {
            id: format!("t{idx}"),
            role: TurnRole::User,
            content: format!("hello {idx}"),
            ts: Utc::now(),
            prompt_tokens: 10,
            completion_tokens: 0,
            cost_usd: 0.0,
        }
    }

    #[tokio::test]
    async fn in_memory_session_records_turns() {
        let mgr = SessionManager::new(None, serde_json::json!({}));
        mgr.add_turn(sample_turn(1)).await.unwrap();
        mgr.add_turn(sample_turn(2)).await.unwrap();
        let snap = mgr.snapshot().await;
        // SessionStart + 2 Turn events.
        assert_eq!(snap.events.len(), 3);
    }

    #[tokio::test]
    async fn persisted_session_round_trips_via_resume() {
        let dir = TempDir::new().expect("tempdir");
        let mgr = SessionManager::new(Some(dir.path().to_path_buf()), serde_json::json!({}));
        let id = mgr.current_id().await;
        mgr.add_turn(sample_turn(1)).await.unwrap();

        // Drop the in-memory state by spinning up a fresh manager + resume.
        let mgr2 = SessionManager::new(Some(dir.path().to_path_buf()), serde_json::json!({}));
        mgr2.resume(&id).await.expect("resume must find the session");
        let snap = mgr2.snapshot().await;
        assert_eq!(snap.id, id);
        // SessionStart + 1 Turn.
        assert_eq!(snap.events.len(), 2);
    }

    #[tokio::test]
    async fn list_is_empty_without_store() {
        let mgr = SessionManager::new(None, serde_json::json!({}));
        let ids = mgr.list().await.unwrap();
        assert!(ids.is_empty());
    }
}
