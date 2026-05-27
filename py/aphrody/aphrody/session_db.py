# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2026 aphrody contributors
"""SQLite-backed session database with FTS5 search and triggers."""

import json
import logging
import random
import re
import sqlite3
import threading
import time
from collections.abc import Callable
from pathlib import Path
from typing import Any, TypeVar

T = TypeVar("T")
logger = logging.getLogger(__name__)

DEFAULT_DB_PATH = (
    Path(__file__).resolve().parents[2] / "var" / "secrets" / "sessions.db"
)
SCHEMA_VERSION = 12
_WAL_INCOMPAT_MARKERS = ("locking protocol", "not authorized", "disk i/o error")

_wal_fallback_warned_lock = threading.Lock()
_wal_fallback_warned_paths = set()
_last_init_error = None


def get_last_init_error() -> str | None:
    """Retrieve the last recorded session database initialization error."""
    return _last_init_error


def _set_last_init_error(err: str | None) -> None:
    global _last_init_error
    _last_init_error = err


def apply_wal_with_fallback(
    conn: sqlite3.Connection,
    *,
    db_label: str = "state.db",
) -> str:
    """Set WAL mode on the connection, falling back to DELETE on failure."""
    try:
        conn.execute("PRAGMA journal_mode=WAL")
        return "wal"
    except sqlite3.OperationalError as exc:
        msg = str(exc).lower()
        if not any(marker in msg for marker in _WAL_INCOMPAT_MARKERS):
            raise
        _log_wal_fallback_once(db_label, exc)
        conn.execute("PRAGMA journal_mode=DELETE")
        return "delete"


def _log_wal_fallback_once(db_label: str, exc: Exception) -> None:
    with _wal_fallback_warned_lock:
        if db_label in _wal_fallback_warned_paths:
            return
        _wal_fallback_warned_paths.add(db_label)
    logger.warning(
        "%s: WAL journal_mode unsupported on this filesystem (%s) — "
        "falling back to journal_mode=DELETE (slower rollback-journal "
        "mode; reduces concurrency but works on NFS/SMB/FUSE).",
        db_label,
        exc,
    )


SCHEMA_SQL = """
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    source TEXT NOT NULL,
    user_id TEXT,
    model TEXT,
    model_config TEXT,
    system_prompt TEXT,
    parent_session_id TEXT,
    started_at REAL NOT NULL,
    ended_at REAL,
    end_reason TEXT,
    message_count INTEGER DEFAULT 0,
    tool_call_count INTEGER DEFAULT 0,
    input_tokens INTEGER DEFAULT 0,
    output_tokens INTEGER DEFAULT 0,
    cache_read_tokens INTEGER DEFAULT 0,
    cache_write_tokens INTEGER DEFAULT 0,
    reasoning_tokens INTEGER DEFAULT 0,
    billing_provider TEXT,
    billing_base_url TEXT,
    billing_mode TEXT,
    estimated_cost_usd REAL,
    actual_cost_usd REAL,
    cost_status TEXT,
    cost_source TEXT,
    pricing_version TEXT,
    title TEXT,
    api_call_count INTEGER DEFAULT 0,
    handoff_state TEXT,
    handoff_platform TEXT,
    handoff_error TEXT,
    FOREIGN KEY (parent_session_id) REFERENCES sessions(id)
);

CREATE TABLE IF NOT EXISTS messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL REFERENCES sessions(id),
    role TEXT NOT NULL,
    content TEXT,
    tool_call_id TEXT,
    tool_calls TEXT,
    tool_name TEXT,
    timestamp REAL NOT NULL,
    token_count INTEGER,
    finish_reason TEXT,
    reasoning TEXT,
    reasoning_content TEXT,
    reasoning_details TEXT,
    codex_reasoning_items TEXT,
    codex_message_items TEXT,
    platform_message_id TEXT
);

CREATE TABLE IF NOT EXISTS state_meta (
    key TEXT PRIMARY KEY,
    value TEXT
);

CREATE INDEX IF NOT EXISTS idx_sessions_source ON sessions(source);
CREATE INDEX IF NOT EXISTS idx_sessions_parent ON sessions(parent_session_id);
CREATE INDEX IF NOT EXISTS idx_sessions_started ON sessions(started_at DESC);
CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id, timestamp);
"""

