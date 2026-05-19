// SPDX-License-Identifier: Apache-2.0
//! Smoke tests for `aphrody-cost` — pricing, budget guard, NDJSON log.

use aphrody_cost::{
    BudgetGuard, CostCalculator, CostError, ModelPricing, NdjsonUsageLog, PricingTable, Provider,
    UsageEvent,
};
use chrono::Utc;
use tempfile::tempdir;

// ---------------------------------------------------------------------------
// PricingTable defaults
// ---------------------------------------------------------------------------

#[test]
fn default_table_has_all_three_providers() {
    let table = PricingTable::default_table();
    assert!(!table.is_empty(), "default table must not be empty");
    assert!(table.len() >= 6, "expected >= 6 rows, got {}", table.len());

    for provider in Provider::all() {
        let count = table.by_provider(provider).count();
        assert!(
            count >= 1,
            "expected >= 1 row for {:?}, got {count}",
            provider
        );
    }

    // Spot-check canonical models.
    assert!(table.lookup(Provider::Anthropic, "claude-opus-4-7").is_some());
    assert!(table.lookup(Provider::Anthropic, "claude-sonnet-4-6").is_some());
    assert!(table.lookup(Provider::Anthropic, "claude-haiku-4-5").is_some());
    assert!(table.lookup(Provider::Gemini, "gemini-2.5-pro").is_some());
    assert!(table.lookup(Provider::Gemini, "gemini-2.5-flash").is_some());
    assert!(table.lookup(Provider::Antigravity, "antigravity-prime").is_some());
}

// ---------------------------------------------------------------------------
// Cost math
// ---------------------------------------------------------------------------

#[test]
fn calculate_anthropic_opus_cost_is_correct() {
    let table = PricingTable::default_table();
    let calc = CostCalculator::new(table);

    // 1M input @ $15 + 1M output @ $75 = $90 (no cache tokens).
    let cost = calc
        .calculate(
            Provider::Anthropic,
            "claude-opus-4-7",
            1_000_000,
            1_000_000,
            0,
            0,
        )
        .expect("calculate succeeds");
    assert!(
        (cost - 90.0).abs() < 1e-6,
        "expected $90.0, got ${cost}"
    );
}

#[test]
fn calculate_with_cache_tokens_discounts() {
    // Cache reads on Anthropic Opus are 10% of input ($1.50 vs $15).
    // Use a tiny custom table so the math is trivially verifiable.
    let table = PricingTable::new(vec![ModelPricing {
        provider: Provider::Anthropic,
        model: "test-model".to_string(),
        input_per_million_tokens: 10.0,
        output_per_million_tokens: 30.0,
        cache_read_per_million: Some(1.0),  // 10% of input
        cache_write_per_million: Some(12.5),
    }]);
    let calc = CostCalculator::new(table);

    // 1M input + 1M output + 1M cache_read + 1M cache_write
    // = 10 + 30 + 1 + 12.5 = $53.5
    let cost = calc
        .calculate(
            Provider::Anthropic,
            "test-model",
            1_000_000,
            1_000_000,
            1_000_000,
            1_000_000,
        )
        .expect("calculate succeeds");
    assert!(
        (cost - 53.5).abs() < 1e-6,
        "expected $53.5, got ${cost}"
    );

    // Sanity: cache_read at $1/M is cheaper than input at $10/M.
    let with_cache = calc
        .calculate(Provider::Anthropic, "test-model", 0, 0, 1_000_000, 0)
        .unwrap();
    let without_cache = calc
        .calculate(Provider::Anthropic, "test-model", 1_000_000, 0, 0, 0)
        .unwrap();
    assert!(
        with_cache < without_cache,
        "cache read ({with_cache}) should be cheaper than input ({without_cache})"
    );
}

