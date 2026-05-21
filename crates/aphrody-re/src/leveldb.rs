// SPDX-License-Identifier: Apache-2.0
//! Read-only LevelDB key/value enumerator (WS6b).
//!
//! Chromium stores Local Storage, Session Storage, IndexedDB metadata and
//! `shared_proto_db` as on-disk [LevelDB] databases. This module opens such a
//! directory and returns every key/value pair as a JSON-serializable
//! [`LevelDbEntry`], for forensic / RE consumption (`aphrody re leveldb <dir>`).
//!
//! [LevelDB]: https://github.com/google/leveldb
//!
//! ## Why pure-Rust ([`rusty_leveldb`])
//!
//! aphrody is Apache-2.0 with a no-C, no-GPL policy. The canonical C++ LevelDB
//! bindings (`leveldb-sys`) would pull a C toolchain and a non-Rust dependency;
//! `rusty_leveldb` is an MIT-licensed, pure-Rust reimplementation with the same
//! block/SSTable/WAL format, so it reads the same files Chromium writes.
//!
//! ## Forensic safety — never mutate the source
//!
//! Opening a LevelDB **replays its write-ahead log into the directory** and may
//! rewrite the `MANIFEST`/`CURRENT` files. Live Chromium profiles also hold a
//! `LOCK` on the directory. To stay strictly non-destructive, [`dump`]:
//!
//! 1. recursively copies the target directory into a private scratch
//!    [`tempfile::TempDir`],
//! 2. opens the **copy** with `create_if_missing = false` and log/manifest reuse
//!    disabled, then
//! 3. iterates every entry and drops the temp dir on return.
//!
//! The source bytes on disk are read but never written.
//!
//! ## Output shape — no raw binary in JSON
//!
//! LevelDB keys and values are arbitrary bytes. We never splice raw binary into
//! a JSON string. Each entry carries the key as lowercase hex (always present)
//! plus a best-effort lossless UTF-8 view (`*_utf8`, only `Some` when the bytes
//! are valid UTF-8), and the value is previewed up to [`VALUE_PREVIEW_LIMIT`]
//! bytes to keep the report bounded.
//!
//! ## Quickstart
//!
//! ```no_run
//! use aphrody_re::leveldb::dump;
//!
//! let entries = dump(std::path::Path::new("/path/to/Local Storage/leveldb"))?;
//! for e in &entries {
//!     println!("{} ({} bytes)", e.key_utf8.as_deref().unwrap_or(&e.key_hex), e.value_len);
//! }
//! # Ok::<(), aphrody_re::leveldb::LevelDbError>(())
//! ```

use std::path::Path;

use rusty_leveldb::{LdbIterator as _, Options, DB};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Maximum number of value bytes decoded into [`LevelDbEntry::value_utf8`].
///
/// `value_len` always reflects the true, full length; only the UTF-8 preview is
/// truncated. Keeps the JSON report bounded for large blobs (Chromium values
/// routinely reach tens of kilobytes).
pub const VALUE_PREVIEW_LIMIT: usize = 4096;

/// Errors returned by [`dump`].
#[derive(Debug, Error)]
pub enum LevelDbError {
    /// The target path does not exist or is not a directory. A LevelDB database
    /// is always a directory (it holds `CURRENT`, `MANIFEST-*`, `*.ldb`, …).
    #[error("not a LevelDB directory: {0}")]
    NotADirectory(String),

