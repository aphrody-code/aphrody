// SPDX-License-Identifier: Apache-2.0
//! Schema versioning (R3.6) — forward-compatible memory record migration.
//!
//! On-disk envelopes (JSONL mailboxes, SQLite columns serialised via
//! [`serde_json`], snapshot exports) must survive evolution of the in-process
//! [`crate::types::MemoryRecord`] type without losing data. This module owns
//! the *versioned* on-the-wire shapes and the lossless `V1 → V2` upgrade path.
//!
//! ## Scope
//!
//! - `MemoryRecordV1` — the shape used on disk before 2026-05-19 (no version
//!   tag, no `updated_at_unix_ms`).
//! - `MemoryRecordV2` — current shape: explicit [`SchemaVersion`] tag and a
//!   second timestamp for mutation tracking.
//! - [`AnyMemoryRecord`] — discriminated union that deserializes either
//!   version from JSON and exposes [`AnyMemoryRecord::into_v2`] to normalize.
//! - [`upgrade_jsonl`] — streaming migration of a JSONL file (one envelope per
//!   line), in-place safe via a temp + rename swap.
//!
//! ## Decoupling rationale
//!
//! The live [`crate::types::MemoryRecord`] is what the rest of the crate
//! manipulates in memory. The versioned types here are **frozen snapshots**
//! of past on-disk shapes; they never change after release. A new schema bump
//! ships an additional `MemoryRecordV3` struct + an `upgrade()` from V2 — the
//! existing variants stay untouched, guaranteeing every past JSONL file in
//! the wild can still round-trip.
//!
//! `From` conversions wire the schema layer back into the live type so callers
//! can `let live: types::MemoryRecord = any.into_v2().into();`.
//!
//! ## Verify
//!
//! `cargo test -p aphrody-memory schema::` (12 tests cover detection, upgrade
//! preservation, roundtrip stability, JSONL migration with mixed-version
//! inputs).

use std::path::Path;

use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::types::{MemoryError, MemoryRecord, now_unix_ms};

// ─────────────────────────────────────────────────────────────────────────────
// SchemaVersion — explicit on-disk tag.
// ─────────────────────────────────────────────────────────────────────────────

/// Stable integer discriminant for [`MemoryRecordV2`] and successors.
///
/// Stored as the JSON number `1` or `2` (not a string) so future bumps can
/// extend the enum without touching existing on-disk payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(into = "u32", try_from = "u32")]
pub enum SchemaVersion {
    /// Pre-2026-05-19: no explicit tag on disk; inferred by structural sniffing.
    V1,
    /// Adds explicit `schema_version` field and `updated_at_unix_ms`.
    V2,
}

impl SchemaVersion {
    /// Stable integer code persisted on disk.
    #[must_use]
    pub const fn as_u32(self) -> u32 {
        match self {
            Self::V1 => 1,
            Self::V2 => 2,
        }
    }

    /// Latest version known to this build — what new writes should adopt.
    #[must_use]
    pub const fn current() -> Self {
        Self::V2
    }
}

impl From<SchemaVersion> for u32 {
    fn from(v: SchemaVersion) -> Self {
        v.as_u32()
    }
}

impl TryFrom<u32> for SchemaVersion {
    type Error = String;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::V1),
            2 => Ok(Self::V2),
            other => Err(format!("unknown schema_version: {other}")),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// MemoryRecordV1 — frozen pre-versioning shape.
// ─────────────────────────────────────────────────────────────────────────────

/// On-disk record shape used before [`SchemaVersion::V2`] was introduced.
///
/// Frozen: never mutate the fields, never derive new traits that would change
/// serialisation. To extend the schema, add `MemoryRecordV3` and a `V2 → V3`
/// upgrade — leave V1 alone.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryRecordV1 {
    /// Stable identifier — provider-assigned or locally generated.
    pub id: String,
    /// Logical owner (user_id / session owner).
    pub agent_id: String,
    /// Free-form text body.
    pub content: String,
    /// Caller-chosen labels.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Wall-clock creation timestamp (epoch milliseconds, UTC).
    pub created_at_unix_ms: u64,
    /// Provider-specific extras.
    #[serde(default = "default_metadata_value")]
    pub metadata: serde_json::Value,
}

fn default_metadata_value() -> serde_json::Value {
    serde_json::Value::Object(serde_json::Map::new())
}

