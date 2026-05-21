// SPDX-License-Identifier: Apache-2.0
//! Provider-to-provider migration (R3.4) — copy records between any pair of
//! [`MemoryProvider`] implementations with a dry-run preview.
//!
//! ## Why a separate module
//!
//! The four shipped providers (`Mem0`, `Honcho`, `SqliteLocal`, plus the legacy
//! [`crate::MemoryBackend`] tier) deliberately keep their surfaces narrow —
//! adding a `migrate_to(&dyn MemoryProvider)` method to the trait would couple
//! every backend to every other and break dyn-compatibility. Instead, this
//! module operates on `&dyn MemoryProvider` references, so any two backends
//! that satisfy the trait can be wired together without modifying either side.
//!
//! ## Semantics
//!
//! - Migration is scoped by `agent_id` because [`crate::types::MemoryQuery`]
//!   requires one — providers like Mem0 partition storage per user and have
//!   no native "dump everything" call. Callers iterating over multiple agents
//!   invoke [`migrate`] in a loop.
//! - The target keeps the source `id` field verbatim. HTTP-backed providers
//!   (Mem0, Honcho) ignore client-supplied ids and adopt their own, but the
//!   diff still records the *intent* using the original id so a dry-run
//!   preview is meaningful.
//! - Existing target records with the same id are **overwritten** — the
//!   pragmatic choice for a one-shot migration; an idempotent re-run leaves
//!   the target in the same state as the source.
//! - Errors on individual records (network blip, validation rejection) are
//!   collected into `skipped_ids` rather than aborting the whole sweep, so a
//!   partial migration is observable and resumable.
//!
//! ## Verify
//!
//! `cargo test -p aphrody-memory migrate::` covers the SQLite ↔ SQLite path
//! end-to-end (dry-run leaves target empty, real run copies every record,
//! re-run is idempotent).

use serde::{Deserialize, Serialize};

use crate::provider::MemoryProvider;
use crate::types::{MemoryError, MemoryQuery, ProviderKind};

/// JSON-serializable diff returned by [`migrate`].
///
/// Field choices mirror the `R3.4` PLAN verify ("dry-run + JSON diff") — every
/// number here can be cross-checked from CLI output without a re-fetch.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MigrationDiff {
    /// Source provider discriminator.
    pub from: ProviderKind,
    /// Target provider discriminator.
    pub to: ProviderKind,
    /// Agent scope (records outside this scope are untouched).
    pub agent_id: String,
    /// Count of records read from the source.
    pub source_count: usize,
    /// Count of records present on the target *before* the migration began.
    /// Useful to detect target pollution after the fact (a clean target has
    /// `target_count_before == 0`).
    pub target_count_before: usize,
    /// Count of records actually written to the target (zero when `dry_run`).
    pub migrated: usize,
    /// Count of records that failed to write (network error, validation).
    pub skipped: usize,
    /// Whether this was a preview-only run.
    pub dry_run: bool,
    /// Stable source ids that were (or would be) copied. Capped at
    /// [`MAX_ID_PREVIEW`] to keep CLI output bounded; full enumeration is
    /// the source's responsibility.
    pub migrated_ids: Vec<String>,
    /// Source ids that hit an error on write. Always populated regardless of
    /// the cap because operators need every failure for retry.
    pub skipped_ids: Vec<(String, String)>,
}

/// Cap on `migrated_ids` length — beyond this the CLI summary still shows
/// `migrated: N` accurately, but the id preview is truncated.
pub const MAX_ID_PREVIEW: usize = 100;

