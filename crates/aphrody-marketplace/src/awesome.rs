// SPDX-License-Identifier: Apache-2.0
//! Awesome-list curator — ingests `topic:awesome-rust` repositories from
//! GitHub, ranks them by a multi-criteria score, and emits a stack-policy
//! manifest consumed by the sub-agent `UserPromptSubmit` hook.
//!
//! Pure-data primitives ([`AwesomeRepo`], [`RankCriteria`], [`rank`],
//! [`to_stack_policy`]) are always available so tests and downstream
//! tooling stay deterministic. Network ingestion via `octocrab` is gated
//! behind the `github` cargo feature.
//!
//! # Score
//!
//! ```text
//! score = stars        * w_stars      // popularity
//!       + recency_norm * w_recency    // last commit, 1.0 if <30d, 0.0 if >365d
//!       + license_oss  * w_license    // 1.0 if Apache-2.0/MIT/BSD/MPL-2.0, else 0.0
//!       + active_bonus * w_active     // 1.0 if not archived, else 0.0
//! ```
//!
//! Weights are configurable via [`RankCriteria`]; defaults are tuned for
//! the aphrody Rust-2026 cap (Rust-only listings, recent commits favoured).

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// One row of a curated awesome list (post-fetch, pre-rank).
///
/// `Eq` is intentionally not derived: [`AwesomeRepo::score`] is `f32` (no
/// total order). Use [`AwesomeRepo::full_name`] as the equality key when
/// deduplicating.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AwesomeRepo {
    /// `owner/name` — stable GitHub identifier.
    pub full_name: String,
    /// Star count at fetch time.
    pub stars: u64,
    /// UTC timestamp of the default-branch HEAD commit.
    pub last_commit: DateTime<Utc>,
    /// SPDX identifier (e.g. `"Apache-2.0"`, `"MIT"`) — `None` if unknown.
    pub license: Option<String>,
    /// GitHub topics (used to disambiguate Rust/Wasm/MCP lists).
    #[serde(default)]
    pub topics: Vec<String>,
    /// `true` if the repository is archived (downranked).
    #[serde(default)]
    pub archived: bool,
    /// Marketing description, trimmed to 200 chars.
    #[serde(default)]
    pub description: String,
    /// Computed by [`rank`]. Persisted for transparency.
    #[serde(default)]
    pub score: f32,
}

/// Configurable weights for [`rank`]. See module docs for the formula.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RankCriteria {
    pub w_stars: f32,
    pub w_recency: f32,
    pub w_license: f32,
    pub w_active: f32,
    /// Repos older than this are dropped (post-rank filter). Default 365 days.
    pub max_age_days: i64,
    /// Repos below this star floor are dropped. Default 100.
    pub min_stars: u64,
}

impl Default for RankCriteria {
    fn default() -> Self {
        // Stars dominate (raw count, will be log-normalised inside `rank`),
        // recency is the second-strongest signal — a 50k★ repo with a 2-year-old
        // last commit must rank below a 5k★ repo updated last week.
        Self {
            w_stars: 0.40,
            w_recency: 0.35,
            w_license: 0.15,
            w_active: 0.10,
            max_age_days: 365,
            min_stars: 100,
        }
    }
}

/// OSI / Apache / Mozilla family — non-viral, safe for Apache-2.0 distribution.
const PERMISSIVE_LICENSES: &[&str] = &[
    "Apache-2.0",
    "MIT",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "MPL-2.0",
    "Unlicense",
    "CC0-1.0",
    "Zlib",
];

/// Sort `repos` by descending score (stable). Mutates each row's `score`.
///
/// Repos that fail the criteria filters (archived, below `min_stars`, older
/// than `max_age_days`) are **removed** so the caller does not need to
/// re-filter downstream.
pub fn rank(mut repos: Vec<AwesomeRepo>, c: &RankCriteria, now: DateTime<Utc>) -> Vec<AwesomeRepo> {
    repos.retain(|r| !r.archived && r.stars >= c.min_stars);
    repos.retain(|r| now.signed_duration_since(r.last_commit) <= Duration::days(c.max_age_days));

    for r in &mut repos {
        r.score = compute_score(r, c, now);
    }
    repos.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    repos
}

