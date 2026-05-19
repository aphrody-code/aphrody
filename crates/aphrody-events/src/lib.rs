// SPDX-License-Identifier: Apache-2.0
//! aphrody-events — in-process async event bus for cross-crate observability.
//!
//! The event bus complements the existing aphrody runtime crates with a
//! lightweight publish / subscribe surface that has no external broker
//! dependency: every [`Event`] is dispatched to in-process [`Subscriber`]s and,
//! optionally, broadcast to bounded `tokio::sync::broadcast` channels for
//! consumers that prefer a raw `Stream` view.
//!
//! ## Why another crate?
//!
//! The workspace already has structured tracing (`tracing`), persistent
//! session audit (`aphrody-session`) and a lifecycle hook dispatcher
//! (`aphrody-hooks`). None of them solve the "type-safe, in-process,
//! multi-consumer message fan-out" problem cleanly:
//!
//! * `tracing` is one-way: spans + events flow to subscribers but consumers
//!   cannot subscribe to logical topics without coupling to a layer.
//! * `aphrody-session` is an append-only archive, not a live pub-sub bus.
//! * `aphrody-hooks` runs *external commands*; this crate runs *in-process
//!   async callbacks*.
//!
//! ## Cohesion with sibling crates
//!
//! * **`aphrody-session`** — re-uses its atomic-write NDJSON pattern
//!   (tmp-file + flush + rename) for [`NdjsonSinkSubscriber`]'s file mode and
//!   borrows the [`thiserror`] error envelope shape (transparent
//!   `Io`/`Json` forwards + domain-specific variants).
//! * **`aphrody-hooks`** — mirrors the `HookDispatcher::run_hooks` matching
//!   loop, except the dispatcher here is purely async and never spawns a
//!   subprocess.
//! * **`aphrody-marketplace`** — mirrors the `HashMap`-backed registry +
//!   `find_by_kind` pattern in [`MemorySinkSubscriber`]'s buffered view.
//!
//! ## Quickstart
//!
//! ```no_run
//! use std::sync::Arc;
//! use aphrody_events::{Event, EventBus, EventLevel, CounterSubscriber};
//!
//! # async fn run() -> Result<(), aphrody_events::EventsError> {
//! let bus = EventBus::new(1024);
//! let counter = Arc::new(CounterSubscriber::new("counter", Vec::<String>::new()));
//! bus.subscribe(counter.clone()).await;
//!
//! let event = Event::new("router.request", EventLevel::Info, "aphrody-router",
//!     serde_json::json!({"provider": "anthropic"}));
//! bus.publish(event).await?;
//!
//! let counts = counter.counts().await;
//! assert_eq!(counts.get("router.request"), Some(&1));
//! # Ok(()) }
//! ```

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::fs as afs;
use tokio::io::AsyncWriteExt;
use tokio::sync::{RwLock, broadcast};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Default broadcast-channel capacity if the caller passes `0` to
/// [`EventBus::new`]. Matches the `tokio::sync::broadcast` recommendation of
/// powers of two for ring-buffer slot allocation.
pub const DEFAULT_CHANNEL_CAPACITY: usize = 1024;

/// All recoverable failures surfaced by the bus.
///
/// Shape mirrors [`aphrody_session::SessionError`] — `Io`/`Json` transparent
/// forwards plus domain-specific named variants for the local invariants
/// the bus is in charge of upholding.
#[derive(Debug, Error)]
pub enum EventsError {
    /// Filesystem error while opening / flushing an [`NdjsonSinkSubscriber`].
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// JSON (de)serialisation failed.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    /// The broadcast channel has been closed (no more receivers).
    #[error("broadcast channel closed")]
    ChannelClosed,
    /// The named subscriber is not currently registered.
    #[error("subscriber not found: {0}")]
    SubscriberNotFound(String),
    /// The bounded broadcast channel is saturated — the slowest receiver
    /// has fallen `capacity` events behind. This is purely informational:
    /// in-process subscribers still receive every event.
    #[error("broadcast channel full (capacity = {capacity})")]
    ChannelFull {
        /// Configured channel capacity at the point the lag was detected.
        capacity: usize,
    },
}