    /// Failed to create the scratch copy of the database before opening it.
    #[error("failed to stage a read-only copy of the database: {0}")]
    Stage(#[source] std::io::Error),

    /// The underlying `rusty_leveldb` engine rejected the database (missing
    /// `CURRENT`, corrupt manifest, truncated SSTable, …).
    #[error("leveldb engine error: {0}")]
    Engine(#[source] rusty_leveldb::Status),
}

/// One key/value pair from a LevelDB database.
///
/// Stable, self-describing JSON via `serde`. Raw bytes never appear unencoded:
/// keys are always hex, and the UTF-8 fields are `Some` only when the bytes are
/// valid UTF-8 (value additionally truncated to [`VALUE_PREVIEW_LIMIT`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LevelDbEntry {
    /// Lowercase hex encoding of the full key bytes. Always populated.
    pub key_hex: String,
    /// The key decoded as UTF-8, or `None` when the key is not valid UTF-8.
    pub key_utf8: Option<String>,
    /// True, full length of the value in bytes (independent of the preview cap).
    pub value_len: usize,
    /// Best-effort UTF-8 preview of the value, truncated at a UTF-8 char
    /// boundary to at most [`VALUE_PREVIEW_LIMIT`] bytes. `None` when the
    /// (possibly truncated) prefix is not valid UTF-8 — callers wanting the raw
    /// bytes should re-read the database directly rather than decode from JSON.
    pub value_utf8: Option<String>,
}

/// Open a LevelDB database directory read-only and return every key/value pair.
///
/// The source directory is **never modified**: it is copied into a private temp
/// directory which is opened instead (see the module docs for the rationale).
/// Entries are returned in LevelDB iteration order (sorted by the internal
/// comparator, i.e. lexicographic over the user key bytes).
///
/// # Errors
///
/// - [`LevelDbError::NotADirectory`] if `dir` is missing or not a directory.
/// - [`LevelDbError::Stage`] if the scratch copy cannot be created.
/// - [`LevelDbError::Engine`] if `rusty_leveldb` fails to open or read the copy
///   (no `CURRENT`, corrupt manifest, truncated table, …).
pub fn dump(dir: &Path) -> Result<Vec<LevelDbEntry>, LevelDbError> {
    if !dir.is_dir() {
        return Err(LevelDbError::NotADirectory(dir.display().to_string()));
    }

    // Stage a non-destructive copy. The TempDir is RAII-cleaned on drop, even on
    // the error paths below.
    let scratch = tempfile::Builder::new()
        .prefix("aphrody-re-leveldb-")
        .tempdir()
        .map_err(LevelDbError::Stage)?;
    let db_copy = scratch.path().join("db");
    copy_dir_shallow(dir, &db_copy).map_err(LevelDbError::Stage)?;

    // Read-only intent: do not create a fresh DB if the copy turned out empty,
    // and disable log/manifest reuse so the open is as side-effect-free as the
    // engine allows. `rusty_leveldb`'s default `CompressorList` registers both
    // the none and Snappy codecs, so Chromium's Snappy-compressed blocks decode
    // transparently.
    let mut opt = Options::default();
    opt.create_if_missing = false;
    opt.error_if_exists = false;
    opt.reuse_logs = false;
    opt.reuse_manifest = false;
    opt.paranoid_checks = true;

    let mut db = DB::open(&db_copy, opt).map_err(LevelDbError::Engine)?;
    let mut iter = db.new_iter().map_err(LevelDbError::Engine)?;

    let mut out = Vec::new();
    while let Some((key, value)) = iter.next() {
        out.push(LevelDbEntry {
            key_hex: hex::encode(&key),
            key_utf8: String::from_utf8(key).ok(),
            value_len: value.len(),
            value_utf8: utf8_preview(&value, VALUE_PREVIEW_LIMIT),
        });
    }

    Ok(out)
}

/// Best-effort UTF-8 preview of `bytes`, capped at `limit` bytes.
///
/// Returns `Some(text)` when the prefix (truncated at a UTF-8 char boundary so
/// we never split a multi-byte sequence) is valid UTF-8, otherwise `None`. We
/// deliberately do **not** lossy-replace invalid bytes: a `None` is an honest
/// signal that the value is binary, rather than a string sprinkled with U+FFFD
/// that a downstream consumer might mistake for real content.
fn utf8_preview(bytes: &[u8], limit: usize) -> Option<String> {
    let slice = &bytes[..bytes.len().min(limit)];
    match std::str::from_utf8(slice) {
        Ok(s) => Some(s.to_owned()),
        // The cut may have landed mid-character; back off to the last valid
        // boundary and retry so a clean prefix still previews.
        Err(e) => {
            let valid = &slice[..e.valid_up_to()];
            // Only treat it as a successful preview if we actually truncated
            // (the tail error is a boundary artifact, not genuinely-binary data
            // earlier in the buffer).
            if e.valid_up_to() > 0 && bytes.len() > limit {
                std::str::from_utf8(valid).ok().map(str::to_owned)
            } else {
                None
            }
        },
    }
}

/// Copy the immediate file children of `src` into a fresh `dst` directory.
///
/// LevelDB databases are flat (no subdirectories), so a shallow copy of the
/// regular files is sufficient and avoids following any stray symlinks. `dst`
/// is created; `src` is read-only.
fn copy_dir_shallow(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_file() {
            std::fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a real on-disk LevelDB, write a few keys, then `dump` it and assert
    /// the round-trip (hex + UTF-8 views, full length, ordering).
    #[test]
    fn dump_round_trips_known_keys() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("local_storage");

        {
            let mut opt = Options::default();
            opt.create_if_missing = true;
            let mut db = DB::open(&db_path, opt).expect("create db");
            db.put(b"META:https://example.com", b"\x01\x00")
                .expect("put 1");
            db.put(b"_app.localhost\x00\x01key", b"hello world")
                .expect("put 2");
            db.flush().expect("flush");
        }

        let entries = dump(&db_path).expect("dump");
        assert_eq!(entries.len(), 2, "expected 2 entries, got {entries:?}");

        let by_key: std::collections::HashMap<&str, &LevelDbEntry> =
            entries.iter().map(|e| (e.key_hex.as_str(), e)).collect();

        let k1 = hex::encode(b"META:https://example.com");
        let e1 = by_key.get(k1.as_str()).expect("entry 1 present");
        assert_eq!(e1.key_utf8.as_deref(), Some("META:https://example.com"));
        assert_eq!(e1.value_len, 2);
        // Value 0x01 0x00 is not printable text but IS valid UTF-8 (both are
        // control points), so the preview is Some.
        assert_eq!(e1.value_utf8.as_deref(), Some("\u{1}\u{0}"));

        let k2 = hex::encode(b"_app.localhost\x00\x01key");
        let e2 = by_key.get(k2.as_str()).expect("entry 2 present");
        // The key contains a NUL + 0x01, still valid UTF-8.
        assert_eq!(e2.value_len, 11);
        assert_eq!(e2.value_utf8.as_deref(), Some("hello world"));
    }

    /// A genuinely-binary value (invalid UTF-8 in the middle) must yield a hex
    /// key and a `None` UTF-8 value — never raw bytes spliced into JSON.
    #[test]
    fn binary_value_has_none_utf8_preview() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("shared_proto_db");

        {
            let mut opt = Options::default();
            opt.create_if_missing = true;
            let mut db = DB::open(&db_path, opt).expect("create db");
            // 0xFF 0xFE is invalid UTF-8.
            db.put(b"blob", &[0x00, 0xFF, 0xFE, 0x41]).expect("put");
            db.flush().expect("flush");
        }

        let entries = dump(&db_path).expect("dump");
        let e = entries.iter().find(|e| e.key_utf8.as_deref() == Some("blob")).expect("found");
        assert_eq!(e.value_len, 4);
        assert!(e.value_utf8.is_none(), "binary value must not decode to UTF-8");
        // Round-trips through JSON without losing the hex key.
        let json = serde_json::to_value(e).expect("serialize");
        assert_eq!(json["key_hex"], hex::encode(b"blob"));
        assert!(json["value_utf8"].is_null());
        assert_eq!(json["value_len"], 4);
    }