FTS_SQL = """
CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
    content
);

CREATE TRIGGER IF NOT EXISTS messages_fts_insert AFTER INSERT ON messages BEGIN
    INSERT INTO messages_fts(rowid, content) VALUES (
        new.id,
        COALESCE(new.content, '') || ' ' || COALESCE(new.tool_name, '') || ' ' || COALESCE(new.tool_calls, '')
    );
END;

CREATE TRIGGER IF NOT EXISTS messages_fts_delete AFTER DELETE ON messages BEGIN
    DELETE FROM messages_fts WHERE rowid = old.id;
END;

CREATE TRIGGER IF NOT EXISTS messages_fts_update AFTER UPDATE ON messages BEGIN
    DELETE FROM messages_fts WHERE rowid = old.id;
    INSERT INTO messages_fts(rowid, content) VALUES (
        new.id,
        COALESCE(new.content, '') || ' ' || COALESCE(new.tool_name, '') || ' ' || COALESCE(new.tool_calls, '')
    );
END;
"""

FTS_TRIGRAM_SQL = """
CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts_trigram USING fts5(
    content,
    tokenize='trigram'
);

CREATE TRIGGER IF NOT EXISTS messages_fts_trigram_insert AFTER INSERT ON messages BEGIN
    INSERT INTO messages_fts_trigram(rowid, content) VALUES (
        new.id,
        COALESCE(new.content, '') || ' ' || COALESCE(new.tool_name, '') || ' ' || COALESCE(new.tool_calls, '')
    );
END;

CREATE TRIGGER IF NOT EXISTS messages_fts_trigram_delete AFTER DELETE ON messages BEGIN
    DELETE FROM messages_fts_trigram WHERE rowid = old.id;
END;

CREATE TRIGGER IF NOT EXISTS messages_fts_trigram_update AFTER UPDATE ON messages BEGIN
    DELETE FROM messages_fts_trigram WHERE rowid = old.id;
    INSERT INTO messages_fts_trigram(rowid, content) VALUES (
        new.id,
        COALESCE(new.content, '') || ' ' || COALESCE(new.tool_name, '') || ' ' || COALESCE(new.tool_calls, '')
    );
END;
"""


