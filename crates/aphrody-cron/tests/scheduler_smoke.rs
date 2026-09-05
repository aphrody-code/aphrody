// SPDX-License-Identifier: Apache-2.0
//! End-to-end smoke tests for the aphrody-cron scheduler.
//!
//! These tests are intentionally deterministic: they drive the scheduler
//! through [`Scheduler::tick_at`] with explicit [`DateTime<Utc>`] values
//! rather than wall-clock time, so they're stable on CI and on slow
//! Windows runners.
//!
//! The `tokio` interval-based test demonstrates that [`Schedule::Every`]
//! cooperates with `tokio::time::pause`/`advance` for callers that wire
//! the scheduler into an actual runtime.

use std::time::Duration;

use aphrody_cron::{
    ActionDescriptor, InMemoryJobStore, Job, JobStore, JsonFileJobStore,
    Schedule, Scheduler, SchedulerError,
};
use chrono::{Duration as ChronoDuration, NaiveTime, TimeZone, Utc};

fn tick_action() -> ActionDescriptor {
    ActionDescriptor::Event {
        name: "tick".into(),
        payload: serde_json::json!({"src": "aphrody-cron"}),
    }
}

#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn every_duration_fires_after_interval() {
    let mut sched = Scheduler::new();
    let t0 = Utc.with_ymd_and_hms(2026, 5, 19, 12, 0, 0).unwrap();
    sched.set_last_tick(t0);

    let job = Job::new(
        "every-30s",
        Schedule::every(Duration::from_secs(30)),
        tick_action(),
    );
    sched.add_job(job).unwrap();

    // Before the interval elapses: no firing.
    let fired = sched.tick_at(t0 + ChronoDuration::seconds(10));
    assert!(fired.is_empty(), "should not fire after 10s for 30s interval");

    // After 30s: one firing.
    let fired = sched.tick_at(t0 + ChronoDuration::seconds(35));
    assert_eq!(fired, vec!["every-30s".to_string()]);

    // Another 30s window — second firing.
    let fired = sched.tick_at(t0 + ChronoDuration::seconds(70));
    assert_eq!(fired, vec!["every-30s".to_string()]);
}

#[test]
fn at_daily_time_fires_once_per_day() {
    let mut sched = Scheduler::new();
    // 11:59:00 today.
    let t_before = Utc.with_ymd_and_hms(2026, 5, 19, 11, 59, 0).unwrap();
    sched.set_last_tick(t_before);

    let job = Job::new(
        "noon-bell",
        Schedule::at_daily(NaiveTime::from_hms_opt(12, 0, 0).unwrap()),
        tick_action(),
    );
    sched.add_job(job).unwrap();

    // 12:00:30 → due (>= 12:00:00).
    let fired = sched.tick_at(Utc.with_ymd_and_hms(2026, 5, 19, 12, 0, 30).unwrap());
    assert_eq!(fired, vec!["noon-bell".to_string()]);

    // 14:00 same day → not due again.
    let fired = sched.tick_at(Utc.with_ymd_and_hms(2026, 5, 19, 14, 0, 0).unwrap());
    assert!(fired.is_empty(), "daily job already fired today");

    // 12:00:01 next day → due.
    let fired = sched.tick_at(Utc.with_ymd_and_hms(2026, 5, 20, 12, 0, 1).unwrap());
    assert_eq!(fired, vec!["noon-bell".to_string()]);
}

#[test]
fn remove_job_stops_firing() {
    let mut sched = Scheduler::new();
    let t0 = Utc.with_ymd_and_hms(2026, 5, 19, 12, 0, 0).unwrap();
    sched.set_last_tick(t0);

    sched
        .add_job(Job::new(
            "ping",
            Schedule::every(Duration::from_secs(10)),
            tick_action(),
        ))
        .unwrap();

    let fired = sched.tick_at(t0 + ChronoDuration::seconds(15));
    assert_eq!(fired, vec!["ping".to_string()]);

    let removed = sched.remove_job("ping").unwrap();
    assert_eq!(removed.id, "ping");

    let fired = sched.tick_at(t0 + ChronoDuration::seconds(45));
    assert!(fired.is_empty(), "no jobs registered → no firings");

    // Removing twice yields a structured error.
    let err = sched.remove_job("ping").unwrap_err();
    assert!(matches!(err, SchedulerError::JobNotFound(_)));
}

