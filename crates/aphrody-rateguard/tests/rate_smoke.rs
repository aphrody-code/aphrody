// SPDX-License-Identifier: Apache-2.0
//! Integration smoke tests for aphrody-rateguard.
//!
//! All tests run on a paused tokio runtime so refill behaviour is fully
//! deterministic — no real wall-clock sleeps.

use aphrody_rateguard::{
    default_provider_quotas, Provider, ProviderQuota, Quota, RateError, RateGuard, TokenBucket,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

/// Helper: build a guard with a single (provider, model) entry, no overrides.
fn guard_with(provider: Provider, default: Quota) -> RateGuard {
    let mut quotas = HashMap::new();
    let pq = match provider {
        Provider::Anthropic => ProviderQuota::Anthropic {
            default,
            per_model: HashMap::new(),
        },
        Provider::Gemini => ProviderQuota::Gemini {
            default,
            per_model: HashMap::new(),
        },
        Provider::Antigravity => ProviderQuota::Antigravity {
            default,
            per_model: HashMap::new(),
        },
    };
    quotas.insert(provider, pq);
    RateGuard::new(quotas)
}

// ----------------------------------------------------------------------------
// 1. TokenBucket — refill semantics
// ----------------------------------------------------------------------------

#[tokio::test(start_paused = true)]
async fn token_bucket_refills_at_rate() {
    // 10 token capacity, 10 tokens/sec refill.
    let bucket = TokenBucket::new(10, 10.0);
    // Drain it fully.
    bucket.try_consume(10).expect("starts full");
    assert_eq!(bucket.available(), 0);

    // Advance 500 ms → should refill ~5 tokens.
    tokio::time::advance(Duration::from_millis(500)).await;
    let available = bucket.available();
    assert!(
        (4..=5).contains(&available),
        "expected ~5 tokens after 500ms, got {available}",
    );

    // Advance another 2 s → should saturate at capacity (10).
    tokio::time::advance(Duration::from_secs(2)).await;
    assert_eq!(bucket.available(), 10);
}

// ----------------------------------------------------------------------------
// 2 + 3. try_consume happy + sad paths
// ----------------------------------------------------------------------------

#[tokio::test(start_paused = true)]
async fn try_consume_succeeds_when_available() {
    let bucket = TokenBucket::new(5, 1.0);
    bucket.try_consume(1).unwrap();
    bucket.try_consume(2).unwrap();
    bucket.try_consume(2).unwrap();
    assert_eq!(bucket.available(), 0);
}

#[tokio::test(start_paused = true)]
async fn try_consume_fails_when_empty() {
    // 2 token capacity, 2 tokens/sec → 500 ms / token.
    let bucket = TokenBucket::new(2, 2.0);
    bucket.try_consume(2).unwrap();
    let err = bucket.try_consume(1).unwrap_err();
    match err {
        RateError::Throttled { wait_for_ms } => {
            // Need 0.5 s to refill 1 token → ~500 ms (allow ceil rounding).
            assert!(
                (450..=520).contains(&wait_for_ms),
                "expected ~500ms wait, got {wait_for_ms}ms",
            );
        }
        other => panic!("expected Throttled, got {other:?}"),
    }
}

// ----------------------------------------------------------------------------
// 4. Default provider quotas — exactly three providers, all populated.
// ----------------------------------------------------------------------------

#[test]
fn default_provider_quotas_has_three_providers() {
    let q = default_provider_quotas();
    assert_eq!(q.len(), 3);
    for p in Provider::all() {
        let bundle = q.get(&p).unwrap_or_else(|| panic!("missing {p}"));
        let def = bundle.default_quota();
        assert!(def.rpm > 0, "{p} default rpm must be > 0");
        assert!(def.tpm > 0, "{p} default tpm must be > 0");
        assert!(!bundle.per_model().is_empty(), "{p} should have ≥1 override");
    }
}

// ----------------------------------------------------------------------------
// 5. RateGuard — RPM throttling after burst.
// ----------------------------------------------------------------------------

#[tokio::test(start_paused = true)]
async fn rate_guard_throttles_after_rpm_burst() {
    // 3 RPM / 1_000_000 TPM → 0.05 req/sec refill.
    let guard = guard_with(Provider::Anthropic, Quota::new(3, 1_000_000));

    // First 3 acquires should succeed (burst).
    for i in 0..3 {
        guard
            .try_acquire(Provider::Anthropic, "claude-test", 100)
            .unwrap_or_else(|e| panic!("burst call {i} failed: {e:?}"));
    }
    // 4th call must throttle.
    let err = guard
        .try_acquire(Provider::Anthropic, "claude-test", 100)
        .unwrap_err();
    matches::assert_throttled(&err);
}

// ----------------------------------------------------------------------------
// 6. RateGuard — TPM throttling.
// ----------------------------------------------------------------------------

#[tokio::test(start_paused = true)]
async fn rate_guard_throttles_after_tpm_exceeded() {
    // 1_000 RPM (huge) but only 1_000 TPM (tiny) → second 600-token call trips TPM.
    let guard = guard_with(Provider::Gemini, Quota::new(1_000, 1_000));
    guard.try_acquire(Provider::Gemini, "gemini-flash", 600).unwrap();
    let err = guard
        .try_acquire(Provider::Gemini, "gemini-flash", 600)
        .unwrap_err();
    matches::assert_throttled(&err);
}

// ----------------------------------------------------------------------------
// 7. RateGuard.acquire_blocking — sleeps then succeeds.
// ----------------------------------------------------------------------------

#[tokio::test(start_paused = true)]
async fn acquire_blocking_waits_then_succeeds() {
    // 2 RPM / 10_000 TPM, refill = 2/60 req/sec ≈ 30 s/req.
    // Drain RPM bucket, then ask for one more — guard should sleep ~30 s.
    let guard = Arc::new(
        guard_with(Provider::Antigravity, Quota::new(2, 10_000)).with_max_wait_ms(120_000),
    );
    guard
        .try_acquire(Provider::Antigravity, "antigravity-prime", 50)
        .unwrap();
    guard
        .try_acquire(Provider::Antigravity, "antigravity-prime", 50)
        .unwrap();

    // Drive the future + the timer side-by-side: spawn the wait on the runtime.
    let guard_clone = Arc::clone(&guard);
    let handle = tokio::spawn(async move {
        guard_clone
            .acquire_blocking(Provider::Antigravity, "antigravity-prime", 50)
            .await
    });

    // Allow the spawned task to reach its first `sleep` await.
    tokio::task::yield_now().await;
    // Move virtual time forward past the refill window.
    tokio::time::advance(Duration::from_secs(60)).await;
    // Yield again so the timer wake fires.
    tokio::task::yield_now().await;

    let res = handle.await.expect("join");
    res.expect("blocking acquire should eventually succeed");
}

// ----------------------------------------------------------------------------
// 8. RateGuard.acquire_blocking — refuses above ceiling.
// ----------------------------------------------------------------------------

#[tokio::test(start_paused = true)]
async fn acquire_blocking_returns_wait_too_long_above_threshold() {
    // 1 RPM / 1_000_000 TPM → ~60 s/req. Set ceiling to 5 s to force failure.
    let guard = guard_with(Provider::Anthropic, Quota::new(1, 1_000_000)).with_max_wait_ms(5_000);
    guard
        .try_acquire(Provider::Anthropic, "claude-test", 10)
        .unwrap();
    let err = guard
        .acquire_blocking(Provider::Anthropic, "claude-test", 10)
        .await
        .unwrap_err();
    match err {
        RateError::WaitTooLong { requested_ms, max_ms } => {
            assert!(requested_ms > max_ms, "{requested_ms} should exceed {max_ms}");
            assert_eq!(max_ms, 5_000);
        }
        other => panic!("expected WaitTooLong, got {other:?}"),
    }
}

// ----------------------------------------------------------------------------
// 9. Unknown model falls back to default quota transparently.
// ----------------------------------------------------------------------------

#[tokio::test(start_paused = true)]
async fn unknown_model_falls_back_to_default_quota() {
    let mut overrides = HashMap::new();
    overrides.insert("known-model".to_string(), Quota::new(1, 1));
    let mut quotas = HashMap::new();
    quotas.insert(
        Provider::Gemini,
        ProviderQuota::Gemini {
            default: Quota::new(100, 100_000),
            per_model: overrides,
        },
    );
    let guard = RateGuard::new(quotas);

    // Unknown model must resolve to the default — should succeed many times.
    for _ in 0..50 {
        guard
            .try_acquire(Provider::Gemini, "mystery-model", 100)
            .expect("default quota should allow burst");
    }

    // And the resolved quota lookup confirms the fallback.
    let q = guard.quota_for(Provider::Gemini, "mystery-model").unwrap();
    assert_eq!(q, Quota::new(100, 100_000));
}

// ----------------------------------------------------------------------------
// Bonus 10: unknown provider → structured error.
// ----------------------------------------------------------------------------

#[tokio::test(start_paused = true)]
async fn unknown_provider_is_reported() {
    let guard = RateGuard::new(HashMap::new());
    let err = guard
        .try_acquire(Provider::Anthropic, "anything", 1)
        .unwrap_err();
    match err {
        RateError::UnknownProvider(p) => assert_eq!(p, Provider::Anthropic),
        other => panic!("expected UnknownProvider, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Tiny matcher helpers — keeps each test body focused on the scenario.
// ---------------------------------------------------------------------------
mod matches {
    use super::RateError;

    pub(crate) fn assert_throttled(err: &RateError) {
        match err {
            RateError::Throttled { wait_for_ms } => {
                assert!(*wait_for_ms > 0, "throttle must report a positive wait");
            }
            other => panic!("expected Throttled, got {other:?}"),
        }
    }
}
