// SPDX-License-Identifier: Apache-2.0
//! # ASAR archive reader — pure Rust, list + extract only
//!
//! Electron applications ship their JavaScript/asset bundle in an **ASAR**
//! archive (`app.asar`). The format is a thin envelope around a Chromium
//! `base::Pickle` plus a JSON directory tree and a concatenated blob of file
//! data. This module reads that format with **no external dependencies**
//! beyond [`serde_json`] (already a workspace dep) and **never executes**
//! anything: it only enumerates entries ([`list`]) and slices raw bytes out
//! of the archive ([`read_file`]).
//!
//! ## On-disk layout
//!
//! ```text
//! offset 0   u32 LE  pickle-size-of-header-size   (always 4)
//! offset 4   u32 LE  pickle payload size          (= 4 + json_len + padding)
//! offset 8   u32 LE  size of the JSON string field (= json_len + padding..)
//! offset 12  u32 LE  json_len   (length of the UTF-8 JSON directory header)
//! offset 16  ..      JSON header (json_len bytes), then alignment padding
//! data_start ..      concatenated file contents
//! ```
//!
//! `data_start` is `16 + (pickle_payload_size - 4)` — i.e. the JSON field is
//! padded to a 4-byte boundary by the Pickle writer, and file `offset` values
//! in the JSON are **relative to `data_start`**.
//!
//! The JSON header is a recursive tree:
//!
//! ```json
//! { "files": {
//!     "main.js": { "size": 42, "offset": "0" },
//!     "lib":     { "files": { "util.js": { "size": 7, "offset": "42" } } },
//!     "blob.bin":{ "size": 99, "offset": "49", "unpacked": true }
//! }}
//! ```
//!
//! Note that `offset` is a **decimal string** in real Electron archives
//! (the values can exceed `2^53` and JSON numbers are lossy), so this reader
//! accepts both string and integer encodings.
//!
//! ## Scope
//!
//! LIST / EXTRACT only. No integrity checks, no `.unpacked` sidecar
//! resolution, no execution. Malformed input always yields an [`AsarError`]
//! — this module is `#![forbid(unsafe_code)]` (inherited) and never panics.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

