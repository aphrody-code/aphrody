// SPDX-License-Identifier: Apache-2.0
//! End-to-end retry smoke tests for `aphrody-retry`.
//!
//! All async paths run under `tokio::time::pause` so the wall-clock never
//! advances — even on a slow CI runner the suite finishes in milliseconds.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use aphrody_retry::{
    HttpError, HttpStatusClassifier, JitterMode, RetryClassifier, RetryDecision, RetryError,
    RetryOutcome, RetryPolicy, next_delay_with_rng, retry, retry_with_stats,
};
use rand::{SeedableRng, rngs::StdRng};

// ---------------------------------------------------------------------------
// Test fixtures
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct CountingClassifier {
    decision: RetryDecision,
}

#[async_trait::async_trait]
impl RetryClassifier<HttpError> for CountingClassifier {
    fn classify(&self, _error: &HttpError) -> RetryDecision {
        self.decision.clone()
    }
}

// ---------------------------------------------------------------------------
// Pure-logic checks
// ---------------------------------------------------------------------------

#[test]
fn exponential_backoff_doubles_delay() {
    let policy = RetryPolicy {
        jitter: JitterMode::None,
        base_delay_ms: 100,
        multiplier: 2.0,
        max_delay_ms: 10_000,
        ..RetryPolicy::default()
    };
    let mut rng = StdRng::seed_from_u64(0);

    let d1 = next_delay_with_rng(&policy, 1, None, &mut rng, None);
    let d2 = next_delay_with_rng(&policy, 2, None, &mut rng, None);
    let d3 = next_delay_with_rng(&policy, 3, None, &mut rng, None);

    assert_eq!(d1, Duration::from_millis(100));
    assert_eq!(d2, Duration::from_millis(200));
    assert_eq!(d3, Duration::from_millis(400));
}

#[test]
fn jitter_full_returns_value_in_range() {
    let policy = RetryPolicy {
        jitter: JitterMode::Full,
        base_delay_ms: 1_000,
        multiplier: 1.0,
        max_delay_ms: 10_000,
        ..RetryPolicy::default()
    };
    let mut rng = StdRng::seed_from_u64(7);
    for _ in 0..50 {
        let d = next_delay_with_rng(&policy, 1, None, &mut rng, None);
        assert!(d <= Duration::from_millis(1_000), "got {d:?}");
    }
}

#[test]
fn policy_default_max_attempts_5() {
    assert_eq!(RetryPolicy::default().max_attempts, 5);
}

#[test]
fn decorrelated_jitter_differs_per_attempt() {
    // Flake-resistant: run N=20 samples, expect at least 5 distinct values
    // (with multiplier=1 and the upper bound = prev*3, the chance of all
    // 20 hitting the same integer is vanishing).
    let policy = RetryPolicy {
        jitter: JitterMode::Decorrelated,
        base_delay_ms: 100,
        multiplier: 1.0,
        max_delay_ms: 30_000,
        ..RetryPolicy::default()
    };
    let mut rng = StdRng::seed_from_u64(123);
    let mut seen = std::collections::HashSet::new();
    let mut prev = None;
    for attempt in 1..=20 {
        let d = next_delay_with_rng(&policy, attempt, None, &mut rng, prev);
        let ms = d.as_millis() as u64;
        seen.insert(ms);
        prev = Some(ms);
    }
    assert!(
        seen.len() > 5,
        "decorrelated jitter looks frozen: {} distinct samples",
        seen.len()
    );
}

// ---------------------------------------------------------------------------
// Async driver — paused-time tests
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn retry_succeeds_after_n_failures() {
    let counter = Arc::new(AtomicU32::new(0));
    let policy = RetryPolicy {
        max_attempts: 5,
        base_delay_ms: 50,
        max_delay_ms: 1_000,
        multiplier: 2.0,
        jitter: JitterMode::None,
        ..RetryPolicy::default()
    };
    let classifier = HttpStatusClassifier;
    let c_for_closure = counter.clone();
    let res: Result<u32, RetryError<HttpError>> = retry(&policy, &classifier, move || {
        let cc = c_for_closure.clone();
        async move {
            let n = cc.fetch_add(1, Ordering::SeqCst) + 1;
            if n < 4 {
                Err(HttpError {
                    status: 503,
                    retry_after_ms: None,
                })
            } else {
                Ok(n)
            }
        }
    })
    .await;
    assert!(matches!(res, Ok(4)));
    assert_eq!(counter.load(Ordering::SeqCst), 4);
}

#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn retry_exhausts_returns_last_error() {
    let policy = RetryPolicy {
        max_attempts: 3,
        base_delay_ms: 10,
        max_delay_ms: 1_000,
        multiplier: 2.0,
        jitter: JitterMode::None,
        ..RetryPolicy::default()
    };
    let attempts = Arc::new(AtomicU32::new(0));
    let attempts_c = attempts.clone();
    let res: Result<(), RetryError<HttpError>> =
        retry(&policy, &HttpStatusClassifier, move || {
            let a = attempts_c.clone();
            async move {
                a.fetch_add(1, Ordering::SeqCst);
                Err::<(), _>(HttpError {
                    status: 500,
                    retry_after_ms: None,
                })
            }
        })
        .await;
    match res {
        Err(RetryError::Exhausted { attempts, .. }) => assert_eq!(attempts, 3),
        other => panic!("expected Exhausted, got {other:?}"),
    }
    assert_eq!(attempts.load(Ordering::SeqCst), 3);
}