/// Copy every record belonging to `agent_id` from `from` to `to`.
///
/// `dry_run = true` exercises every read path (so misconfigured source creds
/// fail loudly) but performs zero writes — the diff still reports
/// `source_count` and would-be `migrated_ids` so an operator can preview the
/// blast radius.
///
/// # Errors
///
/// - The initial source `search()` call surfaces unchanged (no `agent_id` ⇒
///   no migration; that's the caller's bug).
/// - `health()` on the target before any write surfaces unchanged so a dead
///   target stops the migration before partial damage.
/// - Per-record write failures are *collected* into `skipped_ids`, not
///   returned, so callers see a complete diff even when half the records
///   bounced.
pub async fn migrate(
    from: &dyn MemoryProvider,
    to: &dyn MemoryProvider,
    agent_id: &str,
    dry_run: bool,
) -> Result<MigrationDiff, MemoryError> {
    if agent_id.is_empty() {
        return Err(MemoryError::InvalidArgument(
            "migrate requires a non-empty agent_id".into(),
        ));
    }

    // 1. Read source: every record for this agent. We omit `limit` so the
    //    provider returns its native maximum (Mem0 caps at 100, Honcho at
    //    100, SqliteLocal unbounded). For >100 records on hosted providers,
    //    pagination is a follow-up.
    let source_records = from
        .search(MemoryQuery::for_agent(agent_id))
        .await?;

    // 2. Pre-flight the target. health() is the cheapest call on every
    //    provider; if it fails we surface unchanged.
    to.health().await?;

    // 3. Count what was already there. For a clean migration this is 0.
    //    We use search() not a dedicated count — every provider supports
    //    search() but only SqliteLocal exposes a row count cheaply.
    let target_count_before = to
        .search(MemoryQuery::for_agent(agent_id))
        .await?
        .len();

    let source_count = source_records.len();
    let mut migrated = 0usize;
    let mut skipped = 0usize;
    let mut migrated_ids: Vec<String> = Vec::with_capacity(source_count.min(MAX_ID_PREVIEW));
    let mut skipped_ids: Vec<(String, String)> = Vec::new();

    for rec in source_records {
        let id_for_preview = rec.id.clone();
        if dry_run {
            // No write — just record the intent.
            migrated += 1;
            if migrated_ids.len() < MAX_ID_PREVIEW {
                migrated_ids.push(id_for_preview);
            }
            continue;
        }
        match to.add(rec).await {
            Ok(_) => {
                migrated += 1;
                if migrated_ids.len() < MAX_ID_PREVIEW {
                    migrated_ids.push(id_for_preview);
                }
            }
            Err(e) => {
                skipped += 1;
                skipped_ids.push((id_for_preview, e.to_string()));
            }
        }
    }

    Ok(MigrationDiff {
        from: from.provider_kind(),
        to: to.provider_kind(),
        agent_id: agent_id.to_string(),
        source_count,
        target_count_before,
        migrated,
        skipped,
        dry_run,
        migrated_ids,
        skipped_ids,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sqlite_local::SqliteLocalProvider;
    use crate::types::MemoryRecord;

    fn rec(id: &str, agent: &str, content: &str) -> MemoryRecord {
        let mut r = MemoryRecord::new(id, agent, content);
        // Pin the id explicitly so we can assert preserved identity.
        r.id = id.into();
        r
    }

    #[tokio::test]
    async fn migrate_empty_agent_id_rejected() {
        let src = SqliteLocalProvider::open_in_memory().unwrap();
        let dst = SqliteLocalProvider::open_in_memory().unwrap();
        let err = migrate(&src, &dst, "", false).await.unwrap_err();
        assert!(matches!(err, MemoryError::InvalidArgument(_)));
    }

    #[tokio::test]
    async fn migrate_sqlite_to_sqlite_copies_all() {
        let src = SqliteLocalProvider::open_in_memory().unwrap();
        let dst = SqliteLocalProvider::open_in_memory().unwrap();

        // Seed source with 3 records under the same agent.
        for i in 0..3 {
            let id = format!("rec-{i}");
            src.add(rec(&id, "agent-A", &format!("body-{i}"))).await.unwrap();
        }
        // Plus one record under a different agent — must NOT migrate.
        src.add(rec("decoy", "agent-OTHER", "ignored")).await.unwrap();

        let diff = migrate(&src, &dst, "agent-A", false).await.unwrap();

        assert_eq!(diff.from, ProviderKind::SqliteLocal);
        assert_eq!(diff.to, ProviderKind::SqliteLocal);
        assert_eq!(diff.agent_id, "agent-A");
        assert_eq!(diff.source_count, 3);
        assert_eq!(diff.target_count_before, 0);
        assert_eq!(diff.migrated, 3);
        assert_eq!(diff.skipped, 0);
        assert!(!diff.dry_run);
        assert_eq!(diff.migrated_ids.len(), 3);
        assert!(diff.skipped_ids.is_empty());

        // Target must hold every source record, ids preserved.
        for i in 0..3 {
            let id = format!("rec-{i}");
            let fetched = dst.get(&id).await.unwrap();
            assert!(fetched.is_some(), "id {id} missing on target");
            assert_eq!(fetched.unwrap().agent_id, "agent-A");
        }
        // Decoy must NOT have leaked.
        assert!(dst.get("decoy").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn migrate_dry_run_does_not_write() {
        let src = SqliteLocalProvider::open_in_memory().unwrap();
        let dst = SqliteLocalProvider::open_in_memory().unwrap();
        src.add(rec("only", "agent-A", "x")).await.unwrap();

        let diff = migrate(&src, &dst, "agent-A", true).await.unwrap();

        assert!(diff.dry_run);
        assert_eq!(diff.source_count, 1);
        assert_eq!(diff.migrated, 1, "dry-run still reports intent");
        assert_eq!(diff.skipped, 0);
        assert!(dst.get("only").await.unwrap().is_none(), "dry-run must not write");
    }

    #[tokio::test]
    async fn migrate_is_idempotent_on_rerun() {
        let src = SqliteLocalProvider::open_in_memory().unwrap();
        let dst = SqliteLocalProvider::open_in_memory().unwrap();
        for i in 0..2 {
            src.add(rec(&format!("r-{i}"), "agent-A", "x")).await.unwrap();
        }

        let first = migrate(&src, &dst, "agent-A", false).await.unwrap();
        assert_eq!(first.migrated, 2);
        assert_eq!(first.target_count_before, 0);

        // Re-run — every id already exists on target.
        let second = migrate(&src, &dst, "agent-A", false).await.unwrap();
        assert_eq!(second.source_count, 2);
        assert_eq!(second.target_count_before, 2, "target already populated");
        assert_eq!(second.migrated, 2, "INSERT OR REPLACE keeps the count");

        // Target row count must still be 2 — no duplicates.
        let after = dst.search(MemoryQuery::for_agent("agent-A")).await.unwrap();
        assert_eq!(after.len(), 2);
    }

    #[tokio::test]
    async fn migrate_diff_serializes_round_trip() {
        let src = SqliteLocalProvider::open_in_memory().unwrap();
        let dst = SqliteLocalProvider::open_in_memory().unwrap();
        src.add(rec("only", "agent-A", "x")).await.unwrap();
        let diff = migrate(&src, &dst, "agent-A", true).await.unwrap();
        let json = serde_json::to_string(&diff).unwrap();
        let back: MigrationDiff = serde_json::from_str(&json).unwrap();
        assert_eq!(back, diff);
        assert!(json.contains("\"from\":\"sqlite_local\""), "snake_case ProviderKind: {json}");
        assert!(json.contains("\"dry_run\":true"));
    }

    #[tokio::test]
    async fn migrate_caps_id_preview_at_max() {
        let src = SqliteLocalProvider::open_in_memory().unwrap();
        let dst = SqliteLocalProvider::open_in_memory().unwrap();
        // Seed MAX_ID_PREVIEW + 5 records so the preview is forced to truncate.
        for i in 0..(MAX_ID_PREVIEW + 5) {
            src.add(rec(&format!("id-{i:04}"), "agent-A", "x")).await.unwrap();
        }
        let diff = migrate(&src, &dst, "agent-A", true).await.unwrap();
        assert_eq!(diff.source_count, MAX_ID_PREVIEW + 5);
        assert_eq!(diff.migrated, MAX_ID_PREVIEW + 5, "count is accurate");
        assert_eq!(
            diff.migrated_ids.len(),
            MAX_ID_PREVIEW,
            "preview is capped, not the count"
        );
    }
}
