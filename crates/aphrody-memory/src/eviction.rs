// SPDX-License-Identifier: Apache-2.0
//! Eviction policies (R3.5) — TTL, LRU, MaxSize.
//!
//! Policies are pure transforms over a slice of [`crate::types::MemoryRecord`].
//! They never perform I/O and never panic on user input, which makes them
//! trivial to wire from any caller:
//!
//! ```no_run
//! use aphrody_memory::eviction::EvictionPolicy;
//! use aphrody_memory::types::{MemoryRecord, now_unix_ms};
//!
//! let policy = EvictionPolicy::Ttl { ttl_ms: 60_000 };
//! let records: Vec<MemoryRecord> = vec![];
//! let to_delete: Vec<String> = policy.evict(&records, now_unix_ms());
//! ```
//!
//! ## Decoupling rationale
//!
//! Eviction is *not* baked into [`crate::provider::MemoryProvider`] because
//! the four backends in this crate (Mem0, Honcho, SqliteLocal, plus the legacy
//! [`crate::MemoryBackend`] tier) have very different deletion costs and
//! semantics — HTTP round-trips for Mem0/Honcho, single SQL statements for
//! SqliteLocal, file-rewrite for JSONL. Pushing the decision (*which* ids to
//! evict) up to the caller and keeping the action (*how* to delete) inside
//! the provider lets each layer specialise without coupling.
//!
//! Config schema (`~/.aphrody/memory.json` → `eviction` field) :
//!
//! ```json
//! { "kind": "ttl", "ttl_ms": 604800000 }
//! { "kind": "lru", "max_records": 10000 }
//! { "kind": "max_size", "max_records": 10000 }
//! ```
//!
//! ## Verify
//!
//! `cargo test -p aphrody-memory eviction::` covers the matrix below.

use serde::{Deserialize, Serialize};

use crate::types::MemoryRecord;

/// Eviction policy variants — serializable as an internally-tagged enum so a
/// `memory.json` config can pick the strategy by name without a wrapper.
///
/// Semantics summary:
///
/// | Variant   | `evict` returns                                | `can_add(n)` returns                 |
/// |-----------|------------------------------------------------|---------------------------------------|
/// | `Ttl`     | ids of records older than `ttl_ms` from `now`  | always `true`                         |
/// | `Lru`     | ids of records past the `max_records` cap      | always `true` (trim happens later)    |
/// | `MaxSize` | ids of records past the `max_records` cap      | `false` when at or above `max_records`|
///
/// The split between `Lru` and `MaxSize` is intentional: `Lru` is a *soft cap*
/// (writes always succeed, oldest records get reaped by a background sweeper)
/// while `MaxSize` is a *hard cap* (writes are refused once full, useful for
/// quota-bound contexts like free-tier Mem0 accounts).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EvictionPolicy {
    /// Evict records whose `created_at_unix_ms` is more than `ttl_ms`
    /// milliseconds before `now`. Records exactly at the cutoff are kept.
    Ttl {
        /// Maximum age in milliseconds before a record becomes eligible for
        /// eviction.
        ttl_ms: u64,
    },
    /// Keep the `max_records` newest records (by `created_at_unix_ms`),
    /// returning the ids of every record beyond that cap. Stable order is
    /// not guaranteed for records sharing the exact same timestamp.
    Lru {
        /// Inclusive cap — at most this many records survive the policy.
        max_records: usize,
    },
    /// Same eviction behaviour as [`Self::Lru`], but [`Self::can_add`] returns
    /// `false` once `current_count >= max_records`, so callers refuse the
    /// write instead of evicting older state. Use when retention guarantees
    /// matter more than freshness.
    MaxSize {
        /// Inclusive cap — adds beyond this count are rejected.
        max_records: usize,
    },
}

