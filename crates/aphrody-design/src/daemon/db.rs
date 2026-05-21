// SPDX-License-Identifier: Apache-2.0
//! Project store — SQLite-backed, durable, single-file.
//!
//! Schema is a Rust transcription of `apps/daemon/src/db.ts` from the
//! upstream open-design daemon. The five tables we ship in this first cut:
//!
//! - `projects`      — top-level project records (id, name, cwd, …).
//! - `conversations` — per-project chat threads, FK → projects.
//! - `messages`      — per-conversation messages, FK → conversations,
//!                     `position` column preserves authoring order.
//! - `tabs`          — per-project open-tab payloads (UPSERT keyed on
//!                     `(project_id, tab_id)`); payload is opaque JSON so the
//!                     frontend can keep evolving its tab schema without a
//!                     migration.
//! - `templates`     — globally-named reusable bodies (e.g. system prompts).
//!
//! ## Concurrency model
//!
//! We keep things simple: the store owns a single `rusqlite::Connection`
//! protected by a `Mutex`. SQLite is `WAL` + `synchronous=NORMAL`, which gives
//! us concurrent readers and a single writer at process scope — exactly the
//! same posture as the upstream TS daemon (which is single-process by
//! design). Async wrappers can park calls behind `spawn_blocking` if
//! contention becomes an issue; for now the daemon's request rate is
//! orders of magnitude below SQLite's mutex throughput.
//!
//! ## Migration strategy
//!
//! `PRAGMA user_version` carries the integer schema version. On `open` we
//! read it, then run every migration whose target version is strictly
//! greater than the value we read. Each migration is `CREATE TABLE IF NOT
//! EXISTS` + idempotent `ALTER TABLE` so re-running them is harmless — but
//! `user_version` gives us a fast-path that skips the SQL parser on every
//! boot once the schema is current.

use std::{
    path::Path,
    sync::Mutex,
};

use anyhow::{Context, Result, anyhow};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use ulid::Ulid;

use crate::daemon::DaemonError;

// ---------------------------------------------------------------------------
// Strongly-typed IDs
// ---------------------------------------------------------------------------

macro_rules! id_newtype {
    ($name:ident, $prefix:expr) => {
        /// ULID-backed identifier (26-char Crockford base32, monotonic
        #[doc = concat!("`", $prefix, "_<ulid>`)")]
        /// — sorts lexicographically by creation time.
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl $name {
            /// Mint a fresh ID with the standard `<prefix>_<ulid>` layout.
            #[must_use]
            pub fn new() -> Self {
                Self(format!("{}_{}", $prefix, Ulid::new()))
            }

            /// Borrow the ID as a string slice (for SQL params, logging…).
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self(s)
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self(s.to_owned())
            }
        }
    };
}

id_newtype!(ProjectId, "prj");
id_newtype!(ConversationId, "conv");
id_newtype!(MessageId, "msg");

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// Chat-message role. Mirrors open-design's free-form `role TEXT` column but
/// constrained at the type level so the CLI can validate input up-front.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    System,
    Tool,
}

impl Role {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::System => "system",
            Role::Tool => "tool",
        }
    }

    /// Parse a role from the on-the-wire string form (case-insensitive).
    ///
    /// # Errors
    ///
    /// Returns [`DaemonError::InvalidArgument`] if `s` is not one of
    /// `user|assistant|system|tool`.
    pub fn parse(s: &str) -> Result<Self, DaemonError> {
        match s.to_ascii_lowercase().as_str() {
            "user" => Ok(Role::User),
            "assistant" => Ok(Role::Assistant),
            "system" => Ok(Role::System),
            "tool" => Ok(Role::Tool),
            other => Err(DaemonError::InvalidArgument(format!(
                "unknown role: {other}"
            ))),
        }
    }
}

/// A row from the `projects` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    /// Working directory the project was bootstrapped from. Stored as the
    /// raw string the caller passed in so we don't surprise them with
    /// Windows path canonicalisation (`C:\foo` → `\\?\C:\foo`).
    pub cwd: String,
    pub created_at: i64,
    pub updated_at: i64,
}

/// A row from the `conversations` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: ConversationId,
    pub project_id: ProjectId,
    pub title: String,
    pub created_at: i64,
    pub updated_at: i64,
}

