// SPDX-License-Identifier: Apache-2.0
//! # `LanceDbBackend`
//!
//! A vector-native [`MemoryBackend`] backed by [LanceDB](https://lancedb.com),
//! ported from `openclaw/extensions/memory-lancedb` (TypeScript) to pure Rust.
//!
//! ## Why LanceDB
//!
//! LanceDB is the best in-class embedded vector database in the Rust ecosystem
//! today: zero-copy Arrow-native storage, optional ANN index, and a
//! single-file on-disk layout that we can ship inside `var/data/` with no
//! external server.  Compared to the brute-force search in [`crate::sqlite`]
//! and [`crate::hnsw`], it scales gracefully past 100k rows.
//!
//! ## Schema
//!
//! Each [`LanceDbBackend`] manages a single LanceDB table named `memory_items`
//! with the following Arrow schema:
//!
//! | column      | type                                  | notes                          |
//! |-------------|---------------------------------------|--------------------------------|
//! | `id`        | `Utf8`                                | caller-supplied storage key    |
//! | `ns`        | `Utf8`                                | namespace                      |
//! | `content`   | `Utf8`                                | record body                    |
//! | `vector`    | `FixedSizeList<Float32, ndims>`       | embedding (fixed dim per table) |
//! | `metadata`  | `Utf8`                                | JSON-encoded metadata blob     |
//! | `created_at`| `Int64`                               | unix millis                    |
//! | `updated_at`| `Int64`                               | unix millis                    |
//!
//! The vector dimensionality is fixed at table-creation time; rejecting
//! mismatched-length embeddings is enforced by [`Self::put`].
//!
//! ## Embedding ownership
//!
//! As specified in the task brief, this backend does **not** integrate an
//! embedding provider — callers supply pre-computed `f32` vectors.  This keeps
//! aphrody-memory free of OpenAI / Ollama / Candle dependencies, matching the
//! "Embedding-as-a-Service is out-of-scope" contract in `CLAUDE.md` §1.
//!
//! ## Source reference
//!
//! Ported from `openclaw/extensions/memory-lancedb/` (TypeScript runtime
//! wrapper + `api.ts` + `config.ts`).  The Rust port collapses the
//! `runtime-loader` indirection (Node platform-detection shim) since the
//! `lancedb` crate is statically linked.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use arrow_array::{
    Array, FixedSizeListArray, Float32Array, Int64Array, RecordBatch, StringArray,
    builder::{FixedSizeListBuilder, Float32Builder},
};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use futures::TryStreamExt;
use lancedb::{
    Connection, Table,
    connection::ConnectBuilder,
    query::{ExecutableQuery, QueryBase},
};
use tokio::sync::Mutex;
use tracing::{debug, trace};
use uuid::Uuid;

use crate::{Embedding, MemoryBackend, MemoryError, MemoryRecord};

const TABLE_NAME: &str = "memory_items";
const VECTOR_COLUMN: &str = "vector";

// ---------------------------------------------------------------------------
// Schema construction
// ---------------------------------------------------------------------------

fn build_schema(ndims: i32) -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("ns", DataType::Utf8, false),
        Field::new("content", DataType::Utf8, false),
        Field::new(
            VECTOR_COLUMN,
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                ndims,
            ),
            true,
        ),
        Field::new("metadata", DataType::Utf8, false),
        Field::new("created_at", DataType::Int64, false),
        Field::new("updated_at", DataType::Int64, false),
    ]))
}

// ---------------------------------------------------------------------------
// Backend struct
// ---------------------------------------------------------------------------

/// LanceDB-backed memory store.
///
/// Open one instance per logical namespace tree.  The on-disk layout is a
/// LanceDB database directory containing a single `memory_items.lance` table;
/// safe to ship inside `var/data/aphrody-memory/lancedb/`.
pub struct LanceDbBackend {
    path: PathBuf,
    ndims: i32,
    /// We hold the LanceDB [`Connection`] for table re-opens and the
    /// [`Table`] handle directly to avoid round-tripping through `open_table`
    /// on every call.  Both are cheap to clone (`Arc`-internal) but we still
    /// guard mutations with a `Mutex` to serialise concurrent writers (LanceDB
    /// is concurrency-safe but stacking concurrent inserts into the same
    /// fragment is wasteful at low row counts).
    _conn: Connection,
    table: Arc<Mutex<Table>>,
}

