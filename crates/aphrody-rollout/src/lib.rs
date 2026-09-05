// SPDX-License-Identifier: Apache-2.0
//! `aphrody-rollout` — resilient agent-session recorder + resume.
//!
//! A [`RolloutRecorder`] appends timestamped items to a JSONL file through an
//! asynchronous actor: callers enqueue work over an mpsc channel and a single
//! background task owns the [`tokio::fs::File`] + [`BufWriter`], serializing one
//! [`RolloutLine`] per physical line. This keeps file I/O off the caller's path
//! while guaranteeing write ordering. A prior session is recovered with
//! [`resume`], which replays the JSONL line-by-line and tolerates corrupted
//! lines by logging and skipping them.
//!
//! Implementation landed by the Phase 1 ENGINE build (docs/VISION.md, GC-7).
//!
//! This crate is host-only: it relies on `tokio` filesystem primitives and is
//! intended for native (Linux / Windows / macOS) targets, not wasm.
//!
//! ```no_run
//! # use serde_json::json;
//! # async fn demo() -> Result<(), aphrody_rollout::RolloutError> {
//! let rec = aphrody_rollout::RolloutRecorder::new("/tmp/sessions", None).await?;
//! rec.record(json!({ "type": "user", "text": "hello" }));
//! rec.flush().await?;
//! let path = rec.path().to_path_buf();
//! rec.shutdown().await?;
//!
//! let lines = aphrody_rollout::resume(&path).await?;
//! assert_eq!(lines.len(), 1);
//! # Ok(())
//! # }
//! ```

use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tracing::warn;