#[test]
fn json_store_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("jobs.json");
    let store = JsonFileJobStore::new(&path);

    // Empty file → empty load.
    assert!(store.load_jobs().unwrap().is_empty());

    let jobs = vec![
        Job::new(
            "cmd",
            Schedule::every(Duration::from_secs(5)),
            ActionDescriptor::Command { argv: vec!["echo".into(), "hi".into()] },
        ),
        Job::new(
            "http",
            Schedule::cron("*/15 * * * *").unwrap(),
            ActionDescriptor::HttpPost {
                url: "https://example.invalid/hook".into(),
                body: serde_json::json!({"k": "v"}),
            },
        ),
    ];

    store.save_jobs(&jobs).unwrap();
    let back = store.load_jobs().unwrap();
    assert_eq!(back, jobs);

    // Drive through scheduler too.
    let mut sched = Scheduler::new();
    sched.load_from(back);
    assert_eq!(sched.list_jobs().len(), 2);
    assert!(sched.get("cmd").is_some());
    assert!(sched.get("http").is_some());

    // In-memory store parity.
    let mem = InMemoryJobStore::new();
    mem.save_jobs(&sched.dump()).unwrap();
    assert_eq!(mem.load_jobs().unwrap().len(), 2);
}

#[test]
fn tick_returns_multiple_due_jobs() {
    let mut sched = Scheduler::new();
    let t0 = Utc.with_ymd_and_hms(2026, 5, 19, 12, 0, 0).unwrap();
    sched.set_last_tick(t0);

    sched
        .add_job(Job::new(
            "a",
            Schedule::every(Duration::from_secs(10)),
            tick_action(),
        ))
        .unwrap();
    sched
        .add_job(Job::new(
            "b",
            Schedule::every(Duration::from_secs(20)),
            tick_action(),
        ))
        .unwrap();
    sched
        .add_job(Job::new(
            "c",
            Schedule::every(Duration::from_secs(60)),
            tick_action(),
        ))
        .unwrap();

    // After 25s: a (10s) + b (20s) are due, c (60s) is not.
    let mut fired = sched.tick_at(t0 + ChronoDuration::seconds(25));
    fired.sort();
    assert_eq!(fired, vec!["a".to_string(), "b".to_string()]);
}

#[test]
fn invalid_cron_string_rejected() {
    // 5-field requirement violated.
    let r = Schedule::cron("not a cron");
    assert!(matches!(r, Err(SchedulerError::InvalidCron(_))));

    // Out-of-range minute.
    let r = Schedule::cron("99 * * * *");
    assert!(matches!(r, Err(SchedulerError::InvalidCron(_))));

    // Step of zero.
    let r = Schedule::cron("*/0 * * * *");
    assert!(matches!(r, Err(SchedulerError::InvalidCron(_))));

    // Valid 5-field cron passes.
    let s = Schedule::cron("0 12 * * *").unwrap();
    let next = s
        .next_after(Utc.with_ymd_and_hms(2026, 5, 19, 10, 0, 0).unwrap())
        .unwrap();
    assert_eq!(next.hour(), 12);
    assert_eq!(next.minute(), 0);
}

#[test]
fn duplicate_job_id_rejected() {
    let mut sched = Scheduler::new();
    sched
        .add_job(Job::new(
            "dup",
            Schedule::every(Duration::from_secs(60)),
            tick_action(),
        ))
        .unwrap();
    let err = sched
        .add_job(Job::new(
            "dup",
            Schedule::every(Duration::from_secs(120)),
            tick_action(),
        ))
        .unwrap_err();
    assert!(matches!(err, SchedulerError::DuplicateJob(_)));
}

// Bring trait into scope for `.hour()` / `.minute()` in the test above.
use chrono::Timelike;