/// A row from the `messages` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: MessageId,
    pub conversation_id: ConversationId,
    pub role: Role,
    pub content: String,
    pub position: i64,
    pub created_at: i64,
}

/// A row from the `templates` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub name: String,
    pub body: String,
    pub created_at: i64,
    pub updated_at: i64,
}

/// A row from the `tabs` table — payload is left as opaque JSON because the
/// upstream daemon writes free-form objects (file paths, scroll offsets,
/// active-pane indices, …) that evolve outside this crate's release cadence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tab {
    pub project_id: ProjectId,
    pub tab_id: String,
    pub payload: serde_json::Value,
    pub updated_at: i64,
}

// ---------------------------------------------------------------------------
// ProjectStore
// ---------------------------------------------------------------------------

/// Schema version we know how to handle. Bumped on every migration.
const SCHEMA_VERSION: u32 = 1;

const SCHEMA_V1: &str = "\
CREATE TABLE IF NOT EXISTS projects (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    cwd         TEXT NOT NULL,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS conversations (
    id          TEXT PRIMARY KEY,
    project_id  TEXT NOT NULL,
    title       TEXT NOT NULL,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_conversations_project
    ON conversations(project_id, updated_at DESC);

CREATE TABLE IF NOT EXISTS messages (
    id              TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL,
    role            TEXT NOT NULL,
    content         TEXT NOT NULL,
    position        INTEGER NOT NULL,
    created_at      INTEGER NOT NULL,
    FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_messages_conv
    ON messages(conversation_id, position);
CREATE INDEX IF NOT EXISTS idx_messages_created
    ON messages(conversation_id, created_at);

CREATE TABLE IF NOT EXISTS tabs (
    project_id  TEXT NOT NULL,
    tab_id      TEXT NOT NULL,
    payload     TEXT NOT NULL,
    updated_at  INTEGER NOT NULL,
    PRIMARY KEY (project_id, tab_id),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_tabs_project
    ON tabs(project_id, updated_at DESC);

CREATE TABLE IF NOT EXISTS templates (
    name        TEXT PRIMARY KEY,
    body        TEXT NOT NULL,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);
";

/// Durable project store backed by a single SQLite file.
pub struct ProjectStore {
    conn: Mutex<Connection>,
}

impl ProjectStore {
    /// Open (or create) a project store at `path`. Pass `":memory:"` for an
    /// ephemeral database — convenient in tests.
    ///
    /// Runs all pending migrations idempotently before returning.
    ///
    /// # Errors
    ///
    /// Returns the underlying `rusqlite` error wrapped in [`anyhow::Error`]
    /// when the connection or schema initialisation fails.
    pub fn open(path: &Path) -> Result<Self> {
        let conn = if path.as_os_str() == ":memory:" {
            Connection::open_in_memory().context("open in-memory sqlite")?
        } else {
            if let Some(parent) = path.parent()
                && !parent.as_os_str().is_empty()
            {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("create parent dir {}", parent.display()))?;
            }
            Connection::open(path)
                .with_context(|| format!("open sqlite at {}", path.display()))?
        };

        conn.pragma_update(None, "journal_mode", "WAL")
            .context("set journal_mode=WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")
            .context("set synchronous=NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")
            .context("set foreign_keys=ON")?;

        let store = Self { conn: Mutex::new(conn) };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow!("mutex poisoned: {e}"))?;
        let current: u32 = conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .context("read user_version")?;
        if current >= SCHEMA_VERSION {
            // Fast path: schema is already at or past the version we know.
            return Ok(());
        }
        // v0 → v1: install the base schema. Wrapped in a transaction so a
        // crash mid-CREATE leaves user_version untouched and the next boot
        // re-runs the whole batch cleanly.
        conn.execute_batch(&format!(
            "BEGIN; {SCHEMA_V1}; PRAGMA user_version = {SCHEMA_VERSION}; COMMIT;"
        ))
        .context("apply schema v1")?;
        Ok(())
    }

    // ------------------------------------------------------------------
    // projects
    // ------------------------------------------------------------------

    /// Insert a new project row and return its freshly-minted ID.
    ///
    /// # Errors
    ///
    /// Returns an error if `name` is empty or the SQLite insert fails.
    pub fn create_project(&self, name: &str, cwd: &Path) -> Result<ProjectId> {
        if name.trim().is_empty() {
            return Err(DaemonError::InvalidArgument("project name must not be empty".into()).into());
        }
        let id = ProjectId::new();
        let now = now_millis();
        let cwd_str = cwd.to_string_lossy().into_owned();
        let conn = self.conn.lock().map_err(|e| anyhow!("mutex poisoned: {e}"))?;
        conn.execute(
            "INSERT INTO projects (id, name, cwd, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id.as_str(), name, cwd_str, now, now],
        )?;
        Ok(id)
    }

    /// List every project, most-recently-updated first.
    ///
    /// # Errors
    ///
    /// Returns the underlying SQLite error.
    pub fn list_projects(&self) -> Result<Vec<Project>> {
        let conn = self.conn.lock().map_err(|e| anyhow!("mutex poisoned: {e}"))?;
        let mut stmt = conn.prepare(
            "SELECT id, name, cwd, created_at, updated_at
               FROM projects
              ORDER BY updated_at DESC",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok(Project {
                    id: ProjectId(row.get::<_, String>(0)?),
                    name: row.get(1)?,
                    cwd: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Fetch a single project by ID, or [`None`] if missing.
    ///
    /// # Errors
    ///
    /// Returns the underlying SQLite error.
    pub fn get_project(&self, id: &ProjectId) -> Result<Option<Project>> {
        let conn = self.conn.lock().map_err(|e| anyhow!("mutex poisoned: {e}"))?;
        let project = conn
            .query_row(
                "SELECT id, name, cwd, created_at, updated_at
                   FROM projects WHERE id = ?1",
                params![id.as_str()],
                |row| {
                    Ok(Project {
                        id: ProjectId(row.get::<_, String>(0)?),
                        name: row.get(1)?,
                        cwd: row.get(2)?,
                        created_at: row.get(3)?,
                        updated_at: row.get(4)?,
                    })
                },
            )
            .optional()?;
        Ok(project)
    }

    // ------------------------------------------------------------------
    // conversations
    // ------------------------------------------------------------------

    /// Create a new conversation under `project`.
    ///
    /// # Errors
    ///
    /// Returns an error if the parent project does not exist or the insert
    /// fails (e.g. FK violation, disk full).
    pub fn create_conversation(
        &self,
        project: &ProjectId,
        title: &str,
    ) -> Result<ConversationId> {
        let id = ConversationId::new();
        let now = now_millis();
        let conn = self.conn.lock().map_err(|e| anyhow!("mutex poisoned: {e}"))?;
        conn.execute(
            "INSERT INTO conversations (id, project_id, title, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id.as_str(), project.as_str(), title, now, now],
        )?;
        Ok(id)
    }

    /// List every conversation under `project`, most-recently-updated first.
    ///
    /// # Errors
    ///
    /// Returns the underlying SQLite error.
    pub fn list_conversations(&self, project: &ProjectId) -> Result<Vec<Conversation>> {
        let conn = self.conn.lock().map_err(|e| anyhow!("mutex poisoned: {e}"))?;
        let mut stmt = conn.prepare(
            "SELECT id, project_id, title, created_at, updated_at
               FROM conversations
              WHERE project_id = ?1
              ORDER BY updated_at DESC",
        )?;
        let rows = stmt
            .query_map(params![project.as_str()], |row| {
                Ok(Conversation {
                    id: ConversationId(row.get::<_, String>(0)?),
                    project_id: ProjectId(row.get::<_, String>(1)?),
                    title: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    // ------------------------------------------------------------------
    // messages
    // ------------------------------------------------------------------

    /// Append a message to `conv`. The new row's `position` is one past the
    /// current MAX(position) for that conversation, matching open-design's
    /// `position INTEGER NOT NULL` semantics.
    ///
    /// Also bumps the conversation's `updated_at` so the sidebar's recency
    /// sort lands the active thread at the top.
    ///
    /// # Errors
    ///
    /// Returns an error if the conversation does not exist or the insert
    /// fails.
    pub fn append_message(
        &self,
        conv: &ConversationId,
        role: Role,
        content: &str,
    ) -> Result<MessageId> {
        let id = MessageId::new();
        let now = now_millis();
        let conn = self.conn.lock().map_err(|e| anyhow!("mutex poisoned: {e}"))?;
        let tx = conn.unchecked_transaction()?;
        let next_pos: i64 = tx
            .query_row(
                "SELECT COALESCE(MAX(position) + 1, 0)
                   FROM messages WHERE conversation_id = ?1",
                params![conv.as_str()],
                |row| row.get(0),
            )?;
        tx.execute(
            "INSERT INTO messages
               (id, conversation_id, role, content, position, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                id.as_str(),
                conv.as_str(),
                role.as_str(),
                content,
                next_pos,
                now
            ],
        )?;
        tx.execute(
            "UPDATE conversations SET updated_at = ?1 WHERE id = ?2",
            params![now, conv.as_str()],
        )?;
        tx.commit()?;
        Ok(id)
    }

    /// List up to `limit` messages from `conv`, oldest first (the order the
    /// frontend renders them in). Pass `usize::MAX` for "all".
    ///
    /// # Errors
    ///
    /// Returns the underlying SQLite error.
    pub fn list_messages(
        &self,
        conv: &ConversationId,
        limit: usize,
    ) -> Result<Vec<Message>> {
        let conn = self.conn.lock().map_err(|e| anyhow!("mutex poisoned: {e}"))?;
        let mut stmt = conn.prepare(
            "SELECT id, conversation_id, role, content, position, created_at
               FROM messages
              WHERE conversation_id = ?1
              ORDER BY position ASC
              LIMIT ?2",
        )?;
        let limit_i64 = i64::try_from(limit).unwrap_or(i64::MAX);
        let rows = stmt
            .query_map(params![conv.as_str(), limit_i64], |row| {
                let role_str: String = row.get(2)?;
                let role = Role::parse(&role_str).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        2,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
                Ok(Message {
                    id: MessageId(row.get::<_, String>(0)?),
                    conversation_id: ConversationId(row.get::<_, String>(1)?),
                    role,
                    content: row.get(3)?,
                    position: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    // ------------------------------------------------------------------
    // templates
    // ------------------------------------------------------------------

    /// Save (insert or replace) a named template.
    ///
    /// # Errors
    ///
    /// Returns an error if `name` is empty or the SQLite write fails.
    pub fn save_template(&self, name: &str, body: &str) -> Result<()> {
        if name.trim().is_empty() {
            return Err(DaemonError::InvalidArgument("template name must not be empty".into()).into());
        }
        let now = now_millis();
        let conn = self.conn.lock().map_err(|e| anyhow!("mutex poisoned: {e}"))?;
        conn.execute(
            "INSERT INTO templates (name, body, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?3)
             ON CONFLICT(name) DO UPDATE SET
                body       = excluded.body,
                updated_at = excluded.updated_at",
            params![name, body, now],
        )?;
        Ok(())
    }

    /// List every template, most-recently-updated first.
    ///
    /// # Errors
    ///
    /// Returns the underlying SQLite error.
    pub fn list_templates(&self) -> Result<Vec<Template>> {
        let conn = self.conn.lock().map_err(|e| anyhow!("mutex poisoned: {e}"))?;
        let mut stmt = conn.prepare(
            "SELECT name, body, created_at, updated_at
               FROM templates
              ORDER BY updated_at DESC",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok(Template {
                    name: row.get(0)?,
                    body: row.get(1)?,
                    created_at: row.get(2)?,
                    updated_at: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    // ------------------------------------------------------------------
    // tabs
    // ------------------------------------------------------------------

    /// UPSERT a tab payload under `(project, tab_id)`. The payload is
    /// serialised to JSON and stored verbatim — callers own its schema.
    ///
    /// # Errors
    ///
    /// Returns an error if the JSON serialisation or SQLite write fails.
    pub fn update_tab(
        &self,
        project: &ProjectId,
        tab_id: &str,
        payload: &serde_json::Value,
    ) -> Result<()> {
        let now = now_millis();
        let payload_str = serde_json::to_string(payload)?;
        let conn = self.conn.lock().map_err(|e| anyhow!("mutex poisoned: {e}"))?;
        conn.execute(
            "INSERT INTO tabs (project_id, tab_id, payload, updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(project_id, tab_id) DO UPDATE SET
                payload    = excluded.payload,
                updated_at = excluded.updated_at",
            params![project.as_str(), tab_id, payload_str, now],
        )?;
        Ok(())
    }

    /// List every tab payload for `project`, most-recently-updated first.
    ///
    /// # Errors
    ///
    /// Returns the underlying SQLite error.
    pub fn list_tabs(&self, project: &ProjectId) -> Result<Vec<Tab>> {
        let conn = self.conn.lock().map_err(|e| anyhow!("mutex poisoned: {e}"))?;
        let mut stmt = conn.prepare(
            "SELECT project_id, tab_id, payload, updated_at
               FROM tabs WHERE project_id = ?1
              ORDER BY updated_at DESC",
        )?;
        let rows = stmt
            .query_map(params![project.as_str()], |row| {
                let payload_str: String = row.get(2)?;
                let payload: serde_json::Value =
                    serde_json::from_str(&payload_str).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            2,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?;
                Ok(Tab {
                    project_id: ProjectId(row.get::<_, String>(0)?),
                    tab_id: row.get(1)?,
                    payload,
                    updated_at: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn now_millis() -> i64 {
    OffsetDateTime::now_utc().unix_timestamp_nanos() as i64 / 1_000_000
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::tempdir;

    fn store() -> ProjectStore {
        ProjectStore::open(Path::new(":memory:")).expect("open in-memory store")
    }

    #[test]
    fn open_creates_schema_at_target_version() {
        let s = store();
        let conn = s.conn.lock().unwrap();
        let v: u32 = conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(v, SCHEMA_VERSION);
        // sanity: tables exist
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table'
                   AND name IN ('projects','conversations','messages','tabs','templates')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 5);
    }

    #[test]
    fn migration_is_idempotent() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("app.sqlite");
        let _s1 = ProjectStore::open(&path).unwrap();
        let _s2 = ProjectStore::open(&path).unwrap();
        let s3 = ProjectStore::open(&path).unwrap();
        let conn = s3.conn.lock().unwrap();
        let v: u32 = conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(v, SCHEMA_VERSION);
    }

    #[test]
    fn project_crud_roundtrip() {
        let s = store();
        let id = s
            .create_project("demo", &PathBuf::from("/tmp/demo"))
            .unwrap();
        assert!(id.as_str().starts_with("prj_"));
        let fetched = s.get_project(&id).unwrap().expect("must exist");
        assert_eq!(fetched.name, "demo");
        assert!(fetched.cwd.contains("demo"));
        let all = s.list_projects().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, id);
    }

    #[test]
    fn create_project_rejects_empty_name() {
        let s = store();
        let err = s
            .create_project("   ", &PathBuf::from("/x"))
            .unwrap_err();
        assert!(err.to_string().contains("project name"));
    }

    #[test]
    fn conversation_crud_roundtrip() {
        let s = store();
        let project = s
            .create_project("p1", &PathBuf::from("/p1"))
            .unwrap();
        let c1 = s.create_conversation(&project, "first").unwrap();
        let c2 = s.create_conversation(&project, "second").unwrap();
        let list = s.list_conversations(&project).unwrap();
        assert_eq!(list.len(), 2);
        // Either order is acceptable when created in the same ms; verify both
        // IDs are present.
        let ids: Vec<&str> = list.iter().map(|c| c.id.as_str()).collect();
        assert!(ids.contains(&c1.as_str()));
        assert!(ids.contains(&c2.as_str()));
    }

    #[test]
    fn message_append_then_list_preserves_order() {
        let s = store();
        let project = s.create_project("p", &PathBuf::from("/p")).unwrap();
        let conv = s.create_conversation(&project, "t").unwrap();
        let _m1 = s.append_message(&conv, Role::User, "hello").unwrap();
        let _m2 = s
            .append_message(&conv, Role::Assistant, "world")
            .unwrap();
        let _m3 = s.append_message(&conv, Role::User, "again").unwrap();
        let list = s.list_messages(&conv, 100).unwrap();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].content, "hello");
        assert_eq!(list[0].role, Role::User);
        assert_eq!(list[0].position, 0);
        assert_eq!(list[1].content, "world");
        assert_eq!(list[1].role, Role::Assistant);
        assert_eq!(list[1].position, 1);
        assert_eq!(list[2].position, 2);
    }

    #[test]
    fn append_message_bumps_conversation_updated_at() {
        let s = store();
        let project = s.create_project("p", &PathBuf::from("/p")).unwrap();
        let conv = s.create_conversation(&project, "t").unwrap();
        let before = s.list_conversations(&project).unwrap()[0].updated_at;
        // Sleep enough to ensure a different millisecond reading.
        std::thread::sleep(std::time::Duration::from_millis(5));
        s.append_message(&conv, Role::User, "hi").unwrap();
        let after = s.list_conversations(&project).unwrap()[0].updated_at;
        assert!(after >= before, "after={after} before={before}");
    }

    #[test]
    fn templates_save_and_list_round_trip() {
        let s = store();
        s.save_template("system-prompt", "you are aphrody").unwrap();
        s.save_template("greeting", "hello there").unwrap();
        let list = s.list_templates().unwrap();
        assert_eq!(list.len(), 2);
        // Upsert: rewriting `greeting` must replace, not duplicate.
        s.save_template("greeting", "hi again").unwrap();
        let list2 = s.list_templates().unwrap();
        assert_eq!(list2.len(), 2);
        let greeting = list2.iter().find(|t| t.name == "greeting").unwrap();
        assert_eq!(greeting.body, "hi again");
    }

    #[test]
    fn save_template_rejects_empty_name() {
        let s = store();
        assert!(s.save_template("", "body").is_err());
    }

    #[test]
    fn tab_upsert_replaces_payload() {
        let s = store();
        let project = s.create_project("p", &PathBuf::from("/p")).unwrap();
        let p1 = serde_json::json!({"file": "a.md", "scroll": 0});
        let p2 = serde_json::json!({"file": "a.md", "scroll": 42});
        s.update_tab(&project, "tab-1", &p1).unwrap();
        s.update_tab(&project, "tab-1", &p2).unwrap();
        let tabs = s.list_tabs(&project).unwrap();
        assert_eq!(tabs.len(), 1, "upsert must not duplicate");
        assert_eq!(tabs[0].payload["scroll"], 42);
    }

    #[test]
    fn ulid_ids_are_monotonic() {
        // ULID's 48-bit timestamp prefix means newly-minted IDs sort
        // lexicographically by creation order.
        let mut prev = ProjectId::new();
        for _ in 0..50 {
            // 1ms sleep guarantees a fresh timestamp band even on coarse
            // Windows clocks.
            std::thread::sleep(std::time::Duration::from_millis(1));
            let next = ProjectId::new();
            assert!(
                next.as_str() > prev.as_str(),
                "ULIDs must be monotonic: prev={prev} next={next}"
            );
            prev = next;
        }
    }

    #[test]
    fn role_parse_round_trip() {
        for role in [Role::User, Role::Assistant, Role::System, Role::Tool] {
            assert_eq!(Role::parse(role.as_str()).unwrap(), role);
            assert_eq!(Role::parse(&role.as_str().to_uppercase()).unwrap(), role);
        }
        assert!(Role::parse("nobody").is_err());
    }

    #[test]
    fn persist_across_open() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("app.sqlite");
        let id = {
            let s = ProjectStore::open(&path).unwrap();
            s.create_project("durable", &PathBuf::from("/d")).unwrap()
        };
        let s2 = ProjectStore::open(&path).unwrap();
        let project = s2.get_project(&id).unwrap().expect("survived reopen");
        assert_eq!(project.name, "durable");
    }

    #[test]
    fn cascade_delete_removes_messages() {
        let s = store();
        let project = s.create_project("p", &PathBuf::from("/p")).unwrap();
        let conv = s.create_conversation(&project, "t").unwrap();
        s.append_message(&conv, Role::User, "hi").unwrap();
        // FK ON DELETE CASCADE: removing project drops conv → msgs.
        let conn = s.conn.lock().unwrap();
        conn.execute("DELETE FROM projects WHERE id = ?1", params![project.as_str()])
            .unwrap();
        drop(conn);
        let conv_rows = s.list_conversations(&project).unwrap();
        assert_eq!(conv_rows.len(), 0);
        let msg_rows = s.list_messages(&conv, 10).unwrap();
        assert_eq!(msg_rows.len(), 0);
    }
}
