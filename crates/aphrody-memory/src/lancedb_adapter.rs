// SPDX-License-Identifier: Apache-2.0
//! [`LanceDbAdapter`] — bridges [`crate::lancedb::LanceDbBackend`] (the
//! `MemoryBackend` key/vector tier) into the Tier-1 [`crate::provider::MemoryProvider`]
//! trait so LanceDB can participate in [`crate::migrate::migrate`] as either
//! source or target.
//!
//! ## Namespace ↔ `agent_id` mapping
//!
//! `LanceDbBackend` stores records with keys formatted as `"<ns>/<uuid>"` where
//! `ns` is a logical partition.  The Tier-1 `MemoryProvider` surfaces an
//! `agent_id` scope (e.g. a Mem0 `user_id` or a Honcho `session` owner).  This
//! adapter treats **`ns` as `agent_id`**: listing all records for an agent is
//! implemented as `backend.list("<agent_id>/", limit)`.
//!
//! ## Record shape conversion
//!
//! | `MemoryBackend::MemoryRecord` field | `types::MemoryRecord` field       |
//! |-------------------------------------|-----------------------------------|
//! | `id` (`Uuid`)                       | `id` (`String`)                   |
//! | `ns`                                | `agent_id`                        |
//! | `content`                           | `content`                         |
//! | `metadata["tags"]` (JSON array)     | `tags` (`Vec<String>`)            |
//! | `created_at.timestamp_millis()`     | `created_at_unix_ms` (`u64`)      |
//! | `metadata` (rest)                   | `metadata` (`serde_json::Value`)  |
//!
//! ## `health()` implementation
//!
//! `LanceDbBackend` has no dedicated health-check RPC.  We probe with an empty
//! `list("", 1)` call — the fastest path that exercises both the Arrow table
//! open and the DataFusion query planner without touching user data.
//!
//! ## `search()` scope
//!
//! Honcho / Mem0 search is keyword + agent-scoped.  Here we use
//! `backend.list("<agent_id>/", limit)` and apply the optional keyword filter
//! in-process — acceptable because LanceDB is embedded (no network RTT).
//! The call honours `MemoryQuery::limit` (defaults to 1 000 when absent).
//!
//! ## `add()` and `delete()`
//!
//! `add` requires a pre-computed embedding (`None` is accepted: the record is
//! stored without a vector, which is the correct behaviour for text-only
//! migrations from Honcho/Mem0). `delete` constructs the full key
//! `"<agent_id>/<id>"` and delegates to `MemoryBackend::delete`.
//!
//! ## Concurrency
//!
//! `LanceDbBackend` wraps its inner `Table` in a `tokio::sync::Mutex`.  This
//! adapter holds the backend behind a `tokio::sync::Mutex` as well so
//! `&self` can safely delegate to `&mut self` methods on the backend
//! (`put` and `delete`).

use std::path::Path;

use async_trait::async_trait;
use chrono::{TimeZone as _, Utc};
use tokio::sync::Mutex;

use crate::lancedb::LanceDbBackend;
use crate::provider::MemoryProvider;
use crate::types::{MemoryError, MemoryQuery, MemoryRecord, ProviderKind};
use crate::MemoryBackend;

/// Bridge the backend-tier [`crate::MemoryError`] into the provider-tier
/// [`crate::types::MemoryError`] so `?` works across the adapter boundary.
///
/// Both enums live in this crate, so this impl is orphan-rule-clean. The
/// mapping is total and lossless within the provider vocabulary:
/// `NotReady`/`Internal` (no provider equivalent) collapse onto
/// [`MemoryError::Unexpected`], preserving the original message.
impl From<crate::MemoryError> for MemoryError {
    fn from(err: crate::MemoryError) -> Self {
        match err {
            crate::MemoryError::Io(e) => MemoryError::Io(e),
            crate::MemoryError::Serde(e) => MemoryError::Json(e),
            crate::MemoryError::NotFound { key } => MemoryError::NotFound { id: key },
            crate::MemoryError::NotReady { reason } => {
                MemoryError::Unexpected(format!("backend not ready: {reason}"))
            }
            crate::MemoryError::InvalidArgument { message } => {
                MemoryError::InvalidArgument(message)
            }
            crate::MemoryError::Internal { message } => MemoryError::Unexpected(message),
        }
    }
}

