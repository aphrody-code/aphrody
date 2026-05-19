// SPDX-License-Identifier: Apache-2.0
//! Integration smoke tests for `aphrody-events`.
//!
//! These tests exercise the public surface end-to-end: subscriber dispatch,
//! topic filtering, NDJSON sink durability, broadcast stream wiring,
//! unsubscribe semantics, ring-buffer eviction and monotonic id allocation.

use std::sync::Arc;

use aphrody_events::{
    CounterSubscriber, Event, EventBus, EventId, EventLevel, EventsError, MemorySinkSubscriber,
    NdjsonSinkSubscriber, Subscriber,
};
use async_trait::async_trait;
use tokio::sync::Mutex;

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

struct CollectingSubscriber {
    name: String,
    topics: Vec<String>,
    received: Mutex<Vec<Event>>,
}

impl CollectingSubscriber {
    fn new(name: &str, topics: Vec<String>) -> Self {
        Self {
            name: name.to_string(),
            topics,
            received: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl Subscriber for CollectingSubscriber {
    fn name(&self) -> &str {
        &self.name
    }
    fn topics(&self) -> &[String] {
        &self.topics
    }
    async fn on_event(&self, event: &Event) {
        self.received.lock().await.push(event.clone());
    }
}

fn evt(topic: &str) -> Event {
    Event::new(
        topic,
        EventLevel::Info,
        "test",
        serde_json::json!({"topic": topic}),
    )
}

// ---------------------------------------------------------------------------
// 1. publish_dispatches_to_all_subscribers
// ---------------------------------------------------------------------------

#[tokio::test]
async fn publish_dispatches_to_all_subscribers() {
    let bus = EventBus::new(0);
    let a = Arc::new(CollectingSubscriber::new("a", Vec::<String>::new()));
    let b = Arc::new(CollectingSubscriber::new("b", Vec::<String>::new()));
    bus.subscribe(a.clone()).await;
    bus.subscribe(b.clone()).await;

    let delivered = bus.publish(evt("session.start")).await.unwrap();
    // 2 async subscribers + 0 broadcast receivers
    assert_eq!(delivered, 2);

    assert_eq!(a.received.lock().await.len(), 1);
    assert_eq!(b.received.lock().await.len(), 1);
}

// ---------------------------------------------------------------------------
// 2. topic_filter_only_matching_receives
// ---------------------------------------------------------------------------

#[tokio::test]
async fn topic_filter_only_matching_receives() {
    let bus = EventBus::new(0);
    let a = Arc::new(CollectingSubscriber::new("a", vec!["a.*".into()]));
    let b = Arc::new(CollectingSubscriber::new("b", vec!["b.*".into()]));
    bus.subscribe(a.clone()).await;
    bus.subscribe(b.clone()).await;

    bus.publish(evt("a.x")).await.unwrap();
    bus.publish(evt("b.y")).await.unwrap();
    bus.publish(evt("c.z")).await.unwrap();

    let a_rx = a.received.lock().await;
    let b_rx = b.received.lock().await;
    assert_eq!(a_rx.len(), 1);
    assert_eq!(a_rx[0].topic, "a.x");
    assert_eq!(b_rx.len(), 1);
    assert_eq!(b_rx[0].topic, "b.y");
}

// ---------------------------------------------------------------------------
// 3. publish_increments_counter_subscriber
// ---------------------------------------------------------------------------

#[tokio::test]
async fn publish_increments_counter_subscriber() {
    let bus = EventBus::new(0);
    let counter = Arc::new(CounterSubscriber::new("counter", Vec::<String>::new()));
    bus.subscribe(counter.clone()).await;

    bus.publish(evt("router.request")).await.unwrap();
    bus.publish(evt("router.request")).await.unwrap();
    bus.publish(evt("router.response")).await.unwrap();

    let counts = counter.counts().await;
    assert_eq!(counts.get("router.request"), Some(&2));
    assert_eq!(counts.get("router.response"), Some(&1));
    assert_eq!(counter.total().await, 3);
}

// ---------------------------------------------------------------------------
// 4. ndjson_sink_appends_event
// ---------------------------------------------------------------------------

#[tokio::test]
async fn ndjson_sink_appends_event() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested").join("events.ndjson");

    let bus = EventBus::new(0);
    let sink = Arc::new(NdjsonSinkSubscriber::new(
        "ndjson",
        path.clone(),
        Vec::<String>::new(),
    ));
    bus.subscribe(sink.clone()).await;

    bus.publish(evt("alpha")).await.unwrap();
    bus.publish(evt("beta")).await.unwrap();
    bus.publish(evt("gamma")).await.unwrap();

    let raw = tokio::fs::read_to_string(&path).await.unwrap();
    let lines: Vec<&str> = raw.lines().collect();
    assert_eq!(lines.len(), 3, "expected 3 ndjson lines, got: {raw}");

    let first: Event = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(first.topic, "alpha");
    let third: Event = serde_json::from_str(lines[2]).unwrap();
    assert_eq!(third.topic, "gamma");
}

// ---------------------------------------------------------------------------
// 5. broadcast_receiver_gets_events
// ---------------------------------------------------------------------------

#[tokio::test]
async fn broadcast_receiver_gets_events() {
    let bus = EventBus::new(16);
    let mut rx = bus.recv();

    let event_count = 4usize;
    let handle = tokio::spawn(async move {
        let mut collected = Vec::new();
        for _ in 0..event_count {
            collected.push(rx.recv().await.unwrap());
        }
        collected
    });

    // Yield once so the spawned task has a chance to attach the receiver
    // before we start publishing.
    tokio::task::yield_now().await;

    for i in 0..event_count {
        let topic = format!("stream.{i}");
        bus.publish(evt(&topic)).await.unwrap();
    }

    let collected = handle.await.unwrap();
    assert_eq!(collected.len(), event_count);
    assert_eq!(collected[0].topic, "stream.0");
    assert_eq!(collected[3].topic, "stream.3");
}

// ---------------------------------------------------------------------------
// 6. unsubscribe_removes_subscriber
// ---------------------------------------------------------------------------

#[tokio::test]
async fn unsubscribe_removes_subscriber() {
    let bus = EventBus::new(0);
    let a = Arc::new(CollectingSubscriber::new("a", Vec::<String>::new()));
    let b = Arc::new(CollectingSubscriber::new("b", Vec::<String>::new()));
    bus.subscribe(a.clone()).await;
    bus.subscribe(b.clone()).await;
    assert_eq!(bus.subscriber_count().await, 2);

    bus.unsubscribe("a").await.unwrap();
    assert_eq!(bus.subscriber_count().await, 1);

    bus.publish(evt("after.unsub")).await.unwrap();
    assert_eq!(a.received.lock().await.len(), 0);
    assert_eq!(b.received.lock().await.len(), 1);

    let err = bus.unsubscribe("missing").await.unwrap_err();
    assert!(matches!(err, EventsError::SubscriberNotFound(name) if name == "missing"));
}

// ---------------------------------------------------------------------------
// 7. memory_sink_caps_at_capacity
// ---------------------------------------------------------------------------

#[tokio::test]
async fn memory_sink_caps_at_capacity() {
    let bus = EventBus::new(0);
    let sink = Arc::new(MemorySinkSubscriber::new(
        "mem",
        5,
        Vec::<String>::new(),
    ));
    bus.subscribe(sink.clone()).await;

    for i in 0..10 {
        let topic = format!("evt.{i}");
        bus.publish(evt(&topic)).await.unwrap();
    }

    assert_eq!(sink.len().await, 5, "ring buffer must cap at capacity");
    let kept = sink.events().await;
    // Oldest 5 dropped; the survivors are 5..10 in insertion order.
    assert_eq!(kept[0].topic, "evt.5");
    assert_eq!(kept[4].topic, "evt.9");
}

// ---------------------------------------------------------------------------
// 8. event_id_monotonic_increment
// ---------------------------------------------------------------------------

#[tokio::test]
async fn event_id_monotonic_increment() {
    let a = EventId::next();
    let b = EventId::next();
    let c = EventId::next();
    assert!(a.as_u64() < b.as_u64());
    assert!(b.as_u64() < c.as_u64());
    // Display format is "evt-<16 hex>".
    let s = a.to_string();
    assert!(s.starts_with("evt-"));
    assert_eq!(s.len(), "evt-".len() + 16);
}