impl MemoryRecordV1 {
    /// Lossless upgrade: copies every field and sets `updated_at_unix_ms` to
    /// `created_at_unix_ms` (V1 records had no mutation timestamp; treating
    /// "creation == last update" is the only zero-loss choice).
    #[must_use]
    pub fn upgrade(self) -> MemoryRecordV2 {
        let updated = self.created_at_unix_ms;
        MemoryRecordV2 {
            schema_version: SchemaVersion::V2,
            id: self.id,
            agent_id: self.agent_id,
            content: self.content,
            tags: self.tags,
            created_at_unix_ms: self.created_at_unix_ms,
            updated_at_unix_ms: updated,
            metadata: self.metadata,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// MemoryRecordV2 — current on-disk shape.
// ─────────────────────────────────────────────────────────────────────────────

/// Current versioned on-disk record. Adds [`SchemaVersion`] discriminator and
/// `updated_at_unix_ms`. Field order is significant for `serde_json` output
/// (we want `schema_version` first so future readers can short-circuit).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryRecordV2 {
    /// Explicit version tag — always [`SchemaVersion::V2`] for this struct.
    pub schema_version: SchemaVersion,
    /// Stable identifier.
    pub id: String,
    /// Logical owner.
    pub agent_id: String,
    /// Free-form text body.
    pub content: String,
    /// Caller-chosen labels.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Wall-clock creation timestamp (epoch milliseconds, UTC).
    pub created_at_unix_ms: u64,
    /// Wall-clock last-mutation timestamp (epoch milliseconds, UTC).
    pub updated_at_unix_ms: u64,
    /// Provider-specific extras.
    #[serde(default = "default_metadata_value")]
    pub metadata: serde_json::Value,
}

impl MemoryRecordV2 {
    /// Wrap a live [`MemoryRecord`] in the V2 on-disk envelope, copying the
    /// creation timestamp into `updated_at_unix_ms`.
    #[must_use]
    pub fn from_live(rec: MemoryRecord) -> Self {
        let updated = rec.created_at_unix_ms;
        Self {
            schema_version: SchemaVersion::V2,
            id: rec.id,
            agent_id: rec.agent_id,
            content: rec.content,
            tags: rec.tags,
            created_at_unix_ms: rec.created_at_unix_ms,
            updated_at_unix_ms: updated,
            metadata: rec.metadata,
        }
    }

    /// Promote a live record to V2 with a caller-supplied mutation timestamp.
    ///
    /// Useful when a backend tracks updates explicitly (SQLite `UPDATE`,
    /// HTTP PATCH) and wants to surface it on disk.
    #[must_use]
    pub fn from_live_with_updated(rec: MemoryRecord, updated_at_unix_ms: u64) -> Self {
        Self {
            schema_version: SchemaVersion::V2,
            id: rec.id,
            agent_id: rec.agent_id,
            content: rec.content,
            tags: rec.tags,
            created_at_unix_ms: rec.created_at_unix_ms,
            updated_at_unix_ms,
            metadata: rec.metadata,
        }
    }
}

impl From<MemoryRecordV2> for MemoryRecord {
    fn from(v: MemoryRecordV2) -> Self {
        // The live `MemoryRecord` does not carry `updated_at_unix_ms` (yet).
        // It survives lossily in `metadata["_updated_at_unix_ms"]` so a future
        // V3 → live conversion can recover it without parsing the on-disk file
        // a second time.
        let mut metadata = v.metadata;
        if let serde_json::Value::Object(ref mut map) = metadata {
            map.insert(
                "_updated_at_unix_ms".to_string(),
                serde_json::Value::from(v.updated_at_unix_ms),
            );
        }
        Self {
            id: v.id,
            agent_id: v.agent_id,
            content: v.content,
            tags: v.tags,
            created_at_unix_ms: v.created_at_unix_ms,
            metadata,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// AnyMemoryRecord — version-agnostic deserialisation envelope.
// ─────────────────────────────────────────────────────────────────────────────

/// Discriminated union over every supported on-disk version.
///
/// Use [`AnyMemoryRecord::from_json_str`] / [`AnyMemoryRecord::from_json_value`]
/// to parse a payload of unknown version, then [`AnyMemoryRecord::into_v2`] to
/// normalise to the latest shape. This is the standard ingestion pattern for
/// JSONL replay, snapshot import, and migration tooling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnyMemoryRecord {
    /// Pre-versioning payload — detected when `schema_version` is absent.
    V1(MemoryRecordV1),
    /// Post-versioning payload — detected when `schema_version: 2` is present.
    V2(MemoryRecordV2),
}

impl AnyMemoryRecord {
    /// Parse a JSON string of unknown version.
    ///
    /// Detection rule: if the top-level object has a `schema_version` field
    /// equal to `2`, decode as [`MemoryRecordV2`]; otherwise fall back to
    /// [`MemoryRecordV1`]. Future versions extend this match.
    ///
    /// # Errors
    /// Returns [`MemoryError::Json`] for malformed JSON or
    /// [`MemoryError::InvalidArgument`] when an unknown version tag is present.
    pub fn from_json_str(s: &str) -> Result<Self, MemoryError> {
        let value: serde_json::Value = serde_json::from_str(s)?;
        Self::from_json_value(value)
    }

    /// Parse from a pre-decoded `serde_json::Value`.
    ///
    /// # Errors
    /// See [`Self::from_json_str`].
    pub fn from_json_value(value: serde_json::Value) -> Result<Self, MemoryError> {
        let version_field = value.get("schema_version").and_then(|v| v.as_u64());
        match version_field {
            Some(2) => {
                let v2: MemoryRecordV2 = serde_json::from_value(value)?;
                Ok(Self::V2(v2))
            }
            Some(1) => {
                let v1: MemoryRecordV1 = serde_json::from_value(value)?;
                Ok(Self::V1(v1))
            }
            Some(other) => Err(MemoryError::InvalidArgument(format!(
                "unsupported schema_version: {other}"
            ))),
            None => {
                let v1: MemoryRecordV1 = serde_json::from_value(value)?;
                Ok(Self::V1(v1))
            }
        }
    }

    /// Normalise to the current schema version, upgrading V1 → V2 if needed.
    #[must_use]
    pub fn into_v2(self) -> MemoryRecordV2 {
        match self {
            Self::V1(v1) => v1.upgrade(),
            Self::V2(v2) => v2,
        }
    }

    /// Inspect the version tag of the underlying record.
    #[must_use]
    pub const fn version(&self) -> SchemaVersion {
        match self {
            Self::V1(_) => SchemaVersion::V1,
            Self::V2(_) => SchemaVersion::V2,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// upgrade_jsonl — streaming file migration.
// ─────────────────────────────────────────────────────────────────────────────

/// Summary returned by [`upgrade_jsonl`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MigrationReport {
    /// Total non-empty lines processed.
    pub total: usize,
    /// Lines that already carried `schema_version: 2` and were re-emitted as-is.
    pub already_v2: usize,
    /// Lines parsed as V1 and upgraded to V2.
    pub upgraded: usize,
    /// Lines we could not parse (preserved verbatim in the output for forensic
    /// inspection — never silently dropped).
    pub skipped: usize,
}

/// Migrate a JSONL file in place from mixed V1/V2 envelopes to a uniform V2
/// stream.
///
/// Writes to `<output>.tmp` first, then atomically renames over `output`,
/// so a crash mid-write leaves the original file intact. Unparseable lines
/// are echoed verbatim and counted as `skipped` — destructive coercion is
/// never silent.
///
/// `input` and `output` may point at the same file (in-place migration).
///
/// # Errors
/// - [`MemoryError::Io`] for filesystem failures.
/// - [`MemoryError::Json`] is *not* propagated for individual lines (those
///   become `skipped`); only an outright malformed write target surfaces.
pub async fn upgrade_jsonl(
    input: impl AsRef<Path>,
    output: impl AsRef<Path>,
) -> Result<MigrationReport, MemoryError> {
    let input = input.as_ref();
    let output = output.as_ref();

    let file = fs::File::open(input).await?;
    let reader = BufReader::new(file);

    // Write to a sibling temp file in the same directory so the final rename
    // is atomic on every filesystem aphrody targets (ext4, NTFS, APFS, WASI).
    let tmp = match output.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => {
            let mut p = parent.to_path_buf();
            let stem = output.file_name().map_or_else(
                || std::ffi::OsString::from("upgrade.tmp"),
                std::ffi::OsStr::to_os_string,
            );
            let mut name = stem;
            name.push(".tmp");
            p.push(name);
            p
        }
        _ => {
            let mut p = std::path::PathBuf::from(output);
            let mut name = p.file_name().map_or_else(
                || std::ffi::OsString::from("upgrade.tmp"),
                std::ffi::OsStr::to_os_string,
            );
            name.push(".tmp");
            p.set_file_name(name);
            p
        }
    };

    let mut tmp_file = fs::File::create(&tmp).await?;

    let mut report = MigrationReport::default();
    let mut lines = reader.lines();
    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }
        report.total += 1;
        match AnyMemoryRecord::from_json_str(&line) {
            Ok(any) => {
                let already = matches!(any, AnyMemoryRecord::V2(_));
                let v2 = any.into_v2();
                let serialised = serde_json::to_string(&v2)?;
                tmp_file.write_all(serialised.as_bytes()).await?;
                tmp_file.write_all(b"\n").await?;
                if already {
                    report.already_v2 += 1;
                } else {
                    report.upgraded += 1;
                }
            }
            Err(_) => {
                // Preserve the original line verbatim — operators can grep
                // the output for malformed payloads after the fact.
                tmp_file.write_all(line.as_bytes()).await?;
                tmp_file.write_all(b"\n").await?;
                report.skipped += 1;
            }
        }
    }

    tmp_file.flush().await?;
    tmp_file.sync_all().await?;
    drop(tmp_file);

    fs::rename(&tmp, output).await?;
    Ok(report)
}

// ─────────────────────────────────────────────────────────────────────────────
// Convenience constructors for the live type.
// ─────────────────────────────────────────────────────────────────────────────

impl From<MemoryRecord> for MemoryRecordV1 {
    fn from(rec: MemoryRecord) -> Self {
        Self {
            id: rec.id,
            agent_id: rec.agent_id,
            content: rec.content,
            tags: rec.tags,
            created_at_unix_ms: rec.created_at_unix_ms,
            metadata: rec.metadata,
        }
    }
}

impl From<MemoryRecord> for MemoryRecordV2 {
    fn from(rec: MemoryRecord) -> Self {
        Self::from_live(rec)
    }
}

/// Build a fresh V2 record with `created_at == updated_at == now()`.
#[must_use]
pub fn new_v2(
    id: impl Into<String>,
    agent_id: impl Into<String>,
    content: impl Into<String>,
) -> MemoryRecordV2 {
    let now = now_unix_ms();
    MemoryRecordV2 {
        schema_version: SchemaVersion::V2,
        id: id.into(),
        agent_id: agent_id.into(),
        content: content.into(),
        tags: Vec::new(),
        created_at_unix_ms: now,
        updated_at_unix_ms: now,
        metadata: default_metadata_value(),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_v1() -> MemoryRecordV1 {
        MemoryRecordV1 {
            id: "id-1".into(),
            agent_id: "agent-A".into(),
            content: "hello".into(),
            tags: vec!["fact".into()],
            created_at_unix_ms: 1_000,
            metadata: serde_json::json!({"src": "test"}),
        }
    }

    // ── Version tag ──────────────────────────────────────────────────────────

    #[test]
    fn schema_version_serde_is_u32() {
        let v = SchemaVersion::V2;
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, "2");
        let back: SchemaVersion = serde_json::from_str("1").unwrap();
        assert_eq!(back, SchemaVersion::V1);
    }

    #[test]
    fn schema_version_rejects_unknown() {
        let r: Result<SchemaVersion, _> = serde_json::from_str("99");
        assert!(r.is_err(), "unknown version must be rejected at the type boundary");
    }

    #[test]
    fn schema_version_current_is_v2() {
        assert_eq!(SchemaVersion::current(), SchemaVersion::V2);
    }

    // ── V1 → V2 lossless upgrade ────────────────────────────────────────────

    #[test]
    fn schema_v1_to_v2_roundtrip() {
        // PLAN verify: this exact test name is the contract.
        let v1 = sample_v1();
        let v2 = v1.clone().upgrade();
        assert_eq!(v2.schema_version, SchemaVersion::V2);
        assert_eq!(v2.id, v1.id);
        assert_eq!(v2.agent_id, v1.agent_id);
        assert_eq!(v2.content, v1.content);
        assert_eq!(v2.tags, v1.tags);
        assert_eq!(v2.created_at_unix_ms, v1.created_at_unix_ms);
        assert_eq!(v2.updated_at_unix_ms, v1.created_at_unix_ms,
            "V1 had no updated_at — must mirror created_at for zero-loss upgrade");
        assert_eq!(v2.metadata, v1.metadata);
    }

    #[test]
    fn schema_v2_serialises_with_version_tag() {
        let v2 = sample_v1().upgrade();
        let json = serde_json::to_string(&v2).unwrap();
        assert!(json.contains("\"schema_version\":2"), "version tag must be present: {json}");
        assert!(json.contains("\"updated_at_unix_ms\":1000"));
    }

    // ── AnyMemoryRecord — version detection ─────────────────────────────────

    #[test]
    fn any_memory_record_detects_v1_when_tag_missing() {
        let raw = r#"{"id":"x","agent_id":"a","content":"c","created_at_unix_ms":42}"#;
        let any = AnyMemoryRecord::from_json_str(raw).unwrap();
        assert_eq!(any.version(), SchemaVersion::V1);
    }

    #[test]
    fn any_memory_record_detects_v2_via_tag() {
        let raw = r#"{
            "schema_version":2,
            "id":"x","agent_id":"a","content":"c",
            "created_at_unix_ms":42,"updated_at_unix_ms":99
        }"#;
        let any = AnyMemoryRecord::from_json_str(raw).unwrap();
        assert_eq!(any.version(), SchemaVersion::V2);
        let v2 = any.into_v2();
        assert_eq!(v2.updated_at_unix_ms, 99);
    }

    #[test]
    fn any_memory_record_rejects_unknown_version() {
        let raw = r#"{"schema_version":7,"id":"x"}"#;
        let err = AnyMemoryRecord::from_json_str(raw).unwrap_err();
        assert!(matches!(err, MemoryError::InvalidArgument(_)));
    }

    #[test]
    fn any_into_v2_is_idempotent() {
        let v2 = sample_v1().upgrade();
        let any = AnyMemoryRecord::V2(v2.clone());
        assert_eq!(any.into_v2(), v2);
    }

    // ── Live ↔ on-disk conversions ──────────────────────────────────────────

    #[test]
    fn live_record_roundtrips_through_v2() {
        let live = MemoryRecord::new("id-1", "agent-A", "body");
        let created = live.created_at_unix_ms;
        let v2: MemoryRecordV2 = live.clone().into();
        assert_eq!(v2.created_at_unix_ms, v2.updated_at_unix_ms,
            "fresh live record collapses to created == updated on disk");
        let back: MemoryRecord = v2.into();
        assert_eq!(back.id, live.id);
        assert_eq!(back.agent_id, live.agent_id);
        assert_eq!(back.content, live.content);
        assert_eq!(back.created_at_unix_ms, created);
        // updated_at is stashed inside metadata for forward compatibility
        let extracted = back.metadata.get("_updated_at_unix_ms").and_then(serde_json::Value::as_u64);
        assert_eq!(extracted, Some(created));
    }

    // ── JSONL streaming migration ───────────────────────────────────────────

    #[tokio::test]
    async fn upgrade_jsonl_migrates_mixed_versions() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("inbox.jsonl");

        // Mix: one V1 line (no schema_version), one V2 line, one blank, one
        // garbage line that must be preserved verbatim.
        let v1_line = r#"{"id":"a","agent_id":"X","content":"hi","created_at_unix_ms":10}"#;
        let v2_line = r#"{"schema_version":2,"id":"b","agent_id":"X","content":"yo","created_at_unix_ms":20,"updated_at_unix_ms":25}"#;
        let garbage = "this is not json";
        let content = format!("{v1_line}\n{v2_line}\n\n{garbage}\n");
        fs::write(&path, content).await.unwrap();

        let report = upgrade_jsonl(&path, &path).await.unwrap();
        assert_eq!(report.total, 3, "blank line must not count");
        assert_eq!(report.upgraded, 1);
        assert_eq!(report.already_v2, 1);
        assert_eq!(report.skipped, 1);

        let migrated = fs::read_to_string(&path).await.unwrap();
        let lines: Vec<&str> = migrated.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(lines.len(), 3);

        // First line — V1 upgraded → must now carry schema_version: 2.
        let first: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(first.get("schema_version").and_then(serde_json::Value::as_u64), Some(2));
        assert_eq!(first.get("updated_at_unix_ms").and_then(serde_json::Value::as_u64), Some(10),
            "upgraded V1 must collapse updated_at onto created_at");

        // Second line — already V2 → version preserved, updated_at intact.
        let second: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(second.get("updated_at_unix_ms").and_then(serde_json::Value::as_u64), Some(25));

        // Third line — garbage → echoed verbatim (operator can grep).
        assert_eq!(lines[2], garbage);
    }

    #[tokio::test]
    async fn upgrade_jsonl_is_atomic_on_empty_input() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("empty.jsonl");
        fs::write(&path, "").await.unwrap();
        let report = upgrade_jsonl(&path, &path).await.unwrap();
        assert_eq!(report, MigrationReport::default());
        let after = fs::read_to_string(&path).await.unwrap();
        assert!(after.is_empty(), "empty input must produce empty output, not delete the file");
    }
}
