// SPDX-License-Identifier: Apache-2.0
//! Offline `MemoryProvider` backed by a single SQLite file (or `:memory:`).
//!
//! - Schema: a flat `memory_records` table with a JSON-encoded `tags` array
//!   so callers can filter by tag via `LIKE '%"tag"%'` without a separate
//!   table. Pragmatic; works up to ~100k rows.
//! - Identity: `add` accepts an empty `id` and generates a UUID v4. Pre-set
//!   ids are honoured (re-import / replay scenarios).
//! - Concurrency: the inner `Connection` lives behind a `tokio::sync::Mutex`
//!   so multiple async tasks can serialise access. SQLite is single-writer.
//! - Bundled libsqlite3: no system dep, builds clean on Linux #1 / Win11 #2.

use std::path::Path;

use async_trait::async_trait;
use rusqlite::{Connection, OpenFlags, params};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::provider::MemoryProvider;
use crate::types::{MemoryError, MemoryQuery, MemoryRecord, ProviderKind};

/// Bootstrap SQL — idempotent (`IF NOT EXISTS`).
const SCHEMA_SQL: &str = "
CREATE TABLE IF NOT EXISTS memory_records (
    id TEXT PRIMARY KEY,
    agent_id TEXT NOT NULL,
    content TEXT NOT NULL,
    tags_json TEXT NOT NULL DEFAULT '[]',
    created_at_unix_ms INTEGER NOT NULL,
    metadata_json TEXT NOT NULL DEFAULT '{}'
);
CREATE INDEX IF NOT EXISTS idx_agent ON memory_records(agent_id);
CREATE INDEX IF NOT EXISTS idx_created ON memory_records(created_at_unix_ms DESC);
";

/// Local SQLite-backed [`MemoryProvider`] — the offline default.
pub struct SqliteLocalProvider {
    conn: Mutex<Connection>,
}

impl std::fmt::Debug for SqliteLocalProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqliteLocalProvider").finish_non_exhaustive()
    }
}

impl SqliteLocalProvider {
    /// Open or create the SQLite file at `path` and run the schema bootstrap.
    ///
    /// # Errors
    /// - [`MemoryError::Sqlite`] if the file cannot be opened or the schema
    ///   migration fails.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, MemoryError> {
        let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        let conn = Connection::open_with_flags(path, flags)?;
        Self::bootstrap(&conn)?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// Open an in-memory SQLite store (testing).
    ///
    /// # Errors
    /// - [`MemoryError::Sqlite`] if the schema migration fails.
    pub fn open_in_memory() -> Result<Self, MemoryError> {
        let conn = Connection::open_in_memory()?;
        Self::bootstrap(&conn)?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    fn bootstrap(conn: &Connection) -> Result<(), MemoryError> {
        conn.execute_batch(SCHEMA_SQL)?;
        Ok(())
    }

    fn row_to_record(
        id: String,
        agent_id: String,
        content: String,
        tags_json: String,
        created_at_unix_ms: i64,
        metadata_json: String,
    ) -> Result<MemoryRecord, MemoryError> {
        let tags: Vec<String> = serde_json::from_str(&tags_json)?;
        let metadata: serde_json::Value = serde_json::from_str(&metadata_json)?;
        let created = u64::try_from(created_at_unix_ms).unwrap_or(0);
        Ok(MemoryRecord {
            id,
            agent_id,
            content,
            tags,
            created_at_unix_ms: created,
            metadata,
        })
    }
}

#[async_trait]
impl MemoryProvider for SqliteLocalProvider {
    async fn add(&self, mut rec: MemoryRecord) -> Result<String, MemoryError> {
        if rec.id.is_empty() {
            rec.id = Uuid::new_v4().to_string();
        }
        let tags_json = serde_json::to_string(&rec.tags)?;
        let metadata_json = serde_json::to_string(&rec.metadata)?;
        // The created_at_unix_ms field saturates at u64::MAX which exceeds the
        // SQLite INTEGER positive range (i64::MAX = ~292 million years); clamp.
        let created = i64::try_from(rec.created_at_unix_ms).unwrap_or(i64::MAX);
        let id_for_return = rec.id.clone();
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT OR REPLACE INTO memory_records \
             (id, agent_id, content, tags_json, created_at_unix_ms, metadata_json) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                rec.id,
                rec.agent_id,
                rec.content,
                tags_json,
                created,
                metadata_json,
            ],
        )?;
        Ok(id_for_return)
    }

    async fn get(&self, id: &str) -> Result<Option<MemoryRecord>, MemoryError> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, agent_id, content, tags_json, created_at_unix_ms, metadata_json \
             FROM memory_records WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id])?;
        match rows.next()? {
            Some(row) => {
                let r = Self::row_to_record(
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                )?;
                Ok(Some(r))
            }
            None => Ok(None),
        }
    }

    async fn search(&self, q: MemoryQuery) -> Result<Vec<MemoryRecord>, MemoryError> {
        // Build the SQL dynamically with bound params (no string interpolation
        // of user input — every value goes through `params!`).
        let mut sql = String::from(
            "SELECT id, agent_id, content, tags_json, created_at_unix_ms, metadata_json \
             FROM memory_records WHERE agent_id = ?1",
        );
        // We accumulate bind parameters as boxed dyn ToSql so they can outlive
        // the conditional branches below.
        let mut binds: Vec<Box<dyn rusqlite::ToSql + Send>> = vec![Box::new(q.agent_id.clone())];

        if let Some(ref needle) = q.q {
            sql.push_str(" AND content LIKE ?");
            sql.push_str(&(binds.len() + 1).to_string());
            // Wrap in % for substring/contains match. Case-insensitive on
            // ASCII via SQLite's default LIKE behaviour.
            binds.push(Box::new(format!("%{needle}%")));
        }

        for tag in &q.tags {
            sql.push_str(" AND tags_json LIKE ?");
            sql.push_str(&(binds.len() + 1).to_string());
            // Match JSON array element by quoting: `["foo","bar"]` contains
            // `"foo"` literally including the surrounding double quotes.
            binds.push(Box::new(format!("%\"{tag}\"%")));
        }

        sql.push_str(" ORDER BY created_at_unix_ms DESC");

        if let Some(limit) = q.limit {
            sql.push_str(" LIMIT ?");
            sql.push_str(&(binds.len() + 1).to_string());
            binds.push(Box::new(i64::from(limit)));
        }

        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(&sql)?;
        let bind_refs: Vec<&dyn rusqlite::ToSql> =
            binds.iter().map(|b| b.as_ref() as &dyn rusqlite::ToSql).collect();
        let mut rows = stmt.query(bind_refs.as_slice())?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let r = Self::row_to_record(
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            )?;
            out.push(r);
        }
        Ok(out)
    }

    async fn delete(&self, id: &str) -> Result<(), MemoryError> {
        let conn = self.conn.lock().await;
        conn.execute("DELETE FROM memory_records WHERE id = ?1", params![id])?;
        Ok(())
    }

    async fn health(&self) -> Result<(), MemoryError> {
        let conn = self.conn.lock().await;
        // Cheap round-trip — must succeed if the connection is alive.
        let n: i64 = conn.query_row("SELECT 1", [], |row| row.get(0))?;
        if n == 1 {
            Ok(())
        } else {
            Err(MemoryError::Unexpected(format!("health check returned {n}")))
        }
    }

    fn provider_kind(&self) -> ProviderKind {
        ProviderKind::SqliteLocal
    }
}