    /// A long value must be previewed up to the cap while `value_len` reports
    /// the true length.
    #[test]
    fn long_value_is_truncated_to_preview_limit() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("big");

        let big = "a".repeat(VALUE_PREVIEW_LIMIT * 2);
        {
            let mut opt = Options::default();
            opt.create_if_missing = true;
            let mut db = DB::open(&db_path, opt).expect("create db");
            db.put(b"k", big.as_bytes()).expect("put");
            db.flush().expect("flush");
        }

        let entries = dump(&db_path).expect("dump");
        let e = entries.iter().find(|e| e.key_utf8.as_deref() == Some("k")).expect("found");
        assert_eq!(e.value_len, VALUE_PREVIEW_LIMIT * 2);
        assert_eq!(
            e.value_utf8.as_deref().map(str::len),
            Some(VALUE_PREVIEW_LIMIT),
            "preview must be capped at VALUE_PREVIEW_LIMIT"
        );
    }

    #[test]
    fn missing_directory_is_not_a_directory_error() {
        let err = dump(Path::new("/nonexistent/aphrody/leveldb/path")).unwrap_err();
        assert!(matches!(err, LevelDbError::NotADirectory(_)), "got {err:?}");
    }

    #[test]
    fn empty_or_non_leveldb_directory_is_engine_error() {
        // An existing directory with no CURRENT file is not a LevelDB; with
        // create_if_missing = false the engine must refuse it.
        let dir = tempfile::tempdir().expect("tempdir");
        let err = dump(dir.path()).unwrap_err();
        assert!(matches!(err, LevelDbError::Engine(_)), "got {err:?}");
    }

    #[test]
    fn utf8_preview_backs_off_to_char_boundary() {
        // A 2-byte UTF-8 char (é = 0xC3 0xA9) straddling the cap must back off
        // cleanly rather than return None.
        let mut bytes = vec![b'x'; 3];
        bytes.extend_from_slice(&[0xC3, 0xA9]); // 'é'
        // limit lands between the two bytes of 'é'.
        let preview = utf8_preview(&bytes, 4);
        assert_eq!(preview.as_deref(), Some("xxx"));
    }
}
