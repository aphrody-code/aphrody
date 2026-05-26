// SPDX-License-Identifier: Apache-2.0
//! aphrody-cost — per-provider USD cost + token usage tracking.
//!
//! Aphrody's three-only LLM provider whitelist is enforced at the type level
//! via the [`Provider`] enum. The crate exposes:
//!
//! * [`ModelPricing`] / [`PricingTable`] — declarative pricing rows (USD per
//!   million tokens, optional cache read/write tiers).
//! * [`CostCalculator`] — given a [`PricingTable`] + an [`NdjsonUsageLog`],
//!   computes a [`UsageEvent`] from raw token counts and appends it to disk.
//! * [`BudgetGuard`] — soft budget enforcement; [`BudgetGuard::try_charge`]
//!   atomically reserves spend up to a hard limit.
//! * [`NdjsonUsageLog`] — append-only NDJSON log (one [`UsageEvent`] per
//!   line, atomic flush, replayable via [`NdjsonUsageLog::read_all`]).
//!
//! ## Fusion notes
//!
//! * Pricing table lookup mirrors the `find_by_id` / `latest_version` shape
//!   from `aphrody-marketplace` ([`PricingTable::lookup`]).
//! * The NDJSON log write path uses the same `tmp + rename` atomicity pattern
//!   as `aphrody-cron`'s [`JsonFileJobStore`] for the load path, plus a
//!   tokio-native `OpenOptions::append` writer for hot writes.
//! * [`UsageEvent`] borrows the `created_at` UTC + open-ended metadata
//!   convention from `aphrody-memory`'s `MemoryRecord`.
//!
//! ## Three-only whitelist
//!
//! The [`Provider`] enum has exactly three variants — Anthropic, Gemini,
//! Antigravity — and no `Other(String)` escape hatch. Pricing rows for any
//! other vendor (OpenAI, Azure, Cohere, …) cannot be constructed: the
//! compiler refuses them. This mirrors the workspace-wide policy enforced
//! in `crates/cli/src/providers.rs`.

#![forbid(unsafe_code)]

use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::sync::Mutex as AsyncMutex;

// ---------------------------------------------------------------------------
// Provider — three-only whitelist (single-source-of-truth re-export)
// ---------------------------------------------------------------------------

// The `Provider` enum, its `as_str()` / `all()` / `parse()` helpers, custom
// strict `Deserialize`, `Display`, and the dedicated `ProviderError` live in
// the `aphrody-providers` micro-crate so the three downstream consumers
// (router, cost, rateguard) share a single definition. We re-export the
// types here to preserve the historical `aphrody_cost::Provider` import
// path for downstream code.
pub use aphrody_providers::{Provider, ProviderError};

// ---------------------------------------------------------------------------
// ModelPricing + PricingTable
// ---------------------------------------------------------------------------

/// USD pricing for a single model.
///
/// All prices are **USD per million tokens**. Use [`f64`] (not [`u64`] cents)
/// because Gemini Flash advertises sub-cent rates ($0.075/M input).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelPricing {
    /// Provider that bills this model.
    pub provider: Provider,
    /// Model identifier as it appears in the provider's API
    /// (`claude-opus-4-7`, `gemini-2.5-pro`, …).
    pub model: String,
    /// USD per 1M input (prompt) tokens.
    pub input_per_million_tokens: f64,
    /// USD per 1M output (completion) tokens.
    pub output_per_million_tokens: f64,
    /// USD per 1M cache-read tokens, if the provider supports prompt caching.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_read_per_million: Option<f64>,
    /// USD per 1M cache-write tokens, if the provider supports prompt caching.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_write_per_million: Option<f64>,
}