// ---------------------------------------------------------------------------
// EventId
// ---------------------------------------------------------------------------

/// Monotonically increasing 64-bit event identifier, allocated from a
/// process-local atomic counter.
///
/// No external `uuid` dependency: ids are unique within a single process run,
/// which is exactly what an in-process pub-sub bus needs. Cross-process
/// correlation is the caller's responsibility (use [`Event::correlation_id`]
/// for that).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventId(u64);

/// Process-wide atomic source for [`EventId::next`]. Starts at `1` so the
/// value `0` is reserved as a "null" sentinel for downstream code.
static EVENT_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

impl EventId {
    /// Allocate the next monotonic id. Wrapping is treated as unreachable
    /// (2^64 events at 1 GHz would still take ~584 years).
    #[must_use]
    pub fn next() -> Self {
        Self(EVENT_ID_COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Construct an id from a raw `u64`. Used by callers that replay an
    /// existing event log without going through [`Self::next`].
    #[must_use]
    pub fn from_raw(value: u64) -> Self {
        Self(value)
    }

    /// Underlying numeric value.
    #[must_use]
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for EventId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "evt-{:016x}", self.0)
    }
}

// ---------------------------------------------------------------------------
// EventLevel
// ---------------------------------------------------------------------------

/// Severity tier attached to every [`Event`]. Aligned with the syslog /
/// `tracing` ladder so subscribers can apply the same threshold semantics
/// they use elsewhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EventLevel {
    /// Hyper-fine-grained, expected to be filtered out in production.
    Trace,
    /// Developer diagnostic detail.
    Debug,
    /// Normal operational event — the default level.
    Info,
    /// Recoverable anomaly worth surfacing in dashboards.
    Warn,
    /// Failure or other notable abnormal condition.
    Error,
}

impl EventLevel {
    /// Lowercase canonical identifier used in JSON / CLI surfaces.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Trace => "trace",
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

// ---------------------------------------------------------------------------
// Event
// ---------------------------------------------------------------------------

/// The unit of communication on the bus.
///
/// `Event`s are immutable once constructed. Callers populate them via
/// [`Event::new`] (auto-assigned id + timestamp) or [`Event::with_correlation`]
/// (extends `new` with a cross-process correlation token).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    /// Monotonic id assigned at construction time.
    pub id: EventId,
    /// UTC timestamp the event was created at.
    pub ts: DateTime<Utc>,
    /// Dot-delimited topic name, e.g. `router.request` / `tool.invoked`.
    pub topic: String,
    /// Severity tier.
    pub level: EventLevel,
    /// Free-form identifier of the publishing component
    /// (typically a crate or subsystem name).
    pub source: String,
    /// Arbitrary JSON payload — the schema is owned by the topic.
    pub payload: serde_json::Value,
    /// Optional cross-process correlation id (e.g. a request id propagated
    /// from an upstream service). `None` for purely local events.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
}

impl Event {
    /// Construct a new event with the next monotonic [`EventId`] and the
    /// current UTC timestamp.
    #[must_use]
    pub fn new(
        topic: impl Into<String>,
        level: EventLevel,
        source: impl Into<String>,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            id: EventId::next(),
            ts: Utc::now(),
            topic: topic.into(),
            level,
            source: source.into(),
            payload,
            correlation_id: None,
        }
    }

    /// Same as [`Self::new`] but additionally tags the event with a
    /// cross-process correlation id.
    #[must_use]
    pub fn with_correlation(
        topic: impl Into<String>,
        level: EventLevel,
        source: impl Into<String>,
        payload: serde_json::Value,
        correlation_id: impl Into<String>,
    ) -> Self {
        let mut ev = Self::new(topic, level, source, payload);
        ev.correlation_id = Some(correlation_id.into());
        ev
    }
}

// ---------------------------------------------------------------------------
// Topic matching
// ---------------------------------------------------------------------------

