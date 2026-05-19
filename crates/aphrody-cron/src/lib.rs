// SPDX-License-Identifier: Apache-2.0
//! aphrody-cron — small async-friendly scheduler for the aphrody runtime.
//!
//! Three flavors of [`Schedule`] are supported:
//!
//! * [`Schedule::Every`] — fixed interval (`tokio::time::Duration`-like).
//! * [`Schedule::At`]    — fires once per day at a [`chrono::NaiveTime`].
//! * [`Schedule::Cron`]  — minimal 5-field cron-ish expression
//!                         (`min hour dom month dow`) parsed in-crate.
//!
//! The crate intentionally **does not execute** anything: it owns the
//! "when" (scheduling) and the "what" (an [`ActionDescriptor`] that the
//! caller renders into an actual side effect). This keeps the dependency
//! graph minimal and the unit tests deterministic (uses
//! [`tokio::time::pause`] / [`tokio::time::advance`] in test mode).
//!
//! ## Cohesion with the wider aphrody workspace
//!
//! * **`aphrody-skills-runtime`** stores skill `cron:` triggers verbatim;
//!   this crate is the canonical parser/dispatcher.
//! * **`aphrody-mcp`** uses [`Scheduler::tick`] to periodically refresh
//!   OAuth tokens before they expire (HTTP descriptor).
//! * **`aphrody-design-daemon`** persists jobs via [`JsonFileJobStore`]
//!   alongside its SQLite project store.
//!
//! See `tests/scheduler_smoke.rs` for the executable contract.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration as StdDuration;

use chrono::{DateTime, Datelike, NaiveTime, TimeZone, Timelike, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// All recoverable failures emitted by the scheduler and its job stores.
#[derive(Debug, Error)]
pub enum SchedulerError {
    /// The cron expression could not be parsed.
    #[error("invalid cron expression: {0}")]
    InvalidCron(String),
    /// Filesystem error while loading/saving a [`JsonFileJobStore`].
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
    /// JSON (de)serialization failed.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    /// A job with the same id already exists.
    #[error("job already exists: {0}")]
    DuplicateJob(String),
    /// Job id not found in the scheduler.
    #[error("job not found: {0}")]
    JobNotFound(String),
}

// ---------------------------------------------------------------------------
// Schedule
// ---------------------------------------------------------------------------

/// Time-domain definition for a recurring job.
///
/// `Schedule` is serializable so it can be persisted in JSON, sent over
/// A2A channels, or embedded in a SKILL.md frontmatter block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Schedule {
    /// Fire every `interval_secs` seconds, starting at the moment the
    /// job is added (the first tick after `now() + interval`).
    Every { interval_secs: u64 },
    /// Fire once per UTC day at the given wall-clock time.
    At { hour: u32, minute: u32, second: u32 },
    /// Minimal cron expression (`min hour dom month dow`).
    Cron { expr: String },
}

impl Schedule {
    /// Convenience constructor: fixed interval.
    #[must_use]
    pub fn every(d: StdDuration) -> Self {
        Self::Every { interval_secs: d.as_secs().max(1) }
    }

    /// Convenience constructor: daily time-of-day.
    #[must_use]
    pub fn at_daily(t: NaiveTime) -> Self {
        Self::At { hour: t.hour(), minute: t.minute(), second: t.second() }
    }

    /// Convenience constructor: cron expression with validation.
    pub fn cron(expr: impl Into<String>) -> Result<Self, SchedulerError> {
        let expr = expr.into();
        // Validate by attempting a parse.
        let _ = CronExpr::parse(&expr)?;
        Ok(Self::Cron { expr })
    }

    /// Compute the next fire instant strictly after `from`.
    pub fn next_after(&self, from: DateTime<Utc>) -> Option<DateTime<Utc>> {
        match self {
            Self::Every { interval_secs } => {
                let secs = (*interval_secs).max(1) as i64;
                Some(from + chrono::Duration::seconds(secs))
            }
            Self::At { hour, minute, second } => {
                let t = NaiveTime::from_hms_opt(*hour, *minute, *second)?;
                let today = from.date_naive().and_time(t);
                let today_utc = Utc.from_utc_datetime(&today);
                if today_utc > from {
                    Some(today_utc)
                } else {
                    Some(today_utc + chrono::Duration::days(1))
                }
            }
            Self::Cron { expr } => CronExpr::parse(expr).ok()?.next_after(from),
        }
    }
}