/// Errors produced by the rollout recorder and resume path.
#[derive(Debug, thiserror::Error)]
pub enum RolloutError {
    /// An underlying filesystem / I/O operation failed.
    #[error("rollout I/O error: {0}")]
    Io(#[from] io::Error),
    /// A value could not be encoded to JSON.
    #[error("rollout JSON encode error: {0}")]
    Encode(#[from] serde_json::Error),
    /// The background writer task is gone, so the command could not be served.
    #[error("rollout writer channel closed: {0}")]
    ChannelClosed(&'static str),
}

/// One physical line of a rollout JSONL file.
///
/// Serialized as a single JSON object `{"ts": "...", "item": ...}` where `ts`
/// is an RFC3339 UTC timestamp and `item` is the arbitrary payload recorded by
/// the caller.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RolloutLine {
    /// RFC3339 UTC timestamp (millisecond precision) of when the line was enqueued.
    pub ts: String,
    /// Arbitrary JSON payload supplied to [`RolloutRecorder::record`].
    pub item: Value,
}

/// Commands sent to the background writer task.
///
/// Kept private: callers drive the writer through [`RolloutRecorder`] methods.
enum RolloutCmd {
    /// Append a line to the buffered writer.
    Append(RolloutLine),
    /// Flush + fsync everything written so far, then acknowledge.
    Flush(oneshot::Sender<io::Result<()>>),
    /// Flush, close the file, then acknowledge before the task exits.
    Shutdown(oneshot::Sender<()>),
}

/// Resilient JSONL session recorder backed by a background writer task.
///
/// Cloning is cheap: clones share the same mpsc [`mpsc::Sender`] and therefore
/// the same underlying file. The first clone to call [`shutdown`](Self::shutdown)
/// consumes the [`JoinHandle`]; surviving clones can still [`record`](Self::record)
/// and [`flush`](Self::flush) until the channel closes.
#[derive(Clone)]
pub struct RolloutRecorder {
    tx: mpsc::Sender<RolloutCmd>,
    path: PathBuf,
    session_id: String,
    /// Owned by clones so the writer task is not detached prematurely.
    /// Wrapped in `Arc<Mutex<..>>` so a single clone can `join` it on shutdown.
    handle: std::sync::Arc<std::sync::Mutex<Option<JoinHandle<()>>>>,
}

impl RolloutRecorder {
    /// Create a new recorder writing to `dir/rollout-{ts}-{uuid}.jsonl`.
    ///
    /// The directory is created if missing. `session_id` defaults to a fresh
    /// UUID v4 when `None`. The file is opened in append mode and a background
    /// writer task is spawned immediately.
    pub async fn new(
        dir: impl AsRef<Path>,
        session_id: Option<String>,
    ) -> Result<Self, RolloutError> {
        let dir = dir.as_ref();
        tokio::fs::create_dir_all(dir).await?;

        let session_id = session_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        // Filename-safe timestamp: colons replaced by dashes for portability.
        let file_ts = filename_timestamp(SystemTime::now());
        let filename = format!("rollout-{file_ts}-{session_id}.jsonl");
        let path = dir.join(filename);

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await?;

        let (tx, rx) = mpsc::channel::<RolloutCmd>(256);
        let handle = tokio::spawn(writer_task(file, rx));

        Ok(Self {
            tx,
            path,
            session_id,
            handle: std::sync::Arc::new(std::sync::Mutex::new(Some(handle))),
        })
    }

    /// Path of the JSONL file this recorder writes to.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Session id embedded in the filename.
    #[must_use]
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Enqueue an item for writing, stamped with the current UTC time.
    ///
    /// Non-blocking: if the bounded channel is momentarily full, the line is
    /// dropped after logging rather than blocking the caller. To guarantee a
    /// write reached disk, call [`flush`](Self::flush) afterward.
    pub fn record(&self, item: Value) {
        let line = RolloutLine {
            ts: rfc3339_timestamp(SystemTime::now()),
            item,
        };
        if let Err(err) = self.tx.try_send(RolloutCmd::Append(line)) {
            match err {
                mpsc::error::TrySendError::Full(_) => {
                    warn!("rollout channel full; line dropped (call flush to backpressure)");
                }
                mpsc::error::TrySendError::Closed(_) => {
                    warn!("rollout writer task gone; line dropped");
                }
            }
        }
    }

    /// Wait until everything enqueued so far is written and fsync'd.
    pub async fn flush(&self) -> Result<(), RolloutError> {
        let (ack_tx, ack_rx) = oneshot::channel();
        self.tx
            .send(RolloutCmd::Flush(ack_tx))
            .await
            .map_err(|_| RolloutError::ChannelClosed("flush"))?;
        ack_rx
            .await
            .map_err(|_| RolloutError::ChannelClosed("flush ack"))??;
        Ok(())
    }

    /// Flush all pending writes and shut the writer task down cleanly.
    ///
    /// Consumes the recorder. The owned [`JoinHandle`] (if this clone still
    /// holds it) is awaited so the task has fully exited on return.
    pub async fn shutdown(self) -> Result<(), RolloutError> {
        let (ack_tx, ack_rx) = oneshot::channel();
        self.tx
            .send(RolloutCmd::Shutdown(ack_tx))
            .await
            .map_err(|_| RolloutError::ChannelClosed("shutdown"))?;
        ack_rx
            .await
            .map_err(|_| RolloutError::ChannelClosed("shutdown ack"))?;

        // Join the task if this clone owns the handle.
        let handle = {
            let mut guard = self
                .handle
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            guard.take()
        };
        if let Some(handle) = handle
            && let Err(err) = handle.await
        {
            warn!("rollout writer task join failed: {err}");
        }
        Ok(())
    }
}

/// Background task: owns the file + buffered writer and serializes all writes.
async fn writer_task(file: File, mut rx: mpsc::Receiver<RolloutCmd>) {
    let mut writer = BufWriter::new(file);

    while let Some(cmd) = rx.recv().await {
        match cmd {
            RolloutCmd::Append(line) => {
                if let Err(err) = write_line(&mut writer, &line).await {
                    warn!("rollout append failed: {err}");
                }
            }
            RolloutCmd::Flush(ack) => {
                let _ = ack.send(flush_and_sync(&mut writer).await);
            }
            RolloutCmd::Shutdown(ack) => {
                if let Err(err) = flush_and_sync(&mut writer).await {
                    warn!("rollout shutdown flush failed: {err}");
                }
                let _ = ack.send(());
                break;
            }
        }
    }
}

/// Serialize a line to JSON, append a newline, and write it to the buffer.
async fn write_line(writer: &mut BufWriter<File>, line: &RolloutLine) -> io::Result<()> {
    let mut json = serde_json::to_string(line)?;
    json.push('\n');
    writer.write_all(json.as_bytes()).await
}

/// Flush the buffered writer to the OS, then fsync the file to durable storage.
async fn flush_and_sync(writer: &mut BufWriter<File>) -> io::Result<()> {
    writer.flush().await?;
    writer.get_ref().sync_all().await?;
    Ok(())
}

/// Replay a JSONL rollout file into [`RolloutLine`]s, in order.
///
/// Blank lines are skipped. Lines that fail to parse are logged with
/// [`tracing::warn`] and skipped rather than failing the whole resume; this
/// makes recovery robust against a truncated final write or a single corrupt
/// record.
pub async fn resume(path: impl AsRef<Path>) -> Result<Vec<RolloutLine>, RolloutError> {
    let path = path.as_ref();
    let text = tokio::fs::read_to_string(path).await?;

    let mut lines = Vec::new();
    for (idx, raw) in text.lines().enumerate() {
        if raw.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<RolloutLine>(raw) {
            Ok(line) => lines.push(line),
            Err(err) => {
                warn!(
                    "skipping corrupted rollout line {} in {}: {err}",
                    idx + 1,
                    path.display()
                );
            }
        }
    }
    Ok(lines)
}

/// Milliseconds since the Unix epoch for `t` (0 if `t` predates the epoch).
fn epoch_millis(t: SystemTime) -> u64 {
    t.duration_since(UNIX_EPOCH)
        .map(|d| {
            let ms = d.as_millis();
            u64::try_from(ms).unwrap_or(u64::MAX)
        })
        .unwrap_or(0)
}

/// Format `t` as an RFC3339 UTC timestamp with millisecond precision.
///
/// Implemented manually (no chrono/time dep) from epoch milliseconds using the
/// civil-from-days algorithm by Howard Hinnant.
fn rfc3339_timestamp(t: SystemTime) -> String {
    let (y, mo, d, h, mi, s, ms) = civil_from_millis(epoch_millis(t));
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}.{ms:03}Z")
}

/// Filename-safe UTC timestamp: `YYYY-MM-DDTHH-MM-SS` (colons -> dashes).
fn filename_timestamp(t: SystemTime) -> String {
    let (y, mo, d, h, mi, s, _ms) = civil_from_millis(epoch_millis(t));
    format!("{y:04}-{mo:02}-{d:02}T{h:02}-{mi:02}-{s:02}")
}

/// Decompose epoch milliseconds into civil UTC `(year, month, day, hour, min, sec, milli)`.
///
/// Uses Howard Hinnant's `civil_from_days` algorithm (public domain) for the
/// date portion; the time-of-day portion is straightforward modular arithmetic.
fn civil_from_millis(millis: u64) -> (i64, u32, u32, u32, u32, u32, u32) {
    let total_secs = (millis / 1000) as i64;
    let ms = (millis % 1000) as u32;

    let days = total_secs.div_euclid(86_400);
    let secs_of_day = total_secs.rem_euclid(86_400);

    let hour = (secs_of_day / 3600) as u32;
    let minute = ((secs_of_day % 3600) / 60) as u32;
    let second = (secs_of_day % 60) as u32;

    // civil_from_days: days since 1970-01-01 -> (year, month, day).
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    let year = if month <= 2 { y + 1 } else { y };

    (year, month, day, hour, minute, second, ms)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn civil_from_millis_known_epoch() {
        // 1970-01-01T00:00:00.000Z
        assert_eq!(civil_from_millis(0), (1970, 1, 1, 0, 0, 0, 0));
    }

    #[test]
    fn rfc3339_known_instant() {
        // 1_700_000_000_123 ms => 2023-11-14T22:13:20.123Z
        let t = UNIX_EPOCH + std::time::Duration::from_millis(1_700_000_000_123);
        assert_eq!(rfc3339_timestamp(t), "2023-11-14T22:13:20.123Z");
    }

    #[tokio::test]
    async fn record_flush_produces_valid_jsonl() {
        let dir = tempdir().unwrap();
        let rec = RolloutRecorder::new(dir.path(), Some("sess-1".to_string()))
            .await
            .unwrap();
        assert_eq!(rec.session_id(), "sess-1");
        assert!(rec.path().to_string_lossy().contains("sess-1"));

        let n = 5;
        for i in 0..n {
            rec.record(json!({ "idx": i, "kind": "msg" }));
        }
        rec.flush().await.unwrap();

        let raw = tokio::fs::read_to_string(rec.path()).await.unwrap();
        let lines: Vec<&str> = raw.lines().filter(|l| !l.trim().is_empty()).collect();
        assert_eq!(lines.len(), n);
        for (i, l) in lines.iter().enumerate() {
            let v: RolloutLine = serde_json::from_str(l).unwrap();
            assert!(!v.ts.is_empty());
            assert!(v.ts.ends_with('Z'));
            assert_eq!(v.item["idx"], json!(i));
        }

        rec.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn resume_returns_lines_in_order() {
        let dir = tempdir().unwrap();
        let rec = RolloutRecorder::new(dir.path(), None).await.unwrap();
        for i in 0..4 {
            rec.record(json!({ "n": i }));
        }
        rec.flush().await.unwrap();
        let path = rec.path().to_path_buf();
        rec.shutdown().await.unwrap();

        let lines = resume(&path).await.unwrap();
        assert_eq!(lines.len(), 4);
        for (i, line) in lines.iter().enumerate() {
            assert_eq!(line.item["n"], json!(i));
        }
    }

    #[tokio::test]
    async fn resume_skips_corrupted_and_blank_lines() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("rollout-manual.jsonl");

        let good1 = serde_json::to_string(&RolloutLine {
            ts: "2023-11-14T22:13:20.123Z".to_string(),
            item: json!({ "n": 0 }),
        })
        .unwrap();
        let good2 = serde_json::to_string(&RolloutLine {
            ts: "2023-11-14T22:13:21.000Z".to_string(),
            item: json!({ "n": 1 }),
        })
        .unwrap();

        // good, blank, garbage, partial-json, good
        let contents = format!(
            "{good1}\n\n{{ this is not json }}\n{{\"ts\":\"x\"\n{good2}\n"
        );
        tokio::fs::write(&path, contents).await.unwrap();

        let lines = resume(&path).await.unwrap();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].item["n"], json!(0));
        assert_eq!(lines[1].item["n"], json!(1));
    }

    #[tokio::test]
    async fn shutdown_does_not_lose_data() {
        let dir = tempdir().unwrap();
        let rec = RolloutRecorder::new(dir.path(), None).await.unwrap();
        let n = 10;
        for i in 0..n {
            rec.record(json!({ "seq": i }));
        }
        // Shut down WITHOUT an explicit flush: shutdown must drain pending writes.
        let path = rec.path().to_path_buf();
        rec.shutdown().await.unwrap();

        let lines = resume(&path).await.unwrap();
        assert_eq!(lines.len(), n);
        for (i, line) in lines.iter().enumerate() {
            assert_eq!(line.item["seq"], json!(i));
        }
    }

    #[tokio::test]
    async fn resume_missing_file_errors() {
        let dir = tempdir().unwrap();
        let missing = dir.path().join("nope.jsonl");
        let err = resume(&missing).await.unwrap_err();
        assert!(matches!(err, RolloutError::Io(_)));
    }
}