fn compute_score(r: &AwesomeRepo, c: &RankCriteria, now: DateTime<Utc>) -> f32 {
    // log-normalise stars so a 350k★ giant does not crush every 5k★ entry.
    let log_stars = ((r.stars as f64).max(1.0).log10() / 6.0).min(1.0) as f32;

    let age_days = now.signed_duration_since(r.last_commit).num_days().max(0) as f32;
    let recency = (1.0 - (age_days / c.max_age_days as f32)).clamp(0.0, 1.0);

    let license = match r.license.as_deref() {
        Some(spdx) if PERMISSIVE_LICENSES.contains(&spdx) => 1.0,
        _ => 0.0,
    };

    let active = if r.archived { 0.0 } else { 1.0 };

    log_stars * c.w_stars + recency * c.w_recency + license * c.w_license + active * c.w_active
}

/// Stack-policy manifest emitted to `.claude/policies/stack-whitelist.json`
/// and consumed by the sub-agent `UserPromptSubmit` hook.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StackPolicy {
    /// Schema version for forward compatibility with future hook versions.
    pub schema: String,
    /// UTC timestamp the manifest was generated.
    pub generated_at: DateTime<Utc>,
    /// Curated awesome list sources backing this policy.
    pub sources: Vec<AwesomeRepo>,
    /// One-line directive injected verbatim into sub-agent system prompts.
    pub directive: String,
}

/// Build the stack-policy from a ranked list. `sources` is truncated to the
/// top `keep` rows so the manifest stays under ~30 KB even for `keep = 100`.
#[must_use]
pub fn to_stack_policy(ranked: &[AwesomeRepo], keep: usize) -> StackPolicy {
    let top: Vec<AwesomeRepo> = ranked.iter().take(keep).cloned().collect();
    let directive = format!(
        "You MUST prefer crates listed in the top-{} awesome-rust sources \
         curated below (fact-checked, OSI-licensed, last commit < 365d, stars >= 100). \
         Refuse to add a dependency that is not transitively reachable from these \
         sources unless the user explicitly overrides.",
        top.len()
    );
    StackPolicy {
        schema: "aphrody.stack-policy/v1".to_string(),
        generated_at: Utc::now(),
        sources: top,
        directive,
    }
}

// ---------------------------------------------------------------------------
// Network ingestion (feature-gated, optional)
// ---------------------------------------------------------------------------

#[cfg(feature = "github")]
pub mod fetch {
    //! GitHub Search ingestion. Requires the `github` cargo feature.
    use super::AwesomeRepo;
    use chrono::{DateTime, Utc};
    use octocrab::Octocrab;

    /// Errors surfaced by [`fetch_awesome_rust`].
    #[derive(Debug, thiserror::Error)]
    pub enum FetchError {
        #[error("octocrab error: {0}")]
        Octocrab(#[from] octocrab::Error),
    }

