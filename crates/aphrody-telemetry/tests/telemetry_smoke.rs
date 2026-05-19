// SPDX-License-Identifier: Apache-2.0
//! Integration smoke coverage for aphrody-telemetry.

use std::collections::HashMap;
use std::sync::Arc;
use std::thread;

use aphrody_telemetry::{
    NdjsonSpanExporter, Span, SpanId, SpanKind, SpanStatus, Telemetry, TelemetryError,
};
use tempfile::tempdir;

#[test]
fn span_id_monotonic() {
    // Successive ids must be strictly increasing — the underlying counter is
    // bumped with `fetch_add(1)` under `Relaxed` ordering.
    let a = SpanId::generate();
    let b = SpanId::generate();
    let c = SpanId::generate();
    assert!(a.as_u64() < b.as_u64(), "expected a<b, got {a} vs {b}");
    assert!(b.as_u64() < c.as_u64(), "expected b<c, got {b} vs {c}");
}

#[test]
fn start_span_records_in_spans_vec() {
    let t = Arc::new(Telemetry::new(16));
    let h = t.start_span("root", SpanKind::Server, None);
    // Span is in-flight: present in the buffer but `end` is None.
    let spans = t.dump_spans();
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].name, "root");
    assert_eq!(spans[0].kind, SpanKind::Server);
    assert!(spans[0].end.is_none());
    assert!(spans[0].parent.is_none());
    drop(h);
    // After drop: status finalized to Ok and `end` stamped.
    let after = t.dump_spans();
    assert_eq!(after.len(), 1);
    assert!(after[0].end.is_some());
    assert_eq!(after[0].status, SpanStatus::Ok);
}

#[test]
fn span_handle_drop_finalizes_span() {
    let t = Arc::new(Telemetry::new(8));
    let id;
    {
        let h = t.start_span("scoped", SpanKind::Internal, None);
        id = h.id();
        // Out-of-scope here triggers Drop -> finalize.
    }
    let snap: Vec<Span> = t
        .dump_spans()
        .into_iter()
        .filter(|s| s.id == id)
        .collect();
    assert_eq!(snap.len(), 1);
    assert!(snap[0].end.is_some(), "Drop must close the span");
    assert_eq!(snap[0].status, SpanStatus::Ok);
}

#[test]
fn counter_increments_thread_safely() {
    let t = Arc::new(Telemetry::new(8));
    let c = t.counter("req.count");
    let mut handles = Vec::new();
    for _ in 0..10 {
        let c = Arc::clone(&c);
        handles.push(thread::spawn(move || {
            for _ in 0..100 {
                c.inc();
            }
        }));
    }
    for h in handles {
        h.join().expect("thread join");
    }
    assert_eq!(c.value(), 1_000, "expected 10 threads * 100 increments");
}

#[test]
fn histogram_observe_buckets_correctly() {
    let t = Telemetry::new(8);
    // Buckets define the upper bounds 1, 5, 10. Observations:
    //   0.5  -> <=1, <=5, <=10  (lands in all three)
    //   3.0  -> <=5, <=10
    //   7.0  -> <=10
    //   15.0 -> overflow (no bucket)
    let h = t.histogram("lat.ms", vec![1.0, 5.0, 10.0]).unwrap();
    h.observe(0.5);
    h.observe(3.0);
    h.observe(7.0);
    h.observe(15.0);
    let snap = h.snapshot();
    assert_eq!(snap.bucket_counts, vec![1, 2, 3]);
    assert_eq!(snap.count, 4);
}

#[test]
fn histogram_snapshot_returns_sum_and_count() {
    let t = Telemetry::new(8);
    let h = t.histogram("payload.kb", vec![1.0, 10.0, 100.0]).unwrap();
    h.observe(2.5);
    h.observe(7.5);
    h.observe(42.0);
    let snap = h.snapshot();
    assert_eq!(snap.count, 3);
    // Sum is stored in microseconds fixed-point inside the histogram; the
    // snapshot divides back out — tolerate a tiny rounding wobble.
    let want: f64 = 2.5 + 7.5 + 42.0;
    assert!((snap.sum - want).abs() < 1e-3, "sum={} want={want}", snap.sum);
}

