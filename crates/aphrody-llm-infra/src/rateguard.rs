// SPDX-License-Identifier: Apache-2.0
//! aphrody-rateguard — async token-bucket rate limiter for the aphrody LLM
//! provider matrix.
//!
//! ## Scope
//!
//! Two orthogonal quotas are enforced per `(provider, model)` pair:
//!
//! * **RPM** — requests per minute (one bucket token per call).
//! * **TPM** — tokens per minute (N bucket tokens per call, where N is the
//!   caller-supplied prompt-token estimate).
//!
//! Both quotas use the classic [token-bucket algorithm][bucket]: a capacity
//! (peak burst), a continuous refill rate (steady-state throughput), and an
//! atomic `try_consume` operation. The [`RateGuard`] facade composes one
//! bucket per axis, per `(provider, model)` slot.
//!
//! [bucket]: https://en.wikipedia.org/wiki/Token_bucket
//!
//! ## Three-only provider whitelist
//!
//! The [`Provider`] enum mirrors `aphrody-router` and `aphrody-cost`: exactly
//! three variants (Anthropic, Gemini, Antigravity), no `Other(String)` escape
//! hatch. The default quotas table [`default_provider_quotas`] ships realistic
//! 2026-Q2 tier-4 values for each provider plus per-model overrides for the
//! canonical aphrody model surface.
//!
//! ## Tokio time integration
//!
//! All time arithmetic uses [`tokio::time::Instant`] so the deterministic
//! [`tokio::time::pause`] + [`tokio::time::advance`] testing primitives work
//! out of the box. The integration tests in `tests/rate_smoke.rs` rely on
//! this to validate refill / throttle / wait behaviour without sleeping for
//! real wall-clock seconds.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::time::Duration;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::time::Instant;

// ---------------------------------------------------------------------------
// Provider — three-only whitelist (single-source-of-truth re-export)
// ---------------------------------------------------------------------------

// The `Provider` enum, its `as_str()` / `all()` / `parse()` helpers, custom
// strict `Deserialize`, `Display`, and the dedicated `ProviderError` live in
// the `aphrody-providers` micro-crate so the three downstream consumers
// (router, cost, rateguard) share a single definition. The historical
// `aphrody_rateguard::Provider` import path is preserved via re-export.
pub use aphrody_providers::{Provider, ProviderError};

// ---------------------------------------------------------------------------
// RateError
// ---------------------------------------------------------------------------

/// Recoverable failures emitted by this crate.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum RateError {
    /// `try_acquire` would push a bucket below zero; the caller should retry
    /// after `wait_for_ms` milliseconds (or call [`RateGuard::acquire_blocking`]
    /// to let the guard sleep for them).
    #[error("rate limited; retry after {wait_for_ms} ms")]
    Throttled {
        /// Milliseconds to wait before the next attempt is likely to succeed.
        wait_for_ms: u64,
    },
    /// `acquire_blocking` would have to wait longer than the configured ceiling.
    #[error("would wait {requested_ms} ms but ceiling is {max_ms} ms")]
    WaitTooLong {
        /// Milliseconds the bucket said it needed.
        requested_ms: u64,
        /// Configured upper bound (default 60_000).
        max_ms: u64,
    },
    /// No quota registered for the given [`Provider`].
    #[error("no quota registered for provider {0}")]
    UnknownProvider(Provider),
    /// The model identifier was rejected — caller should fall back to the
    /// default quota (most code paths actually do this transparently; this
    /// error is reserved for opt-in strict mode).
    #[error("unknown model: {0}")]
    UnknownModel(String),
}

/// Convenience alias.
pub type RateResult<T> = Result<T, RateError>;

// ---------------------------------------------------------------------------
// Quota
// ---------------------------------------------------------------------------

/// Paired (requests-per-minute, tokens-per-minute) quota.
///
/// Both axes are independent: a model with `rpm = 100` and `tpm = 1_000_000`
/// can hit either ceiling first, depending on whether the workload is
/// chatty-short or sparse-long.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Quota {
    /// Requests per minute.
    pub rpm: u32,
    /// Tokens per minute (sum of input + output, conservative).
    pub tpm: u64,
}

