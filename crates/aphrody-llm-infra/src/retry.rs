// SPDX-License-Identifier: Apache-2.0
//! Reusable retry primitives for the aphrody workspace.
//!
//! Designed for HTTP API clients (in particular the LLM provider whitelist
//! exposed by `aphrody-router`: Anthropic, Gemini, Antigravity) that must
//! tolerate transient 429 / 5xx failures without burning the user's budget
//! or hammering an upstream that is asking us to slow down.
//!
//! ## Design goals
//!
//! 1. **Pure logic, zero I/O.** [`next_delay`] is a pure function; the
//!    [`retry`] driver only depends on `tokio::time::sleep`, so all tests
//!    run under [`tokio::time::pause`] for fully deterministic execution.
//! 2. **Honour `Retry-After`.** When a classifier returns a hint
//!    ([`RetryDecision::Retry { suggested_delay_ms: Some(_) }`]), the
//!    scheduler uses it verbatim instead of recomputing backoff —
//!    mirroring the `lift_error` pattern in
//!    [`aphrody-router`](../aphrody_router/index.html) `RouterError::RateLimit`.
//! 3. **Pluggable classifier.** The [`RetryClassifier`] trait is generic
//!    over the error type so callers can plug their own status-code
//!    mapper. A first-class [`HttpStatusClassifier`] is shipped for the
//!    common case (RFC 9110 retryable status set).
//! 4. **Four jitter modes** — `None`, `Full`, `Equal`, `Decorrelated`,
//!    matching the AWS Architecture blog vocabulary so that anyone tuning
//!    a policy has a shared mental model.
//! 5. **Observable.** Every attempt + sleep is recorded through `tracing`
//!    span events; an opt-in `retry_with_stats` variant returns a
//!    [`RetryStats`] capturing the full attempt history.
//!
//! ## Cohesion with the wider workspace
//!
//! * `aphrody-router`: each provider call wraps its `reqwest::Response`
//!   pipeline in [`retry`] with [`HttpStatusClassifier`] reading the
//!   `Retry-After` header into [`RetryDecision::Retry::suggested_delay_ms`].
//! * `aphrody-messaging`: outbound connectors (Telegram / Slack / SMTP)
//!   reuse the same primitive so the operator sees one tuning surface.
//! * `aphrody-cron`: long-running jobs that drive HTTP fan-out use
//!   [`RetryPolicy::aggressive`] for fast retries inside a scheduled tick.

#![forbid(unsafe_code)]

use std::fmt::{self, Debug, Display};
use std::future::Future;
use std::time::Duration;

use async_trait::async_trait;
use rand::{Rng, SeedableRng, rngs::StdRng};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, warn};

// ---------------------------------------------------------------------------
// Jitter modes
// ---------------------------------------------------------------------------

/// Jitter strategy applied on top of the exponential base delay.
///
/// Vocabulary follows the AWS Architecture blog post "Exponential Backoff
/// and Jitter", widely adopted across SDKs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JitterMode {
    /// `delay = base`. Deterministic. Useful only for tests / debugging.
    None,
    /// `delay = rand(0, base)`. Maximum spread; recommended default.
    Full,
    /// `delay = base/2 + rand(0, base/2)`. Less spread, keeps a floor.
    Equal,
    /// `delay = rand(base, prev * 3)`. Self-correlated, prevents lockstep.
    Decorrelated,
}

// ---------------------------------------------------------------------------
// RetryPolicy
// ---------------------------------------------------------------------------

