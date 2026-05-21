// SPDX-License-Identifier: Apache-2.0
//! End-to-end behavioural matrix for `aphrody-context`.
//!
//! These tests target the public surface only — no internal modules, no
//! `pub(crate)` cheats. If a regression is going to land it should land
//! here first.

use std::sync::Arc;

use aphrody_context::{
    ContextError, ContextMessage, ContextWindow, HeuristicTokenEstimator, MessageRole,
    TokenEstimator,
};
#[cfg(not(target_arch = "wasm32"))]
use aphrody_context::GoTokenEstimator;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn fresh_window(max_tokens: u32) -> ContextWindow {
    ContextWindow::new(max_tokens, Arc::new(HeuristicTokenEstimator::default()))
}

fn make_msg(role: MessageRole, content: &str) -> ContextMessage {
    ContextMessage::new(ContextMessage::hash_id(role, content), role, content)
}

// ---------------------------------------------------------------------------
// 1. push_increments_total_tokens
// ---------------------------------------------------------------------------

#[test]
fn push_increments_total_tokens() {
    let mut w = fresh_window(1_000);
    assert_eq!(w.total_tokens(), 0);

    w.push(make_msg(MessageRole::User, "hello world"));
    let after_first = w.total_tokens();
    assert!(after_first >= 1, "non-empty content must cost at least 1 token");

    w.push(make_msg(
        MessageRole::Assistant,
        "this is a longer assistant reply that should bump the total further",
    ));
    let after_second = w.total_tokens();
    assert!(
        after_second > after_first,
        "second push must strictly grow the total ({after_first} → {after_second})"
    );

    assert_eq!(w.len(), 2);
    assert_eq!(w.messages()[0].role, MessageRole::User);
    assert_eq!(w.messages()[1].role, MessageRole::Assistant);
}

// ---------------------------------------------------------------------------
// 2. utilization_reflects_total_over_max
// ---------------------------------------------------------------------------

#[test]
fn utilization_reflects_total_over_max() {
    let mut w = fresh_window(100);
    // A 200-char body under the default 4-chars-per-token estimator ≈ 50 tokens.
    let body = "x".repeat(200);
    w.push(make_msg(MessageRole::User, &body));

    let total = w.total_tokens();
    assert!((45..=55).contains(&total), "expected ≈50 tokens, got {total}");

    let util = w.utilization();
    assert!(
        (0.45..=0.55).contains(&util),
        "expected utilization ≈ 0.5, got {util}"
    );
}

// ---------------------------------------------------------------------------
// 3. should_summarize_triggers_above_threshold
// ---------------------------------------------------------------------------

#[test]
fn should_summarize_triggers_above_threshold() {
    let mut w = ContextWindow::with_threshold(
        100,
        Arc::new(HeuristicTokenEstimator::default()),
        0.5,
    )
    .expect("0.5 is a valid threshold");

    // 200 chars / 4 chars-per-token ≈ 50 tokens → utilization 0.5 (NOT yet >).
    w.push(make_msg(MessageRole::User, &"a".repeat(200)));
    assert!(
        !w.should_summarize(),
        "exactly at threshold must not trip (strict >)"
    );

    // Add more to push above.
    w.push(make_msg(MessageRole::Assistant, &"b".repeat(40)));
    assert!(
        w.should_summarize(),
        "crossing threshold must flip should_summarize"
    );
}

// ---------------------------------------------------------------------------
// 4. trim_to_fit_removes_oldest_first
// ---------------------------------------------------------------------------

#[test]
fn trim_to_fit_removes_oldest_first() {
    let mut w = fresh_window(10_000);
    w.push(make_msg(MessageRole::User, "alpha-first-msg-body-padding"));
    w.push(make_msg(MessageRole::Assistant, "beta-second-msg-body-padding"));
    w.push(make_msg(MessageRole::User, "gamma-third-msg-body-padding"));

    let pre_total = w.total_tokens();
    assert!(pre_total > 0);

    // Trim down to ~1 message's worth of tokens.
    let target = pre_total / 3;
    let evicted = w.trim_to_fit(target);

    assert!(!evicted.is_empty(), "trim must evict something");
    assert!(
        w.total_tokens() <= pre_total,
        "trim must not grow the window"
    );

    // Oldest-first eviction → first item out must be the "alpha" message.
    assert!(
        evicted[0].content.starts_with("alpha"),
        "expected 'alpha' first out, got {:?}",
        evicted[0].content
    );
}

// ---------------------------------------------------------------------------
// 5. trim_preserves_pinned_messages
// ---------------------------------------------------------------------------