impl Quota {
    /// Build a new quota.
    #[must_use]
    pub const fn new(rpm: u32, tpm: u64) -> Self {
        Self { rpm, tpm }
    }

    /// Refill rate for the RPM bucket, requests per second.
    #[must_use]
    pub fn to_rps(&self) -> f64 {
        f64::from(self.rpm) / 60.0
    }

    /// Refill rate for the TPM bucket, tokens per second.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn to_tps(&self) -> f64 {
        // u64 -> f64 precision loss is acceptable here: TPM values live in the
        // single-digit millions, well below f64's 53-bit mantissa exactness.
        self.tpm as f64 / 60.0
    }
}

// ---------------------------------------------------------------------------
// ProviderQuota
// ---------------------------------------------------------------------------

/// Per-provider quota bundle: a default applied to any unrecognised model
/// plus an optional per-model override table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProviderQuota {
    /// Anthropic Messages API tier.
    Anthropic {
        /// Quota applied when the model is unknown.
        default: Quota,
        /// Per-model overrides (e.g. `claude-opus-4-7`).
        per_model: HashMap<String, Quota>,
    },
    /// Gemini public API tier.
    Gemini {
        /// Quota applied when the model is unknown.
        default: Quota,
        /// Per-model overrides (e.g. `gemini-2.5-pro`).
        per_model: HashMap<String, Quota>,
    },
    /// Antigravity / Vertex preview tier.
    Antigravity {
        /// Quota applied when the model is unknown.
        default: Quota,
        /// Per-model overrides (e.g. `antigravity-prime`).
        per_model: HashMap<String, Quota>,
    },
}

impl ProviderQuota {
    /// Default quota for the bundle, regardless of variant.
    #[must_use]
    pub fn default_quota(&self) -> Quota {
        match self {
            Self::Anthropic { default, .. }
            | Self::Gemini { default, .. }
            | Self::Antigravity { default, .. } => *default,
        }
    }

    /// Per-model override table.
    #[must_use]
    pub fn per_model(&self) -> &HashMap<String, Quota> {
        match self {
            Self::Anthropic { per_model, .. }
            | Self::Gemini { per_model, .. }
            | Self::Antigravity { per_model, .. } => per_model,
        }
    }

    /// Resolve the quota for `model`, falling back to the default.
    #[must_use]
    pub fn resolve(&self, model: &str) -> Quota {
        self.per_model()
            .get(model)
            .copied()
            .unwrap_or_else(|| self.default_quota())
    }

    /// Which [`Provider`] this bundle is for.
    #[must_use]
    pub fn provider(&self) -> Provider {
        match self {
            Self::Anthropic { .. } => Provider::Anthropic,
            Self::Gemini { .. } => Provider::Gemini,
            Self::Antigravity { .. } => Provider::Antigravity,
        }
    }
}