/// Errors returned by [`list`] and [`read_file`].
///
/// Every malformed-input path returns one of these variants instead of
/// panicking, so callers can feed arbitrary untrusted bytes safely.
#[derive(Debug, Error)]
pub enum AsarError {
    /// The archive is shorter than the 16-byte envelope, or a declared length
    /// runs past the end of the slice.
    #[error("asar archive truncated: {0}")]
    Truncated(&'static str),

    /// The Pickle envelope fields are internally inconsistent (e.g. the
    /// declared header-size field is not the canonical `4`, or the payload
    /// size does not bracket the JSON length).
    #[error("malformed asar pickle envelope: {0}")]
    Envelope(&'static str),

    /// The JSON directory header is not valid UTF-8.
    #[error("asar header is not valid UTF-8: {0}")]
    Utf8(#[from] std::str::Utf8Error),

    /// The JSON directory header failed to parse.
    #[error("asar header JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    /// The JSON tree had an unexpected shape (missing `files`, a node that is
    /// neither a file nor a directory, a non-numeric `size`/`offset`, …).
    #[error("malformed asar directory tree: {0}")]
    Tree(String),

    /// [`read_file`] was asked for a path that does not exist in the archive.
    #[error("path not found in asar archive: {0}")]
    NotFound(String),

    /// [`read_file`] targeted a directory rather than a file.
    #[error("path is a directory, not a file: {0}")]
    IsDirectory(String),

    /// A file's `offset + size` runs past the end of the data section.
    #[error("asar file data out of bounds: {0}")]
    DataOutOfBounds(String),
}

/// One entry in an ASAR archive, as returned by [`list`].
///
/// Paths are full POSIX-style paths from the archive root, using `/` as the
/// separator and never starting with `/` (e.g. `"lib/util.js"`). Directory
/// entries have `size == 0`, `offset == 0`, and `is_dir == true`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AsarEntry {
    /// Full POSIX-style path from the archive root (`/`-separated).
    pub path: String,
    /// File size in bytes. `0` for directories.
    pub size: u64,
    /// Offset of the file contents relative to the start of the data section
    /// (right after the aligned JSON header). `0` for directories.
    pub offset: u64,
    /// `true` if this entry is a directory.
    pub is_dir: bool,
    /// `true` if the entry's `"unpacked"` flag is set (its bytes live in the
    /// sibling `app.asar.unpacked/` directory rather than inside the archive).
    pub unpacked: bool,
}

/// Parsed envelope: where the JSON header lives and where data begins.
struct Envelope<'a> {
    /// The raw JSON header bytes (exactly `json_len` long).
    header_json: &'a [u8],
    /// Byte offset within the whole archive where the data section starts.
    data_start: usize,
}

/// The canonical Pickle "size of the header-size field" — a single `u32`.
const PICKLE_HEADER_SIZE_FIELD: u32 = 4;

/// Read a little-endian `u32` at `off`, or [`AsarError::Truncated`].
fn read_u32_le(bytes: &[u8], off: usize, what: &'static str) -> Result<u32, AsarError> {
    let end = off.checked_add(4).ok_or(AsarError::Truncated(what))?;
    let slice = bytes.get(off..end).ok_or(AsarError::Truncated(what))?;
    // `slice` is exactly 4 bytes here, so the conversion cannot fail.
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

/// Parse the 16-byte Pickle envelope and locate the JSON header + data start.
fn parse_envelope(bytes: &[u8]) -> Result<Envelope<'_>, AsarError> {
    // [0]  header-size-of-size (canonical 4)
    let size_of_size = read_u32_le(bytes, 0, "header size-of-size field")?;
    if size_of_size != PICKLE_HEADER_SIZE_FIELD {
        return Err(AsarError::Envelope("size-of-size field is not 4"));
    }

    // [4]  outer pickle payload size = 4 (string-size field) + string field.
    let payload_size = read_u32_le(bytes, 4, "pickle payload size field")?;

    // [8]  size of the inner (header) pickle = 4 (json_len field) + json + pad.
    let string_field_size = read_u32_le(bytes, 8, "header string field size")?;

    // [12] json_len: actual UTF-8 byte length of the JSON header.
    let json_len = read_u32_le(bytes, 12, "header json length")?;

    let payload_size = payload_size as usize;
    let string_field_size = string_field_size as usize;
    let json_len = json_len as usize;

    // The inner pickle holds the u32 `json_len` field (4 bytes) followed by
    // the JSON bytes and 4-byte alignment padding, so it must be at least
    // `4 + json_len` long.
    let min_inner = 4usize
        .checked_add(json_len)
        .ok_or(AsarError::Envelope("json length overflow"))?;
    if string_field_size < min_inner {
        return Err(AsarError::Envelope(
            "header pickle smaller than declared JSON length",
        ));
    }
    // outer payload = 4 (the u32 holding the inner-pickle size) + inner pickle.
    let expected_payload = 4usize
        .checked_add(string_field_size)
        .ok_or(AsarError::Envelope("payload size overflow"))?;
    if payload_size != expected_payload {
        return Err(AsarError::Envelope(
            "payload size does not match header pickle size",
        ));
    }

    // JSON starts at byte 16 (after the four leading u32 fields), runs
    // json_len bytes.
    let json_start = 16usize;
    let json_end = json_start
        .checked_add(json_len)
        .ok_or(AsarError::Truncated("header json"))?;
    let header_json = bytes
        .get(json_start..json_end)
        .ok_or(AsarError::Truncated("header json"))?;

    // Data begins after the two outer u32 fields ([0], [4]) and the entire
    // outer payload.
    let data_start = 8usize
        .checked_add(payload_size)
        .ok_or(AsarError::Envelope("data start overflow"))?;
    if data_start > bytes.len() {
        return Err(AsarError::Truncated("data section start past end"));
    }

    Ok(Envelope {
        header_json,
        data_start,
    })
}

/// Parse a node's numeric `size`/`offset`, accepting string or integer JSON.
fn parse_u64_field(node: &serde_json::Map<String, Value>, key: &str) -> Result<u64, AsarError> {
    match node.get(key) {
        Some(Value::String(s)) => s
            .parse::<u64>()
            .map_err(|e| AsarError::Tree(format!("`{key}` not a u64 string: {e}"))),
        Some(Value::Number(n)) => n
            .as_u64()
            .ok_or_else(|| AsarError::Tree(format!("`{key}` not a u64 number"))),
        Some(_) => Err(AsarError::Tree(format!("`{key}` has wrong JSON type"))),
        None => Err(AsarError::Tree(format!("missing `{key}`"))),
    }
}

/// Recursively walk one directory node (`{ "files": { name: node, .. } }`),
/// pushing entries into `out`. `prefix` is the POSIX path of `node` itself
/// (empty for the root).
fn walk(
    node: &serde_json::Map<String, Value>,
    prefix: &str,
    out: &mut Vec<AsarEntry>,
) -> Result<(), AsarError> {
    let files = node
        .get("files")
        .ok_or_else(|| AsarError::Tree(format!("directory `{prefix}` has no `files` map")))?;
    let files = files
        .as_object()
        .ok_or_else(|| AsarError::Tree(format!("`files` of `{prefix}` is not an object")))?;

    // BTreeMap to get a deterministic, sorted listing regardless of the JSON
    // object's insertion order.
    let sorted: BTreeMap<&String, &Value> = files.iter().collect();

    for (name, child) in sorted {
        if name.is_empty() || name.contains('/') || name == "." || name == ".." {
            return Err(AsarError::Tree(format!("invalid entry name `{name}`")));
        }
        let child = child
            .as_object()
            .ok_or_else(|| AsarError::Tree(format!("entry `{name}` is not an object")))?;

        let full_path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };

        if child.contains_key("files") {
            // Directory: emit the dir entry, then recurse.
            out.push(AsarEntry {
                path: full_path.clone(),
                size: 0,
                offset: 0,
                is_dir: true,
                unpacked: false,
            });
            walk(child, &full_path, out)?;
        } else {
            // File: must carry size + offset (unless unpacked, where offset is
            // absent — Electron stores those out-of-band).
            let unpacked = matches!(child.get("unpacked"), Some(Value::Bool(true)));
            let size = if child.contains_key("size") {
                parse_u64_field(child, "size")?
            } else if unpacked {
                0
            } else {
                return Err(AsarError::Tree(format!("file `{full_path}` missing `size`")));
            };
            let offset = if unpacked {
                // Unpacked files have no in-archive offset.
                0
            } else {
                parse_u64_field(child, "offset")?
            };
            out.push(AsarEntry {
                path: full_path,
                size,
                offset,
                is_dir: false,
                unpacked,
            });
        }
    }
    Ok(())
}