#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn classifier_stop_returns_immediately() {
    let classifier = CountingClassifier {
        decision: RetryDecision::Stop,
    };
    let attempts = Arc::new(AtomicU32::new(0));
    let attempts_c = attempts.clone();
    let policy = RetryPolicy {
        max_attempts: 10,
        base_delay_ms: 1,
        ..RetryPolicy::default()
    };
    let res: Result<(), RetryError<HttpError>> = retry(&policy, &classifier, move || {
        let a = attempts_c.clone();
        async move {
            a.fetch_add(1, Ordering::SeqCst);
            Err::<(), _>(HttpError {
                status: 404,
                retry_after_ms: None,
            })
        }
    })
    .await;
    assert!(matches!(res, Err(RetryError::Stopped { .. })));
    assert_eq!(
        attempts.load(Ordering::SeqCst),
        1,
        "Stop must short-circuit"
    );
}

#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn http_classifier_429_with_retry_after_honored() {
    // Returns 429 with hint=5000ms twice, then succeeds. We assert that
    // total_sleep_ms accounts for the hint, not the policy backoff.
    let counter = Arc::new(AtomicU32::new(0));
    let policy = RetryPolicy {
        max_attempts: 5,
        base_delay_ms: 50, // would be much smaller than 5000 hint
        max_delay_ms: 60_000,
        multiplier: 2.0,
        jitter: JitterMode::None,
        ..RetryPolicy::default()
    };
    let counter_c = counter.clone();
    let mut f = move || {
        let cc = counter_c.clone();
        async move {
            let n = cc.fetch_add(1, Ordering::SeqCst) + 1;
            if n < 3 {
                Err(HttpError {
                    status: 429,
                    retry_after_ms: Some(5_000),
                })
            } else {
                Ok(n)
            }
        }
    };
    let (res, stats) = retry_with_stats(&policy, &HttpStatusClassifier, &mut f).await;
    assert!(matches!(res, Ok(3)));
    assert_eq!(stats.total_attempts, 3);
    assert_eq!(stats.total_sleep_ms, 10_000, "two 5000ms hint sleeps");
    assert_eq!(stats.final_outcome, RetryOutcome::Ok);
}

#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn http_classifier_500_retries_with_default_backoff() {
    let policy = RetryPolicy {
        max_attempts: 4,
        base_delay_ms: 100,
        max_delay_ms: 60_000,
        multiplier: 2.0,
        jitter: JitterMode::None,
        ..RetryPolicy::default()
    };
    let counter = Arc::new(AtomicU32::new(0));
    let cc = counter.clone();
    let mut f = move || {
        let c2 = cc.clone();
        async move {
            let n = c2.fetch_add(1, Ordering::SeqCst) + 1;
            if n < 3 {
                Err(HttpError {
                    status: 500,
                    retry_after_ms: None,
                })
            } else {
                Ok::<u32, HttpError>(n)
            }
        }
    };
    let (res, stats) = retry_with_stats(&policy, &HttpStatusClassifier, &mut f).await;
    assert!(matches!(res, Ok(3)));
    // Two sleeps: attempt 1→2 = 100ms, attempt 2→3 = 200ms (multiplier=2, JitterNone).
    assert_eq!(stats.total_sleep_ms, 300);
}

#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn http_classifier_404_stops_immediately() {
    let policy = RetryPolicy {
        max_attempts: 10,
        base_delay_ms: 1,
        ..RetryPolicy::default()
    };
    let attempts = Arc::new(AtomicU32::new(0));
    let ac = attempts.clone();
    let res: Result<(), RetryError<HttpError>> =
        retry(&policy, &HttpStatusClassifier, move || {
            let a = ac.clone();
            async move {
                a.fetch_add(1, Ordering::SeqCst);
                Err::<(), _>(HttpError {
                    status: 404,
                    retry_after_ms: None,
                })
            }
        })
        .await;
    assert!(matches!(res, Err(RetryError::Stopped { .. })));
    assert_eq!(attempts.load(Ordering::SeqCst), 1);
}

#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn first_attempt_success_records_one_attempt() {
    let policy = RetryPolicy::default();
    let mut f = || async { Ok::<u32, HttpError>(42) };
    let (res, stats) = retry_with_stats(&policy, &HttpStatusClassifier, &mut f).await;
    assert_eq!(res.unwrap(), 42);
    assert_eq!(stats.total_attempts, 1);
    assert_eq!(stats.total_sleep_ms, 0);
    assert_eq!(stats.final_outcome, RetryOutcome::Ok);
}