/// Default quotas table — covers the canonical aphrody model surface across
/// Anthropic Tier-4, Gemini paid, and Antigravity (Vertex) preview as of the
/// 2026-Q2 refresh. Numbers are intentionally conservative (round down on
/// throughput, round up on burst headroom) so production deployments rarely
/// surface false-positive `Throttled` errors.
#[must_use]
pub fn default_provider_quotas() -> HashMap<Provider, ProviderQuota> {
    let mut out = HashMap::with_capacity(3);

    // ---- Anthropic — Tier 4 baseline = 5_000 RPM / 2_000_000 TPM. ---------
    let mut anthropic_overrides = HashMap::new();
    anthropic_overrides.insert(
        "claude-opus-4-7".to_string(),
        Quota::new(4_000, 800_000),
    );
    anthropic_overrides.insert(
        "claude-sonnet-4-6".to_string(),
        Quota::new(5_000, 2_000_000),
    );
    anthropic_overrides.insert(
        "claude-haiku-4-5".to_string(),
        Quota::new(5_000, 4_000_000),
    );
    out.insert(
        Provider::Anthropic,
        ProviderQuota::Anthropic {
            default: Quota::new(5_000, 2_000_000),
            per_model: anthropic_overrides,
        },
    );

    // ---- Gemini — paid tier baseline = 1_000 RPM / 4_000_000 TPM. ---------
    let mut gemini_overrides = HashMap::new();
    gemini_overrides.insert("gemini-2.5-pro".to_string(), Quota::new(1_000, 2_000_000));
    gemini_overrides.insert(
        "gemini-3.5-flash".to_string(),
        Quota::new(2_000, 4_000_000),
    );
    out.insert(
        Provider::Gemini,
        ProviderQuota::Gemini {
            default: Quota::new(1_000, 4_000_000),
            per_model: gemini_overrides,
        },
    );

    // ---- Antigravity — Vertex preview baseline = 600 RPM / 1_000_000 TPM. -
    let mut antigravity_overrides = HashMap::new();
    antigravity_overrides.insert(
        "antigravity-prime".to_string(),
        Quota::new(600, 1_000_000),
    );
    out.insert(
        Provider::Antigravity,
        ProviderQuota::Antigravity {
            default: Quota::new(600, 1_000_000),
            per_model: antigravity_overrides,
        },
    );

    out
}

// ---------------------------------------------------------------------------
// TokenBucket
// ---------------------------------------------------------------------------

/// Classic token-bucket primitive with interior mutability.
///
/// * `capacity` — maximum number of tokens the bucket may hold (burst).
/// * `refill_per_sec` — continuous refill rate (steady state).
/// * `tokens` — current balance (fractional; refilled continuously).
/// * `last_refill` — last [`tokio::time::Instant`] the balance was rolled
///   forward; updated lazily on each operation.
///
/// All state is wrapped in a single [`Mutex`] for simplicity — the critical
/// section is < 10 lines so contention is negligible at expected QPS.
#[derive(Debug)]
pub struct TokenBucket {
    capacity: u64,
    refill_per_sec: f64,
    inner: Mutex<TokenBucketInner>,
}

#[derive(Debug)]
struct TokenBucketInner {
    tokens: f64,
    last_refill: Instant,
}

impl TokenBucket {
    /// Build a new bucket starting **full** (tokens = capacity).
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn new(capacity: u64, refill_per_sec: f64) -> Self {
        Self {
            capacity,
            refill_per_sec,
            inner: Mutex::new(TokenBucketInner {
                tokens: capacity as f64,
                last_refill: Instant::now(),
            }),
        }
    }

    /// Bucket capacity (burst ceiling).
    #[must_use]
    pub fn capacity(&self) -> u64 {
        self.capacity
    }

    /// Steady-state refill rate, tokens per second.
    #[must_use]
    pub fn refill_per_sec(&self) -> f64 {
        self.refill_per_sec
    }

    /// Try to consume `n` tokens; returns [`RateError::Throttled`] with a
    /// suggested wait if the bucket is starved.
    ///
    /// # Errors
    /// - [`RateError::Throttled`] when the current balance (post-refill) is
    ///   below `n`. The wait hint is rounded up to the next millisecond.
    pub fn try_consume(&self, n: u64) -> RateResult<()> {
        let mut inner = self.inner.lock();
        Self::refill_locked(&mut inner, self.capacity, self.refill_per_sec);
        #[allow(clippy::cast_precision_loss)]
        let need = n as f64;
        if inner.tokens + 1e-9 >= need {
            inner.tokens -= need;
            return Ok(());
        }
        let deficit = need - inner.tokens;
        let wait_secs = if self.refill_per_sec > 0.0 {
            deficit / self.refill_per_sec
        } else {
            // A zero-refill bucket can never recover — surface a very large
            // wait so the caller's WaitTooLong ceiling trips deterministically.
            f64::from(u32::MAX)
        };
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let wait_ms = (wait_secs * 1_000.0).ceil().max(1.0) as u64;
        Err(RateError::Throttled { wait_for_ms: wait_ms })
    }