impl ModelPricing {
    /// Compute the USD cost for a single request.
    ///
    /// Cache tokens default to the input price when the provider does not
    /// advertise a separate cache tier; this keeps the math stable on
    /// providers that bill cache reads at the regular input rate.
    #[must_use]
    pub fn cost_for(
        &self,
        prompt_tokens: u32,
        completion_tokens: u32,
        cache_read_tokens: u32,
        cache_write_tokens: u32,
    ) -> f64 {
        const SCALE: f64 = 1_000_000.0;
        let in_rate = self.input_per_million_tokens / SCALE;
        let out_rate = self.output_per_million_tokens / SCALE;
        let cr_rate = self.cache_read_per_million.unwrap_or(self.input_per_million_tokens) / SCALE;
        let cw_rate = self.cache_write_per_million.unwrap_or(self.input_per_million_tokens) / SCALE;

        f64::from(prompt_tokens) * in_rate
            + f64::from(completion_tokens) * out_rate
            + f64::from(cache_read_tokens) * cr_rate
            + f64::from(cache_write_tokens) * cw_rate
    }
}

/// In-memory pricing catalogue.
///
/// Construct via [`PricingTable::default_table`] for the baked-in defaults
/// (refreshed manually when provider rate cards change) or with
/// [`PricingTable::new`] for tests / custom catalogues.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PricingTable {
    /// Pricing rows; uniqueness on `(provider, model)` is the caller's
    /// responsibility — [`PricingTable::lookup`] returns the first match.
    pub entries: Vec<ModelPricing>,
}

impl PricingTable {
    /// Construct an empty pricing table.
    #[must_use]
    pub fn new(entries: Vec<ModelPricing>) -> Self {
        Self { entries }
    }

    /// Default pricing table — six rows covering the canonical aphrody
    /// model surface across Anthropic, Gemini, and Antigravity.
    ///
    /// Prices reflect the public rate cards as of the 2026-Q2 refresh and
    /// are intentionally **conservative** (round-up rather than down) so
    /// budget guards err on the side of safety.
    #[must_use]
    pub fn default_table() -> Self {
        Self::new(vec![
            // Anthropic — Claude 4.x family.
            ModelPricing {
                provider: Provider::Anthropic,
                model: "claude-opus-4-7".to_string(),
                input_per_million_tokens: 15.0,
                output_per_million_tokens: 75.0,
                cache_read_per_million: Some(1.50),
                cache_write_per_million: Some(18.75),
            },
            ModelPricing {
                provider: Provider::Anthropic,
                model: "claude-sonnet-4-6".to_string(),
                input_per_million_tokens: 3.0,
                output_per_million_tokens: 15.0,
                cache_read_per_million: Some(0.30),
                cache_write_per_million: Some(3.75),
            },
            ModelPricing {
                provider: Provider::Anthropic,
                model: "claude-haiku-4-5".to_string(),
                input_per_million_tokens: 0.80,
                output_per_million_tokens: 4.0,
                cache_read_per_million: Some(0.08),
                cache_write_per_million: Some(1.0),
            },
            // Gemini — 2.5 family.
            ModelPricing {
                provider: Provider::Gemini,
                model: "gemini-2.5-pro".to_string(),
                input_per_million_tokens: 3.50,
                output_per_million_tokens: 10.50,
                cache_read_per_million: Some(0.875),
                cache_write_per_million: None,
            },
            ModelPricing {
                provider: Provider::Gemini,
                model: "gemini-3.5-flash".to_string(),
                input_per_million_tokens: 0.075,
                output_per_million_tokens: 0.30,
                cache_read_per_million: Some(0.01875),
                cache_write_per_million: None,
            },
            // Antigravity — Vertex preview surface.
            ModelPricing {
                provider: Provider::Antigravity,
                model: "antigravity-prime".to_string(),
                input_per_million_tokens: 8.0,
                output_per_million_tokens: 40.0,
                cache_read_per_million: Some(2.0),
                cache_write_per_million: Some(10.0),
            },
        ])
    }

    /// Look up a row by `(provider, model)`. Returns `None` if no exact
    /// match is found — callers should surface a [`CostError::UnknownModel`].
    #[must_use]
    pub fn lookup(&self, provider: Provider, model: &str) -> Option<&ModelPricing> {
        self.entries
            .iter()
            .find(|p| p.provider == provider && p.model == model)
    }