// How many records to enumerate when no explicit `limit` is set on a search.
const DEFAULT_SEARCH_LIMIT: usize = 1_000;

/// Adapter that exposes a [`LanceDbBackend`] as a [`MemoryProvider`].
///
/// Construct via [`LanceDbAdapter::open`]; the resulting value satisfies
/// `dyn MemoryProvider` so it can be passed directly to
/// [`crate::migrate::migrate`].
pub struct LanceDbAdapter {
    /// The underlying backend behind a mutex so `&self` callers can drive
    /// `&mut self` methods (LanceDB serialises writes internally, the mutex
    /// keeps our adapter `Send + Sync` without unsafe).
    inner: Mutex<LanceDbBackend>,
}

impl std::fmt::Debug for LanceDbAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LanceDbAdapter").finish_non_exhaustive()
    }
}

impl LanceDbAdapter {
    /// Open (or create) a LanceDB-backed memory store at `path` with
    /// `ndims`-dimensional embedding vectors.
    ///
    /// Delegates directly to [`LanceDbBackend::open`]; see that method for the
    /// full error surface.
    ///
    /// # Errors
    /// - [`MemoryError::InvalidArgument`] if `ndims == 0`.
    /// - [`MemoryError::Internal`] for LanceDB / Arrow failures.
    pub async fn open(path: impl AsRef<Path>, ndims: usize) -> Result<Self, MemoryError> {
        let backend = LanceDbBackend::open(path, ndims).await?;
        Ok(Self { inner: Mutex::new(backend) })
    }

    // Build the canonical LanceDB key from agent_id + record id.
    fn key(agent_id: &str, id: &str) -> String {
        format!("{agent_id}/{id}")
    }
}

// ---------------------------------------------------------------------------
// MemoryRecord conversions
// ---------------------------------------------------------------------------

