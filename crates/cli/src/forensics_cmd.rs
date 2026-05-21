// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors
//
// `aphrody forensics` — reproducible, non-interactive forensic extraction
// (WS7 of docs/plans/antigravity-exploitation.md).
//
// SECURITY CONTRACT (non-negotiable, enforced by code below):
//   This command MAPS, CLASSIFIES, and DUMPS SCHEMA only. It MUST NEVER
//   print, copy, or otherwise emit secret VALUES:
//     - no access_token / refresh_token bytes,
//     - no cookie values,
//     - no `ItemTable` / `cookies` / `secret://` row contents.
//   For SQLite it reads `sqlite_master` (table NAMES + CREATE statements)
//   ONLY; value columns are never SELECTed. The Credential Manager path
//   (host-only, future extension) reports presence + expiry ONLY.
//
// All output is JSON written under a caller-supplied `--out` directory; the
// caller is responsible for keeping that directory gitignored.

use std::path::PathBuf;

use clap::Subcommand;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use serde::Serialize;

/// Actions for the `forensics` subcommand.
///
/// Backed by `walkdir` + `rayon` (filesystem map) and `rusqlite` in
/// strict read-only mode (schema dump). The set starts intentionally small
/// (`map`, `sqlite`) and is designed to be extended (leveldb, asar, credman)
/// without changing the security contract documented at the module level.
#[derive(Subcommand, Debug, Clone)]
pub(crate) enum ForensicsAction {
    /// Map a target directory: parallel walk emitting `{ path, size, ext }`
    /// per file to `<out>/map.json`.
    ///
    /// Reads only directory metadata + filename extensions — file CONTENTS
    /// are never opened, so no secret material can leak through this path.
    ///
    /// Example: aphrody forensics map --target ~/.gemini --out var/data/x
    Map {
        /// Directory to walk recursively.
        #[arg(long)]
        target: PathBuf,
        /// Output directory. Created if absent. `map.json` is written here.
        #[arg(long)]
        out: PathBuf,
    },
    /// Dump the schema of a SQLite database, read-only.
    ///
    /// Opens the DB with `SQLITE_OPEN_READ_ONLY` and reads
    /// `SELECT name, sql FROM sqlite_master` — table/index NAMES and their
    /// CREATE statements only. Value columns of `ItemTable` / `cookies` /
    /// `secret`-style tables are NEVER selected, so no secret bytes are
    /// emitted. Output is JSON on stdout.
    ///
    /// Example: aphrody forensics sqlite --db state.vscdb | jq '.tables[].name'
    Sqlite {
        /// Path to the SQLite database file (opened read-only).
        #[arg(long)]
        db: PathBuf,
    },
}

/// Files at or below this size are SHA-256 hashed in `map.json`. Larger files
/// (and any whose path matches the secret denylist) record `sha256: null`.
/// 1 MiB keeps the map cheap and avoids hashing multi-hundred-MB stores.
const MAX_HASH_BYTES: u64 = 1_048_576;

/// Path substrings (matched case-insensitively against the FULL path) that
/// mark an artefact as secret-bearing. Such files are recorded metadata-only:
/// they are NEVER opened, so they are never hashed. Mirrors the
/// Chromium/Electron secret stores documented in
/// `docs/research/google-local-install-map.md` §4.
const SECRET_PATTERNS: &[&str] = &[
    "cookies",
    "login data",
    "web data",
    "credential",
    "token",
    "local state",
    "leveldb",
    ".ldb",
    "session storage",
    "local storage",
    "indexeddb",
    "autofill",
    "password",
    "user_history",
    "preferences.txtpb",
    "global_preferences",
];

/// `true` if `path` matches the secret denylist and must stay metadata-only.
fn is_secret_path(path: &std::path::Path) -> bool {
    let lower = path.to_string_lossy().to_ascii_lowercase();
    SECRET_PATTERNS.iter().any(|p| lower.contains(p))
}