    /// Current available tokens (floored, integer).
    #[must_use]
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    pub fn available(&self) -> u64 {
        let mut inner = self.inner.lock();
        Self::refill_locked(&mut inner, self.capacity, self.refill_per_sec);
        inner.tokens.max(0.0).floor() as u64
    }

    /// Estimated [`Duration`] until `n` tokens are available.
    #[must_use]
    #[allow(clippy::cast_precision_loss, clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    pub fn time_to_n(&self, n: u64) -> Duration {
        let mut inner = self.inner.lock();
        Self::refill_locked(&mut inner, self.capacity, self.refill_per_sec);
        let need = n as f64 - inner.tokens;
        if need <= 0.0 {
            return Duration::ZERO;
        }
        if self.refill_per_sec <= 0.0 {
            return Duration::from_secs(u64::from(u32::MAX));
        }
        let secs = need / self.refill_per_sec;
        let millis = (secs * 1_000.0).ceil().max(1.0) as u64;
        Duration::from_millis(millis)
    }

    fn refill_locked(inner: &mut TokenBucketInner, capacity: u64, rate: f64) {
        let now = Instant::now();
        if now <= inner.last_refill {
            return;
        }
        let elapsed = now.saturating_duration_since(inner.last_refill);
        let added = elapsed.as_secs_f64() * rate;
        if added > 0.0 {
            #[allow(clippy::cast_precision_loss)]
            let cap_f = capacity as f64;
            inner.tokens = (inner.tokens + added).min(cap_f);
            inner.last_refill = now;
        }
    }
}

// ---------------------------------------------------------------------------
// RateGuard
// ---------------------------------------------------------------------------

/// Default ceiling (60_000 ms = 60 s) for [`RateGuard::acquire_blocking`].
pub const DEFAULT_MAX_WAIT_MS: u64 = 60_000;

/// Composite rate guard: maintains one RPM bucket + one TPM bucket per
/// `(provider, model)` pair, lazily instantiated from the registered
/// [`ProviderQuota`] tables.
pub struct RateGuard {
    quotas: HashMap<Provider, ProviderQuota>,
    request_buckets: Mutex<HashMap<(Provider, String), TokenBucket>>,
    token_buckets: Mutex<HashMap<(Provider, String), TokenBucket>>,
    max_wait_ms: u64,
}

impl std::fmt::Debug for RateGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RateGuard")
            .field("providers", &self.quotas.keys().collect::<Vec<_>>())
            .field("max_wait_ms", &self.max_wait_ms)
            .finish()
    }
}

impl RateGuard {
    /// Build a guard from a quota table (most callers pass
    /// [`default_provider_quotas`]).
    #[must_use]
    pub fn new(quotas: HashMap<Provider, ProviderQuota>) -> Self {
        Self {
            quotas,
            request_buckets: Mutex::new(HashMap::new()),
            token_buckets: Mutex::new(HashMap::new()),
            max_wait_ms: DEFAULT_MAX_WAIT_MS,
        }
    }

    /// Override the blocking-acquire wait ceiling.
    #[must_use]
    pub fn with_max_wait_ms(mut self, max_wait_ms: u64) -> Self {
        self.max_wait_ms = max_wait_ms;
        self
    }

    /// Effective wait ceiling.
    #[must_use]
    pub fn max_wait_ms(&self) -> u64 {
        self.max_wait_ms
    }

    /// Look up the resolved [`Quota`] for a `(provider, model)` pair.
    ///
    /// # Errors
    /// - [`RateError::UnknownProvider`] if no quota table is registered.
    pub fn quota_for(&self, provider: Provider, model: &str) -> RateResult<Quota> {
        let bundle = self
            .quotas
            .get(&provider)
            .ok_or(RateError::UnknownProvider(provider))?;
        Ok(bundle.resolve(model))
    }