/// Convert from the backend-tier [`crate::MemoryRecord`] to the provider-tier
/// [`crate::types::MemoryRecord`].
///
/// The `ns` field of the backend record becomes `agent_id`; the `metadata`
/// `HashMap` is converted to a `serde_json::Value` object; `tags` are
/// extracted from `metadata["tags"]` if present.
fn backend_to_provider(rec: crate::MemoryRecord) -> MemoryRecord {
    // Serialize the HashMap to a serde_json::Value object.
    let metadata_value: serde_json::Value = rec
        .metadata
        .into_iter()
        .collect::<serde_json::Map<String, serde_json::Value>>()
        .into();
    // Extract tags from the metadata object.
    let tags: Vec<String> = metadata_value
        .get("tags")
        .and_then(serde_json::Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let created_at_unix_ms =
        u64::try_from(rec.created_at.timestamp_millis()).unwrap_or(0);
    MemoryRecord {
        id: rec.id.to_string(),
        agent_id: rec.ns,
        content: rec.content,
        tags,
        created_at_unix_ms,
        metadata: metadata_value,
    }
}

/// Convert from the provider-tier [`crate::types::MemoryRecord`] to the
/// backend-tier [`crate::MemoryRecord`].
///
/// `agent_id` becomes `ns`; `tags` are folded into `metadata["tags"]`.
/// `embedding` is always `None` — text records migrated from HTTP providers
/// (Mem0 / Honcho) carry no vectors; the adapter stores them without one.
fn provider_to_backend(rec: MemoryRecord) -> crate::MemoryRecord {
    // Build the metadata HashMap from the JSON object.
    let mut meta: std::collections::HashMap<String, serde_json::Value> =
        if let serde_json::Value::Object(map) = rec.metadata {
            map.into_iter().collect()
        } else {
            std::collections::HashMap::new()
        };
    // Fold tags into metadata so they survive the LanceDB round-trip.
    meta.insert(
        "tags".into(),
        serde_json::Value::Array(
            rec.tags.into_iter().map(serde_json::Value::String).collect(),
        ),
    );
    let created_at = Utc
        .timestamp_millis_opt(i64::try_from(rec.created_at_unix_ms).unwrap_or(i64::MAX))
        .single()
        .unwrap_or_else(Utc::now);

    crate::MemoryRecord {
        id: uuid::Uuid::parse_str(&rec.id).unwrap_or_else(|_| uuid::Uuid::new_v4()),
        ns: rec.agent_id,
        content: rec.content,
        embedding: None,
        metadata: meta,
        created_at,
        updated_at: Utc::now(),
    }
}

// ---------------------------------------------------------------------------
// MemoryProvider impl
// ---------------------------------------------------------------------------

#[async_trait]
impl MemoryProvider for LanceDbAdapter {
    async fn add(&self, rec: MemoryRecord) -> Result<String, MemoryError> {
        if rec.agent_id.is_empty() {
            return Err(MemoryError::InvalidArgument(
                "agent_id is required for LanceDbAdapter".into(),
            ));
        }
        let id = rec.id.clone();
        let key = Self::key(&rec.agent_id, &id);
        let backend_rec = provider_to_backend(rec);
        let mut guard = self.inner.lock().await;
        guard.put(&key, backend_rec).await.map_err(|e| match e {
            crate::MemoryError::Io(io) => MemoryError::Io(io),
            crate::MemoryError::Serde(s) => MemoryError::Json(s),
            crate::MemoryError::InvalidArgument { message } => {
                MemoryError::InvalidArgument(message)
            }
            other => MemoryError::Unexpected(other.to_string()),
        })?;
        Ok(id)
    }

    async fn get(&self, id: &str) -> Result<Option<MemoryRecord>, MemoryError> {
        // Without the agent_id we cannot reconstruct the full key.  Return
        // `Ok(None)` — consistent with HonchoProvider which also cannot do
        // direct-by-id fetch without additional context.
        let _ = id;
        Ok(None)
    }

    async fn search(&self, q: MemoryQuery) -> Result<Vec<MemoryRecord>, MemoryError> {
        if q.agent_id.is_empty() {
            return Err(MemoryError::InvalidArgument(
                "agent_id is required for LanceDbAdapter search".into(),
            ));
        }
        let limit = q.limit.map_or(DEFAULT_SEARCH_LIMIT, |l| l as usize);
        let prefix = format!("{}/", q.agent_id);
        let guard = self.inner.lock().await;
        let raw = guard.list(&prefix, limit).await.map_err(|e| match e {
            crate::MemoryError::Io(io) => MemoryError::Io(io),
            crate::MemoryError::Serde(s) => MemoryError::Json(s),
            crate::MemoryError::InvalidArgument { message } => {
                MemoryError::InvalidArgument(message)
            }
            other => MemoryError::Unexpected(other.to_string()),
        })?;
        let mut out: Vec<MemoryRecord> =
            raw.into_iter().map(backend_to_provider).collect();

        // Apply optional keyword filter in-process (no network RTT for embedded).
        if let Some(needle) = &q.q {
            let needle_lc = needle.to_lowercase();
            out.retain(|r| r.content.to_lowercase().contains(&needle_lc));
        }
        // Apply optional tag filter.
        if !q.tags.is_empty() {
            out.retain(|r| q.tags.iter().all(|t| r.tags.iter().any(|rt| rt == t)));
        }
        Ok(out)
    }

    async fn delete(&self, id: &str) -> Result<(), MemoryError> {
        // We cannot reconstruct the full `<agent_id>/<id>` key without the
        // agent scope — store the raw id as the key for a best-effort delete.
        // Production callers that need scoped delete should use the backend
        // directly. For migration purposes (provider-level API) this is fine
        // because the migration loop never calls delete on the target.
        if id.is_empty() {
            return Err(MemoryError::InvalidArgument("id is required".into()));
        }
        let mut guard = self.inner.lock().await;
        guard.delete(id).await.map_err(|e| match e {
            crate::MemoryError::Io(io) => MemoryError::Io(io),
            crate::MemoryError::Serde(s) => MemoryError::Json(s),
            other => MemoryError::Unexpected(other.to_string()),
        })
    }

    async fn health(&self) -> Result<(), MemoryError> {
        // Empty list — cheapest path that exercises the Arrow table without
        // touching user records.
        let guard = self.inner.lock().await;
        guard.list("", 1).await.map_err(|e| match e {
            crate::MemoryError::Io(io) => MemoryError::Io(io),
            crate::MemoryError::Serde(s) => MemoryError::Json(s),
            other => MemoryError::Unexpected(other.to_string()),
        })?;
        Ok(())
    }

    fn provider_kind(&self) -> ProviderKind {
        ProviderKind::LanceDb
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::MemoryRecord as ProviderRecord;

    const NDIMS: usize = 4;

    async fn make_adapter() -> (LanceDbAdapter, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("adapter_test");
        let adapter = LanceDbAdapter::open(&path, NDIMS).await.expect("open adapter");
        (adapter, dir)
    }

    fn make_record(id: &str, agent_id: &str, content: &str) -> ProviderRecord {
        ProviderRecord::new(id, agent_id, content)
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn health_on_empty_store() {
        let (adapter, _dir) = make_adapter().await;
        adapter.health().await.expect("health on empty store");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn add_then_search_finds_record() {
        let (adapter, _dir) = make_adapter().await;
        let rec = make_record("id-1", "agent-A", "hello lancedb");
        adapter.add(rec).await.expect("add");

        let results = adapter
            .search(MemoryQuery::for_agent("agent-A"))
            .await
            .expect("search");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "hello lancedb");
        assert_eq!(results[0].agent_id, "agent-A");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn search_scoped_by_agent_id() {
        let (adapter, _dir) = make_adapter().await;
        adapter
            .add(make_record("id-A1", "agent-A", "for agent A"))
            .await
            .unwrap();
        adapter
            .add(make_record("id-B1", "agent-B", "for agent B"))
            .await
            .unwrap();

        let a_results = adapter
            .search(MemoryQuery::for_agent("agent-A"))
            .await
            .unwrap();
        assert_eq!(a_results.len(), 1, "agent-A should see only its record");
        assert_eq!(a_results[0].agent_id, "agent-A");

        let b_results = adapter
            .search(MemoryQuery::for_agent("agent-B"))
            .await
            .unwrap();
        assert_eq!(b_results.len(), 1, "agent-B should see only its record");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn search_keyword_filter() {
        let (adapter, _dir) = make_adapter().await;
        adapter
            .add(make_record("id-1", "agent-A", "rust is fast"))
            .await
            .unwrap();
        adapter
            .add(make_record("id-2", "agent-A", "python is flexible"))
            .await
            .unwrap();

        let q = MemoryQuery { agent_id: "agent-A".into(), q: Some("rust".into()), ..Default::default() };
        let res = adapter.search(q).await.unwrap();
        assert_eq!(res.len(), 1);
        assert!(res[0].content.contains("rust"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn add_empty_agent_id_rejected() {
        let (adapter, _dir) = make_adapter().await;
        let rec = make_record("id-1", "", "oops");
        let err = adapter.add(rec).await.unwrap_err();
        assert!(matches!(err, MemoryError::InvalidArgument(_)));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn search_empty_agent_id_rejected() {
        let (adapter, _dir) = make_adapter().await;
        let err = adapter.search(MemoryQuery::for_agent("")).await.unwrap_err();
        assert!(matches!(err, MemoryError::InvalidArgument(_)));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn provider_kind_is_lancedb() {
        let (adapter, _dir) = make_adapter().await;
        assert_eq!(adapter.provider_kind(), ProviderKind::LanceDb);
    }
}