/// One filesystem entry in the `map.json` output.
#[derive(Serialize, Debug)]
struct MapEntry {
    /// Absolute or target-relative path as reported by `walkdir`.
    path: String,
    /// File size in bytes (from the directory entry metadata).
    size: u64,
    /// Lower-cased file extension (without the dot), or `null` if none.
    ext: Option<String>,
    /// SHA-256 hex digest of the file contents, for non-secret files at or
    /// below [`MAX_HASH_BYTES`]. `null` for larger files, read errors, or any
    /// file on the secret denylist (which is never opened).
    sha256: Option<String>,
    /// Last-modified time as an RFC 3339 string (UTC), or `null` if the
    /// platform/filesystem does not expose it.
    modified: Option<String>,
    /// Last-modified time as Unix epoch seconds, or `null` if unavailable.
    mtime_unix: Option<i64>,
    /// `true` when the path matched the secret denylist and only metadata was
    /// recorded (contents never opened, so `sha256` is always `null`).
    secret_meta_only: bool,
}

/// Top-level `map.json` document.
#[derive(Serialize, Debug)]
struct MapReport {
    /// The target directory that was walked.
    target: String,
    /// Total number of regular files mapped.
    file_count: usize,
    /// Number of files for which a SHA-256 was computed (non-secret, at or
    /// below [`MAX_HASH_BYTES`]).
    hashed_count: usize,
    /// Number of files recorded metadata-only because their path matched the
    /// secret denylist (contents never opened).
    secret_meta_only_count: usize,
    /// Size cap (bytes) above which files are not hashed.
    max_hash_bytes: u64,
    /// Per-file entries.
    files: Vec<MapEntry>,
}

/// One table (or index/trigger/view) row from `sqlite_master`.
#[derive(Serialize, Debug)]
struct SqliteObject {
    /// Object type: `table`, `index`, `view`, or `trigger`.
    #[serde(rename = "type")]
    kind: String,
    /// Object name.
    name: String,
    /// The CREATE statement (schema). May be `null` for internal objects.
    sql: Option<String>,
}

/// Top-level SQLite schema dump document.
#[derive(Serialize, Debug)]
struct SqliteReport {
    /// Path to the database that was inspected.
    db: String,
    /// Number of schema objects found.
    object_count: usize,
    /// Schema objects (names + CREATE statements only — no row values).
    tables: Vec<SqliteObject>,
}

/// Run a `forensics` action.
///
/// Async to match the rest of the CLI dispatch surface; the work itself is
/// CPU/IO bound and synchronous (no `.await` inside), which keeps the
/// command fully deterministic and non-interactive.
pub(crate) async fn run(action: ForensicsAction) -> miette::Result<()> {
    match action {
        ForensicsAction::Map { target, out } => map(&target, &out),
        ForensicsAction::Sqlite { db } => sqlite_schema(&db),
    }
}