    /// Try to atomically consume 1 RPM token + `prompt_tokens_estimate` TPM
    /// tokens.
    ///
    /// On `Throttled`, the larger of the two suggested waits is returned so
    /// the caller knows the soonest moment both buckets will be satisfied.
    ///
    /// # Errors
    /// - [`RateError::UnknownProvider`] if the provider is not registered.
    /// - [`RateError::Throttled`] if either bucket is starved. The RPM bucket
    ///   is NOT charged when the TPM bucket fails (best-effort atomicity).
    pub fn try_acquire(
        &self,
        provider: Provider,
        model: &str,
        prompt_tokens_estimate: u64,
    ) -> RateResult<()> {
        let quota = self.quota_for(provider, model)?;

        // Snapshot wait times first so we can decide without consuming any
        // tokens if either bucket would fail.
        let rpm_wait = self.peek_request_wait(provider, model, &quota, 1);
        let tpm_wait =
            self.peek_token_wait(provider, model, &quota, prompt_tokens_estimate);

        if rpm_wait > 0 || tpm_wait > 0 {
            let wait_for_ms = rpm_wait.max(tpm_wait);
            tracing::debug!(
                target: "aphrody_rateguard",
                provider = %provider,
                model,
                rpm_wait,
                tpm_wait,
                "throttled",
            );
            return Err(RateError::Throttled { wait_for_ms });
        }

        // Both buckets have capacity — commit now. We re-take each lock; the
        // worst case is a benign race where another caller drains a bucket
        // between the peek and the commit (we surface that as Throttled too).
        self.consume_request(provider, model, &quota, 1)?;
        // If the token consume fails (race), refund the request token so the
        // bucket pair stays consistent.
        if let Err(e) = self.consume_token(provider, model, &quota, prompt_tokens_estimate) {
            self.refund_request(provider, model, 1);
            return Err(e);
        }
        Ok(())
    }