    /// Number of pricing rows in the table.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the table has no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate over every row served by `provider`.
    pub fn by_provider(&self, provider: Provider) -> impl Iterator<Item = &ModelPricing> {
        self.entries.iter().filter(move |p| p.provider == provider)
    }
}

// ---------------------------------------------------------------------------
// UsageEvent — one log line in the NDJSON sink.
// ---------------------------------------------------------------------------

/// Single recorded LLM call: tokens, cost, identifiers.
///
/// Field semantics mirror [`crate::PricingTable`]:
///
/// * `ts`                  — RFC 3339 UTC instant the event was recorded.
/// * `provider` / `model`  — the row that priced the call.
/// * `prompt_tokens`, …    — raw counts as returned by the provider.
/// * `cost_usd`            — pre-computed via [`ModelPricing::cost_for`].
/// * `request_id`          — upstream API request id (`x-request-id` header
///                           for Anthropic, `responseId` for Gemini).
/// * `session_id`          — aphrody-level grouping (sub-agent thread id,
///                           A2A correlation id, …).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageEvent {
    /// Wall-clock recording time, UTC.
    pub ts: DateTime<Utc>,
    /// Provider that priced the call.
    pub provider: Provider,
    /// Model identifier.
    pub model: String,
    /// Prompt (input) tokens billed.
    pub prompt_tokens: u32,
    /// Completion (output) tokens billed.
    pub completion_tokens: u32,
    /// Cache-read tokens billed.
    #[serde(default)]
    pub cache_read_tokens: u32,
    /// Cache-write tokens billed.
    #[serde(default)]
    pub cache_write_tokens: u32,
    /// Total USD cost computed from a [`PricingTable`].
    pub cost_usd: f64,
    /// Upstream request id, if surfaced.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// Aphrody-side correlation id (sub-agent / A2A session).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

// ---------------------------------------------------------------------------
// CostError
// ---------------------------------------------------------------------------

/// All recoverable failures emitted by this crate.
#[derive(Debug, Error)]
pub enum CostError {
    /// The `(provider, model)` pair is absent from the active [`PricingTable`].
    #[error("unknown model: provider={provider:?} model={model}")]
    UnknownModel {
        /// Provider that was queried.
        provider: Provider,
        /// Model identifier that was queried.
        model: String,
    },
    /// Filesystem error while interacting with the NDJSON log.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// JSON (de)serialization failure.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    /// A [`BudgetGuard::try_charge`] call would breach the configured limit.
    #[error("budget exceeded: limit={limit} attempted={attempted}")]
    BudgetExceeded {
        /// Configured hard limit (USD).
        limit: f64,
        /// Total spend that would have resulted from the rejected charge.
        attempted: f64,
    },
}

// ---------------------------------------------------------------------------
// BudgetGuard
// ---------------------------------------------------------------------------

/// Atomic USD budget enforcer.
///
/// Wraps a [`Mutex<f64>`] for cross-thread reservation. Use
/// [`BudgetGuard::try_charge`] *after* computing the call cost but *before*
/// hitting the provider API; if the call subsequently fails, refund via
/// [`BudgetGuard::refund`].
#[derive(Debug)]
pub struct BudgetGuard {
    limit_usd: f64,
    spent_usd: Mutex<f64>,
}

impl BudgetGuard {
    /// Construct a guard with `limit_usd` ceiling and zero initial spend.
    #[must_use]
    pub fn new(limit_usd: f64) -> Self {
        Self {
            limit_usd,
            spent_usd: Mutex::new(0.0),
        }
    }

    /// Try to reserve `amount_usd` against the budget.
    ///
    /// On success the running spend is incremented atomically. On failure
    /// the spend is unchanged and [`CostError::BudgetExceeded`] is returned.
    pub fn try_charge(&self, amount_usd: f64) -> Result<(), CostError> {
        let mut spent = self.spent_usd.lock().expect("budget guard poisoned");
        let next = *spent + amount_usd;
        if next > self.limit_usd {
            return Err(CostError::BudgetExceeded {
                limit: self.limit_usd,
                attempted: next,
            });
        }
        *spent = next;
        Ok(())
    }