impl EvictionPolicy {
    /// Decide which records in `records` should be removed under this policy,
    /// returning their ids in evaluation order.
    ///
    /// - `records` may contain any agents — the caller filters by `agent_id`
    ///   *before* calling this method if per-agent scoping is desired.
    /// - `now_unix_ms` is injected to keep the function deterministic (tests
    ///   never read the wall clock).
    ///
    /// Always returns an allocated `Vec` (possibly empty); never panics.
    #[must_use]
    pub fn evict(&self, records: &[MemoryRecord], now_unix_ms: u64) -> Vec<String> {
        match *self {
            Self::Ttl { ttl_ms } => {
                let cutoff = now_unix_ms.saturating_sub(ttl_ms);
                records
                    .iter()
                    .filter(|r| r.created_at_unix_ms < cutoff)
                    .map(|r| r.id.clone())
                    .collect()
            },
            Self::Lru { max_records } | Self::MaxSize { max_records } => {
                if records.len() <= max_records {
                    return Vec::new();
                }
                let mut sorted: Vec<&MemoryRecord> = records.iter().collect();
                // Sort newest-first; stable so records with identical timestamps
                // keep their original relative order.
                sorted.sort_by(|a, b| {
                    b.created_at_unix_ms.cmp(&a.created_at_unix_ms)
                });
                sorted
                    .into_iter()
                    .skip(max_records)
                    .map(|r| r.id.clone())
                    .collect()
            },
        }
    }

    /// Should the caller accept a new write when the store currently holds
    /// `current_count` records?
    ///
    /// Only [`Self::MaxSize`] ever returns `false`; [`Self::Ttl`] and
    /// [`Self::Lru`] always allow the write (eviction happens out-of-band).
    #[must_use]
    pub const fn can_add(&self, current_count: usize) -> bool {
        match *self {
            Self::MaxSize { max_records } => current_count < max_records,
            Self::Ttl { .. } | Self::Lru { .. } => true,
        }
    }