// ---------------------------------------------------------------------------
// Action descriptors (no execution — just a serializable intent)
// ---------------------------------------------------------------------------

/// What should happen when a [`Job`] fires. The scheduler hands these
/// back to the caller; actual side effects live in higher layers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActionDescriptor {
    /// Run an OS command (argv, no shell expansion).
    Command { argv: Vec<String> },
    /// POST JSON body to an HTTP endpoint.
    HttpPost { url: String, body: serde_json::Value },
    /// Emit an in-process event by name (the caller's bus owns it).
    Event { name: String, payload: serde_json::Value },
}

// ---------------------------------------------------------------------------
// Job
// ---------------------------------------------------------------------------

/// A scheduled unit of work.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub schedule: Schedule,
    pub action: ActionDescriptor,
    #[serde(default)]
    pub last_fired: Option<DateTime<Utc>>,
    /// Reference point for the very first fire. Set by the scheduler
    /// when the job is registered (or restored from a [`JobStore`]).
    /// `None` means "use the previous tick reference instead".
    #[serde(default)]
    pub registered_at: Option<DateTime<Utc>>,
}

impl Job {
    /// Builder-style constructor.
    pub fn new(
        id: impl Into<String>,
        schedule: Schedule,
        action: ActionDescriptor,
    ) -> Self {
        Self {
            id: id.into(),
            schedule,
            action,
            last_fired: None,
            registered_at: None,
        }
    }

    /// Compute the next fire time strictly after `now`, taking the last
    /// fire timestamp (or, failing that, the registration timestamp)
    /// into account.
    pub fn next_after(&self, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
        let base = self
            .last_fired
            .or(self.registered_at)
            .unwrap_or(now);
        self.schedule.next_after(base)
    }
}

// ---------------------------------------------------------------------------
// Job store trait + impls
// ---------------------------------------------------------------------------

/// Pluggable persistence layer for the scheduler.
pub trait JobStore: Send + Sync {
    /// Load all known jobs.
    fn load_jobs(&self) -> Result<Vec<Job>, SchedulerError>;
    /// Persist the supplied list of jobs (full replace).
    fn save_jobs(&self, jobs: &[Job]) -> Result<(), SchedulerError>;
}

/// In-memory store useful for tests and ephemeral schedulers.
#[derive(Debug, Default)]
pub struct InMemoryJobStore {
    inner: std::sync::Mutex<Vec<Job>>,
}

impl InMemoryJobStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl JobStore for InMemoryJobStore {
    fn load_jobs(&self) -> Result<Vec<Job>, SchedulerError> {
        Ok(self.inner.lock().expect("store poisoned").clone())
    }
    fn save_jobs(&self, jobs: &[Job]) -> Result<(), SchedulerError> {
        *self.inner.lock().expect("store poisoned") = jobs.to_vec();
        Ok(())
    }
}

/// JSON-file backed store. Atomic write via temp-file + rename.
#[derive(Debug, Clone)]
pub struct JsonFileJobStore {
    path: PathBuf,
}