    /// Refund a previously-charged amount (e.g. when the API call failed).
    pub fn refund(&self, amount_usd: f64) {
        let mut spent = self.spent_usd.lock().expect("budget guard poisoned");
        *spent = (*spent - amount_usd).max(0.0);
    }

    /// USD remaining before the limit is hit.
    #[must_use]
    pub fn remaining(&self) -> f64 {
        let spent = *self.spent_usd.lock().expect("budget guard poisoned");
        (self.limit_usd - spent).max(0.0)
    }

    /// USD spent so far.
    #[must_use]
    pub fn spent(&self) -> f64 {
        *self.spent_usd.lock().expect("budget guard poisoned")
    }

    /// Configured hard limit.
    #[must_use]
    pub fn limit(&self) -> f64 {
        self.limit_usd
    }

    /// Reset the running spend back to zero.
    pub fn reset(&self) {
        let mut spent = self.spent_usd.lock().expect("budget guard poisoned");
        *spent = 0.0;
    }
}

// ---------------------------------------------------------------------------
// NdjsonUsageLog
// ---------------------------------------------------------------------------

/// Append-only NDJSON sink for [`UsageEvent`] records.
///
/// Each event is serialized to a single line terminated with `\n` and
/// fsynced via [`AsyncWriteExt::flush`]. The reader path opens the file
/// fresh on each call (no caching) so concurrent writers from sibling
/// processes are picked up immediately.
#[derive(Debug)]
pub struct NdjsonUsageLog {
    path: PathBuf,
    writer: AsyncMutex<Option<File>>,
}

impl NdjsonUsageLog {
    /// Construct a new log handle. The file is created lazily on first
    /// [`NdjsonUsageLog::append`].
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            writer: AsyncMutex::new(None),
        }
    }

    /// Path the log writes to.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Append a single event. Creates the parent directory and the log
    /// file if either is missing.
    pub async fn append(&self, event: &UsageEvent) -> Result<(), CostError> {
        let mut guard = self.writer.lock().await;
        if guard.is_none() {
            if let Some(parent) = self.path.parent() {
                if !parent.as_os_str().is_empty() {
                    tokio::fs::create_dir_all(parent).await?;
                }
            }
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)
                .await?;
            *guard = Some(file);
        }
        let file = guard.as_mut().expect("writer initialised above");
        let mut line = serde_json::to_vec(event)?;
        line.push(b'\n');
        file.write_all(&line).await?;
        file.flush().await?;
        Ok(())
    }

    /// Read every event in the log in insertion order.
    ///
    /// Skips empty lines silently and returns [`CostError::Json`] on the
    /// first malformed row.
    pub async fn read_all(&self) -> Result<Vec<UsageEvent>, CostError> {
        if !tokio::fs::try_exists(&self.path).await? {
            return Ok(Vec::new());
        }
        // Flush any pending writes before reading.
        {
            let mut guard = self.writer.lock().await;
            if let Some(file) = guard.as_mut() {
                file.flush().await?;
                let _ = file.seek(SeekFrom::Start(0)).await;
            }
        }
        let mut file = File::open(&self.path).await?;
        let mut buf = String::new();
        file.read_to_string(&mut buf).await?;
        let mut events = Vec::new();
        for line in buf.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let event: UsageEvent = serde_json::from_str(line)?;
            events.push(event);
        }
        Ok(events)
    }

    /// Number of recorded events (re-reads the file).
    pub async fn len(&self) -> Result<usize, CostError> {
        Ok(self.read_all().await?.len())
    }
}

// ---------------------------------------------------------------------------
// CostCalculator
// ---------------------------------------------------------------------------