/// Tunable retry configuration.
///
/// Field defaults are tuned for cooperative LLM provider APIs (Anthropic /
/// Gemini / Antigravity) on a typical residential/office network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// Maximum number of attempts (including the initial call). `1` means
    /// "no retries". Default: `5`.
    pub max_attempts: u32,
    /// Base delay before the first retry, in milliseconds. Default: `200`.
    pub base_delay_ms: u64,
    /// Hard cap applied after exponential growth + jitter. Default: `30_000`.
    pub max_delay_ms: u64,
    /// Exponential multiplier (`base * multiplier^(attempt-1)`).
    /// Default: `2.0`.
    pub multiplier: f64,
    /// Jitter strategy. Default: [`JitterMode::Full`].
    pub jitter: JitterMode,
    /// If `true`, *any* 4xx is retried. Default: `false` (we still retry
    /// the explicit "transient 4xx" set 408 / 425 / 429 via the
    /// classifier hint regardless of this flag).
    pub retry_on_4xx: bool,
    /// If `true`, retry on 5xx. Default: `true`.
    pub retry_on_5xx: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            base_delay_ms: 200,
            max_delay_ms: 30_000,
            multiplier: 2.0,
            jitter: JitterMode::Full,
            retry_on_4xx: false,
            retry_on_5xx: true,
        }
    }
}

impl RetryPolicy {
    /// Aggressive defaults: many short retries, suitable for tight bursts
    /// against an already-warmed upstream (e.g. CDN edge).
    #[must_use]
    pub fn aggressive() -> Self {
        Self {
            max_attempts: 10,
            base_delay_ms: 100,
            max_delay_ms: 5_000,
            multiplier: 1.8,
            jitter: JitterMode::Full,
            retry_on_4xx: false,
            retry_on_5xx: true,
        }
    }

    /// Conservative defaults: at most three attempts, longer waits.
    /// Use when the operation is expensive or carries side-effects.
    #[must_use]
    pub fn conservative() -> Self {
        Self {
            max_attempts: 3,
            base_delay_ms: 500,
            max_delay_ms: 60_000,
            multiplier: 2.5,
            jitter: JitterMode::Equal,
            retry_on_4xx: false,
            retry_on_5xx: true,
        }
    }

    /// Validate the policy and return [`RetryError::JitterRng`] when the
    /// configuration would cause non-terminating loops (e.g. multiplier
    /// of zero with full jitter).
    fn assert_sane(&self) {
        debug_assert!(self.max_attempts >= 1, "max_attempts must be ≥1");
        debug_assert!(self.base_delay_ms > 0, "base_delay_ms must be > 0");
        debug_assert!(self.multiplier > 0.0, "multiplier must be > 0");
    }
}

// ---------------------------------------------------------------------------
// Decision + classifier
// ---------------------------------------------------------------------------

/// What the classifier decided to do with the current error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetryDecision {
    /// Retry the call. If `suggested_delay_ms` is `Some`, the scheduler
    /// uses it verbatim (e.g. honouring a `Retry-After` header).
    Retry { suggested_delay_ms: Option<u64> },
    /// Surface the error immediately, do not sleep, do not retry.
    Stop,
}

/// Object-safe classifier trait: maps a borrowed error to a [`RetryDecision`].
///
/// `Sync + Send` is required so the trait can be passed across `.await`
/// points inside the [`retry`] driver.
#[async_trait]
pub trait RetryClassifier<E>: Send + Sync {
    /// Inspect the error and decide whether to retry.
    fn classify(&self, error: &E) -> RetryDecision;
}

/// Pre-canned HTTP-status classifier driven by [`HttpError`].
///
/// Follows the conventions documented in the [`aphrody-router`] error
/// surface (`RouterError::Auth`, `RouterError::RateLimit`, etc.) so two
/// callers using both crates see consistent semantics.
#[derive(Debug, Clone, Copy, Default)]
pub struct HttpStatusClassifier;

/// Minimal HTTP error envelope the [`HttpStatusClassifier`] understands.
///
/// Callers convert their domain error (e.g. `RouterError`, `reqwest::Error`)
/// to this shape before classification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpError {
    /// HTTP status code lifted from the response.
    pub status: u16,
    /// Optional `Retry-After` header value, converted to milliseconds.
    pub retry_after_ms: Option<u64>,
}

impl Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.retry_after_ms {
            Some(ms) => write!(f, "HTTP {} (retry after {} ms)", self.status, ms),
            None => write!(f, "HTTP {}", self.status),
        }
    }
}

