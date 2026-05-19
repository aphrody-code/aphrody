// SPDX-License-Identifier: Apache-2.0
//! Smoke tests for the T-5 module-split API (`frame`, `classifier`,
//! `passthrough`).

use aphrody_terminal_json_out::{
    FramePayload, FrameSource, StdoutFrame, classify_line, emit_frame, frame_to_jsonl,
    monotonic_ts_millis, passthrough,
};
use tokio::io::BufReader;

// ── 1. Text frame round-trip via NDJSON ─────────────────────────────────────

#[test]
fn frame_stdout_text_roundtrip() {
    let frame = StdoutFrame::text(FrameSource::App, "hello");
    let mut buf = Vec::new();
    emit_frame(&mut buf, &frame).expect("emit ok");
    let line = std::str::from_utf8(&buf).expect("utf8");
    assert!(line.ends_with('\n'), "expected trailing newline, got {line:?}");
    let parsed: serde_json::Value = serde_json::from_str(line.trim_end()).expect("valid json");
    assert_eq!(parsed["source"], "app");
    assert_eq!(parsed["payload"]["kind"], "text");
    assert_eq!(parsed["payload"]["value"], "hello");

    // Strong-typed round-trip via StdoutFrame.
    let back: StdoutFrame = serde_json::from_str(line.trim_end()).expect("typed round-trip");
    assert_eq!(back.source, FrameSource::App);
    match back.payload {
        FramePayload::Text(s) => assert_eq!(s, "hello"),
        other => panic!("expected Text payload, got {other:?}"),
    }
}

// ── 2. Already-JSON input is preserved (not double-wrapped) ─────────────────

#[tokio::test]
async fn passthrough_app_json_is_preserved() {
    let input = b"{\"event\":\"foo\"}\n";
    let reader = BufReader::new(&input[..]);
    let mut out = Vec::new();
    let n = passthrough(reader, &mut out, FrameSource::App).await.expect("passthrough ok");
    assert_eq!(n, 1, "expected exactly one frame, got {n}");

    let text = String::from_utf8(out).expect("utf8 output");
    let frame: StdoutFrame = serde_json::from_str(text.trim_end()).expect("valid frame");
    match frame.payload {
        FramePayload::Json(v) => {
            assert_eq!(v["event"], "foo", "JSON payload was not preserved verbatim");
            assert!(v.is_object(), "payload must be a JSON object, not a string");
        },
        FramePayload::Text(s) => panic!("app-JSON line was wrapped as Text({s:?})"),
        FramePayload::Bytes(_) => panic!("app-JSON line was wrapped as Bytes"),
    }
}

// ── 3. Mixed input emits one frame per line, each typed correctly ───────────

#[tokio::test]
async fn passthrough_handles_mixed_lines() {
    let input = b"hello\n{\"x\":1}\nworld\n";
    let reader = BufReader::new(&input[..]);
    let mut out = Vec::new();
    let n = passthrough(reader, &mut out, FrameSource::App).await.expect("passthrough ok");
    assert_eq!(n, 3, "expected 3 frames, got {n}");

    let text = String::from_utf8(out).expect("utf8 output");
    let frames: Vec<StdoutFrame> = text
        .lines()
        .map(|l| serde_json::from_str::<StdoutFrame>(l).expect("valid frame"))
        .collect();
    assert_eq!(frames.len(), 3);
    assert!(matches!(frames[0].payload, FramePayload::Text(ref s) if s == "hello"));
    assert!(matches!(frames[1].payload, FramePayload::Json(_)));
    assert!(matches!(frames[2].payload, FramePayload::Text(ref s) if s == "world"));
}

// ── 4. Output is valid NDJSON (one record per `\n`-delimited line) ──────────

#[tokio::test]
async fn frames_are_one_per_line() {
    let input = b"alpha\nbeta\ngamma\n{\"k\":42}\n";
    let reader = BufReader::new(&input[..]);
    let mut out = Vec::new();
    let n = passthrough(reader, &mut out, FrameSource::App).await.expect("passthrough ok");
    assert_eq!(n, 4);

    let text = String::from_utf8(out).expect("utf8 output");
    // Trailing newline expected.
    assert!(text.ends_with('\n'), "NDJSON must end with newline");
    let lines: Vec<&str> = text.trim_end().split('\n').collect();
    assert_eq!(lines.len(), 4, "expected 4 NDJSON records, got {}", lines.len());
    for (i, l) in lines.iter().enumerate() {
        let parsed: serde_json::Value =
            serde_json::from_str(l).unwrap_or_else(|e| panic!("line {i} not valid JSON: {e} — {l:?}"));
        assert!(parsed["ts"].is_number(), "line {i}: ts must be a number");
        assert_eq!(parsed["source"], "app", "line {i}: source mismatch");
        assert!(parsed["payload"]["kind"].is_string(), "line {i}: payload.kind missing");
    }
}

// ── 5. Malformed JSON falls back to text (never errors) ─────────────────────

#[test]
fn classify_rejects_malformed_json() {
    let p = classify_line("{not valid json");
    match p {
        FramePayload::Text(s) => assert_eq!(s, "{not valid json"),
        other => panic!("expected Text fallback, got {other:?}"),
    }
}

// ── 6. Timestamps are monotonic across a tight emission loop ────────────────

#[test]
fn frame_ts_is_monotonic() {
    let mut prev: u64 = 0;
    for i in 0..10 {
        let ts = monotonic_ts_millis();
        assert!(
            ts >= prev,
            "ts not monotonic at iteration {i}: {ts} < {prev}"
        );
        prev = ts;
    }

    // Same property when frames are constructed back-to-back.
    let mut last_ts: u64 = 0;
    for i in 0..10 {
        let f = StdoutFrame::text(FrameSource::App, "x");
        assert!(
            f.ts >= last_ts,
            "frame.ts not monotonic at iteration {i}: {} < {last_ts}",
            f.ts
        );
        last_ts = f.ts;
    }
}

// ── Bonus: frame_to_jsonl matches emit_frame output ─────────────────────────

#[test]
fn frame_to_jsonl_matches_emit_frame() {
    let f = StdoutFrame::with_ts(1_700_000_000_000, FrameSource::Err, FramePayload::Text("e".into()));
    let s = frame_to_jsonl(&f).expect("render ok");
    let mut buf = Vec::new();
    emit_frame(&mut buf, &f).expect("emit ok");
    assert_eq!(s.as_bytes(), buf.as_slice());
}