impl LanceDbBackend {
    /// Open (or create) a LanceDB-backed memory store rooted at `path`,
    /// using `ndims`-dimensional embeddings.
    ///
    /// The first call to a given `path` will create the underlying
    /// `memory_items` table with the specified schema.  Subsequent opens read
    /// the existing schema; the supplied `ndims` is then validated against
    /// the stored vector column.
    ///
    /// # Errors
    ///
    /// - [`MemoryError::InvalidArgument`] if `ndims == 0`.
    /// - [`MemoryError::Internal`] for any underlying LanceDB / Arrow failure
    ///   (table creation, schema mismatch on reopen, etc.).
    pub async fn open(
        path: impl AsRef<Path>,
        ndims: usize,
    ) -> Result<Self, MemoryError> {
        if ndims == 0 {
            return Err(MemoryError::InvalidArgument {
                message: "ndims must be > 0".into(),
            });
        }
        let path = path.as_ref().to_owned();
        let ndims_i32 = i32::try_from(ndims).map_err(|_| MemoryError::InvalidArgument {
            message: format!("ndims {ndims} does not fit in i32"),
        })?;

        // Ensure the parent dir exists so LanceDB can create its layout files.
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            tokio::fs::create_dir_all(parent).await.map_err(MemoryError::Io)?;
        }
        tokio::fs::create_dir_all(&path).await.map_err(MemoryError::Io)?;

        let uri = path.to_string_lossy().into_owned();
        let conn = ConnectBuilder::new(&uri)
            .execute()
            .await
            .map_err(|e| MemoryError::Internal {
                message: format!("lancedb connect({uri}): {e}"),
            })?;

        // Open existing table or create an empty one with our schema.
        let table = match conn.open_table(TABLE_NAME).execute().await {
            Ok(tbl) => tbl,
            Err(_open_err) => {
                let schema = build_schema(ndims_i32);
                conn.create_empty_table(TABLE_NAME, schema)
                    .execute()
                    .await
                    .map_err(|e| MemoryError::Internal {
                        message: format!("lancedb create_empty_table: {e}"),
                    })?
            }
        };

        // Validate the stored schema dimensionality matches.
        let stored_schema =
            table.schema().await.map_err(|e| MemoryError::Internal {
                message: format!("lancedb schema(): {e}"),
            })?;
        let stored_ndims = vector_dim(&stored_schema)?;
        if stored_ndims != ndims_i32 {
            return Err(MemoryError::InvalidArgument {
                message: format!(
                    "ndims mismatch: caller passed {ndims_i32}, table on disk uses {stored_ndims}"
                ),
            });
        }

        debug!(uri = %uri, ndims, "opened lancedb backend");
        Ok(Self {
            path,
            ndims: ndims_i32,
            _conn: conn,
            table: Arc::new(Mutex::new(table)),
        })
    }

    /// Filesystem path the backend was opened with (mostly for diagnostics).
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Embedding dimensionality this backend was opened with.
    #[must_use]
    pub fn ndims(&self) -> usize {
        usize::try_from(self.ndims).unwrap_or(0)
    }

    // -----------------------------------------------------------------------
    // Internal: row count helper used by tests / list defaults.
    // -----------------------------------------------------------------------

    #[allow(dead_code)]
    async fn row_count(&self) -> Result<usize, MemoryError> {
        let guard = self.table.lock().await;
        guard.count_rows(None).await.map_err(|e| MemoryError::Internal {
            message: format!("lancedb count_rows: {e}"),
        })
    }
}

fn vector_dim(schema: &Schema) -> Result<i32, MemoryError> {
    let field = schema.field_with_name(VECTOR_COLUMN).map_err(|e| MemoryError::Internal {
        message: format!("lancedb schema missing '{VECTOR_COLUMN}' column: {e}"),
    })?;
    match field.data_type() {
        DataType::FixedSizeList(_, n) => Ok(*n),
        other => Err(MemoryError::Internal {
            message: format!("'{VECTOR_COLUMN}' column has unexpected type {other:?}"),
        }),
    }
}

// ---------------------------------------------------------------------------
// Encoders: build a 1-row RecordBatch from a MemoryRecord
// ---------------------------------------------------------------------------