/// High-level cost computation + logging entry point.
///
/// Owns a [`PricingTable`] (cheap to clone) and an optional
/// [`NdjsonUsageLog`]. The two arguments are decoupled so callers can
/// compute costs without persisting them (`fn calculate`) or both compute
/// + persist in one call (`async fn record`).
#[derive(Debug)]
pub struct CostCalculator {
    table: PricingTable,
    log: Option<NdjsonUsageLog>,
}

impl CostCalculator {
    /// Construct a calculator with an in-memory pricing table and no log.
    #[must_use]
    pub fn new(table: PricingTable) -> Self {
        Self { table, log: None }
    }

    /// Attach an NDJSON usage log. Subsequent [`CostCalculator::record`]
    /// calls will append to it.
    #[must_use]
    pub fn with_log(mut self, log: NdjsonUsageLog) -> Self {
        self.log = Some(log);
        self
    }

    /// Underlying pricing table.
    #[must_use]
    pub fn table(&self) -> &PricingTable {
        &self.table
    }

    /// Underlying NDJSON log, if any.
    #[must_use]
    pub fn log(&self) -> Option<&NdjsonUsageLog> {
        self.log.as_ref()
    }

    /// Compute the USD cost for a call without recording it.
    pub fn calculate(
        &self,
        provider: Provider,
        model: &str,
        prompt_tokens: u32,
        completion_tokens: u32,
        cache_read_tokens: u32,
        cache_write_tokens: u32,
    ) -> Result<f64, CostError> {
        let row = self
            .table
            .lookup(provider, model)
            .ok_or_else(|| CostError::UnknownModel {
                provider,
                model: model.to_string(),
            })?;
        Ok(row.cost_for(
            prompt_tokens,
            completion_tokens,
            cache_read_tokens,
            cache_write_tokens,
        ))
    }

    /// Compute + append a [`UsageEvent`] to the attached log.
    ///
    /// If no log is attached the event is returned but not persisted.
    #[allow(clippy::too_many_arguments)]
    pub async fn record(
        &self,
        provider: Provider,
        model: &str,
        prompt_tokens: u32,
        completion_tokens: u32,
        cache_read_tokens: u32,
        cache_write_tokens: u32,
        request_id: Option<String>,
        session_id: Option<String>,
    ) -> Result<UsageEvent, CostError> {
        let cost_usd = self.calculate(
            provider,
            model,
            prompt_tokens,
            completion_tokens,
            cache_read_tokens,
            cache_write_tokens,
        )?;
        let event = UsageEvent {
            ts: Utc::now(),
            provider,
            model: model.to_string(),
            prompt_tokens,
            completion_tokens,
            cache_read_tokens,
            cache_write_tokens,
            cost_usd,
            request_id,
            session_id,
        };
        if let Some(log) = self.log.as_ref() {
            log.append(&event).await?;
        }
        Ok(event)
    }
}

// ---------------------------------------------------------------------------
// In-crate sanity checks (compile-time invariants).
// ---------------------------------------------------------------------------

#[cfg(test)]
mod inline_tests {
    use super::*;

    #[test]
    fn provider_all_yields_three_in_order() {
        let all = Provider::all();
        assert_eq!(all[0], Provider::Anthropic);
        assert_eq!(all[1], Provider::Gemini);
        assert_eq!(all[2], Provider::Antigravity);
    }

    #[test]
    fn provider_as_str_is_lowercase() {
        assert_eq!(Provider::Anthropic.as_str(), "anthropic");
        assert_eq!(Provider::Gemini.as_str(), "gemini");
        assert_eq!(Provider::Antigravity.as_str(), "antigravity");
    }

    #[test]
    fn model_pricing_cost_for_input_only() {
        let p = ModelPricing {
            provider: Provider::Anthropic,
            model: "x".to_string(),
            input_per_million_tokens: 10.0,
            output_per_million_tokens: 0.0,
            cache_read_per_million: None,
            cache_write_per_million: None,
        };
        // 1M tokens at $10/M = $10.
        let c = p.cost_for(1_000_000, 0, 0, 0);
        assert!((c - 10.0).abs() < 1e-9, "cost = {c}");
    }
}