    /// Fetch up to `top_n` repositories matching the awesome-rust topic family.
    ///
    /// Uses GitHub Search API (`topic:awesome-rust`) sorted by stars descending.
    /// Pagination handled internally; on rate-limit, propagates the underlying
    /// octocrab error so the caller can back off.
    pub async fn fetch_awesome_rust(
        client: &Octocrab,
        top_n: usize,
    ) -> Result<Vec<AwesomeRepo>, FetchError> {
        // GitHub Search API accepts `sort` as a string: "stars" | "forks" |
        // "help-wanted-issues" | "updated". octocrab exposes this via
        // `SearchRepositoriesBuilder::sort(&str)`; it intentionally does not
        // reuse `params::repos::Sort` (different endpoint surface).
        let page = client
            .search()
            .repositories("topic:awesome-rust")
            .sort("stars")
            .order("desc")
            .per_page(100u8.min(top_n as u8))
            .send()
            .await?;

        let mut out = Vec::with_capacity(top_n);
        for repo in page.items.into_iter().take(top_n) {
            out.push(AwesomeRepo {
                full_name: repo.full_name.clone().unwrap_or_default(),
                stars: repo.stargazers_count.unwrap_or(0) as u64,
                last_commit: repo
                    .pushed_at
                    .unwrap_or_else(|| DateTime::<Utc>::from_timestamp(0, 0).unwrap_or_default()),
                license: repo.license.as_ref().map(|l| l.spdx_id.clone()),
                topics: repo.topics.clone().unwrap_or_default(),
                archived: repo.archived.unwrap_or(false),
                description: repo
                    .description
                    .clone()
                    .unwrap_or_default()
                    .chars()
                    .take(200)
                    .collect(),
                score: 0.0,
            });
        }
        Ok(out)
    }
}

// ---------------------------------------------------------------------------
// Tests — deterministic, no network
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn make(name: &str, stars: u64, days_ago: i64, license: &str, archived: bool) -> AwesomeRepo {
        let now = Utc.with_ymd_and_hms(2026, 5, 19, 0, 0, 0).unwrap();
        AwesomeRepo {
            full_name: name.to_string(),
            stars,
            last_commit: now - Duration::days(days_ago),
            license: Some(license.to_string()),
            topics: vec!["awesome-rust".to_string()],
            archived,
            description: String::new(),
            score: 0.0,
        }
    }

    #[test]
    fn rank_orders_by_score_descending() {
        let now = Utc.with_ymd_and_hms(2026, 5, 19, 0, 0, 0).unwrap();
        let repos = vec![
            make("rust-unofficial/awesome-rust", 50_000, 7, "Apache-2.0", false),
            make("old/abandoned", 100_000, 800, "MIT", false),
            make("recent/small", 500, 1, "MIT", false),
        ];
        let ranked = rank(repos, &RankCriteria::default(), now);

        assert_eq!(ranked.len(), 2, "old/abandoned must be filtered (>365d)");
        assert_eq!(ranked[0].full_name, "rust-unofficial/awesome-rust");
        assert!(
            ranked[0].score > ranked[1].score,
            "score must be strictly decreasing"
        );
    }

    #[test]
    fn rank_drops_archived_and_low_star() {
        let now = Utc.with_ymd_and_hms(2026, 5, 19, 0, 0, 0).unwrap();
        let repos = vec![
            make("archived/big", 80_000, 30, "MIT", true),
            make("tiny/repo", 50, 10, "MIT", false),
            make("good/repo", 5_000, 5, "Apache-2.0", false),
        ];
        let ranked = rank(repos, &RankCriteria::default(), now);
        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].full_name, "good/repo");
    }

    #[test]
    fn license_signal_breaks_star_tie() {
        let now = Utc.with_ymd_and_hms(2026, 5, 19, 0, 0, 0).unwrap();
        let permissive = make("permissive/repo", 10_000, 5, "Apache-2.0", false);
        let unknown = AwesomeRepo {
            license: None,
            ..make("unknown/repo", 10_000, 5, "MIT", false)
        };
        let ranked = rank(vec![unknown, permissive], &RankCriteria::default(), now);
        assert_eq!(ranked[0].full_name, "permissive/repo");
    }

    #[test]
    fn manifest_round_trips_through_json() {
        let now = Utc.with_ymd_and_hms(2026, 5, 19, 0, 0, 0).unwrap();
        let repos = rank(
            vec![make("a/b", 5_000, 5, "MIT", false)],
            &RankCriteria::default(),
            now,
        );
        let policy = to_stack_policy(&repos, 10);
        let json = serde_json::to_string(&policy).expect("serialize");
        let back: StackPolicy = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.schema, "aphrody.stack-policy/v1");
        assert_eq!(back.sources.len(), 1);
        assert!(back.directive.contains("top-1"));
    }
}