    /// Stable lower-case identifier — useful for metrics labels and logs.
    #[must_use]
    pub const fn kind_str(&self) -> &'static str {
        match *self {
            Self::Ttl { .. } => "ttl",
            Self::Lru { .. } => "lru",
            Self::MaxSize { .. } => "max_size",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(id: &str, ts: u64) -> MemoryRecord {
        MemoryRecord {
            id: id.into(),
            agent_id: "agent-A".into(),
            content: "x".into(),
            tags: Vec::new(),
            created_at_unix_ms: ts,
            metadata: serde_json::Value::Object(serde_json::Map::new()),
        }
    }

    // ── TTL ────────────────────────────────────────────────────────────────

    #[test]
    fn policy_ttl_evicts_after_n_secs() {
        // PLAN verify: this exact test name is the contract.
        let policy = EvictionPolicy::Ttl { ttl_ms: 5_000 };
        let now = 100_000;
        let records = vec![
            rec("old", 90_000),  // 10s old → evict
            rec("mid", 96_000),  // 4s old → keep (younger than ttl)
            rec("new", 99_999),  // 1ms old → keep
        ];
        let evicted = policy.evict(&records, now);
        assert_eq!(evicted, vec!["old".to_string()]);
    }

    #[test]
    fn policy_ttl_keeps_at_cutoff() {
        // Exactly-at-cutoff records are retained (strict `<` semantics).
        let policy = EvictionPolicy::Ttl { ttl_ms: 1_000 };
        let records = vec![rec("edge", 9_000)];
        let evicted = policy.evict(&records, 10_000);
        assert!(evicted.is_empty(), "record at the cutoff boundary must survive");
    }

    #[test]
    fn policy_ttl_handles_clock_underflow() {
        // `now < ttl_ms` would underflow naive subtraction — saturating math
        // must keep us at zero, evicting nothing.
        let policy = EvictionPolicy::Ttl { ttl_ms: u64::MAX };
        let records = vec![rec("any", 42)];
        let evicted = policy.evict(&records, 1_000);
        assert!(evicted.is_empty());
    }

    // ── LRU ────────────────────────────────────────────────────────────────

    #[test]
    fn policy_lru_keeps_most_recent() {
        let policy = EvictionPolicy::Lru { max_records: 2 };
        let records = vec![
            rec("oldest", 1_000),
            rec("middle", 2_000),
            rec("newest", 3_000),
        ];
        let mut evicted = policy.evict(&records, 0);
        evicted.sort(); // order-independent assertion
        assert_eq!(evicted, vec!["oldest".to_string()]);
    }

    #[test]
    fn policy_lru_no_eviction_under_cap() {
        let policy = EvictionPolicy::Lru { max_records: 10 };
        let records = vec![rec("a", 1), rec("b", 2)];
        assert!(policy.evict(&records, 0).is_empty());
    }

    #[test]
    fn policy_lru_can_add_always_true() {
        let policy = EvictionPolicy::Lru { max_records: 1 };
        assert!(policy.can_add(0));
        assert!(policy.can_add(1));
        assert!(policy.can_add(usize::MAX));
    }

    // ── MaxSize ────────────────────────────────────────────────────────────

    #[test]
    fn policy_max_size_rejects_when_full() {
        let policy = EvictionPolicy::MaxSize { max_records: 3 };
        assert!(policy.can_add(0));
        assert!(policy.can_add(2));
        assert!(!policy.can_add(3), "at-cap must reject the new write");
        assert!(!policy.can_add(4), "over-cap must reject too");
    }

    #[test]
    fn policy_max_size_evicts_overflow_like_lru() {
        let policy = EvictionPolicy::MaxSize { max_records: 1 };
        let records = vec![rec("old", 1), rec("new", 2)];
        let evicted = policy.evict(&records, 0);
        assert_eq!(evicted, vec!["old".to_string()]);
    }

    // ── Shared ─────────────────────────────────────────────────────────────

    #[test]
    fn policy_empty_input_returns_empty() {
        for policy in [
            EvictionPolicy::Ttl { ttl_ms: 1 },
            EvictionPolicy::Lru { max_records: 1 },
            EvictionPolicy::MaxSize { max_records: 1 },
        ] {
            assert!(policy.evict(&[], 0).is_empty());
        }
    }

    #[test]
    fn policy_kind_str_is_stable() {
        assert_eq!(EvictionPolicy::Ttl { ttl_ms: 0 }.kind_str(), "ttl");
        assert_eq!(EvictionPolicy::Lru { max_records: 0 }.kind_str(), "lru");
        assert_eq!(EvictionPolicy::MaxSize { max_records: 0 }.kind_str(), "max_size");
    }

    #[test]
    fn policy_serde_roundtrip_ttl() {
        let policy = EvictionPolicy::Ttl { ttl_ms: 604_800_000 };
        let json = serde_json::to_string(&policy).expect("serialize");
        assert!(json.contains("\"kind\":\"ttl\""), "kind tag must be present: {json}");
        assert!(json.contains("\"ttl_ms\":604800000"), "ttl_ms must be present: {json}");
        let back: EvictionPolicy = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, policy);
    }

    #[test]
    fn policy_serde_roundtrip_lru() {
        let policy = EvictionPolicy::Lru { max_records: 10_000 };
        let json = serde_json::to_string(&policy).expect("serialize");
        assert!(json.contains("\"kind\":\"lru\""));
        let back: EvictionPolicy = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, policy);
    }

    #[test]
    fn policy_serde_roundtrip_max_size() {
        let policy = EvictionPolicy::MaxSize { max_records: 256 };
        let json = serde_json::to_string(&policy).expect("serialize");
        assert!(json.contains("\"kind\":\"max_size\""), "snake_case rename must apply: {json}");
        let back: EvictionPolicy = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, policy);
    }

    #[test]
    fn policy_serde_accepts_config_shape() {
        // The `~/.aphrody/memory.json` shape promised by the doc-comment must
        // deserialize cleanly so a typo never silently degrades to a default.
        let cfg = r#"{"kind":"ttl","ttl_ms":1000}"#;
        let p: EvictionPolicy = serde_json::from_str(cfg).unwrap();
        assert_eq!(p, EvictionPolicy::Ttl { ttl_ms: 1_000 });
    }
}