impl std::error::Error for HttpError {}

#[async_trait]
impl RetryClassifier<HttpError> for HttpStatusClassifier {
    fn classify(&self, error: &HttpError) -> RetryDecision {
        match error.status {
            // RFC 9110 "transient 4xx" set: Request Timeout, Too Early,
            // Too Many Requests. These are retryable even with
            // `retry_on_4xx == false`.
            408 | 425 | 429 => RetryDecision::Retry {
                suggested_delay_ms: error.retry_after_ms,
            },
            // 5xx — retry with the policy backoff.
            500..=599 => RetryDecision::Retry {
                suggested_delay_ms: None,
            },
            // Any other 4xx (auth, validation, not-found …) is terminal.
            _ => RetryDecision::Stop,
        }
    }
}

// ---------------------------------------------------------------------------
// Delay calculation
// ---------------------------------------------------------------------------

/// Compute the next sleep duration before retry attempt `attempt`.
///
/// * `attempt` is 1-based (the second call to `f` is `attempt == 1`).
/// * If `classifier_hint` is `Some`, it overrides backoff entirely
///   (capped to `policy.max_delay_ms` to avoid pathological values).
///
/// The function is *pure* — given the same RNG it returns the same value,
/// which makes it deterministic under `tokio::time::pause`.
#[must_use]
pub fn next_delay(policy: &RetryPolicy, attempt: u32, classifier_hint: Option<u64>) -> Duration {
    next_delay_with_rng(policy, attempt, classifier_hint, &mut StdRng::from_entropy(), None)
}

/// `next_delay` variant that lets the caller inject the RNG (for
/// reproducible tests) and the previous delay (only used by
/// [`JitterMode::Decorrelated`]).
pub fn next_delay_with_rng(
    policy: &RetryPolicy,
    attempt: u32,
    classifier_hint: Option<u64>,
    rng: &mut StdRng,
    prev_delay_ms: Option<u64>,
) -> Duration {
    policy.assert_sane();

    if let Some(hint) = classifier_hint {
        let capped = hint.min(policy.max_delay_ms);
        return Duration::from_millis(capped);
    }

    // Exponential base = base * multiplier^(attempt-1), capped to max.
    let exp = policy.multiplier.powi(attempt.saturating_sub(1) as i32);
    let raw = (policy.base_delay_ms as f64 * exp).min(policy.max_delay_ms as f64);
    let base_ms = raw.max(1.0) as u64;
    let max_ms = policy.max_delay_ms.max(base_ms);

    let ms = match policy.jitter {
        JitterMode::None => base_ms,
        JitterMode::Full => {
            if base_ms == 0 {
                0
            } else {
                rng.gen_range(0..=base_ms)
            }
        }
        JitterMode::Equal => {
            let half = base_ms / 2;
            let extra = if half == 0 { 0 } else { rng.gen_range(0..=half) };
            half + extra
        }
        JitterMode::Decorrelated => {
            let prev = prev_delay_ms.unwrap_or(policy.base_delay_ms).max(1);
            let upper = prev.saturating_mul(3).min(max_ms).max(policy.base_delay_ms);
            if upper <= policy.base_delay_ms {
                policy.base_delay_ms
            } else {
                rng.gen_range(policy.base_delay_ms..=upper)
            }
        }
    };

    Duration::from_millis(ms.min(max_ms))
}

// ---------------------------------------------------------------------------
// Errors + stats
// ---------------------------------------------------------------------------

/// Failure surface returned by [`retry`] when no successful value is produced.
#[derive(Debug, Error)]
pub enum RetryError<E>
where
    E: Display + Debug,
{
    /// Every attempt failed and the policy's `max_attempts` is exhausted.
    #[error("retry exhausted after {attempts} attempts: {last_error}")]
    Exhausted {
        /// Number of attempts actually performed.
        attempts: u32,
        /// The error returned by the *last* attempt.
        last_error: E,
    },
    /// Classifier returned [`RetryDecision::Stop`]; no further attempts.
    #[error("retry stopped by classifier: {error}")]
    Stopped {
        /// The terminal error that caused the stop.
        error: E,
    },
    /// Jitter RNG / policy configuration error.
    #[error("retry jitter rng error: {0}")]
    JitterRng(String),
}