fn record_to_batch(
    schema: &SchemaRef,
    key: &str,
    rec: &MemoryRecord,
    ndims: i32,
) -> Result<RecordBatch, MemoryError> {
    let id = StringArray::from(vec![key.to_owned()]);
    let ns = StringArray::from(vec![rec.ns.clone()]);
    let content = StringArray::from(vec![rec.content.clone()]);
    let metadata = StringArray::from(vec![serde_json::to_string(&rec.metadata)?]);
    let created_at = Int64Array::from(vec![rec.created_at.timestamp_millis()]);
    let updated_at = Int64Array::from(vec![rec.updated_at.timestamp_millis()]);

    let vector_array = build_vector_array(rec.embedding.as_ref(), ndims)?;

    RecordBatch::try_new(
        Arc::clone(schema),
        vec![
            Arc::new(id),
            Arc::new(ns),
            Arc::new(content),
            Arc::new(vector_array),
            Arc::new(metadata),
            Arc::new(created_at),
            Arc::new(updated_at),
        ],
    )
    .map_err(|e| MemoryError::Internal {
        message: format!("RecordBatch::try_new: {e}"),
    })
}

fn build_vector_array(
    emb: Option<&Embedding>,
    ndims: i32,
) -> Result<FixedSizeListArray, MemoryError> {
    let mut builder = FixedSizeListBuilder::new(Float32Builder::with_capacity(ndims as usize), ndims);
    match emb {
        Some(v) => {
            let expected = usize::try_from(ndims).unwrap_or(0);
            if v.len() != expected {
                return Err(MemoryError::InvalidArgument {
                    message: format!(
                        "embedding length {} does not match table ndims {ndims}",
                        v.len()
                    ),
                });
            }
            for f in v {
                builder.values().append_value(*f);
            }
            builder.append(true);
        }
        None => {
            for _ in 0..ndims {
                builder.values().append_null();
            }
            builder.append(false);
        }
    }
    Ok(builder.finish())
}

// ---------------------------------------------------------------------------
// Decoders: batch → Vec<(MemoryRecord, Option<score>)>
// ---------------------------------------------------------------------------

fn millis_to_dt(millis: i64) -> DateTime<Utc> {
    Utc.timestamp_millis_opt(millis).single().unwrap_or_else(Utc::now)
}

fn parse_uuid_from_key(key: &str) -> Uuid {
    key.rsplit('/')
        .next()
        .and_then(|tail| Uuid::parse_str(tail).ok())
        .unwrap_or_else(Uuid::new_v4)
}