class SessionDB:
    """SQLite-backed session storage with FTS5 search."""

    _WRITE_MAX_RETRIES = 15
    _WRITE_RETRY_MIN_S = 0.020
    _WRITE_RETRY_MAX_S = 0.150
    _CHECKPOINT_EVERY_N_WRITES = 50
    _CONTENT_JSON_PREFIX = "\x00json:"

    def __init__(self, db_path: Path | None = None):
        self.db_path = db_path or DEFAULT_DB_PATH
        self.db_path.parent.mkdir(parents=True, exist_ok=True)

        self._lock = threading.Lock()
        self._write_count = 0
        try:
            self._conn = sqlite3.connect(
                str(self.db_path),
                check_same_thread=False,
                timeout=1.0,
                isolation_level=None,
            )
            self._conn.row_factory = sqlite3.Row
            apply_wal_with_fallback(self._conn, db_label="sessions.db")
            self._conn.execute("PRAGMA foreign_keys=ON")
            self._init_schema()
        except Exception as exc:
            _set_last_init_error(f"{type(exc).__name__}: {exc}")
            raise

    def _execute_write(self, fn: Callable[[sqlite3.Connection], T]) -> T:
        last_err: Exception | None = None
        for attempt in range(self._WRITE_MAX_RETRIES):
            try:
                with self._lock:
                    self._conn.execute("BEGIN IMMEDIATE")
                    try:
                        result = fn(self._conn)
                        self._conn.commit()
                    except BaseException:
                        try:
                            self._conn.rollback()
                        except Exception:
                            pass
                        raise
                self._write_count += 1
                if self._write_count % self._CHECKPOINT_EVERY_N_WRITES == 0:
                    self._try_wal_checkpoint()
                return result
            except sqlite3.OperationalError as exc:
                err_msg = str(exc).lower()
                if "locked" in err_msg or "busy" in err_msg:
                    last_err = exc
                    if attempt < self._WRITE_MAX_RETRIES - 1:
                        jitter = random.uniform(
                            self._WRITE_RETRY_MIN_S,
                            self._WRITE_RETRY_MAX_S,
                        )
                        time.sleep(jitter)
                        continue
                raise
        raise last_err or sqlite3.OperationalError(
            "database is locked after max retries"
        )

    def _try_wal_checkpoint(self) -> None:
        try:
            with self._lock:
                self._conn.execute("PRAGMA wal_checkpoint(PASSIVE)")
        except Exception:
            pass

    def close(self):
        """Close the session database connection."""
        with self._lock:
            if self._conn:
                try:
                    self._conn.execute("PRAGMA wal_checkpoint(PASSIVE)")
                except Exception:
                    pass
                self._conn.close()
                self._conn = None

    @staticmethod
    def _parse_schema_columns(schema_sql: str) -> dict[str, dict[str, str]]:
        ref = sqlite3.connect(":memory:")
        try:
            ref.executescript(schema_sql)
            table_columns: dict[str, dict[str, str]] = {}
            for (tbl,) in ref.execute(
                "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'"
            ).fetchall():
                cols: dict[str, str] = {}
                for row in ref.execute(
                    f'PRAGMA table_info("{tbl}")'
                ).fetchall():
                    col_name = row[1]
                    col_type = row[2] or ""
                    notnull = row[3]
                    default = row[4]
                    pk = row[5]
                    parts = [col_type] if col_type else []
                    if notnull and not pk:
                        parts.append("NOT NULL")
                    if default is not None:
                        parts.append(f"DEFAULT {default}")
                    cols[col_name] = " ".join(parts)
                table_columns[tbl] = cols
            return table_columns
        finally:
            ref.close()

    def _reconcile_columns(self, cursor: sqlite3.Cursor) -> None:
        expected = self._parse_schema_columns(SCHEMA_SQL)
        for table_name, declared_cols in expected.items():
            try:
                rows = cursor.execute(
                    f'PRAGMA table_info("{table_name}")'
                ).fetchall()
            except sqlite3.OperationalError:
                continue
            live_cols = set()
            for row in rows:
                name = row[1] if isinstance(row, (tuple, list)) else row["name"]
                live_cols.add(name)

            for col_name, col_type in declared_cols.items():
                if col_name not in live_cols:
                    safe_name = col_name.replace('"', '""')
                    try:
                        cursor.execute(
                            f'ALTER TABLE "{table_name}" ADD COLUMN "{safe_name}" {col_type}'
                        )
                    except sqlite3.OperationalError as exc:
                        logger.debug(
                            "reconcile %s.%s: %s", table_name, col_name, exc
                        )

    def _init_schema(self):
        cursor = self._conn.cursor()
        cursor.executescript(SCHEMA_SQL)
        cursor.executescript(FTS_SQL)
        cursor.executescript(FTS_TRIGRAM_SQL)
        self._reconcile_columns(cursor)

        try:
            cursor.execute(
                "CREATE INDEX IF NOT EXISTS idx_messages_platform_msg_id "
                "ON messages(session_id, platform_message_id) "
                "WHERE platform_message_id IS NOT NULL"
            )
        except sqlite3.OperationalError as exc:
            logger.debug("idx_messages_platform_msg_id create skipped: %s", exc)

        cursor.execute("SELECT version FROM schema_version LIMIT 1")
        row = cursor.fetchone()
        if row is None:
            cursor.execute(
                "INSERT INTO schema_version (version) VALUES (?)",
                (SCHEMA_VERSION,),
            )
        else:
            current_version = (
                row["version"] if isinstance(row, sqlite3.Row) else row[0]
            )
            if current_version < SCHEMA_VERSION:
                cursor.execute(
                    "UPDATE schema_version SET version = ?",
                    (SCHEMA_VERSION,),
                )

    def create_session(self, session_id: str, source: str, **kwargs) -> str:
        """Create a new session record.

        Args:
            session_id: Unique identifier for the session.
            source: Source metadata (e.g. cli, voice).
            **kwargs: Additional session fields.

        Returns:
            The created session_id.
        """
        self._insert_session_row(session_id, source, **kwargs)
        return session_id

    def _insert_session_row(
        self,
        session_id: str,
        source: str,
        model: str | None = None,
        model_config: dict[str, Any] | None = None,
        system_prompt: str | None = None,
        user_id: str | None = None,
        parent_session_id: str | None = None,
    ) -> None:
        def _do(conn):
            conn.execute(
                """INSERT OR IGNORE INTO sessions (id, source, user_id, model, model_config,
                   system_prompt, parent_session_id, started_at)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?)""",
                (
                    session_id,
                    source,
                    user_id,
                    model,
                    json.dumps(model_config) if model_config else None,
                    system_prompt,
                    parent_session_id,
                    time.time(),
                ),
            )

        self._execute_write(_do)

    def append_message(
        self,
        session_id: str,
        role: str,
        content: str | None = None,
        tool_name: str | None = None,
        tool_calls: Any = None,
        tool_call_id: str | None = None,
        token_count: int | None = None,
        finish_reason: str | None = None,
        reasoning: str | None = None,
        reasoning_content: str | None = None,
        reasoning_details: Any = None,
        codex_reasoning_items: Any = None,
        codex_message_items: Any = None,
        platform_message_id: str | None = None,
    ) -> int:
        """Append a new message to the session.

        Saves metadata, content, and token counts, then increments
        the session's message counter.
        """
        reasoning_details_json = (
            json.dumps(reasoning_details) if reasoning_details else None
        )
        codex_items_json = (
            json.dumps(codex_reasoning_items) if codex_reasoning_items else None
        )
        codex_message_items_json = (
            json.dumps(codex_message_items) if codex_message_items else None
        )
        tool_calls_json = json.dumps(tool_calls) if tool_calls else None
        stored_content = self._encode_content(content)

        num_tool_calls = 0
        if tool_calls is not None:
            num_tool_calls = (
                len(tool_calls) if isinstance(tool_calls, list) else 1
            )

        def _do(conn):
            cursor = conn.execute(
                """INSERT INTO messages (session_id, role, content, tool_call_id,
                   tool_calls, tool_name, timestamp, token_count, finish_reason,
                   reasoning, reasoning_content, reasoning_details, codex_reasoning_items,
                   codex_message_items, platform_message_id)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)""",
                (
                    session_id,
                    role,
                    stored_content,
                    tool_call_id,
                    tool_calls_json,
                    tool_name,
                    time.time(),
                    token_count,
                    finish_reason,
                    reasoning,
                    reasoning_content,
                    reasoning_details_json,
                    codex_items_json,
                    codex_message_items_json,
                    platform_message_id,
                ),
            )
            msg_id = cursor.lastrowid

            if num_tool_calls > 0:
                conn.execute(
                    """UPDATE sessions SET message_count = message_count + 1,
                       tool_call_count = tool_call_count + ? WHERE id = ?""",
                    (num_tool_calls, session_id),
                )
            else:
                conn.execute(
                    "UPDATE sessions SET message_count = message_count + 1 WHERE id = ?",
                    (session_id,),
                )
            return msg_id

        return self._execute_write(_do)

    def get_messages(self, session_id: str) -> list[dict[str, Any]]:
        """Retrieve all messages for a session, ordered by id."""
        with self._lock:
            cursor = self._conn.execute(
                "SELECT * FROM messages WHERE session_id = ? ORDER BY id",
                (session_id,),
            )
            rows = cursor.fetchall()
        result = []
        for row in rows:
            msg = dict(row)
            if "content" in msg:
                msg["content"] = self._decode_content(msg["content"])
            if msg.get("tool_calls"):
                try:
                    msg["tool_calls"] = json.loads(msg["tool_calls"])
                except (json.JSONDecodeError, TypeError):
                    msg["tool_calls"] = []
            result.append(msg)
        return result

    def search_messages(
        self,
        query: str,
        source_filter: list[str] | None = None,
        exclude_sources: list[str] | None = None,
        role_filter: list[str] | None = None,
        limit: int = 20,
        offset: int = 0,
        sort: str | None = None,
    ) -> list[dict[str, Any]]:
        """Search messages using FTS5 (or LIKE fallback for short CJK)."""
        if not query or not query.strip():
            return []

        query = self._sanitize_fts5_query(query)
        if not query:
            return []

        if isinstance(sort, str):
            sort_norm = sort.strip().lower()
            if sort_norm not in ("newest", "oldest"):
                sort_norm = None
        else:
            sort_norm = None

        if sort_norm == "newest":
            order_by_sql = "ORDER BY m.timestamp DESC, rank"
        elif sort_norm == "oldest":
            order_by_sql = "ORDER BY m.timestamp ASC, rank"
        else:
            order_by_sql = "ORDER BY rank"

        where_clauses = ["messages_fts MATCH ?"]
        params: list = [query]

        if source_filter is not None:
            source_placeholders = ",".join("?" for _ in source_filter)
            where_clauses.append(f"s.source IN ({source_placeholders})")
            params.extend(source_filter)

        if exclude_sources is not None:
            exclude_placeholders = ",".join("?" for _ in exclude_sources)
            where_clauses.append(f"s.source NOT IN ({exclude_placeholders})")
            params.extend(exclude_sources)

        if role_filter:
            role_placeholders = ",".join("?" for _ in role_filter)
            where_clauses.append(f"m.role IN ({role_placeholders})")
            params.extend(role_filter)

        where_sql = " AND ".join(where_clauses)
        params.extend([limit, offset])

        sql = f"""
            SELECT
                m.id,
                m.session_id,
                m.role,
                snippet(messages_fts, 0, '>>>', '<<<', '...', 40) AS snippet,
                m.content,
                m.timestamp,
                m.tool_name,
                s.source,
                s.model,
                s.started_at AS session_started
            FROM messages_fts
            JOIN messages m ON m.id = messages_fts.rowid
            JOIN sessions s ON s.id = m.session_id
            WHERE {where_sql}
            {order_by_sql}
            LIMIT ? OFFSET ?
        """

        is_cjk = self._contains_cjk(query)
        if is_cjk:
            raw_query = query.strip('"').strip()
            cjk_count = self._count_cjk(raw_query)

            _tokens_for_check = [
                t
                for t in raw_query.split()
                if t.upper() not in {"AND", "OR", "NOT"}
                and self._contains_cjk(t)
            ]
            _any_short_cjk = any(
                self._count_cjk(t) < 3 for t in _tokens_for_check
            )

            if cjk_count >= 3 and not _any_short_cjk:
                tokens = raw_query.split()
                parts = []
                for tok in tokens:
                    if tok.upper() in {"AND", "OR", "NOT"}:
                        parts.append(tok)
                    else:
                        parts.append('"' + tok.replace('"', '""') + '"')
                trigram_query = " ".join(parts)
                tri_where = ["messages_fts_trigram MATCH ?"]
                tri_params: list = [trigram_query]
                if source_filter is not None:
                    tri_where.append(
                        f"s.source IN ({','.join('?' for _ in source_filter)})"
                    )
                    tri_params.extend(source_filter)
                if exclude_sources is not None:
                    tri_where.append(
                        f"s.source NOT IN ({','.join('?' for _ in exclude_sources)})"
                    )
                    tri_params.extend(exclude_sources)
                if role_filter:
                    tri_where.append(
                        f"m.role IN ({','.join('?' for _ in role_filter)})"
                    )
                    tri_params.extend(role_filter)
                tri_sql = f"""
                    SELECT
                        m.id,
                        m.session_id,
                        m.role,
                        snippet(messages_fts_trigram, 0, '>>>', '<<<', '...', 40) AS snippet,
                        m.content,
                        m.timestamp,
                        m.tool_name,
                        s.source,
                        s.model,
                        s.started_at AS session_started
                    FROM messages_fts_trigram
                    JOIN messages m ON m.id = messages_fts_trigram.rowid
                    JOIN sessions s ON s.id = m.session_id
                    WHERE {" AND ".join(tri_where)}
                    {order_by_sql}
                    LIMIT ? OFFSET ?
                """
                tri_params.extend([limit, offset])
                with self._lock:
                    try:
                        tri_cursor = self._conn.execute(tri_sql, tri_params)
                    except sqlite3.OperationalError:
                        matches = []
                    else:
                        matches = [dict(row) for row in tri_cursor.fetchall()]
            else:
                non_op_tokens = [
                    t
                    for t in raw_query.split()
                    if t.upper() not in {"AND", "OR", "NOT"}
                ] or [raw_query]
                token_clauses = []
                like_params: list = []
                for tok in non_op_tokens:
                    esc = (
                        tok.replace("\\", "\\\\")
                        .replace("%", "\\%")
                        .replace("_", "\\_")
                    )
                    token_clauses.append(
                        "(m.content LIKE ? ESCAPE '\\' OR m.tool_name LIKE ? ESCAPE '\\' OR m.tool_calls LIKE ? ESCAPE '\\')"
                    )
                    like_params += [f"%{esc}%", f"%{esc}%", f"%{esc}%"]
                like_where = [f"({' OR '.join(token_clauses)})"]
                if source_filter is not None:
                    like_where.append(
                        f"s.source IN ({','.join('?' for _ in source_filter)})"
                    )
                    like_params.extend(source_filter)
                if exclude_sources is not None:
                    like_where.append(
                        f"s.source NOT IN ({','.join('?' for _ in exclude_sources)})"
                    )
                    like_params.extend(exclude_sources)
                if role_filter:
                    like_where.append(
                        f"m.role IN ({','.join('?' for _ in role_filter)})"
                    )
                    like_params.extend(role_filter)
                like_sql = f"""
                    SELECT m.id, m.session_id, m.role,
                           substr(m.content,
                                  max(1, instr(m.content, ?) - 40),
                                  120) AS snippet,
                           m.content, m.timestamp, m.tool_name,
                           s.source, s.model, s.started_at AS session_started
                    FROM messages m
                    JOIN sessions s ON s.id = m.session_id
                    WHERE {" AND ".join(like_where)}
                    ORDER BY m.timestamp DESC
                    LIMIT ? OFFSET ?
                """
                like_params.extend([limit, offset])
                like_params = [non_op_tokens[0], *like_params]
                with self._lock:
                    like_cursor = self._conn.execute(like_sql, like_params)
                    matches = [dict(row) for row in like_cursor.fetchall()]
        else:
            with self._lock:
                try:
                    cursor = self._conn.execute(sql, params)
                except sqlite3.OperationalError:
                    return []
                else:
                    matches = [dict(row) for row in cursor.fetchall()]

        for match in matches:
            try:
                with self._lock:
                    ctx_cursor = self._conn.execute(
                        """WITH target AS (
                               SELECT session_id, timestamp, id
                               FROM messages
                               WHERE id = ?
                           )
                           SELECT role, content
                           FROM (
                               SELECT m.id, m.timestamp, m.role, m.content
                               FROM messages m
                               JOIN target t ON t.session_id = m.session_id
                               WHERE (m.timestamp < t.timestamp)
                                  OR (m.timestamp = t.timestamp AND m.id < t.id)
                               ORDER BY m.timestamp DESC, m.id DESC
                               LIMIT 1
                           )
                           UNION ALL
                           SELECT role, content
                           FROM messages
                           WHERE id = ?
                           UNION ALL
                           SELECT role, content
                           FROM (
                               SELECT m.id, m.timestamp, m.role, m.content
                               FROM messages m
                               JOIN target t ON t.session_id = m.session_id
                               WHERE (m.timestamp > t.timestamp)
                                  OR (m.timestamp = t.timestamp AND m.id > t.id)
                               ORDER BY m.timestamp ASC, m.id ASC
                               LIMIT 1
                           )""",
                        (match["id"], match["id"]),
                    )
                    context_msgs = []
                    for r in ctx_cursor.fetchall():
                        raw = r["content"]
                        decoded = self._decode_content(raw)
                        if isinstance(decoded, list):
                            text_parts = [
                                p.get("text", "")
                                for p in decoded
                                if isinstance(p, dict)
                                and p.get("type") == "text"
                            ]
                            text = " ".join(t for t in text_parts if t).strip()
                            preview = text or "[multimodal content]"
                        elif isinstance(decoded, str):
                            preview = decoded
                        else:
                            preview = ""
                        context_msgs.append(
                            {"role": r["role"], "content": preview[:200]}
                        )
                match["context"] = context_msgs
            except Exception:
                match["context"] = []

        for match in matches:
            match.pop("content", None)

        return matches

    @classmethod
    def _encode_content(cls, content: Any) -> Any:
        if content is None or isinstance(content, (str, bytes, int, float)):
            return content
        try:
            return cls._CONTENT_JSON_PREFIX + json.dumps(content)
        except (TypeError, ValueError):
            return str(content)

    @classmethod
    def _decode_content(cls, content: Any) -> Any:
        if isinstance(content, str) and content.startswith(
            cls._CONTENT_JSON_PREFIX
        ):
            try:
                return json.loads(content[len(cls._CONTENT_JSON_PREFIX) :])
            except (json.JSONDecodeError, TypeError):
                return content
        return content

    @staticmethod
    def _sanitize_fts5_query(query: str) -> str:
        _quoted_parts: list = []

        def _preserve_quoted(m: re.Match) -> str:
            _quoted_parts.append(m.group(0))
            return f"\x00Q{len(_quoted_parts) - 1}\x00"

        sanitized = re.sub(r'"[^"]*"', _preserve_quoted, query)
        sanitized = re.sub(r"[+{}()\"^]", " ", sanitized)
        sanitized = re.sub(r"\*+", "*", sanitized)
        sanitized = re.sub(r"(^|\s)\*", r"\1", sanitized)
        sanitized = re.sub(r"(?i)^(AND|OR|NOT)\b\s*", "", sanitized.strip())
        sanitized = re.sub(r"(?i)\s+(AND|OR|NOT)\s*$", "", sanitized.strip())
        sanitized = re.sub(r"\b(\w+(?:[._-]\w+)+)\b", r'"\1"', sanitized)

        for i, quoted in enumerate(_quoted_parts):
            sanitized = sanitized.replace(f"\x00Q{i}\x00", quoted)

        return sanitized.strip()

    @staticmethod
    def _is_cjk_codepoint(cp: int) -> bool:
        return (
            0x4E00 <= cp <= 0x9FFF
            or 0x3400 <= cp <= 0x4DBF
            or 0x20000 <= cp <= 0x2A6DF
            or 0x3000 <= cp <= 0x303F
            or 0x3040 <= cp <= 0x309F
            or 0x30A0 <= cp <= 0x30FF
            or 0xAC00 <= cp <= 0xD7AF
        )

    @staticmethod
    def _contains_cjk(text: str) -> bool:
        for ch in text:
            cp = ord(ch)
            if (
                0x4E00 <= cp <= 0x9FFF
                or 0x3400 <= cp <= 0x4DBF
                or 0x20000 <= cp <= 0x2A6DF
                or 0x3000 <= cp <= 0x303F
                or 0x3040 <= cp <= 0x309F
                or 0x30A0 <= cp <= 0x30FF
                or 0xAC00 <= cp <= 0xD7AF
            ):
                return True
        return False

    @classmethod
    def _count_cjk(cls, text: str) -> int:
        return sum(1 for ch in text if cls._is_cjk_codepoint(ord(ch)))