/// Parallel filesystem map. Never opens file contents — extension/metadata
/// only — so it cannot leak secret bytes.
fn map(target: &std::path::Path, out: &std::path::Path) -> miette::Result<()> {
    if !target.is_dir() {
        return Err(miette::miette!(
            "forensics map: target is not a directory: {}",
            target.display()
        ));
    }

    // First pass (sequential): collect regular-file entries from walkdir.
    // walkdir is not a parallel iterator, so we gather then fan out the
    // (cheap) classification work to rayon.
    let mut entries: Vec<walkdir::DirEntry> = Vec::new();
    for entry in walkdir::WalkDir::new(target).follow_links(false) {
        let entry = entry
            .map_err(|e| miette::miette!("forensics map: walk error: {e}"))?;
        if entry.file_type().is_file() {
            entries.push(entry);
        }
    }

    // Second pass (parallel): metadata + extension + mtime + (non-secret,
    // small-file) SHA-256. Secret-denylisted files are NEVER opened, so they
    // never contribute hash bytes — they record metadata only.
    let files: Vec<MapEntry> = entries
        .par_iter()
        .map(|entry| {
            let path = entry.path();
            let meta = entry.metadata().ok();
            let size = meta.as_ref().map(std::fs::Metadata::len).unwrap_or(0);
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_ascii_lowercase());

            let (modified, mtime_unix) = match meta.as_ref().and_then(|m| m.modified().ok()) {
                Some(t) => {
                    let dt: chrono::DateTime<chrono::Utc> = t.into();
                    (Some(dt.to_rfc3339()), Some(dt.timestamp()))
                },
                None => (None, None),
            };

            let secret_meta_only = is_secret_path(path);
            let sha256 = if !secret_meta_only && size <= MAX_HASH_BYTES {
                hash_file(path)
            } else {
                None
            };

            MapEntry {
                path: path.display().to_string(),
                size,
                ext,
                sha256,
                modified,
                mtime_unix,
                secret_meta_only,
            }
        })
        .collect();

    let hashed_count = files.iter().filter(|f| f.sha256.is_some()).count();
    let secret_meta_only_count = files.iter().filter(|f| f.secret_meta_only).count();

    let report = MapReport {
        target: target.display().to_string(),
        file_count: files.len(),
        hashed_count,
        secret_meta_only_count,
        max_hash_bytes: MAX_HASH_BYTES,
        files,
    };

    std::fs::create_dir_all(out).map_err(|e| {
        miette::miette!("forensics map: create out dir {}: {e}", out.display())
    })?;
    let out_file = out.join("map.json");
    let json = serde_json::to_string_pretty(&report)
        .map_err(|e| miette::miette!("forensics map: JSON encode failed: {e}"))?;
    std::fs::write(&out_file, json).map_err(|e| {
        miette::miette!("forensics map: write {}: {e}", out_file.display())
    })?;

    // Confirmation on stdout (machine-readable summary, no secrets).
    let summary = serde_json::json!({
        "wrote": out_file.display().to_string(),
        "file_count": report.file_count,
        "hashed_count": report.hashed_count,
        "secret_meta_only_count": report.secret_meta_only_count,
    });
    println!("{summary}");
    Ok(())
}

/// SHA-256 hex digest of a file's contents, streamed in fixed-size chunks so a
/// (capped, non-secret) file is hashed without holding it all in memory.
/// Returns `None` on any read error. Callers gate this on the size cap +
/// secret denylist, so it is only ever invoked on small, non-secret files.
fn hash_file(path: &std::path::Path) -> Option<String> {
    use std::io::Read as _;

    use sha2::{Digest, Sha256};

    let mut file = std::fs::File::open(path).ok()?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Some(hex::encode(hasher.finalize()))
}

/// Read-only SQLite schema dump.
///
/// SECURITY: opens with `SQLITE_OPEN_READ_ONLY` and reads ONLY
/// `sqlite_master` (object type/name/CREATE-sql). It deliberately performs
/// no `SELECT` against any data table — never `ItemTable`, `cookies`, or
/// `secret`-style rows — so no secret value can ever be emitted.
fn sqlite_schema(db: &std::path::Path) -> miette::Result<()> {
    use rusqlite::{Connection, OpenFlags};

    if !db.is_file() {
        return Err(miette::miette!(
            "forensics sqlite: not a file: {}",
            db.display()
        ));
    }

    // READ_ONLY guarantees we cannot mutate the evidence; NO_MUTEX is safe
    // because the connection is used from a single thread.
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let conn = Connection::open_with_flags(db, flags).map_err(|e| {
        miette::miette!("forensics sqlite: open {}: {e}", db.display())
    })?;

    // SCHEMA ONLY. `sqlite_master` exposes the catalog (name + CREATE sql);
    // we never touch row data of any user table.
    let mut stmt = conn
        .prepare("SELECT type, name, sql FROM sqlite_master ORDER BY type, name")
        .map_err(|e| miette::miette!("forensics sqlite: prepare: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(SqliteObject {
                kind: row.get::<_, String>(0)?,
                name: row.get::<_, String>(1)?,
                sql: row.get::<_, Option<String>>(2)?,
            })
        })
        .map_err(|e| miette::miette!("forensics sqlite: query: {e}"))?;

    let mut tables = Vec::new();
    for row in rows {
        tables.push(row.map_err(|e| miette::miette!("forensics sqlite: row: {e}"))?);
    }

    let report = SqliteReport {
        db: db.display().to_string(),
        object_count: tables.len(),
        tables,
    };

    let json = serde_json::to_string_pretty(&report).map_err(|e| {
        miette::miette!("forensics sqlite: JSON encode failed: {e}")
    })?;
    println!("{json}");
    Ok(())
}