#[test]
fn parent_span_id_linked() {
    let t = Arc::new(Telemetry::new(8));
    let parent = t.start_span("parent", SpanKind::Internal, None);
    let parent_id = parent.id();
    let child = t.start_span("child", SpanKind::Client, Some(parent_id));
    let child_id = child.id();
    drop(child);
    drop(parent);

    let spans = t.dump_spans();
    let child_span = spans
        .iter()
        .find(|s| s.id == child_id)
        .expect("child span recorded");
    let parent_span = spans
        .iter()
        .find(|s| s.id == parent_id)
        .expect("parent span recorded");
    assert_eq!(child_span.parent, Some(parent_id));
    assert_eq!(
        child_span.trace_id, parent_span.trace_id,
        "child must inherit parent trace_id",
    );
}

#[tokio::test]
async fn ndjson_exporter_writes_spans_to_file() -> Result<(), TelemetryError> {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("spans.ndjson");
    let t = Arc::new(Telemetry::new(8));
    for name in ["a", "b", "c"] {
        let h = t.start_span(name, SpanKind::Internal, None);
        h.end();
    }
    let spans = t.dump_spans();
    assert_eq!(spans.len(), 3);

    let exporter = NdjsonSpanExporter::new(&path);
    exporter.export(&spans).await?;

    // Read the file back and verify exactly 3 non-empty lines, each decoding
    // to a Span with the matching name.
    let bytes = tokio::fs::read(&path).await?;
    let raw = String::from_utf8(bytes).expect("utf8 ndjson");
    let lines: Vec<&str> = raw.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines.len(), 3, "expected 3 NDJSON lines, got {}", lines.len());
    let names: Vec<String> = lines
        .iter()
        .map(|l| {
            let s: Span = serde_json::from_str(l).expect("span round-trip");
            s.name
        })
        .collect();
    assert_eq!(names, vec!["a".to_string(), "b".into(), "c".into()]);
    Ok(())
}

#[test]
fn set_attribute_persisted_on_span() {
    let t = Arc::new(Telemetry::new(8));
    let h = t.start_span("rpc.call", SpanKind::Client, None);
    let id = h.id();
    h.set_attribute("rpc.system", "grpc");
    h.set_attribute("rpc.code", 0_i64);
    let mut events = HashMap::new();
    events.insert(
        "retry.attempt".to_string(),
        serde_json::Value::from(1_i64),
    );
    h.add_event("retry.scheduled", events);
    h.set_status(SpanStatus::Error {
        message: "deadline".into(),
    });
    h.end();

    let span = t
        .dump_spans()
        .into_iter()
        .find(|s| s.id == id)
        .expect("span present");
    assert_eq!(
        span.attributes.get("rpc.system"),
        Some(&serde_json::Value::from("grpc")),
    );
    assert_eq!(
        span.attributes.get("rpc.code"),
        Some(&serde_json::Value::from(0_i64)),
    );
    assert_eq!(span.events.len(), 1);
    assert_eq!(span.events[0].name, "retry.scheduled");
    assert!(matches!(span.status, SpanStatus::Error { .. }));
}

#[test]
fn dump_metrics_sorted_by_name() {
    let t = Telemetry::new(8);
    t.counter("zzz").inc();
    t.counter("aaa").add(5);
    let _ = t.histogram("xx.latency", vec![1.0, 10.0]).unwrap();
    let _ = t.histogram("aa.latency", vec![1.0, 10.0]).unwrap();
    let snap = t.dump_metrics();
    assert_eq!(snap.counters.len(), 2);
    assert_eq!(snap.counters[0].name, "aaa");
    assert_eq!(snap.counters[0].value, 5);
    assert_eq!(snap.counters[1].name, "zzz");
    assert_eq!(snap.histograms[0].name, "aa.latency");
    assert_eq!(snap.histograms[1].name, "xx.latency");
}
