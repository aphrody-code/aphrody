// SPDX-License-Identifier: Apache-2.0
//! Persistent local-filesystem search index backed by SQLite FTS5.
//!
//! Mirrors the schema the Google desktop app ships as `local_files.db` (an
//! 852 MB FTS5 index of ~1.59M paths — see
//! `docs/research/google-local-install-map.md`): **metadata only** (path,
//! name, extension, size, mtime — never file contents), so it is cheap and
//! privacy-bounded. Backs `aphrody index build` / `aphrody index search`.
//!
//! ```no_run
//! use aphrody_fsindex::FsIndex;
//! # fn run() -> Result<(), aphrody_fsindex::FsIndexError> {
//! let mut idx = FsIndex::open("index.db")?;
//! let n = idx.build("/home/user/projects")?;
//! println!("indexed {n} entries");
//! for hit in idx.search("readme", 20)? {
//!     println!("{}", hit.path);
//! }
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_code)]

use std::path::Path;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors from the filesystem index.
#[derive(Debug, Error)]
pub enum FsIndexError {
    /// SQLite-level failure (open, schema, query).
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// The query reduced to no searchable terms.
    #[error("empty query")]
    EmptyQuery,
}

/// One indexed filesystem entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileHit {
    /// Absolute path (the unique key).
    pub path: String,
    /// File name component.
    pub name: String,
    /// Lower-cased extension without the dot (empty if none).
    pub extension: String,
    /// Size in bytes (0 for directories).
    pub size: u64,
    /// Last-modified time as a unix timestamp (seconds), or 0 if unavailable.
    pub mtime_unix: i64,
    /// True for directories.
    pub is_dir: bool,
}

/// A persistent local-filesystem index.
pub struct FsIndex {
    conn: Connection,
}

impl FsIndex {
    /// Open (or create) an index at `db_path`.
    ///
    /// # Errors
    ///
    /// [`FsIndexError::Sqlite`] if the database cannot be opened or the schema
    /// cannot be created.
    pub fn open(db_path: impl AsRef<Path>) -> Result<Self, FsIndexError> {
        let conn = Connection::open(db_path)?;
        Self::init(&conn)?;
        Ok(Self { conn })
    }

    /// Open an in-memory index (tests / ephemeral use).
    ///
    /// # Errors
    ///
    /// [`FsIndexError::Sqlite`] on failure.
    pub fn open_in_memory() -> Result<Self, FsIndexError> {
        let conn = Connection::open_in_memory()?;
        Self::init(&conn)?;
        Ok(Self { conn })
    }