    /// Like [`Self::try_acquire`], but sleeps until both buckets are ready.
    ///
    /// # Errors
    /// - [`RateError::UnknownProvider`] if the provider is not registered.
    /// - [`RateError::WaitTooLong`] if the suggested wait exceeds
    ///   [`Self::max_wait_ms`].
    pub async fn acquire_blocking(
        &self,
        provider: Provider,
        model: &str,
        prompt_tokens_estimate: u64,
    ) -> RateResult<()> {
        loop {
            match self.try_acquire(provider, model, prompt_tokens_estimate) {
                Ok(()) => return Ok(()),
                Err(RateError::Throttled { wait_for_ms }) => {
                    if wait_for_ms > self.max_wait_ms {
                        return Err(RateError::WaitTooLong {
                            requested_ms: wait_for_ms,
                            max_ms: self.max_wait_ms,
                        });
                    }
                    tracing::debug!(
                        target: "aphrody_rateguard",
                        provider = %provider,
                        model,
                        wait_for_ms,
                        "acquire_blocking sleeping",
                    );
                    tokio::time::sleep(Duration::from_millis(wait_for_ms)).await;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Currently-available requests (RPM bucket snapshot).
    pub fn available_requests(&self, provider: Provider, model: &str) -> RateResult<u64> {
        let quota = self.quota_for(provider, model)?;
        Ok(self.with_request_bucket(provider, model, &quota, TokenBucket::available))
    }

    /// Currently-available tokens (TPM bucket snapshot).
    pub fn available_tokens(&self, provider: Provider, model: &str) -> RateResult<u64> {
        let quota = self.quota_for(provider, model)?;
        Ok(self.with_token_bucket(provider, model, &quota, TokenBucket::available))
    }

    // ----- internal helpers ------------------------------------------------

    fn peek_request_wait(
        &self,
        provider: Provider,
        model: &str,
        quota: &Quota,
        n: u64,
    ) -> u64 {
        self.with_request_bucket(provider, model, quota, |bucket| {
            #[allow(clippy::cast_precision_loss, clippy::cast_sign_loss, clippy::cast_possible_truncation)]
            if bucket.available() >= n {
                0_u64
            } else {
                bucket.time_to_n(n).as_millis().min(u64::from(u32::MAX) as u128) as u64
            }
        })
    }

    fn peek_token_wait(
        &self,
        provider: Provider,
        model: &str,
        quota: &Quota,
        n: u64,
    ) -> u64 {
        self.with_token_bucket(provider, model, quota, |bucket| {
            #[allow(clippy::cast_precision_loss, clippy::cast_sign_loss, clippy::cast_possible_truncation)]
            if bucket.available() >= n {
                0_u64
            } else {
                bucket.time_to_n(n).as_millis().min(u64::from(u32::MAX) as u128) as u64
            }
        })
    }

    fn consume_request(
        &self,
        provider: Provider,
        model: &str,
        quota: &Quota,
        n: u64,
    ) -> RateResult<()> {
        self.with_request_bucket(provider, model, quota, |b| b.try_consume(n))
    }

    fn consume_token(
        &self,
        provider: Provider,
        model: &str,
        quota: &Quota,
        n: u64,
    ) -> RateResult<()> {
        self.with_token_bucket(provider, model, quota, |b| b.try_consume(n))
    }

    fn refund_request(&self, provider: Provider, model: &str, n: u64) {
        let mut map = self.request_buckets.lock();
        let key = (provider, model.to_string());
        if let Some(bucket) = map.get_mut(&key) {
            let mut inner = bucket.inner.lock();
            #[allow(clippy::cast_precision_loss)]
            let cap = bucket.capacity as f64;
            inner.tokens = (inner.tokens + n as f64).min(cap);
        }
    }

    fn with_request_bucket<R>(
        &self,
        provider: Provider,
        model: &str,
        quota: &Quota,
        f: impl FnOnce(&TokenBucket) -> R,
    ) -> R {
        let mut map = self.request_buckets.lock();
        let key = (provider, model.to_string());
        let bucket = map
            .entry(key)
            .or_insert_with(|| TokenBucket::new(u64::from(quota.rpm), quota.to_rps()));
        f(bucket)
    }

    fn with_token_bucket<R>(
        &self,
        provider: Provider,
        model: &str,
        quota: &Quota,
        f: impl FnOnce(&TokenBucket) -> R,
    ) -> R {
        let mut map = self.token_buckets.lock();
        let key = (provider, model.to_string());
        let bucket = map
            .entry(key)
            .or_insert_with(|| TokenBucket::new(quota.tpm, quota.to_tps()));
        f(bucket)
    }
}

// ---------------------------------------------------------------------------
// Inline sanity tests (pure, no tokio runtime needed)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod inline_tests {
    use super::*;

    #[test]
    fn provider_all_has_three_in_order() {
        let all = Provider::all();
        assert_eq!(all[0], Provider::Anthropic);
        assert_eq!(all[1], Provider::Gemini);
        assert_eq!(all[2], Provider::Antigravity);
    }

    #[test]
    fn quota_to_rps_and_tps() {
        let q = Quota::new(60, 6_000);
        assert!((q.to_rps() - 1.0).abs() < 1e-9);
        assert!((q.to_tps() - 100.0).abs() < 1e-9);
    }

    #[test]
    fn provider_quota_resolves_default_when_model_unknown() {
        let mut overrides = HashMap::new();
        overrides.insert("known".to_string(), Quota::new(1, 1));
        let pq = ProviderQuota::Anthropic {
            default: Quota::new(99, 9_999),
            per_model: overrides,
        };
        assert_eq!(pq.resolve("known"), Quota::new(1, 1));
        assert_eq!(pq.resolve("unknown"), Quota::new(99, 9_999));
        assert_eq!(pq.provider(), Provider::Anthropic);
    }

    #[test]
    fn default_provider_quotas_covers_three_providers() {
        let q = default_provider_quotas();
        assert_eq!(q.len(), 3);
        assert!(q.contains_key(&Provider::Anthropic));
        assert!(q.contains_key(&Provider::Gemini));
        assert!(q.contains_key(&Provider::Antigravity));
    }
}