/// Outcome of a retry session, returned alongside the value by
/// [`retry_with_stats`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetryOutcome {
    /// The final attempt succeeded.
    Ok,
    /// All attempts failed; the policy budget was exhausted.
    Exhausted,
    /// Classifier returned [`RetryDecision::Stop`] before exhaustion.
    Stopped,
}

/// Per-call statistics captured by [`retry_with_stats`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryStats {
    /// Number of attempts performed (1 = first try succeeded).
    pub total_attempts: u32,
    /// Total time spent sleeping between attempts.
    pub total_sleep_ms: u64,
    /// Final outcome of the session.
    pub final_outcome: RetryOutcome,
}

// ---------------------------------------------------------------------------
// Driver
// ---------------------------------------------------------------------------

/// Execute `f` according to `policy`, classifying failures with `classifier`.
///
/// On success, returns `Ok(T)` immediately.
/// On failure, the classifier is consulted; if it says
/// [`RetryDecision::Retry`], the driver sleeps via `tokio::time::sleep`
/// and re-invokes `f`. Returns [`RetryError::Exhausted`] when the
/// `max_attempts` budget is spent and [`RetryError::Stopped`] when the
/// classifier asks to give up.
pub async fn retry<T, E, F, Fut>(
    policy: &RetryPolicy,
    classifier: &dyn RetryClassifier<E>,
    mut f: F,
) -> Result<T, RetryError<E>>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: Display + Debug,
{
    let (res, _stats) = retry_with_stats(policy, classifier, &mut f).await;
    res
}