/// Decode every row of `batch` into ([`MemoryRecord`], Option<distance score>).
///
/// Distance scores are only present on rows returned by `nearest_to(...)`
/// queries; LanceDB exposes them as a synthetic `_distance` Float32 column.
/// We convert L2 distance → cosine-similarity-like score via `1 / (1 + d)` so
/// callers get a monotonically-decreasing comparable to the JSONL/SQLite
/// backends without re-running the math.
fn batch_to_records(batch: &RecordBatch) -> Result<Vec<(MemoryRecord, Option<f32>)>, MemoryError> {
    let schema = batch.schema();
    let n = batch.num_rows();

    let col_id = batch
        .column_by_name("id")
        .ok_or_else(|| MemoryError::Internal { message: "missing 'id' column".into() })?
        .as_any()
        .downcast_ref::<StringArray>()
        .ok_or_else(|| MemoryError::Internal { message: "'id' column not Utf8".into() })?;
    let col_ns = batch
        .column_by_name("ns")
        .ok_or_else(|| MemoryError::Internal { message: "missing 'ns' column".into() })?
        .as_any()
        .downcast_ref::<StringArray>()
        .ok_or_else(|| MemoryError::Internal { message: "'ns' column not Utf8".into() })?;
    let col_content = batch
        .column_by_name("content")
        .ok_or_else(|| MemoryError::Internal { message: "missing 'content' column".into() })?
        .as_any()
        .downcast_ref::<StringArray>()
        .ok_or_else(|| MemoryError::Internal { message: "'content' column not Utf8".into() })?;
    let col_meta = batch
        .column_by_name("metadata")
        .ok_or_else(|| MemoryError::Internal { message: "missing 'metadata' column".into() })?
        .as_any()
        .downcast_ref::<StringArray>()
        .ok_or_else(|| MemoryError::Internal { message: "'metadata' column not Utf8".into() })?;
    let col_created = batch
        .column_by_name("created_at")
        .ok_or_else(|| MemoryError::Internal { message: "missing 'created_at' column".into() })?
        .as_any()
        .downcast_ref::<Int64Array>()
        .ok_or_else(|| MemoryError::Internal { message: "'created_at' not Int64".into() })?;
    let col_updated = batch
        .column_by_name("updated_at")
        .ok_or_else(|| MemoryError::Internal { message: "missing 'updated_at' column".into() })?
        .as_any()
        .downcast_ref::<Int64Array>()
        .ok_or_else(|| MemoryError::Internal { message: "'updated_at' not Int64".into() })?;
    let col_vector = batch
        .column_by_name(VECTOR_COLUMN)
        .ok_or_else(|| MemoryError::Internal {
            message: format!("missing '{VECTOR_COLUMN}' column"),
        })?
        .as_any()
        .downcast_ref::<FixedSizeListArray>()
        .ok_or_else(|| MemoryError::Internal {
            message: format!("'{VECTOR_COLUMN}' not FixedSizeList"),
        })?;
    let col_distance = batch.column_by_name("_distance").and_then(|c| {
        c.as_any().downcast_ref::<Float32Array>().map(<Float32Array as Clone>::clone)
    });

    // Silence unused if datasets ever lack the column.
    let _ = schema;

    let mut out = Vec::with_capacity(n);
    for row in 0..n {
        let id_str = col_id.value(row).to_owned();
        let ns_str = col_ns.value(row).to_owned();
        let content_str = col_content.value(row).to_owned();
        let metadata_str = col_meta.value(row);
        let metadata: HashMap<String, serde_json::Value> = serde_json::from_str(metadata_str)?;

        let embedding = if col_vector.is_null(row) {
            None
        } else {
            let list = col_vector.value(row);
            let values = list
                .as_any()
                .downcast_ref::<Float32Array>()
                .ok_or_else(|| MemoryError::Internal {
                    message: "vector inner not Float32".into(),
                })?;
            let mut v = Vec::with_capacity(values.len());
            for i in 0..values.len() {
                v.push(values.value(i));
            }
            Some(v)
        };

        let score = col_distance.as_ref().map(|d| {
            let dist = d.value(row);
            // Convert L2 distance into a (0,1] similarity-like score:
            // dist=0 → 1.0, dist→∞ → 0.0.  Monotonically decreasing with dist.
            1.0_f32 / (1.0 + dist)
        });

        out.push((
            MemoryRecord {
                id: parse_uuid_from_key(&id_str),
                ns: ns_str,
                content: content_str,
                embedding,
                metadata,
                created_at: millis_to_dt(col_created.value(row)),
                updated_at: millis_to_dt(col_updated.value(row)),
            },
            score,
        ));
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// MemoryBackend impl
// ---------------------------------------------------------------------------

#[async_trait]
impl MemoryBackend for LanceDbBackend {
    async fn put(&mut self, key: &str, value: MemoryRecord) -> Result<(), MemoryError> {
        // LanceDB has no "upsert by primary key" primitive on append-only
        // tables.  We mimic put-replaces-existing by deleting then inserting.
        let guard = self.table.lock().await;
        // Best-effort delete; ignore "no row matched" (returns Ok with count=0).
        let escaped = key.replace('\'', "''");
        guard
            .delete(&format!("id = '{escaped}'"))
            .await
            .map_err(|e| MemoryError::Internal {
                message: format!("lancedb delete (pre-put): {e}"),
            })?;

        let schema = build_schema(self.ndims);
        let batch = record_to_batch(&schema, key, &value, self.ndims)?;
        // LanceDB's `Scannable` is implemented for `Vec<RecordBatch>`, which
        // matches our single-row insert shape exactly.
        guard
            .add(vec![batch])
            .execute()
            .await
            .map_err(|e| MemoryError::Internal {
                message: format!("lancedb add: {e}"),
            })?;
        trace!(key, "lancedb put ok");
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Option<MemoryRecord>, MemoryError> {
        let guard = self.table.lock().await;
        let escaped = key.replace('\'', "''");
        let stream = guard
            .query()
            .only_if(format!("id = '{escaped}'"))
            .limit(1)
            .execute()
            .await
            .map_err(|e| MemoryError::Internal {
                message: format!("lancedb query (get): {e}"),
            })?;
        let batches: Vec<RecordBatch> =
            stream.try_collect().await.map_err(|e| MemoryError::Internal {
                message: format!("lancedb collect (get): {e}"),
            })?;
        for batch in &batches {
            let rows = batch_to_records(batch)?;
            if let Some((rec, _)) = rows.into_iter().next() {
                return Ok(Some(rec));
            }
        }
        Ok(None)
    }

    async fn search(
        &self,
        query: &Embedding,
        top_k: usize,
    ) -> Result<Vec<(MemoryRecord, f32)>, MemoryError> {
        if top_k == 0 {
            return Ok(vec![]);
        }
        if query.is_empty() {
            return Err(MemoryError::InvalidArgument {
                message: "query embedding must not be empty".into(),
            });
        }
        let expected = usize::try_from(self.ndims).unwrap_or(0);
        if query.len() != expected {
            return Err(MemoryError::InvalidArgument {
                message: format!(
                    "query embedding length {} does not match table ndims {expected}",
                    query.len()
                ),
            });
        }

        let guard = self.table.lock().await;
        let stream = guard
            .query()
            .nearest_to(query.as_slice())
            .map_err(|e| MemoryError::Internal {
                message: format!("lancedb nearest_to: {e}"),
            })?
            .limit(top_k)
            .execute()
            .await
            .map_err(|e| MemoryError::Internal {
                message: format!("lancedb execute (search): {e}"),
            })?;
        let batches: Vec<RecordBatch> =
            stream.try_collect().await.map_err(|e| MemoryError::Internal {
                message: format!("lancedb collect (search): {e}"),
            })?;

        let mut scored: Vec<(MemoryRecord, f32)> = Vec::new();
        for batch in &batches {
            for (rec, score) in batch_to_records(batch)? {
                scored.push((rec, score.unwrap_or(0.0)));
            }
        }
        // LanceDB returns rows in ascending _distance, i.e. descending score
        // under our (1 / (1+d)) mapping.  Truncate defensively.
        scored.truncate(top_k);
        Ok(scored)
    }

    async fn delete(&mut self, key: &str) -> Result<(), MemoryError> {
        let guard = self.table.lock().await;
        let escaped = key.replace('\'', "''");
        guard
            .delete(&format!("id = '{escaped}'"))
            .await
            .map_err(|e| MemoryError::Internal {
                message: format!("lancedb delete: {e}"),
            })?;
        Ok(())
    }

    async fn list(&self, prefix: &str, limit: usize) -> Result<Vec<MemoryRecord>, MemoryError> {
        let guard = self.table.lock().await;
        // LanceDB / DataFusion SQL: `LIKE` works on Utf8 columns.  Escape the
        // user-supplied prefix to avoid wildcard injection.
        let escaped = prefix
            .replace('\\', "\\\\")
            .replace('\'', "''")
            .replace('%', "\\%")
            .replace('_', "\\_");
        let predicate = format!("id LIKE '{escaped}%' ESCAPE '\\'");
        let stream = guard
            .query()
            .only_if(predicate)
            .limit(limit)
            .execute()
            .await
            .map_err(|e| MemoryError::Internal {
                message: format!("lancedb query (list): {e}"),
            })?;
        let batches: Vec<RecordBatch> =
            stream.try_collect().await.map_err(|e| MemoryError::Internal {
                message: format!("lancedb collect (list): {e}"),
            })?;

        let mut out = Vec::new();
        for batch in &batches {
            for (rec, _) in batch_to_records(batch)? {
                out.push(rec);
                if out.len() >= limit {
                    return Ok(out);
                }
            }
        }
        Ok(out)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const NDIMS: usize = 4;

    async fn make_backend() -> (LanceDbBackend, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("lancedb");
        let backend = LanceDbBackend::open(&path, NDIMS).await.expect("open lancedb");
        (backend, dir)
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn put_then_get_round_trip() {
        let (mut b, _dir) = make_backend().await;
        let mut rec = MemoryRecord::new("test", "lancedb hello")
            .with_embedding(vec![0.1, 0.2, 0.3, 0.4]);
        rec.metadata.insert("subject".into(), serde_json::Value::String("hi".into()));
        let key = format!("{}/{}", rec.ns, rec.id);
        b.put(&key, rec.clone()).await.expect("put");
        let fetched = b.get(&key).await.expect("get").expect("present");
        assert_eq!(fetched.content, "lancedb hello");
        assert_eq!(fetched.ns, "test");
        assert_eq!(fetched.embedding.as_deref(), Some([0.1_f32, 0.2, 0.3, 0.4].as_slice()));
        assert_eq!(fetched.metadata.get("subject").and_then(|v| v.as_str()), Some("hi"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn delete_is_idempotent() {
        let (mut b, _dir) = make_backend().await;
        let rec = MemoryRecord::new("del", "deleteme").with_embedding(vec![1.0, 0.0, 0.0, 0.0]);
        let key = format!("{}/{}", rec.ns, rec.id);
        b.put(&key, rec).await.expect("put");
        b.delete(&key).await.expect("first delete");
        b.delete(&key).await.expect("second delete is idempotent");
        assert!(b.get(&key).await.expect("get").is_none());
        assert_eq!(b.row_count().await.expect("count"), 0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn search_returns_top_k_in_descending_order() {
        let (mut b, _dir) = make_backend().await;
        let near = MemoryRecord::new("vec", "near").with_embedding(vec![1.0, 0.0, 0.0, 0.0]);
        let mid = MemoryRecord::new("vec", "mid").with_embedding(vec![0.7, 0.7, 0.0, 0.0]);
        let far = MemoryRecord::new("vec", "far").with_embedding(vec![0.0, 1.0, 0.0, 0.0]);
        b.put(&format!("vec/{}", near.id), near).await.unwrap();
        b.put(&format!("vec/{}", mid.id), mid).await.unwrap();
        b.put(&format!("vec/{}", far.id), far).await.unwrap();

        let q = vec![1.0_f32, 0.0, 0.0, 0.0];
        let results = b.search(&q, 3).await.expect("search");
        assert_eq!(results.len(), 3);
        for w in results.windows(2) {
            assert!(w[0].1 >= w[1].1, "scores not non-increasing: {} < {}", w[0].1, w[1].1);
        }
        assert_eq!(results[0].0.content, "near");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn search_with_k_greater_than_n_returns_all() {
        let (mut b, _dir) = make_backend().await;
        for i in 0..3u8 {
            let f = f32::from(i);
            let r = MemoryRecord::new("k", format!("item-{i}"))
                .with_embedding(vec![f, 0.0, 0.0, 0.0]);
            b.put(&format!("k/{}", r.id), r).await.unwrap();
        }
        let r = b.search(&vec![1.0_f32, 0.0, 0.0, 0.0], 99).await.expect("search");
        assert_eq!(r.len(), 3, "k>n must return n, got {}", r.len());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn empty_store_search_returns_empty() {
        let (b, _dir) = make_backend().await;
        let r = b.search(&vec![1.0_f32, 0.0, 0.0, 0.0], 5).await.expect("search");
        assert!(r.is_empty(), "empty store must return no results, got {}", r.len());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn list_with_prefix_filters() {
        let (mut b, _dir) = make_backend().await;
        let r1 = MemoryRecord::new("facts", "alpha").with_embedding(vec![1.0, 0.0, 0.0, 0.0]);
        let r2 = MemoryRecord::new("facts", "beta").with_embedding(vec![0.0, 1.0, 0.0, 0.0]);
        let r3 = MemoryRecord::new("sessions", "gamma").with_embedding(vec![0.0, 0.0, 1.0, 0.0]);
        b.put(&format!("facts/{}", r1.id), r1).await.unwrap();
        b.put(&format!("facts/{}", r2.id), r2).await.unwrap();
        b.put(&format!("sessions/{}", r3.id), r3).await.unwrap();
        let facts = b.list("facts/", 10).await.expect("list");
        assert_eq!(facts.len(), 2, "expected 2 facts, got {}", facts.len());
        let all = b.list("", 10).await.expect("list-all");
        assert_eq!(all.len(), 3);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn ndims_zero_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let result = LanceDbBackend::open(dir.path().join("zerodim"), 0).await;
        assert!(matches!(result, Err(MemoryError::InvalidArgument { .. })));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn put_with_mismatched_ndims_rejected() {
        let (mut b, _dir) = make_backend().await;
        let bad = MemoryRecord::new("bad", "wrong dims").with_embedding(vec![1.0, 0.0]);
        let key = format!("bad/{}", bad.id);
        let result = b.put(&key, bad).await;
        assert!(matches!(result, Err(MemoryError::InvalidArgument { .. })));
    }
}