    fn init(conn: &Connection) -> Result<(), FsIndexError> {
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             CREATE TABLE IF NOT EXISTS files(
                 path        TEXT PRIMARY KEY,
                 parent_path TEXT,
                 name        TEXT NOT NULL,
                 extension   TEXT NOT NULL DEFAULT '',
                 size        INTEGER NOT NULL DEFAULT 0,
                 mtime_unix  INTEGER NOT NULL DEFAULT 0,
                 is_dir      INTEGER NOT NULL DEFAULT 0
             );
             CREATE INDEX IF NOT EXISTS idx_parent ON files(parent_path);
             CREATE VIRTUAL TABLE IF NOT EXISTS files_fts USING fts5(
                 name, path, content='files', content_rowid='rowid'
             );
             -- keep FTS in sync with the content table.
             CREATE TRIGGER IF NOT EXISTS files_ai AFTER INSERT ON files BEGIN
                 INSERT INTO files_fts(rowid, name, path) VALUES (new.rowid, new.name, new.path);
             END;
             CREATE TRIGGER IF NOT EXISTS files_ad AFTER DELETE ON files BEGIN
                 INSERT INTO files_fts(files_fts, rowid, name, path) VALUES('delete', old.rowid, old.name, old.path);
             END;
             CREATE TRIGGER IF NOT EXISTS files_au AFTER UPDATE ON files BEGIN
                 INSERT INTO files_fts(files_fts, rowid, name, path) VALUES('delete', old.rowid, old.name, old.path);
                 INSERT INTO files_fts(rowid, name, path) VALUES (new.rowid, new.name, new.path);
             END;",
        )?;
        Ok(())
    }

    /// Walk `root` recursively and upsert every entry. Returns the number of
    /// entries indexed. Metadata only — file contents are never read.
    ///
    /// Idempotent: re-running updates changed rows (`INSERT … ON CONFLICT`).
    ///
    /// # Errors
    ///
    /// [`FsIndexError::Sqlite`] on a database failure. Filesystem errors on
    /// individual entries (permission denied, etc.) are skipped, not fatal.
    pub fn build(&mut self, root: impl AsRef<Path>) -> Result<usize, FsIndexError> {
        let tx = self.conn.transaction()?;
        let mut count = 0usize;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO files(path, parent_path, name, extension, size, mtime_unix, is_dir)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(path) DO UPDATE SET
                     size=excluded.size, mtime_unix=excluded.mtime_unix, is_dir=excluded.is_dir",
            )?;
            for entry in walkdir::WalkDir::new(root).into_iter().filter_map(Result::ok) {
                let path = entry.path();
                let path_str = path.to_string_lossy().to_string();
                let name = entry.file_name().to_string_lossy().to_string();
                let parent = path.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
                let extension = path
                    .extension()
                    .map(|e| e.to_string_lossy().to_ascii_lowercase())
                    .unwrap_or_default();
                let meta = entry.metadata().ok();
                let is_dir = meta.as_ref().is_some_and(std::fs::Metadata::is_dir);
                let size = if is_dir { 0 } else { meta.as_ref().map_or(0, std::fs::Metadata::len) };
                let mtime_unix = meta
                    .as_ref()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0));
                stmt.execute(rusqlite::params![
                    path_str, parent, name, extension, size, mtime_unix, i32::from(is_dir),
                ])?;
                count += 1;
            }
        }
        tx.commit()?;
        Ok(count)
    }

    /// Full-text search over file names + paths (FTS5). `query` terms are
    /// quoted and prefix-matched (`term*`). Returns up to `limit` hits ranked
    /// by FTS5 `bm25`.
    ///
    /// # Errors
    ///
    /// [`FsIndexError::EmptyQuery`] if `query` has no searchable terms;
    /// [`FsIndexError::Sqlite`] on a database failure.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<FileHit>, FsIndexError> {
        let fts = build_fts_query(query).ok_or(FsIndexError::EmptyQuery)?;
        let mut stmt = self.conn.prepare(
            "SELECT f.path, f.name, f.extension, f.size, f.mtime_unix, f.is_dir
             FROM files_fts JOIN files f ON f.rowid = files_fts.rowid
             WHERE files_fts MATCH ?1
             ORDER BY bm25(files_fts) LIMIT ?2",
        )?;
        let rows = stmt.query_map(rusqlite::params![fts, limit as i64], |row| {
            Ok(FileHit {
                path: row.get(0)?,
                name: row.get(1)?,
                extension: row.get(2)?,
                size: row.get::<_, i64>(3)? as u64,
                mtime_unix: row.get(4)?,
                is_dir: row.get::<_, i32>(5)? != 0,
            })
        })?;
        let mut hits = Vec::new();
        for r in rows {
            hits.push(r?);
        }
        Ok(hits)
    }

    /// Total number of indexed entries.
    ///
    /// # Errors
    ///
    /// [`FsIndexError::Sqlite`] on failure.
    pub fn len(&self) -> Result<usize, FsIndexError> {
        let n: i64 = self.conn.query_row("SELECT COUNT(*) FROM files", [], |r| r.get(0))?;
        Ok(usize::try_from(n).unwrap_or(0))
    }

    /// True when the index holds no entries.
    ///
    /// # Errors
    ///
    /// [`FsIndexError::Sqlite`] on failure.
    pub fn is_empty(&self) -> Result<bool, FsIndexError> {
        Ok(self.len()? == 0)
    }
}

/// Turn a free-text query into a safe FTS5 prefix query: each alphanumeric term
/// is double-quoted and `*`-suffixed. Returns `None` if no terms remain.
fn build_fts_query(query: &str) -> Option<String> {
    let terms: Vec<String> = query
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|t| !t.is_empty())
        .map(|t| format!("\"{t}\"*"))
        .collect();
    if terms.is_empty() {
        None
    } else {
        Some(terms.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn build_and_search_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("readme.md"), b"x").unwrap();
        fs::write(dir.path().join("notes.txt"), b"yy").unwrap();
        fs::create_dir(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src").join("main.rs"), b"fn main(){}").unwrap();

        let mut idx = FsIndex::open_in_memory().unwrap();
        let n = idx.build(dir.path()).unwrap();
        assert!(n >= 4, "indexed {n} entries");
        assert_eq!(idx.len().unwrap(), n);

        let hits = idx.search("readme", 10).unwrap();
        assert!(hits.iter().any(|h| h.name == "readme.md"));

        // prefix match
        let hits = idx.search("mai", 10).unwrap();
        assert!(hits.iter().any(|h| h.name == "main.rs" && h.extension == "rs"));

        // directory indexed
        let hits = idx.search("src", 10).unwrap();
        assert!(hits.iter().any(|h| h.is_dir && h.name == "src"));
    }

    #[test]
    fn rebuild_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), b"a").unwrap();
        let mut idx = FsIndex::open_in_memory().unwrap();
        let n1 = idx.build(dir.path()).unwrap();
        let n2 = idx.build(dir.path()).unwrap();
        assert_eq!(n1, n2);
        assert_eq!(idx.len().unwrap(), n1); // no duplicate rows
    }

    #[test]
    fn empty_query_is_rejected() {
        let idx = FsIndex::open_in_memory().unwrap();
        assert!(matches!(idx.search("   !!! ", 10), Err(FsIndexError::EmptyQuery)));
    }

    #[test]
    fn fts_query_quotes_and_prefixes() {
        assert_eq!(build_fts_query("foo bar").as_deref(), Some("\"foo\"* \"bar\"*"));
        assert_eq!(build_fts_query("a.b-c").as_deref(), Some("\"a\"* \"b\"* \"c\"*"));
        assert_eq!(build_fts_query("  "), None);
    }
}
