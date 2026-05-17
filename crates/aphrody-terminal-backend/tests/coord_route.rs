// SPDX-License-Identifier: Apache-2.0
//! Integration tests for the `/coord` route — the A2A coord JSONL proxy
//! exposed by `aphrody-terminal-backend::coord`.
//!
//! Covers:
//!   1. snapshot mode reads all lines (`coord_proxy_serves_jsonl_snapshot`)
//!   2. missing file → empty body, no error (`coord_proxy_missing_file_returns_empty`)
//!   3. SSE envelope is well-formed (`coord_proxy_sse_format`)
//!   4. `tail -f` emits lines appended after subscription
//!      (`coord_proxy_tail_stream_emits_new_lines`)

use std::time::Duration;

use aphrody_terminal_backend::coord::{
    CoordProxy, CoordResponseKind, NDJSON_CONTENT_TYPE, SSE_CONTENT_TYPE,
};
use futures_util::StreamExt;
use tempfile::tempdir;
use tokio::io::AsyncWriteExt;

/// Collect a stream into a `Vec<String>`, panicking on the first error.
async fn collect_ok(
    mut stream: impl futures_util::Stream<
        Item = Result<String, aphrody_terminal_backend::coord::CoordError>,
    > + Unpin,
) -> Vec<String> {
    let mut out = Vec::new();
    while let Some(item) = stream.next().await {
        out.push(item.expect("stream item should be Ok"));
    }
    out
}

#[tokio::test]
async fn coord_proxy_serves_jsonl_snapshot() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("inbox-from-aphrody.jsonl");
    let body = "{\"id\":1}\n{\"id\":2}\n{\"id\":3}\n";
    tokio::fs::write(&path, body).await.expect("write fixture");

    let proxy = CoordProxy::new(path.clone());

    // handle_get_coord with no Accept header → NDJSON snapshot.
    let resp = proxy
        .handle_get_coord(None)
        .await
        .expect("handle_get_coord");
    assert_eq!(resp.status, 200);
    assert_eq!(resp.content_type, NDJSON_CONTENT_TYPE);

    let chunks = collect_ok(resp.body).await;
    assert_eq!(chunks.len(), 3, "expected 3 lines, got {chunks:?}");
    assert_eq!(chunks[0], "{\"id\":1}\n");
    assert_eq!(chunks[1], "{\"id\":2}\n");
    assert_eq!(chunks[2], "{\"id\":3}\n");
}

#[tokio::test]
async fn coord_proxy_missing_file_returns_empty() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("does-not-exist.jsonl");
    assert!(!path.exists(), "fixture must not exist");

    let proxy = CoordProxy::new(path);

    // Snapshot path.
    let resp = proxy
        .handle_get_coord(None)
        .await
        .expect("missing file must NOT error");
    assert_eq!(resp.status, 200);
    let chunks = collect_ok(resp.body).await;
    assert!(chunks.is_empty(), "expected empty body, got {chunks:?}");

    // SSE path.
    let resp = proxy
        .handle_get_coord(Some("text/event-stream"))
        .await
        .expect("missing file (SSE) must NOT error");
    assert_eq!(resp.status, 200);
    assert_eq!(resp.content_type, SSE_CONTENT_TYPE);
    let chunks = collect_ok(resp.body).await;
    assert!(
        chunks.is_empty(),
        "expected empty SSE body, got {chunks:?}"
    );
}

#[tokio::test]
async fn coord_proxy_sse_format() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("inbox-from-aphrody.jsonl");
    tokio::fs::write(&path, "{\"a\":1}\n{\"b\":2}\n")
        .await
        .expect("write fixture");

    let proxy = CoordProxy::new(path);
    let resp = proxy
        .handle_get_coord(Some("text/event-stream"))
        .await
        .expect("handle_get_coord SSE");
    assert_eq!(resp.status, 200);
    assert_eq!(resp.content_type, SSE_CONTENT_TYPE);

    let chunks = collect_ok(resp.body).await;
    assert_eq!(chunks.len(), 2, "expected 2 SSE events, got {chunks:?}");
    assert_eq!(chunks[0], "data: {\"a\":1}\n\n");
    assert_eq!(chunks[1], "data: {\"b\":2}\n\n");

    // Each chunk must end with the SSE event terminator `\n\n`.
    for chunk in &chunks {
        assert!(
            chunk.ends_with("\n\n"),
            "SSE chunk missing terminator: {chunk:?}"
        );
        assert!(
            chunk.starts_with("data: "),
            "SSE chunk missing `data: ` prefix: {chunk:?}"
        );
    }

    // Sanity-check the kind classifier independently.
    assert_eq!(
        CoordResponseKind::from_accept(Some("text/event-stream")),
        CoordResponseKind::Sse,
    );
}

#[tokio::test]
async fn coord_proxy_tail_stream_emits_new_lines() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("inbox-from-aphrody.jsonl");

    // Pre-seed the file with one line that should NOT be replayed by
    // tail (tail starts at current EOF, mirroring `tail -f`).
    tokio::fs::write(&path, "{\"old\":true}\n")
        .await
        .expect("seed file");

    let proxy = CoordProxy::new(path.clone()).with_tail_interval(Duration::from_millis(25));
    let mut tail = proxy.tail_stream().await;

    // Give the tail task one polling cycle to seed its offset to EOF.
    tokio::time::sleep(Duration::from_millis(80)).await;

    // Append a fresh line.
    {
        let mut f = tokio::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .await
            .expect("open append");
        f.write_all(b"{\"new\":1}\n").await.expect("append");
        f.flush().await.expect("flush");
    }

    // Wait for the new line, bounded by a 2 s deadline.
    let item = tokio::time::timeout(Duration::from_secs(2), tail.next())
        .await
        .expect("tail stream should emit within 2s")
        .expect("tail stream should not end");
    let line = item.expect("tail item should be Ok");

    assert_eq!(
        line, "{\"new\":1}\n",
        "tail should emit only the freshly appended line, got {line:?}"
    );

    // Append a second line and verify it is also picked up.
    {
        let mut f = tokio::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .await
            .expect("open append #2");
        f.write_all(b"{\"new\":2}\n").await.expect("append #2");
        f.flush().await.expect("flush #2");
    }

    let item = tokio::time::timeout(Duration::from_secs(2), tail.next())
        .await
        .expect("tail stream should emit 2nd line within 2s")
        .expect("tail stream should not end");
    let line = item.expect("tail item should be Ok");
    assert_eq!(line, "{\"new\":2}\n");
}

#[tokio::test]
async fn coord_proxy_tail_stream_waits_for_missing_file() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("not-yet.jsonl");
    assert!(!path.exists());

    let proxy = CoordProxy::new(path.clone()).with_tail_interval(Duration::from_millis(25));
    let mut tail = proxy.tail_stream().await;

    // Create the file and append a line after a short delay.
    tokio::time::sleep(Duration::from_millis(50)).await;
    tokio::fs::write(&path, "").await.expect("create empty");
    tokio::time::sleep(Duration::from_millis(80)).await;
    {
        let mut f = tokio::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .await
            .expect("open append");
        f.write_all(b"{\"late\":true}\n").await.expect("append");
        f.flush().await.expect("flush");
    }

    let item = tokio::time::timeout(Duration::from_secs(2), tail.next())
        .await
        .expect("tail should emit eventually")
        .expect("tail should not end")
        .expect("item should be Ok");
    assert_eq!(item, "{\"late\":true}\n");
}