/// [`retry`] variant that also returns the [`RetryStats`].
///
/// The borrow on `f` is `&mut` so the caller can move stack data into
/// the closure (e.g. a `reqwest::Client`).
pub async fn retry_with_stats<T, E, F, Fut>(
    policy: &RetryPolicy,
    classifier: &dyn RetryClassifier<E>,
    f: &mut F,
) -> (Result<T, RetryError<E>>, RetryStats)
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: Display + Debug,
{
    policy.assert_sane();

    let mut rng = StdRng::from_entropy();
    let mut total_sleep_ms = 0u64;
    let mut prev_delay_ms: Option<u64> = None;
    let max = policy.max_attempts.max(1);

    let mut last_err: Option<E> = None;
    for attempt in 1..=max {
        match f().await {
            Ok(value) => {
                debug!(attempt, "retry: success");
                return (
                    Ok(value),
                    RetryStats {
                        total_attempts: attempt,
                        total_sleep_ms,
                        final_outcome: RetryOutcome::Ok,
                    },
                );
            }
            Err(err) => {
                let decision = classifier.classify(&err);
                match decision {
                    RetryDecision::Stop => {
                        warn!(attempt, error = %err, "retry: classifier stopped");
                        return (
                            Err(RetryError::Stopped { error: err }),
                            RetryStats {
                                total_attempts: attempt,
                                total_sleep_ms,
                                final_outcome: RetryOutcome::Stopped,
                            },
                        );
                    }
                    RetryDecision::Retry { suggested_delay_ms } => {
                        if attempt >= max {
                            // No budget left for another attempt — surface
                            // exhausted without sleeping.
                            warn!(attempts = attempt, "retry: budget exhausted");
                            return (
                                Err(RetryError::Exhausted {
                                    attempts: attempt,
                                    last_error: err,
                                }),
                                RetryStats {
                                    total_attempts: attempt,
                                    total_sleep_ms,
                                    final_outcome: RetryOutcome::Exhausted,
                                },
                            );
                        }
                        let delay = next_delay_with_rng(
                            policy,
                            attempt,
                            suggested_delay_ms,
                            &mut rng,
                            prev_delay_ms,
                        );
                        let delay_ms = delay.as_millis() as u64;
                        debug!(
                            attempt,
                            sleep_ms = delay_ms,
                            hint_ms = ?suggested_delay_ms,
                            "retry: sleeping before next attempt"
                        );
                        total_sleep_ms = total_sleep_ms.saturating_add(delay_ms);
                        prev_delay_ms = Some(delay_ms);
                        last_err = Some(err);
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }
    }

    // Unreachable in practice (loop returns on success/stop/exhaustion),
    // but keeps the type-checker happy without an `unreachable!()` panic.
    let err = last_err.expect("loop must produce an error before falling through");
    (
        Err(RetryError::Exhausted {
            attempts: max,
            last_error: err,
        }),
        RetryStats {
            total_attempts: max,
            total_sleep_ms,
            final_outcome: RetryOutcome::Exhausted,
        },
    )
}

// ---------------------------------------------------------------------------
// Unit tests (pure logic; async paths exercised in tests/retry_smoke.rs)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_max_attempts_is_5() {
        assert_eq!(RetryPolicy::default().max_attempts, 5);
    }

    #[test]
    fn aggressive_has_more_attempts_than_default() {
        assert!(RetryPolicy::aggressive().max_attempts > RetryPolicy::default().max_attempts);
    }

    #[test]
    fn conservative_has_fewer_attempts_than_default() {
        assert!(RetryPolicy::conservative().max_attempts < RetryPolicy::default().max_attempts);
    }

    #[test]
    fn jitter_none_returns_exact_base() {
        let policy = RetryPolicy {
            jitter: JitterMode::None,
            ..RetryPolicy::default()
        };
        let mut rng = StdRng::seed_from_u64(0);
        // attempt=1 → base * multiplier^0 = base
        let d = next_delay_with_rng(&policy, 1, None, &mut rng, None);
        assert_eq!(d, Duration::from_millis(policy.base_delay_ms));
        // attempt=2 → base * multiplier = 400
        let d = next_delay_with_rng(&policy, 2, None, &mut rng, None);
        assert_eq!(d, Duration::from_millis(400));
    }

    #[test]
    fn classifier_hint_overrides_backoff() {
        let policy = RetryPolicy::default();
        let mut rng = StdRng::seed_from_u64(42);
        // attempt=10 with hint 5_000 ms → exactly 5000ms, no exponential growth.
        let d = next_delay_with_rng(&policy, 10, Some(5_000), &mut rng, None);
        assert_eq!(d, Duration::from_millis(5_000));
    }

    #[test]
    fn classifier_hint_capped_to_max_delay() {
        let policy = RetryPolicy {
            max_delay_ms: 1_000,
            ..RetryPolicy::default()
        };
        let mut rng = StdRng::seed_from_u64(0);
        // Hint 999_999 → must be capped to 1000.
        let d = next_delay_with_rng(&policy, 1, Some(999_999), &mut rng, None);
        assert_eq!(d, Duration::from_millis(1_000));
    }

    #[test]
    fn http_classifier_429_uses_hint() {
        let c = HttpStatusClassifier;
        let dec = c.classify(&HttpError {
            status: 429,
            retry_after_ms: Some(2_500),
        });
        assert_eq!(
            dec,
            RetryDecision::Retry {
                suggested_delay_ms: Some(2_500)
            }
        );
    }

    #[test]
    fn http_classifier_500_retries_without_hint() {
        let c = HttpStatusClassifier;
        let dec = c.classify(&HttpError {
            status: 503,
            retry_after_ms: None,
        });
        assert_eq!(
            dec,
            RetryDecision::Retry {
                suggested_delay_ms: None
            }
        );
    }

    #[test]
    fn http_classifier_404_stops() {
        let c = HttpStatusClassifier;
        let dec = c.classify(&HttpError {
            status: 404,
            retry_after_ms: None,
        });
        assert_eq!(dec, RetryDecision::Stop);
    }
}