/// Parse an ASAR archive header and return a flat, sorted listing of every
/// entry (files and directories) with full POSIX-style paths.
///
/// The listing is sorted lexicographically by path within each directory
/// level (directories are walked depth-first, children sorted). Directory
/// entries appear before their contents.
///
/// # Errors
///
/// Returns [`AsarError`] for any malformed input (truncated envelope, bad
/// Pickle fields, invalid UTF-8/JSON header, or an inconsistent directory
/// tree). **Never panics.**
///
/// # Examples
///
/// ```
/// use aphrody_re::asar;
///
/// // A real archive is built by Electron's `asar` packer; see the crate
/// // tests for an in-memory synthetic builder. Garbage never panics:
/// assert!(asar::list(b"not an asar").is_err());
/// ```
pub fn list(bytes: &[u8]) -> Result<Vec<AsarEntry>, AsarError> {
    let env = parse_envelope(bytes)?;
    let text = std::str::from_utf8(env.header_json)?;
    let root: Value = serde_json::from_str(text)?;
    let root = root
        .as_object()
        .ok_or_else(|| AsarError::Tree("root header is not a JSON object".to_owned()))?;

    let mut out = Vec::new();
    walk(root, "", &mut out)?;
    Ok(out)
}

/// Return the raw bytes of a single file entry, identified by its full
/// POSIX-style `path` (e.g. `"lib/util.js"`).
///
/// The bytes are sliced out of the archive's data section using the entry's
/// `offset` and `size` from the JSON header.
///
/// # Errors
///
/// - [`AsarError::NotFound`] if no entry matches `path`.
/// - [`AsarError::IsDirectory`] if `path` names a directory.
/// - [`AsarError::DataOutOfBounds`] if the entry's `offset + size` runs past
///   the end of the archive (or if the file is `unpacked`, since its bytes
///   are not stored inside the archive).
/// - any envelope/header parse error from [`list`].
///
/// **Never panics.**
///
/// # Examples
///
/// ```
/// use aphrody_re::asar;
///
/// // Garbage input errors rather than panicking.
/// assert!(asar::read_file(b"\x00\x00\x00", "main.js").is_err());
/// ```
pub fn read_file(bytes: &[u8], path: &str) -> Result<Vec<u8>, AsarError> {
    let env = parse_envelope(bytes)?;
    let text = std::str::from_utf8(env.header_json)?;
    let root: Value = serde_json::from_str(text)?;
    let root = root
        .as_object()
        .ok_or_else(|| AsarError::Tree("root header is not a JSON object".to_owned()))?;

    let entries = {
        let mut out = Vec::new();
        walk(root, "", &mut out)?;
        out
    };

    let entry = entries
        .iter()
        .find(|e| e.path == path)
        .ok_or_else(|| AsarError::NotFound(path.to_owned()))?;

    if entry.is_dir {
        return Err(AsarError::IsDirectory(path.to_owned()));
    }
    if entry.unpacked {
        // Bytes live in the sibling `app.asar.unpacked/` tree, not here.
        return Err(AsarError::DataOutOfBounds(format!(
            "{path} is unpacked (stored outside the archive)"
        )));
    }

    let start = env
        .data_start
        .checked_add(usize::try_from(entry.offset).map_err(|_| {
            AsarError::DataOutOfBounds(format!("{path}: offset exceeds usize"))
        })?)
        .ok_or_else(|| AsarError::DataOutOfBounds(format!("{path}: offset overflow")))?;
    let size = usize::try_from(entry.size)
        .map_err(|_| AsarError::DataOutOfBounds(format!("{path}: size exceeds usize")))?;
    let end = start
        .checked_add(size)
        .ok_or_else(|| AsarError::DataOutOfBounds(format!("{path}: end overflow")))?;

    let data = bytes
        .get(start..end)
        .ok_or_else(|| AsarError::DataOutOfBounds(path.to_owned()))?;
    Ok(data.to_vec())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal, well-formed ASAR archive in memory from a JSON header
    /// string and a data section. Mirrors Electron's Pickle writer: the JSON
    /// string field is padded with zero bytes to a 4-byte boundary.
    fn build_asar(header_json: &str, data: &[u8]) -> Vec<u8> {
        let json_bytes = header_json.as_bytes();
        let json_len = json_bytes.len();
        // Pickle aligns the string payload to 4 bytes.
        let padded_len = json_len.div_ceil(4) * 4;
        let pad = padded_len - json_len;

        // Inner (header) pickle = u32 json_len field (4) + padded JSON bytes.
        let string_field_size = 4 + padded_len as u32;
        // Outer payload = u32 holding the inner-pickle size (4) + inner pickle.
        let payload_size = 4 + string_field_size;

        let mut out = Vec::new();
        out.extend_from_slice(&PICKLE_HEADER_SIZE_FIELD.to_le_bytes()); // [0]
        out.extend_from_slice(&payload_size.to_le_bytes()); // [4]
        out.extend_from_slice(&string_field_size.to_le_bytes()); // [8]
        out.extend_from_slice(&(json_len as u32).to_le_bytes()); // [12]
        out.extend_from_slice(json_bytes); // [16..]
        out.extend(std::iter::repeat_n(0u8, pad)); // alignment padding
        // data section
        out.extend_from_slice(data);
        out
    }

    #[test]
    fn round_trip_two_files_in_nested_dirs() {
        // main.js (5 bytes "alert") at offset 0,
        // lib/util.js (3 bytes "abc") at offset 5.
        let header = r#"{
            "files": {
                "main.js": { "size": 5, "offset": "0" },
                "lib": {
                    "files": {
                        "util.js": { "size": 3, "offset": "5" }
                    }
                }
            }
        }"#;
        let data = b"alertabc";
        let archive = build_asar(header, data);

        let entries = list(&archive).expect("list must succeed");
        // Expected: lib (dir), lib/util.js, main.js — sorted within each level.
        let paths: Vec<&str> = entries.iter().map(|e| e.path.as_str()).collect();
        assert_eq!(paths, vec!["lib", "lib/util.js", "main.js"]);

        // Directory entry.
        let lib = &entries[0];
        assert!(lib.is_dir);
        assert_eq!(lib.size, 0);
        assert!(!lib.unpacked);

        // File entries.
        let util = entries.iter().find(|e| e.path == "lib/util.js").unwrap();
        assert!(!util.is_dir);
        assert_eq!(util.size, 3);
        assert_eq!(util.offset, 5);

        let main = entries.iter().find(|e| e.path == "main.js").unwrap();
        assert_eq!(main.size, 5);
        assert_eq!(main.offset, 0);

        // read_file round-trip.
        assert_eq!(read_file(&archive, "main.js").unwrap(), b"alert");
        assert_eq!(read_file(&archive, "lib/util.js").unwrap(), b"abc");
    }

    #[test]
    fn accepts_integer_offset_and_size() {
        // Electron writes string offsets, but accept integers too.
        let header = r#"{"files":{"a.txt":{"size":4,"offset":0}}}"#;
        let archive = build_asar(header, b"data");
        let entries = list(&archive).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].size, 4);
        assert_eq!(read_file(&archive, "a.txt").unwrap(), b"data");
    }

    #[test]
    fn unpacked_file_is_flagged_and_not_extractable() {
        let header = r#"{"files":{"big.bin":{"size":999,"unpacked":true}}}"#;
        let archive = build_asar(header, b"");
        let entries = list(&archive).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].unpacked);
        // Extraction must refuse: bytes are outside the archive.
        let err = read_file(&archive, "big.bin").unwrap_err();
        assert!(matches!(err, AsarError::DataOutOfBounds(_)), "got {err:?}");
    }

    #[test]
    fn read_file_missing_path_errors() {
        let header = r#"{"files":{"a":{"size":1,"offset":"0"}}}"#;
        let archive = build_asar(header, b"x");
        let err = read_file(&archive, "does/not/exist").unwrap_err();
        assert!(matches!(err, AsarError::NotFound(_)), "got {err:?}");
    }

    #[test]
    fn read_file_on_directory_errors() {
        let header = r#"{"files":{"d":{"files":{}}}}"#;
        let archive = build_asar(header, b"");
        let err = read_file(&archive, "d").unwrap_err();
        assert!(matches!(err, AsarError::IsDirectory(_)), "got {err:?}");
    }

    #[test]
    fn out_of_bounds_offset_errors_not_panics() {
        // size+offset run past the (empty) data section.
        let header = r#"{"files":{"a":{"size":100,"offset":"50"}}}"#;
        let archive = build_asar(header, b"short");
        let err = read_file(&archive, "a").unwrap_err();
        assert!(matches!(err, AsarError::DataOutOfBounds(_)), "got {err:?}");
    }

    #[test]
    fn empty_input_is_truncated_not_panic() {
        assert!(matches!(list(b"").unwrap_err(), AsarError::Truncated(_)));
        assert!(matches!(list(b"\x04\x00").unwrap_err(), AsarError::Truncated(_)));
    }

    #[test]
    fn wrong_size_of_size_field_rejected() {
        // size-of-size = 8 instead of canonical 4.
        let mut bytes = 8u32.to_le_bytes().to_vec();
        bytes.extend_from_slice(&[0u8; 12]);
        assert!(matches!(list(&bytes).unwrap_err(), AsarError::Envelope(_)));
    }

    #[test]
    fn inconsistent_payload_size_rejected() {
        // size-of-size=4, but payload_size lies (does not bracket json_len).
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&4u32.to_le_bytes()); // size-of-size
        bytes.extend_from_slice(&999u32.to_le_bytes()); // bogus payload
        bytes.extend_from_slice(&4u32.to_le_bytes()); // string field size
        bytes.extend_from_slice(&2u32.to_le_bytes()); // json_len
        bytes.extend_from_slice(b"{}");
        bytes.extend_from_slice(&[0u8; 2]); // pad
        assert!(matches!(list(&bytes).unwrap_err(), AsarError::Envelope(_)));
    }

    #[test]
    fn malformed_json_header_errors() {
        // Valid envelope, but the JSON header is garbage.
        let archive = build_asar("{not valid json", b"");
        let err = list(&archive).unwrap_err();
        assert!(matches!(err, AsarError::Json(_)), "got {err:?}");
    }

    #[test]
    fn non_utf8_header_errors() {
        // Hand-build an envelope whose "JSON" bytes are invalid UTF-8.
        let json_bytes = &[0xFF, 0xFE, 0xFD, 0xFC];
        let json_len = json_bytes.len() as u32;
        let string_field_size = 4 + json_len; // u32 json_len field + 4-aligned bytes
        let payload_size = 4 + string_field_size;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&4u32.to_le_bytes());
        bytes.extend_from_slice(&payload_size.to_le_bytes());
        bytes.extend_from_slice(&string_field_size.to_le_bytes());
        bytes.extend_from_slice(&json_len.to_le_bytes());
        bytes.extend_from_slice(json_bytes);
        assert!(matches!(list(&bytes).unwrap_err(), AsarError::Utf8(_)));
    }

    #[test]
    fn tree_missing_files_key_errors() {
        let archive = build_asar(r#"{"nope":1}"#, b"");
        let err = list(&archive).unwrap_err();
        assert!(matches!(err, AsarError::Tree(_)), "got {err:?}");
    }

    #[test]
    fn entry_serializes_to_stable_json_shape() {
        let header = r#"{"files":{"a.js":{"size":2,"offset":"0"}}}"#;
        let archive = build_asar(header, b"hi");
        let entries = list(&archive).unwrap();
        let json = serde_json::to_value(&entries[0]).unwrap();
        assert_eq!(json["path"], "a.js");
        assert_eq!(json["size"], 2);
        assert_eq!(json["offset"], 0);
        assert_eq!(json["is_dir"], false);
        assert_eq!(json["unpacked"], false);
    }
}