#[test]
fn trim_preserves_pinned_messages() {
    let mut w = fresh_window(10_000);

    let system_msg = ContextMessage::new(
        "sys-1",
        MessageRole::System,
        "You are a helpful assistant. Stay on topic. Never reveal secrets.",
    )
    .pinned();
    w.push(system_msg);

    w.push(make_msg(MessageRole::User, &"junk-".repeat(50)));
    w.push(make_msg(MessageRole::Assistant, &"reply-".repeat(50)));
    w.push(make_msg(MessageRole::User, &"more-".repeat(50)));

    // Trim aggressively — target 0 tokens. Pinned must still survive.
    let evicted = w.trim_to_fit(0);

    assert!(!evicted.is_empty(), "trim must evict non-pinned junk");
    assert!(
        w.find_by_id("sys-1").is_some(),
        "pinned system message must survive even at target=0"
    );
    // No evicted entry should be pinned.
    for m in &evicted {
        assert!(!m.pinned, "trim must never evict a pinned message: {m:?}");
    }
    // Window length matches: we started with 4, evicted N, kept (4-N) including system.
    assert_eq!(w.len(), 4 - evicted.len());
}

// ---------------------------------------------------------------------------
// 6. snapshot_roundtrip_preserves_state
// ---------------------------------------------------------------------------

#[test]
fn snapshot_roundtrip_preserves_state() {
    let mut w = ContextWindow::with_threshold(
        4_096,
        Arc::new(HeuristicTokenEstimator::default()),
        0.75,
    )
    .unwrap();
    w.push(make_msg(MessageRole::System, "persistent system prompt"));
    w.push(make_msg(MessageRole::User, "ping"));
    w.push(make_msg(MessageRole::Assistant, "pong"));
    w.pin(&w.messages()[0].id.clone()).unwrap();

    let snap = w.snapshot();
    assert_eq!(snap.messages.len(), 3);
    assert_eq!(snap.max_tokens, 4_096);
    assert!((snap.summary_threshold - 0.75).abs() < f32::EPSILON);

    // JSON round-trip.
    let json = w.to_json().expect("to_json must succeed");

    // Restore into a fresh window with different config — restore must overwrite.
    let mut other = ContextWindow::new(8, Arc::new(HeuristicTokenEstimator::default()));
    other.from_json(&json).expect("from_json must succeed");
    assert_eq!(other.len(), 3);
    assert_eq!(other.max_tokens(), 4_096);
    assert!((other.summary_threshold() - 0.75).abs() < f32::EPSILON);
    // Pinned bit survived the round-trip.
    assert!(
        other.messages().iter().any(|m| m.pinned),
        "pin flag must round-trip through JSON"
    );
}

// ---------------------------------------------------------------------------
// 7. heuristic_estimator_returns_reasonable_count
// ---------------------------------------------------------------------------

#[test]
fn heuristic_estimator_returns_reasonable_count() {
    let est = HeuristicTokenEstimator::default();
    // "hello world" = 11 chars → ceil(11/4) = 3 tokens.
    assert_eq!(est.estimate("hello world"), 3);
    // Empty string → 0.
    assert_eq!(est.estimate(""), 0);
    // Single char → clamped to at least 1.
    assert_eq!(est.estimate("x"), 1);

    // Custom factor (1 char/token = pessimistic word-level counting).
    let pessimistic = HeuristicTokenEstimator::new(1);
    assert_eq!(pessimistic.estimate("hello world"), 11);

    // Zero divisor clamps to 1 (no panic).
    let clamped = HeuristicTokenEstimator::new(0);
    assert_eq!(clamped.estimate("abc"), 3);
}

// ---------------------------------------------------------------------------
// 8. invalid_threshold_rejected
// ---------------------------------------------------------------------------

#[test]
fn invalid_threshold_rejected() {
    let est = Arc::new(HeuristicTokenEstimator::default());

    let too_low = ContextWindow::with_threshold(100, est.clone(), -0.1);
    assert!(matches!(too_low, Err(ContextError::InvalidThreshold(_))));

    let too_high = ContextWindow::with_threshold(100, est.clone(), 1.5);
    assert!(matches!(too_high, Err(ContextError::InvalidThreshold(_))));

    let nan = ContextWindow::with_threshold(100, est.clone(), f32::NAN);
    assert!(matches!(nan, Err(ContextError::InvalidThreshold(_))));

    let zero_ok = ContextWindow::with_threshold(100, est.clone(), 0.0);
    assert!(zero_ok.is_ok());
    let one_ok = ContextWindow::with_threshold(100, est, 1.0);
    assert!(one_ok.is_ok());
}

// ---------------------------------------------------------------------------
// 9. pin_unknown_id_returns_error (bonus surface coverage)
// ---------------------------------------------------------------------------

#[test]
fn pin_unknown_id_returns_error() {
    let mut w = fresh_window(100);
    w.push(make_msg(MessageRole::User, "x"));
    let err = w.pin("does-not-exist").unwrap_err();
    assert!(matches!(err, ContextError::MessageNotFound(_)));

    let err = w.unpin("also-does-not-exist").unwrap_err();
    assert!(matches!(err, ContextError::MessageNotFound(_)));
}

// ---------------------------------------------------------------------------
// 10. go_token_estimator_smoke
// ---------------------------------------------------------------------------

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn go_token_estimator_smoke() {
    let est = GoTokenEstimator::default();
    // "hello world" in cl100k is exactly 2 tokens.
    assert_eq!(est.estimate("hello world"), 2);
    assert_eq!(est.estimate(""), 0);
    assert_eq!(est.estimate("x"), 1);
}