impl JsonFileJobStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl JobStore for JsonFileJobStore {
    fn load_jobs(&self) -> Result<Vec<Job>, SchedulerError> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let raw = std::fs::read(&self.path)?;
        if raw.is_empty() {
            return Ok(Vec::new());
        }
        Ok(serde_json::from_slice(&raw)?)
    }
    fn save_jobs(&self, jobs: &[Job]) -> Result<(), SchedulerError> {
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let json = serde_json::to_vec_pretty(jobs)?;
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Scheduler
// ---------------------------------------------------------------------------

/// In-memory scheduler. Use [`Scheduler::tick`] periodically (every
/// second is typical) to harvest due jobs.
#[derive(Debug)]
pub struct Scheduler {
    jobs: BTreeMap<String, Job>,
    last_tick: DateTime<Utc>,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Scheduler {
    #[must_use]
    pub fn new() -> Self {
        Self {
            jobs: BTreeMap::new(),
            last_tick: Utc.timestamp_opt(0, 0).single().unwrap_or_else(Utc::now),
        }
    }

    /// Insert a fresh job. Returns [`SchedulerError::DuplicateJob`] when
    /// the id is already taken. Stamps `registered_at` with the current
    /// `last_tick` if the caller did not supply one (so the first fire
    /// is `last_tick + interval`).
    pub fn add_job(&mut self, mut job: Job) -> Result<(), SchedulerError> {
        if self.jobs.contains_key(&job.id) {
            return Err(SchedulerError::DuplicateJob(job.id));
        }
        if job.registered_at.is_none() {
            job.registered_at = Some(self.last_tick);
        }
        tracing::debug!(target: "aphrody_cron", id = %job.id, "job added");
        self.jobs.insert(job.id.clone(), job);
        Ok(())
    }

    /// Remove a job by id.
    pub fn remove_job(&mut self, id: &str) -> Result<Job, SchedulerError> {
        self.jobs
            .remove(id)
            .ok_or_else(|| SchedulerError::JobNotFound(id.to_string()))
    }

    /// Snapshot of currently registered jobs.
    #[must_use]
    pub fn list_jobs(&self) -> Vec<&Job> {
        self.jobs.values().collect()
    }

    /// Next scheduled fire time for `job_id`, relative to "now".
    pub fn next_fire(&self, job_id: &str) -> Option<DateTime<Utc>> {
        let now = Utc::now();
        self.jobs.get(job_id).and_then(|j| j.next_after(now))
    }

    /// Inspect a job by id.
    pub fn get(&self, id: &str) -> Option<&Job> {
        self.jobs.get(id)
    }

    /// Drive the scheduler at the given `now`. Returns the ids of jobs
    /// that became due since the previous call. Updates `last_fired` on
    /// each fired job.
    pub fn tick_at(&mut self, now: DateTime<Utc>) -> Vec<String> {
        self.last_tick = now;
        let mut fired = Vec::new();
        for job in self.jobs.values_mut() {
            // Determine reference point for "next" computation: prefer
            // the last fire time, fall back to the registration stamp,
            // and finally to `now - 1s` (cannot happen via add_job).
            let reference = job
                .last_fired
                .or(job.registered_at)
                .unwrap_or_else(|| now - chrono::Duration::seconds(1));
            if let Some(next) = job.schedule.next_after(reference) {
                if next <= now {
                    job.last_fired = Some(next);
                    fired.push(job.id.clone());
                }
            }
        }
        fired
    }

    /// Convenience wrapper using the system clock.
    pub fn tick(&mut self) -> Vec<String> {
        self.tick_at(Utc::now())
    }

    /// Replace the internal job set with the supplied collection
    /// (typically loaded from a [`JobStore`]). Backfills missing
    /// `registered_at` with the scheduler's current `last_tick`.
    pub fn load_from(&mut self, jobs: Vec<Job>) {
        self.jobs.clear();
        for mut j in jobs {
            if j.registered_at.is_none() {
                j.registered_at = Some(self.last_tick);
            }
            self.jobs.insert(j.id.clone(), j);
        }
    }

    /// Dump the current job set, suitable for [`JobStore::save_jobs`].
    pub fn dump(&self) -> Vec<Job> {
        self.jobs.values().cloned().collect()
    }

    /// Manually set the "last tick" reference (mostly useful in tests).
    pub fn set_last_tick(&mut self, when: DateTime<Utc>) {
        self.last_tick = when;
    }
}

// ---------------------------------------------------------------------------
// Minimal cron expression parser (5 fields)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
struct CronField {
    /// Sorted, deduplicated allowed values.
    values: Vec<u32>,
}

impl CronField {
    fn parse(raw: &str, min: u32, max: u32) -> Result<Self, SchedulerError> {
        if raw == "*" {
            return Ok(Self { values: (min..=max).collect() });
        }
        let mut out = Vec::new();
        for part in raw.split(',') {
            let (range_part, step) = if let Some((r, s)) = part.split_once('/') {
                let step: u32 = s
                    .parse()
                    .map_err(|_| SchedulerError::InvalidCron(raw.to_string()))?;
                if step == 0 {
                    return Err(SchedulerError::InvalidCron(raw.to_string()));
                }
                (r, step)
            } else {
                (part, 1)
            };
            let (lo, hi) = if range_part == "*" {
                (min, max)
            } else if let Some((a, b)) = range_part.split_once('-') {
                let a: u32 = a
                    .parse()
                    .map_err(|_| SchedulerError::InvalidCron(raw.to_string()))?;
                let b: u32 = b
                    .parse()
                    .map_err(|_| SchedulerError::InvalidCron(raw.to_string()))?;
                (a, b)
            } else {
                let v: u32 = range_part
                    .parse()
                    .map_err(|_| SchedulerError::InvalidCron(raw.to_string()))?;
                (v, v)
            };
            if lo < min || hi > max || lo > hi {
                return Err(SchedulerError::InvalidCron(raw.to_string()));
            }
            let mut v = lo;
            while v <= hi {
                out.push(v);
                v = v.saturating_add(step);
                if step == 0 {
                    break;
                }
            }
        }
        out.sort_unstable();
        out.dedup();
        if out.is_empty() {
            return Err(SchedulerError::InvalidCron(raw.to_string()));
        }
        Ok(Self { values: out })
    }

    fn matches(&self, v: u32) -> bool {
        self.values.binary_search(&v).is_ok()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CronExpr {
    minute: CronField,
    hour: CronField,
    dom: CronField,
    month: CronField,
    dow: CronField,
}

impl CronExpr {
    fn parse(raw: &str) -> Result<Self, SchedulerError> {
        let parts: Vec<&str> = raw.split_ascii_whitespace().collect();
        if parts.len() != 5 {
            return Err(SchedulerError::InvalidCron(raw.to_string()));
        }
        Ok(Self {
            minute: CronField::parse(parts[0], 0, 59)?,
            hour: CronField::parse(parts[1], 0, 23)?,
            dom: CronField::parse(parts[2], 1, 31)?,
            month: CronField::parse(parts[3], 1, 12)?,
            dow: CronField::parse(parts[4], 0, 6)?,
        })
    }

    fn next_after(&self, from: DateTime<Utc>) -> Option<DateTime<Utc>> {
        // Walk forward minute by minute. Capped at ~4 years to avoid
        // infinite loops on impossible specs (e.g. Feb 30).
        let mut cur = from
            .with_second(0)?
            .with_nanosecond(0)?
            + chrono::Duration::minutes(1);
        for _ in 0..(60 * 24 * 366 * 4) {
            if self.matches(cur) {
                return Some(cur);
            }
            cur += chrono::Duration::minutes(1);
        }
        None
    }

    fn matches(&self, dt: DateTime<Utc>) -> bool {
        self.minute.matches(dt.minute())
            && self.hour.matches(dt.hour())
            && self.dom.matches(dt.day())
            && self.month.matches(dt.month())
            && self.dow.matches(dt.weekday().num_days_from_sunday())
    }
}

// ---------------------------------------------------------------------------
// Unit tests (additional smoke coverage lives under tests/)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cron_parse_star_5_fields() {
        let e = CronExpr::parse("* * * * *").unwrap();
        assert_eq!(e.minute.values.len(), 60);
        assert_eq!(e.hour.values.len(), 24);
    }

    #[test]
    fn cron_parse_specific_minute() {
        let e = CronExpr::parse("5 * * * *").unwrap();
        assert_eq!(e.minute.values, vec![5]);
    }

    #[test]
    fn cron_step_every_15min() {
        let e = CronExpr::parse("*/15 * * * *").unwrap();
        assert_eq!(e.minute.values, vec![0, 15, 30, 45]);
    }

    #[test]
    fn cron_invalid_field_count() {
        let r = CronExpr::parse("* * *");
        assert!(matches!(r, Err(SchedulerError::InvalidCron(_))));
    }

    #[test]
    fn cron_invalid_range() {
        let r = CronExpr::parse("70 * * * *");
        assert!(matches!(r, Err(SchedulerError::InvalidCron(_))));
    }

    #[test]
    fn schedule_every_serde_roundtrip() {
        let s = Schedule::every(StdDuration::from_secs(60));
        let j = serde_json::to_string(&s).unwrap();
        let back: Schedule = serde_json::from_str(&j).unwrap();
        assert_eq!(s, back);
    }

    #[test]
    fn action_descriptor_serde_command() {
        let a = ActionDescriptor::Command {
            argv: vec!["aphrody".into(), "doctor".into()],
        };
        let j = serde_json::to_string(&a).unwrap();
        assert!(j.contains("command"));
        let back: ActionDescriptor = serde_json::from_str(&j).unwrap();
        assert_eq!(a, back);
    }
}