#[test]
fn unknown_model_returns_error() {
    let calc = CostCalculator::new(PricingTable::default_table());
    let err = calc
        .calculate(Provider::Anthropic, "claude-nonexistent-9-9", 100, 100, 0, 0)
        .expect_err("unknown model must error");
    match err {
        CostError::UnknownModel { provider, model } => {
            assert_eq!(provider, Provider::Anthropic);
            assert_eq!(model, "claude-nonexistent-9-9");
        }
        other => panic!("expected UnknownModel, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Budget guard
// ---------------------------------------------------------------------------

#[test]
fn budget_guard_blocks_after_limit() {
    let guard = BudgetGuard::new(10.0);
    guard.try_charge(7.5).expect("first charge fits");
    guard.try_charge(2.0).expect("second charge fits");

    let err = guard
        .try_charge(1.0)
        .expect_err("third charge breaches limit");
    match err {
        CostError::BudgetExceeded { limit, attempted } => {
            assert!((limit - 10.0).abs() < 1e-9);
            assert!((attempted - 10.5).abs() < 1e-9, "attempted = {attempted}");
        }
        other => panic!("expected BudgetExceeded, got {other:?}"),
    }

    // Failed charge must NOT increment spent.
    assert!((guard.spent() - 9.5).abs() < 1e-9, "spent = {}", guard.spent());
}

#[test]
fn budget_guard_remaining_decreases() {
    let guard = BudgetGuard::new(100.0);
    assert!((guard.remaining() - 100.0).abs() < 1e-9);
    assert!((guard.limit() - 100.0).abs() < 1e-9);

    guard.try_charge(40.0).unwrap();
    assert!((guard.remaining() - 60.0).abs() < 1e-9);

    guard.try_charge(60.0).unwrap();
    assert!(guard.remaining() < 1e-9, "remaining = {}", guard.remaining());

    // Refund frees budget.
    guard.refund(10.0);
    assert!((guard.remaining() - 10.0).abs() < 1e-9);

    // Reset wipes spend.
    guard.reset();
    assert!((guard.remaining() - 100.0).abs() < 1e-9);
    assert!(guard.spent() < 1e-9);
}

// ---------------------------------------------------------------------------
// NDJSON log
// ---------------------------------------------------------------------------

#[tokio::test]
async fn ndjson_log_roundtrip_appends_and_reads() {
    let dir = tempdir().expect("tempdir");
    let log_path = dir.path().join("usage.ndjson");
    let log = NdjsonUsageLog::new(&log_path);

    // Empty log returns empty Vec.
    let initial = log.read_all().await.expect("read empty");
    assert!(initial.is_empty(), "fresh log should be empty");

    // Append three events.
    let events = vec![
        UsageEvent {
            ts: Utc::now(),
            provider: Provider::Anthropic,
            model: "claude-opus-4-7".to_string(),
            prompt_tokens: 1_000,
            completion_tokens: 2_000,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            cost_usd: 0.165,
            request_id: Some("req-1".to_string()),
            session_id: Some("session-A".to_string()),
        },
        UsageEvent {
            ts: Utc::now(),
            provider: Provider::Gemini,
            model: "gemini-2.5-flash".to_string(),
            prompt_tokens: 10_000,
            completion_tokens: 5_000,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            cost_usd: 0.00225,
            request_id: None,
            session_id: None,
        },
        UsageEvent {
            ts: Utc::now(),
            provider: Provider::Antigravity,
            model: "antigravity-prime".to_string(),
            prompt_tokens: 500,
            completion_tokens: 1_500,
            cache_read_tokens: 100,
            cache_write_tokens: 0,
            cost_usd: 0.0642,
            request_id: Some("req-3".to_string()),
            session_id: None,
        },
    ];
    for ev in &events {
        log.append(ev).await.expect("append");
    }

    // Read back and compare.
    let read = log.read_all().await.expect("read");
    assert_eq!(read.len(), 3, "expected 3 events, got {}", read.len());
    assert_eq!(read[0].provider, Provider::Anthropic);
    assert_eq!(read[0].request_id.as_deref(), Some("req-1"));
    assert_eq!(read[1].provider, Provider::Gemini);
    assert_eq!(read[2].provider, Provider::Antigravity);
    assert_eq!(read[2].cache_read_tokens, 100);
}

#[tokio::test]
async fn record_increments_log_file() {
    let dir = tempdir().expect("tempdir");
    let log_path = dir.path().join("usage.ndjson");
    let calc = CostCalculator::new(PricingTable::default_table())
        .with_log(NdjsonUsageLog::new(&log_path));

    // No events before recording.
    assert_eq!(
        calc.log().expect("log attached").len().await.expect("len"),
        0
    );

    // Record one event — verify event + on-disk count.
    let ev = calc
        .record(
            Provider::Gemini,
            "gemini-2.5-pro",
            1_000,
            500,
            0,
            0,
            Some("req-xyz".to_string()),
            Some("agent-42".to_string()),
        )
        .await
        .expect("record");

    // Cost: 1k input @ $3.50/M + 500 output @ $10.50/M
    // = 0.0035 + 0.00525 = 0.00875
    assert!((ev.cost_usd - 0.00875).abs() < 1e-6, "cost = {}", ev.cost_usd);
    assert_eq!(ev.provider, Provider::Gemini);
    assert_eq!(ev.session_id.as_deref(), Some("agent-42"));

    // File now has exactly one line.
    let n = calc.log().unwrap().len().await.expect("len");
    assert_eq!(n, 1, "expected 1 event on disk, got {n}");

    // Record again — file has two lines.
    calc.record(
        Provider::Anthropic,
        "claude-haiku-4-5",
        100,
        50,
        0,
        0,
        None,
        None,
    )
    .await
    .expect("record");

    let n2 = calc.log().unwrap().len().await.expect("len");
    assert_eq!(n2, 2, "expected 2 events on disk, got {n2}");
}