/// Lightweight topic matcher used by [`Subscriber`]s.
///
/// Empty filter list ⇒ the subscriber receives **every** event ("subscribe
/// all"). Otherwise each entry is matched as:
///
/// * **trailing `*` wildcard** — `router.*` matches `router.request`,
///   `router.error.timeout`, etc.;
/// * **exact match** — `router.request` only.
#[must_use]
pub fn topic_matches(filters: &[String], topic: &str) -> bool {
    if filters.is_empty() {
        return true;
    }
    for filter in filters {
        if let Some(prefix) = filter.strip_suffix('*') {
            if topic.starts_with(prefix) {
                return true;
            }
        } else if filter == topic {
            return true;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Subscriber trait
// ---------------------------------------------------------------------------

/// Async callback invoked once per matching [`Event`].
///
/// Implementations are stored as `Arc<dyn Subscriber + Send + Sync>` on the
/// [`EventBus`]; they MUST be cheap to dispatch (no blocking IO on the hot
/// path beyond `tokio::fs` calls). Heavy consumers should subscribe to the
/// raw broadcast `Receiver` returned by [`EventBus::recv`] instead.
#[async_trait]
pub trait Subscriber: Send + Sync {
    /// Stable identifier used by [`EventBus::unsubscribe`]. Two subscribers
    /// with the same name are treated as duplicates by the bus.
    fn name(&self) -> &str;

    /// Topic filters this subscriber wants to receive. Empty slice = all.
    fn topics(&self) -> &[String];

    /// Invoked once per matching event. Errors are logged via `tracing` and
    /// do not abort other subscribers on the same publish call.
    async fn on_event(&self, event: &Event);
}

// ---------------------------------------------------------------------------
// EventBus
// ---------------------------------------------------------------------------

/// Async in-process publish / subscribe bus.
///
/// Internally backed by:
///
/// * an `Arc<RwLock<Vec<Arc<dyn Subscriber>>>>` for the registered async
///   callbacks (read-heavy: dispatch grabs a read guard, subscription only
///   takes the write guard for a few microseconds);
/// * a bounded `tokio::sync::broadcast::Sender<Event>` for raw stream
///   consumers obtained via [`Self::recv`].
#[derive(Clone)]
pub struct EventBus {
    subscribers: Arc<RwLock<Vec<Arc<dyn Subscriber + Send + Sync>>>>,
    channel: broadcast::Sender<Event>,
    capacity: usize,
}

impl EventBus {
    /// Construct a new bus with the given broadcast-channel capacity.
    /// `0` means "use [`DEFAULT_CHANNEL_CAPACITY`]".
    #[must_use]
    pub fn new(channel_capacity: usize) -> Self {
        let capacity = if channel_capacity == 0 {
            DEFAULT_CHANNEL_CAPACITY
        } else {
            channel_capacity
        };
        let (channel, _) = broadcast::channel(capacity);
        Self {
            subscribers: Arc::new(RwLock::new(Vec::new())),
            channel,
            capacity,
        }
    }

    /// Configured broadcast capacity.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Current number of registered async subscribers.
    pub async fn subscriber_count(&self) -> usize {
        self.subscribers.read().await.len()
    }

    /// Register a new async subscriber. Duplicate names are tolerated — both
    /// instances receive each matching event in registration order.
    pub async fn subscribe(&self, subscriber: Arc<dyn Subscriber + Send + Sync>) {
        let mut guard = self.subscribers.write().await;
        guard.push(subscriber);
    }

    /// Remove the first subscriber whose [`Subscriber::name`] matches the
    /// supplied identifier. Returns [`EventsError::SubscriberNotFound`] if
    /// no match is found.
    pub async fn unsubscribe(&self, name: &str) -> Result<(), EventsError> {
        let mut guard = self.subscribers.write().await;
        let position = guard.iter().position(|sub| sub.name() == name);
        match position {
            Some(index) => {
                guard.remove(index);
                Ok(())
            }
            None => Err(EventsError::SubscriberNotFound(name.to_string())),
        }
    }

    /// Acquire a raw `broadcast::Receiver` for the underlying channel. Each
    /// call returns a fresh receiver — receivers consume events from the
    /// point in time they were created, not retroactively.
    #[must_use]
    pub fn recv(&self) -> broadcast::Receiver<Event> {
        self.channel.subscribe()
    }

    /// Publish an event.
    ///
    /// Dispatch is two-phase:
    ///
    /// 1. **Async fan-out** to every registered [`Subscriber`] whose
    ///    [`Subscriber::topics`] matches the event topic. Errors from
    ///    individual subscribers are logged but never bubble up.
    /// 2. **Broadcast** the event on the bounded channel for raw stream
    ///    consumers. If no receiver is currently attached the send is
    ///    silently dropped — broadcast channels have no "always buffer"
    ///    mode.
    ///
    /// Returns the number of subscribers that received the event (sum of
    /// async callbacks + raw broadcast receivers at the moment of send).
    pub async fn publish(&self, event: Event) -> Result<usize, EventsError> {
        let mut delivered = 0usize;

        // Phase 1 — async subscribers. Snapshot the Vec under a short read
        // lock so we do not hold the lock across .await of user code.
        let snapshot = {
            let guard = self.subscribers.read().await;
            guard.clone()
        };
        for sub in &snapshot {
            if topic_matches(sub.topics(), &event.topic) {
                sub.on_event(&event).await;
                delivered += 1;
            }
        }

        // Phase 2 — raw broadcast stream. `send` returns `Err` only when no
        // receivers are attached; treat that as "nobody is listening" not
        // as a hard error.
        match self.channel.send(event) {
            Ok(count) => delivered += count,
            Err(_) => {
                tracing::trace!("no broadcast receivers attached");
            }
        }

        Ok(delivered)
    }
}

impl std::fmt::Debug for EventBus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventBus")
            .field("capacity", &self.capacity)
            .finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------------------
// NdjsonSinkSubscriber — append-only NDJSON file
// ---------------------------------------------------------------------------

/// Subscriber that appends every matching event to an NDJSON file.
///
/// The append path follows the same pattern as
/// [`aphrody_session::JsonlFileStore::save_session`]: open in append mode,
/// write one serialized line, flush. The directory is created lazily on
/// first write so callers can construct the sink before the target path
/// exists.
pub struct NdjsonSinkSubscriber {
    name: String,
    topics: Vec<String>,
    path: PathBuf,
    lock: tokio::sync::Mutex<()>,
}

impl NdjsonSinkSubscriber {
    /// Construct a new sink.
    pub fn new(name: impl Into<String>, path: impl Into<PathBuf>, topics: Vec<String>) -> Self {
        Self {
            name: name.into(),
            topics,
            path: path.into(),
            lock: tokio::sync::Mutex::new(()),
        }
    }

    /// Path the sink writes to.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    async fn append(&self, event: &Event) -> Result<(), EventsError> {
        // Serialize first so we never open the file with an invalid event in
        // hand.
        let mut line = serde_json::to_vec(event)?;
        line.push(b'\n');

        let _guard = self.lock.lock().await;
        if let Some(parent) = self.path.parent() {
            if !parent.as_os_str().is_empty() {
                afs::create_dir_all(parent).await?;
            }
        }
        let mut file = afs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .await?;
        file.write_all(&line).await?;
        file.flush().await?;
        Ok(())
    }
}

#[async_trait]
impl Subscriber for NdjsonSinkSubscriber {
    fn name(&self) -> &str {
        &self.name
    }

    fn topics(&self) -> &[String] {
        &self.topics
    }

    async fn on_event(&self, event: &Event) {
        if let Err(err) = self.append(event).await {
            tracing::warn!(error = %err, sink = self.name.as_str(), "ndjson sink append failed");
        }
    }
}

// ---------------------------------------------------------------------------
// MemorySinkSubscriber — bounded ring buffer
// ---------------------------------------------------------------------------

/// Subscriber that retains the last *N* events in memory.
///
/// Useful for tests, REPLs, and the `aphrody term` event-tail surface.
/// When the buffer is full the oldest entry is dropped (FIFO ring buffer).
pub struct MemorySinkSubscriber {
    name: String,
    topics: Vec<String>,
    capacity: usize,
    buffer: RwLock<Vec<Event>>,
}

impl MemorySinkSubscriber {
    /// Construct a new memory sink with the given ring-buffer capacity
    /// (`capacity == 0` is treated as `1`).
    #[must_use]
    pub fn new(name: impl Into<String>, capacity: usize, topics: Vec<String>) -> Self {
        let capacity = capacity.max(1);
        Self {
            name: name.into(),
            topics,
            capacity,
            buffer: RwLock::new(Vec::with_capacity(capacity)),
        }
    }

    /// Current number of buffered events.
    pub async fn len(&self) -> usize {
        self.buffer.read().await.len()
    }

    /// Returns `true` if the sink has not yet received any events.
    pub async fn is_empty(&self) -> bool {
        self.buffer.read().await.is_empty()
    }

    /// Snapshot of the currently buffered events (oldest first).
    pub async fn events(&self) -> Vec<Event> {
        self.buffer.read().await.clone()
    }

    /// Drain the internal buffer, returning everything that was held.
    pub async fn drain(&self) -> Vec<Event> {
        let mut guard = self.buffer.write().await;
        std::mem::take(&mut *guard)
    }
}

#[async_trait]
impl Subscriber for MemorySinkSubscriber {
    fn name(&self) -> &str {
        &self.name
    }

    fn topics(&self) -> &[String] {
        &self.topics
    }

    async fn on_event(&self, event: &Event) {
        let mut guard = self.buffer.write().await;
        if guard.len() == self.capacity {
            guard.remove(0);
        }
        guard.push(event.clone());
    }
}

// ---------------------------------------------------------------------------
// CounterSubscriber — per-topic counters
// ---------------------------------------------------------------------------

/// Subscriber that tallies the number of events received per topic.
///
/// Mirrors the `HashMap`-backed registry pattern from
/// [`aphrody_marketplace::MarketplaceIndex`]: cheap reads, single-writer
/// behind a `RwLock`.
pub struct CounterSubscriber {
    name: String,
    topics: Vec<String>,
    counts: RwLock<HashMap<String, u64>>,
}

impl CounterSubscriber {
    /// Construct a new per-topic counter.
    #[must_use]
    pub fn new(name: impl Into<String>, topics: Vec<String>) -> Self {
        Self {
            name: name.into(),
            topics,
            counts: RwLock::new(HashMap::new()),
        }
    }

    /// Snapshot of the current per-topic counts.
    pub async fn counts(&self) -> HashMap<String, u64> {
        self.counts.read().await.clone()
    }

    /// Total events observed across every topic.
    pub async fn total(&self) -> u64 {
        self.counts.read().await.values().sum()
    }

    /// Reset every counter to zero. Returns the previous totals.
    pub async fn reset(&self) -> HashMap<String, u64> {
        let mut guard = self.counts.write().await;
        std::mem::take(&mut *guard)
    }
}

#[async_trait]
impl Subscriber for CounterSubscriber {
    fn name(&self) -> &str {
        &self.name
    }

    fn topics(&self) -> &[String] {
        &self.topics
    }

    async fn on_event(&self, event: &Event) {
        let mut guard = self.counts.write().await;
        *guard.entry(event.topic.clone()).or_insert(0) += 1;
    }
}

// ---------------------------------------------------------------------------
// Unit tests (internal invariants only — integration tests live in /tests).
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topic_match_wildcard_and_exact() {
        assert!(topic_matches(&[], "anything.goes"));
        assert!(topic_matches(&["router.*".into()], "router.request"));
        assert!(topic_matches(
            &["router.*".into()],
            "router.error.timeout"
        ));
        assert!(topic_matches(&["tool.invoked".into()], "tool.invoked"));
        assert!(!topic_matches(&["router.*".into()], "session.start"));
        assert!(!topic_matches(&["tool.invoked".into()], "tool.invoked.x"));
    }

    #[test]
    fn event_id_display_is_padded_hex() {
        let id = EventId::from_raw(0x42);
        assert_eq!(id.to_string(), "evt-0000000000000042");
    }

    #[test]
    fn event_level_str_roundtrip() {
        assert_eq!(EventLevel::Info.as_str(), "info");
        let j = serde_json::to_string(&EventLevel::Error).unwrap();
        assert_eq!(j, "\"error\"");
    }
}
