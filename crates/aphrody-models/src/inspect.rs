// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors
//
// Format detection and header parsing for local model artefacts.
//
// Everything here operates on a byte prefix, is allocation-bounded and has no
// filesystem or network dependency, so it compiles for wasm32 as well. The
// store calls it with the first N bytes of an artefact (see
// `Inspector::PREFIX_BYTES`); a caller holding weights in memory can call the
// same entry point directly.
//
// Supported headers, all parsed by hand against the upstream on-disk layouts:
//
//   * GGUF   (llama.cpp)  - <https://github.com/ggml-org/ggml/blob/master/docs/gguf.md>
//   * GGML   (whisper.cpp) - the legacy `ggml-*.bin` whisper hyper-parameter block
//   * safetensors          - u64 header length + JSON tensor table
//   * ONNX                 - the `ModelProto` protobuf top level
//
// Nothing here executes model data: a header parse is a pure read over a
// length-checked cursor, and every length taken from the file is validated
// against the remaining buffer before it is used.

use std::collections::BTreeMap;

/// How many leading bytes of an artefact are enough to identify it and read
/// its header. GGUF metadata blocks are the largest of the supported headers
/// and comfortably fit; a truncated read degrades to a partial inspection
/// rather than an error.
pub const PREFIX_BYTES: usize = 1 << 20;

/// The artefact container format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum ArtifactFormat {
    /// llama.cpp GGUF (quantised LLM / embedding weights).
    Gguf,
    /// Legacy GGML container as produced by whisper.cpp (`ggml-*.bin`).
    Ggml,
    /// Hugging Face `safetensors`.
    Safetensors,
    /// ONNX `ModelProto` (what ONNX Runtime loads).
    Onnx,
    /// A `PyTorch` archive (zip-based `.pt` / `.bin`).
    PyTorch,
    /// A JSON sidecar (tokenizer, config, preprocessor).
    Json,
    /// Recognised as none of the above.
    Unknown,
}

impl ArtifactFormat {
    /// Stable machine-friendly name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Gguf => "gguf",
            Self::Ggml => "ggml",
            Self::Safetensors => "safetensors",
            Self::Onnx => "onnx",
            Self::PyTorch => "pytorch",
            Self::Json => "json",
            Self::Unknown => "unknown",
        }
    }

    /// Whether aphrody has a local inference backend able to load this format.
    #[must_use]
    pub const fn is_loadable(self) -> bool {
        matches!(self, Self::Gguf | Self::Ggml | Self::Onnx | Self::Safetensors)
    }
}

impl core::fmt::Display for ArtifactFormat {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A parsed artefact header.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Inspection {
    /// Detected container format.
    pub format: ArtifactFormat,
    /// Format-specific header details, when the header could be parsed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Details>,
    /// Set when the format was recognised but the header could not be fully
    /// decoded (truncated prefix, unknown value type, malformed length).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

/// Format-specific header payloads.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Details {
    /// GGUF header + scalar metadata key/values.
    Gguf(GgufHeader),
    /// whisper.cpp GGML hyper-parameters.
    Ggml(GgmlHeader),
    /// safetensors tensor table summary.
    Safetensors(SafetensorsHeader),
    /// ONNX `ModelProto` top-level fields.
    Onnx(OnnxHeader),
}

// ---------------------------------------------------------------------------
// GGUF
// ---------------------------------------------------------------------------

/// Decoded GGUF header.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GgufHeader {
    /// GGUF container version (1, 2 or 3 in the wild).
    pub version: u32,
    /// Number of tensors declared in the tensor-info table.
    pub tensor_count: u64,
    /// Number of metadata key/value pairs declared.
    pub metadata_count: u64,
    /// Scalar metadata rendered as strings, keyed by GGUF key
    /// (`general.architecture`, `llama.context_length`, ...). Array-valued
    /// keys are summarised as `[<type> x <len>]` instead of being expanded.
    pub metadata: BTreeMap<String, String>,
}

impl GgufHeader {
    /// `general.architecture`, the single most useful routing key.
    #[must_use]
    pub fn architecture(&self) -> Option<&str> {
        self.metadata.get("general.architecture").map(String::as_str)
    }

    /// `general.name`, the human label baked in at conversion time.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.metadata.get("general.name").map(String::as_str)
    }
}

/// GGUF metadata value type tags (`gguf_metadata_value_type`).
const GGUF_UINT8: u32 = 0;
const GGUF_INT8: u32 = 1;
const GGUF_UINT16: u32 = 2;
const GGUF_INT16: u32 = 3;
const GGUF_UINT32: u32 = 4;
const GGUF_INT32: u32 = 5;
const GGUF_FLOAT32: u32 = 6;
const GGUF_BOOL: u32 = 7;
const GGUF_STRING: u32 = 8;
const GGUF_ARRAY: u32 = 9;
const GGUF_UINT64: u32 = 10;
const GGUF_INT64: u32 = 11;
const GGUF_FLOAT64: u32 = 12;

const fn gguf_type_name(t: u32) -> &'static str {
    match t {
        GGUF_UINT8 => "u8",
        GGUF_INT8 => "i8",
        GGUF_UINT16 => "u16",
        GGUF_INT16 => "i16",
        GGUF_UINT32 => "u32",
        GGUF_INT32 => "i32",
        GGUF_FLOAT32 => "f32",
        GGUF_BOOL => "bool",
        GGUF_STRING => "str",
        GGUF_ARRAY => "array",
        GGUF_UINT64 => "u64",
        GGUF_INT64 => "i64",
        GGUF_FLOAT64 => "f64",
        _ => "?",
    }
}

// ---------------------------------------------------------------------------
// GGML (whisper.cpp)
// ---------------------------------------------------------------------------

/// whisper.cpp `ggml-*.bin` hyper-parameter block.
///
/// Layout after the 4-byte magic: eleven little-endian `i32` fields, in the
/// order whisper.cpp writes them in `whisper_model_load`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GgmlHeader {
    /// Vocabulary size. 51865 for multilingual, 51864 for the `.en` models.
    pub n_vocab: i32,
    /// Audio encoder context length (mel frames).
    pub n_audio_ctx: i32,
    /// Audio encoder hidden width.
    pub n_audio_state: i32,
    /// Audio encoder attention heads.
    pub n_audio_head: i32,
    /// Audio encoder layers.
    pub n_audio_layer: i32,
    /// Text decoder context length (tokens).
    pub n_text_ctx: i32,
    /// Text decoder hidden width.
    pub n_text_state: i32,
    /// Text decoder attention heads.
    pub n_text_head: i32,
    /// Text decoder layers.
    pub n_text_layer: i32,
    /// Number of mel filterbank channels (80, or 128 for large-v3).
    pub n_mels: i32,
    /// Quantisation / tensor type tag.
    pub ftype: i32,
    /// Model size inferred from `n_audio_state`, e.g. `tiny`, `base`, `large`.
    pub variant: String,
    /// Whether the checkpoint is English-only (derived from `n_vocab`).
    pub english_only: bool,
}

/// Map the audio hidden width onto whisper's published size ladder.
const fn whisper_variant(n_audio_state: i32) -> &'static str {
    match n_audio_state {
        384 => "tiny",
        512 => "base",
        768 => "small",
        1024 => "medium",
        1280 => "large",
        _ => "unknown",
    }
}

// ---------------------------------------------------------------------------
// safetensors
// ---------------------------------------------------------------------------

/// safetensors tensor-table summary.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SafetensorsHeader {
    /// Number of tensors in the table (`__metadata__` excluded).
    pub tensor_count: usize,
    /// Sum of every tensor's element count.
    pub total_elements: u64,
    /// Distinct dtypes present, sorted.
    pub dtypes: Vec<String>,
    /// The `__metadata__` map, when present.
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub metadata: BTreeMap<String, String>,
}

// ---------------------------------------------------------------------------
// ONNX
// ---------------------------------------------------------------------------

/// ONNX `ModelProto` top-level fields.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct OnnxHeader {
    /// `ir_version` (field 1).
    pub ir_version: i64,
    /// `producer_name` (field 2), e.g. `pytorch`, `paddle2onnx`.
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub producer_name: String,
    /// `producer_version` (field 3).
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub producer_version: String,
    /// `opset_import` (field 8) rendered as `domain:version`; the default
    /// ONNX domain is spelled with an empty domain string.
    pub opsets: Vec<String>,
    /// `graph.name` (field 7 -> field 2), when the graph header was reached.
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub graph_name: String,
    /// Number of graph inputs (field 7 -> field 11).
    pub input_count: usize,
    /// Number of graph outputs (field 7 -> field 12).
    pub output_count: usize,
    /// Set when the graph ran past the bytes that were read. Fields the
    /// producer serialised after the graph — `opset_import` among them — are
    /// then simply not in the prefix, and their absence here means "not seen",
    /// not "not present in the file".
    #[serde(default, skip_serializing_if = "core::ops::Not::not")]
    pub graph_truncated: bool,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Identify an artefact and decode its header from a leading byte prefix.
///
/// `hint_name` is the file name (used only to disambiguate extension-only
/// formats such as `.json` sidecars); pass `""` when unavailable. The function
/// never fails: an unrecognised or truncated artefact yields
/// [`ArtifactFormat::Unknown`] or a populated `warning`.
#[must_use]
pub fn inspect_prefix(prefix: &[u8], hint_name: &str) -> Inspection {
    if prefix.starts_with(b"GGUF") {
        return match parse_gguf(prefix) {
            // A partial metadata map is still the useful one: `general.*` keys
            // are written first, and the tail that gets cut off is the
            // tokenizer vocabulary. Keep what was decoded AND say why it
            // stopped, rather than discarding a usable header.
            Ok((header, warning)) => Inspection {
                format: ArtifactFormat::Gguf,
                details: Some(Details::Gguf(header)),
                warning,
            },
            Err(reason) => Inspection {
                format: ArtifactFormat::Gguf,
                details: None,
                warning: Some(reason),
            },
        };
    }

    // whisper.cpp writes GGML_FILE_MAGIC (0x6767_6d6c) as a native-endian u32,
    // which on every target aphrody supports lands as the bytes `lmgg`.
    if prefix.starts_with(b"lmgg") {
        return match parse_ggml_whisper(prefix) {
            Ok(h) => {
                Inspection { format: ArtifactFormat::Ggml, details: Some(Details::Ggml(h)), warning: None }
            }
            Err(reason) => Inspection {
                format: ArtifactFormat::Ggml,
                details: None,
                warning: Some(reason),
            },
        };
    }

    // Zip local-file header: PyTorch `.pt` / `.bin` archives.
    if prefix.starts_with(b"PK\x03\x04") {
        return Inspection { format: ArtifactFormat::PyTorch, details: None, warning: None };
    }

    if let Some(result) = try_safetensors(prefix) {
        return result;
    }

    if let Some(result) = try_onnx(prefix) {
        return result;
    }

    if has_json_extension(hint_name) || looks_like_json(prefix) {
        return Inspection { format: ArtifactFormat::Json, details: None, warning: None };
    }

    Inspection { format: ArtifactFormat::Unknown, details: None, warning: None }
}

/// Whether a file name ends in a `.json` extension, case-insensitively:
/// `Tokenizer.JSON` off a Windows share is the same sidecar.
fn has_json_extension(hint_name: &str) -> bool {
    std::path::Path::new(hint_name)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
}

fn looks_like_json(prefix: &[u8]) -> bool {
    prefix.iter().find(|b| !b.is_ascii_whitespace()).is_some_and(|b| *b == b'{' || *b == b'[')
}

// ---------------------------------------------------------------------------
// Cursor
// ---------------------------------------------------------------------------

/// A bounds-checked little-endian reader over a byte prefix.
///
/// Every read validates the remaining length first, so a malformed or
/// truncated header returns `Err` instead of panicking or over-reading.
struct Cursor<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    const fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    fn take(&mut self, n: usize) -> core::result::Result<&'a [u8], String> {
        let end = self.pos.checked_add(n).ok_or_else(|| "length overflow".to_owned())?;
        let slice = self
            .buf
            .get(self.pos..end)
            .ok_or_else(|| format!("truncated: need {n} bytes at offset {}", self.pos))?;
        self.pos = end;
        Ok(slice)
    }

    fn u8(&mut self) -> core::result::Result<u8, String> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> core::result::Result<u16, String> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    fn u32(&mut self) -> core::result::Result<u32, String> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn u64(&mut self) -> core::result::Result<u64, String> {
        let b = self.take(8)?;
        Ok(u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]))
    }

    fn i32(&mut self) -> core::result::Result<i32, String> {
        Ok(self.u32()?.cast_signed())
    }

    fn f32(&mut self) -> core::result::Result<f32, String> {
        Ok(f32::from_bits(self.u32()?))
    }

    fn f64(&mut self) -> core::result::Result<f64, String> {
        Ok(f64::from_bits(self.u64()?))
    }

    /// A GGUF counted string: length prefix then raw UTF-8 bytes.
    ///
    /// GGUF v1 used a `u32` length; v2 widened it to `u64`.
    fn gguf_string(&mut self, wide_len: bool) -> core::result::Result<String, String> {
        let len = if wide_len { self.u64()? } else { u64::from(self.u32()?) };
        let len = usize::try_from(len).map_err(|_| "string length exceeds usize".to_owned())?;
        // Guard against a corrupt length claiming more than the whole prefix.
        if len > self.buf.len() {
            return Err(format!("string length {len} exceeds buffer"));
        }
        let bytes = self.take(len)?;
        Ok(String::from_utf8_lossy(bytes).into_owned())
    }
}

// ---------------------------------------------------------------------------
// GGUF parser
// ---------------------------------------------------------------------------

/// Parse a GGUF header.
///
/// Returns the header plus an optional warning describing where decoding
/// stopped. `Err` is reserved for a header so malformed that not even the
/// fixed-size prologue (magic, version, counts) could be read.
fn parse_gguf(prefix: &[u8]) -> core::result::Result<(GgufHeader, Option<String>), String> {
    let mut c = Cursor::new(prefix);
    let _magic = c.take(4)?;
    let version = c.u32()?;
    // v1 wrote the counts and every string length as u32; v2+ widened to u64.
    let wide = version >= 2;
    let tensor_count = if wide { c.u64()? } else { u64::from(c.u32()?) };
    let metadata_count = if wide { c.u64()? } else { u64::from(c.u32()?) };

    let mut metadata = BTreeMap::new();
    let mut truncated_at = None;
    for i in 0..metadata_count {
        match read_gguf_kv(&mut c, wide) {
            Ok((key, value)) => {
                metadata.insert(key, value);
            }
            Err(e) => {
                truncated_at = Some(format!("metadata pair {i}/{metadata_count}: {e}"));
                break;
            }
        }
    }

    let warning = truncated_at.map(|reason| {
        format!(
            "decoded {} of {metadata_count} metadata pairs before {reason}",
            metadata.len()
        )
    });

    Ok((GgufHeader { version, tensor_count, metadata_count, metadata }, warning))
}

fn read_gguf_kv(
    c: &mut Cursor<'_>,
    wide: bool,
) -> core::result::Result<(String, String), String> {
    let key = c.gguf_string(wide)?;
    let value_type = c.u32()?;
    let value = read_gguf_value(c, value_type, wide)?;
    Ok((key, value))
}

fn read_gguf_value(
    c: &mut Cursor<'_>,
    value_type: u32,
    wide: bool,
) -> core::result::Result<String, String> {
    Ok(match value_type {
        GGUF_UINT8 => c.u8()?.to_string(),
        GGUF_INT8 => c.u8()?.cast_signed().to_string(),
        GGUF_UINT16 => c.u16()?.to_string(),
        GGUF_INT16 => c.u16()?.cast_signed().to_string(),
        GGUF_UINT32 => c.u32()?.to_string(),
        GGUF_INT32 => c.i32()?.to_string(),
        GGUF_FLOAT32 => c.f32()?.to_string(),
        GGUF_BOOL => (c.u8()? != 0).to_string(),
        GGUF_STRING => c.gguf_string(wide)?,
        GGUF_UINT64 => c.u64()?.to_string(),
        GGUF_INT64 => c.u64()?.cast_signed().to_string(),
        GGUF_FLOAT64 => c.f64()?.to_string(),
        GGUF_ARRAY => {
            let elem_type = c.u32()?;
            let len = if wide { c.u64()? } else { u64::from(c.u32()?) };
            // Arrays are the tokenizer vocab and merge tables: hundreds of
            // thousands of entries, none of them useful as a summary. Skip the
            // payload and record the shape instead.
            skip_gguf_array(c, elem_type, len, wide)?;
            format!("[{} x {len}]", gguf_type_name(elem_type))
        }
        other => return Err(format!("unknown GGUF value type {other}")),
    })
}

fn skip_gguf_array(
    c: &mut Cursor<'_>,
    elem_type: u32,
    len: u64,
    wide: bool,
) -> core::result::Result<(), String> {
    // Fixed-width elements skip in one arithmetic jump; strings and nested
    // arrays must be walked because each entry is self-describing.
    let fixed = match elem_type {
        GGUF_UINT8 | GGUF_INT8 | GGUF_BOOL => Some(1_usize),
        GGUF_UINT16 | GGUF_INT16 => Some(2),
        GGUF_UINT32 | GGUF_INT32 | GGUF_FLOAT32 => Some(4),
        GGUF_UINT64 | GGUF_INT64 | GGUF_FLOAT64 => Some(8),
        _ => None,
    };
    if let Some(width) = fixed {
        let len = usize::try_from(len).map_err(|_| "array length exceeds usize".to_owned())?;
        let total = len.checked_mul(width).ok_or_else(|| "array size overflow".to_owned())?;
        c.take(total)?;
        return Ok(());
    }
    for _ in 0..len {
        match elem_type {
            GGUF_STRING => {
                c.gguf_string(wide)?;
            }
            GGUF_ARRAY => {
                let inner_type = c.u32()?;
                let inner_len = if wide { c.u64()? } else { u64::from(c.u32()?) };
                skip_gguf_array(c, inner_type, inner_len, wide)?;
            }
            other => return Err(format!("unknown GGUF array element type {other}")),
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// GGML (whisper) parser
// ---------------------------------------------------------------------------

fn parse_ggml_whisper(prefix: &[u8]) -> core::result::Result<GgmlHeader, String> {
    let mut c = Cursor::new(prefix);
    let _magic = c.take(4)?;
    let n_vocab = c.i32()?;
    let n_audio_ctx = c.i32()?;
    let n_audio_state = c.i32()?;
    let n_audio_head = c.i32()?;
    let n_audio_layer = c.i32()?;
    let n_text_ctx = c.i32()?;
    let n_text_state = c.i32()?;
    let n_text_head = c.i32()?;
    let n_text_layer = c.i32()?;
    let n_mels = c.i32()?;
    let ftype = c.i32()?;

    // Whisper vocabularies are ~51.8k entries; anything far outside that band
    // means the `lmgg` magic belonged to some other GGML-family container.
    if !(1..=1_000_000).contains(&n_vocab) || n_audio_state <= 0 {
        return Err(format!(
            "GGML magic matched but hyper-parameters are not whisper-shaped (n_vocab={n_vocab}, n_audio_state={n_audio_state})"
        ));
    }

    Ok(GgmlHeader {
        n_vocab,
        n_audio_ctx,
        n_audio_state,
        n_audio_head,
        n_audio_layer,
        n_text_ctx,
        n_text_state,
        n_text_head,
        n_text_layer,
        n_mels,
        ftype,
        variant: whisper_variant(n_audio_state).to_owned(),
        // whisper.cpp reserves 51865 for the multilingual vocabulary and
        // 51864 for the English-only checkpoints.
        english_only: n_vocab == 51_864,
    })
}

// ---------------------------------------------------------------------------
// safetensors parser
// ---------------------------------------------------------------------------

fn try_safetensors(prefix: &[u8]) -> Option<Inspection> {
    // Layout: u64 LE header length, then exactly that many bytes of JSON.
    let raw_len = prefix.get(..8)?;
    let header_len =
        u64::from_le_bytes([raw_len[0], raw_len[1], raw_len[2], raw_len[3], raw_len[4], raw_len[5], raw_len[6], raw_len[7]]);
    // Sanity band: a real header is at least `{}` and never gigabytes.
    if !(2..=(100 << 20)).contains(&header_len) {
        return None;
    }
    let header_len = usize::try_from(header_len).ok()?;
    let available = prefix.len().saturating_sub(8);
    if available < header_len {
        // The declared header runs past our prefix: only claim the format if
        // the visible bytes already look like the tensor table.
        let visible = prefix.get(8..)?;
        if !looks_like_json(visible) {
            return None;
        }
        return Some(Inspection {
            format: ArtifactFormat::Safetensors,
            details: None,
            warning: Some(format!(
                "safetensors header is {header_len} bytes, beyond the {available}-byte prefix read"
            )),
        });
    }

    let json = prefix.get(8..8 + header_len)?;
    if !looks_like_json(json) {
        return None;
    }
    let parsed: serde_json::Value = serde_json::from_slice(json).ok()?;
    let table = parsed.as_object()?;

    let mut metadata = BTreeMap::new();
    if let Some(serde_json::Value::Object(m)) = table.get("__metadata__") {
        for (k, v) in m {
            metadata.insert(k.clone(), v.as_str().map_or_else(|| v.to_string(), ToOwned::to_owned));
        }
    }

    let mut dtypes = Vec::new();
    let mut tensor_count = 0_usize;
    let mut total_elements = 0_u64;
    for (name, entry) in table {
        if name == "__metadata__" {
            continue;
        }
        let Some(obj) = entry.as_object() else { continue };
        tensor_count += 1;
        if let Some(dt) = obj.get("dtype").and_then(serde_json::Value::as_str) {
            let dt = dt.to_owned();
            if !dtypes.contains(&dt) {
                dtypes.push(dt);
            }
        }
        if let Some(shape) = obj.get("shape").and_then(serde_json::Value::as_array) {
            let elements = shape
                .iter()
                .filter_map(serde_json::Value::as_u64)
                .try_fold(1_u64, u64::checked_mul);
            if let Some(e) = elements {
                total_elements = total_elements.saturating_add(e);
            }
        }
    }
    dtypes.sort_unstable();

    Some(Inspection {
        format: ArtifactFormat::Safetensors,
        details: Some(Details::Safetensors(SafetensorsHeader {
            tensor_count,
            total_elements,
            dtypes,
            metadata,
        })),
        warning: None,
    })
}

// ---------------------------------------------------------------------------
// ONNX parser (hand-rolled protobuf wire reader)
// ---------------------------------------------------------------------------

/// Protobuf wire types we handle.
const WIRE_VARINT: u8 = 0;
const WIRE_I64: u8 = 1;
const WIRE_LEN: u8 = 2;
const WIRE_I32: u8 = 5;

struct Pb<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Pb<'a> {
    const fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    const fn done(&self) -> bool {
        self.pos >= self.buf.len()
    }

    fn varint(&mut self) -> Option<u64> {
        let mut value = 0_u64;
        let mut shift = 0_u32;
        loop {
            let byte = *self.buf.get(self.pos)?;
            self.pos += 1;
            value |= u64::from(byte & 0x7F) << shift;
            if byte & 0x80 == 0 {
                return Some(value);
            }
            shift += 7;
            if shift > 63 {
                return None;
            }
        }
    }

    /// Read the next tag, returning `(field_number, wire_type)`.
    fn tag(&mut self) -> Option<(u32, u8)> {
        let key = self.varint()?;
        let field = u32::try_from(key >> 3).ok()?;
        let wire = u8::try_from(key & 0x7).ok()?;
        Some((field, wire))
    }

    fn bytes(&mut self) -> Option<&'a [u8]> {
        let len = usize::try_from(self.varint()?).ok()?;
        let end = self.pos.checked_add(len)?;
        let slice = self.buf.get(self.pos..end)?;
        self.pos = end;
        Some(slice)
    }

    /// Read a length-delimited field, accepting a payload that runs past the
    /// end of the buffer.
    ///
    /// Returns `(visible_bytes, complete)`. This exists because a real
    /// `ModelProto` embeds its whole graph — often hundreds of megabytes — in
    /// field 7, so on any prefix read that one field is guaranteed to be
    /// truncated. Refusing it would throw away the graph header that IS
    /// present, which is exactly the part worth reporting.
    fn bytes_lossy(&mut self) -> Option<(&'a [u8], bool)> {
        let len = usize::try_from(self.varint()?).ok()?;
        let available = self.buf.len().saturating_sub(self.pos);
        let taken = len.min(available);
        let slice = self.buf.get(self.pos..self.pos + taken)?;
        self.pos += taken;
        Some((slice, taken == len))
    }

    fn string(&mut self) -> Option<String> {
        Some(String::from_utf8_lossy(self.bytes()?).into_owned())
    }

    /// Advance past a field whose contents we do not need.
    fn skip(&mut self, wire: u8) -> Option<()> {
        match wire {
            WIRE_VARINT => {
                self.varint()?;
            }
            WIRE_I64 => {
                self.pos = self.pos.checked_add(8)?;
            }
            WIRE_LEN => {
                self.bytes()?;
            }
            WIRE_I32 => {
                self.pos = self.pos.checked_add(4)?;
            }
            _ => return None,
        }
        if self.pos > self.buf.len() { None } else { Some(()) }
    }
}

fn try_onnx(prefix: &[u8]) -> Option<Inspection> {
    // `ModelProto.ir_version` is field 1, varint -> tag byte 0x08. Every ONNX
    // file emitted by a mainstream exporter starts there, and requiring it
    // keeps this heuristic from claiming arbitrary binaries.
    if prefix.first() != Some(&0x08) {
        return None;
    }
    let header = parse_onnx(prefix)?;
    Some(Inspection {
        format: ArtifactFormat::Onnx,
        details: Some(Details::Onnx(header)),
        warning: None,
    })
}

fn parse_onnx(buf: &[u8]) -> Option<OnnxHeader> {
    let mut pb = Pb::new(buf);
    let mut header = OnnxHeader {
        ir_version: 0,
        producer_name: String::new(),
        producer_version: String::new(),
        opsets: Vec::new(),
        graph_name: String::new(),
        input_count: 0,
        output_count: 0,
        graph_truncated: false,
    };

    // The store only ever reads a prefix, and a `ModelProto` routinely runs to
    // hundreds of megabytes, so hitting the end of the buffer mid-field is the
    // NORMAL case, not an error: stop walking and keep whatever was decoded.
    while !pb.done() {
        let Some((field, wire)) = pb.tag() else { break };
        let stepped = match (field, wire) {
            // ModelProto.ir_version
            (1, WIRE_VARINT) => pb.varint().map(|v| {
                header.ir_version = i64::try_from(v).unwrap_or(-1);
            }),
            // ModelProto.producer_name / producer_version
            (2, WIRE_LEN) => pb.string().map(|s| header.producer_name = s),
            (3, WIRE_LEN) => pb.string().map(|s| header.producer_version = s),
            // ModelProto.graph — the payload dwarfs the prefix, so only its
            // own header fields are read and the rest is left untouched.
            // A truncated graph is read as far as it goes, then the walk stops:
            // every later top-level field lies beyond the bytes we hold.
            (7, WIRE_LEN) => match pb.bytes_lossy() {
                Some((graph, complete)) => {
                    read_onnx_graph(graph, &mut header);
                    if complete {
                        Some(())
                    } else {
                        header.graph_truncated = true;
                        None
                    }
                }
                None => None,
            },
            // ModelProto.opset_import (repeated OperatorSetIdProto)
            (8, WIRE_LEN) => pb.bytes().map(|opset| {
                if let Some(rendered) = read_onnx_opset(opset) {
                    header.opsets.push(rendered);
                }
            }),
            _ => pb.skip(wire),
        };
        if stepped.is_none() {
            break;
        }
    }

    // A real ModelProto always carries an ir_version >= 1.
    (header.ir_version >= 1).then_some(header)
}

fn read_onnx_opset(buf: &[u8]) -> Option<String> {
    let mut pb = Pb::new(buf);
    let mut domain = String::new();
    let mut version = 0_i64;
    while !pb.done() {
        let Some((field, wire)) = pb.tag() else { break };
        match (field, wire) {
            (1, WIRE_LEN) => domain = pb.string()?,
            (2, WIRE_VARINT) => version = i64::try_from(pb.varint()?).unwrap_or(-1),
            _ => pb.skip(wire)?,
        }
    }
    Some(format!("{domain}:{version}"))
}

/// Read `GraphProto`'s name and count its inputs/outputs.
///
/// The graph body (nodes, initializers) is skipped: a truncated prefix simply
/// stops the walk early, which is why this returns `()` and mutates in place
/// instead of failing the whole inspection.
fn read_onnx_graph(buf: &[u8], header: &mut OnnxHeader) {
    let mut pb = Pb::new(buf);
    while !pb.done() {
        let Some((field, wire)) = pb.tag() else { break };
        let stepped = match (field, wire) {
            (2, WIRE_LEN) => pb.string().map(|name| header.graph_name = name),
            (11, WIRE_LEN) => pb.bytes().map(|_| header.input_count += 1),
            (12, WIRE_LEN) => pb.bytes().map(|_| header.output_count += 1),
            _ => pb.skip(wire),
        };
        if stepped.is_none() {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- builders ---------------------------------------------------------

    fn gguf_str(s: &str) -> Vec<u8> {
        let mut v = (s.len() as u64).to_le_bytes().to_vec();
        v.extend_from_slice(s.as_bytes());
        v
    }

    /// Minimal but wire-accurate GGUF v3 file: two scalar keys and one array.
    fn build_gguf() -> Vec<u8> {
        let mut v = b"GGUF".to_vec();
        v.extend_from_slice(&3_u32.to_le_bytes()); // version
        v.extend_from_slice(&291_u64.to_le_bytes()); // tensor_count
        v.extend_from_slice(&3_u64.to_le_bytes()); // metadata_count

        v.extend(gguf_str("general.architecture"));
        v.extend_from_slice(&GGUF_STRING.to_le_bytes());
        v.extend(gguf_str("llama"));

        v.extend(gguf_str("llama.context_length"));
        v.extend_from_slice(&GGUF_UINT32.to_le_bytes());
        v.extend_from_slice(&4096_u32.to_le_bytes());

        v.extend(gguf_str("tokenizer.ggml.tokens"));
        v.extend_from_slice(&GGUF_ARRAY.to_le_bytes());
        v.extend_from_slice(&GGUF_STRING.to_le_bytes());
        v.extend_from_slice(&2_u64.to_le_bytes());
        v.extend(gguf_str("<s>"));
        v.extend(gguf_str("</s>"));
        v
    }

    fn build_whisper_ggml(n_audio_state: i32, n_vocab: i32) -> Vec<u8> {
        let mut v = b"lmgg".to_vec();
        for field in
            [n_vocab, 1500, n_audio_state, 6, 4, 448, n_audio_state, 6, 4, 80, 1]
        {
            v.extend_from_slice(&field.to_le_bytes());
        }
        v
    }

    fn build_safetensors(json: &str) -> Vec<u8> {
        let mut v = (json.len() as u64).to_le_bytes().to_vec();
        v.extend_from_slice(json.as_bytes());
        v
    }

    fn pb_tag(field: u32, wire: u8) -> Vec<u8> {
        pb_varint((u64::from(field) << 3) | u64::from(wire))
    }

    fn pb_varint(mut n: u64) -> Vec<u8> {
        let mut out = Vec::new();
        loop {
            let byte = u8::try_from(n & 0x7F).unwrap();
            n >>= 7;
            if n == 0 {
                out.push(byte);
                return out;
            }
            out.push(byte | 0x80);
        }
    }

    fn pb_len_field(field: u32, payload: &[u8]) -> Vec<u8> {
        let mut v = pb_tag(field, WIRE_LEN);
        v.extend(pb_varint(payload.len() as u64));
        v.extend_from_slice(payload);
        v
    }

    fn build_onnx() -> Vec<u8> {
        let mut graph = pb_len_field(2, b"paddle-ocr-det");
        graph.extend(pb_len_field(11, b"\x0a\x01x")); // input `x`
        graph.extend(pb_len_field(12, b"\x0a\x01y")); // output `y`

        let mut opset = pb_tag(2, WIRE_VARINT);
        opset.extend(pb_varint(17));

        let mut v = pb_tag(1, WIRE_VARINT);
        v.extend(pb_varint(8)); // ir_version = 8
        v.extend(pb_len_field(2, b"paddle2onnx"));
        v.extend(pb_len_field(3, b"1.0.6"));
        v.extend(pb_len_field(7, &graph));
        v.extend(pb_len_field(8, &opset));
        v
    }

    // -- GGUF -------------------------------------------------------------

    #[test]
    fn gguf_header_is_decoded() {
        let bytes = build_gguf();
        let got = inspect_prefix(&bytes, "model.gguf");
        assert_eq!(got.format, ArtifactFormat::Gguf);
        assert!(got.warning.is_none(), "{:?}", got.warning);
        let Some(Details::Gguf(h)) = got.details else { panic!("no gguf details") };
        assert_eq!(h.version, 3);
        assert_eq!(h.tensor_count, 291);
        assert_eq!(h.architecture(), Some("llama"));
        assert_eq!(h.metadata.get("llama.context_length").map(String::as_str), Some("4096"));
        // The vocab array is summarised, never expanded.
        assert_eq!(h.metadata.get("tokenizer.ggml.tokens").map(String::as_str), Some("[str x 2]"));
    }

    #[test]
    fn truncated_gguf_keeps_what_it_decoded_and_says_where_it_stopped() {
        let bytes = build_gguf();
        // Cut mid-metadata: the prologue and the first key survive, the rest
        // does not. This is the NORMAL case on a real file, where the
        // tokenizer vocabulary runs past any bounded prefix read.
        let got = inspect_prefix(&bytes[..70], "model.gguf");
        assert_eq!(got.format, ArtifactFormat::Gguf);

        let warning = got.warning.expect("expected a truncation warning");
        assert!(warning.contains("of 3 metadata pairs"), "{warning}");

        // The header that WAS decoded must still be reported: counts, and the
        // `general.*` keys that come first on disk.
        let Some(Details::Gguf(header)) = got.details else { panic!("partial header was dropped") };
        assert_eq!(header.version, 3);
        assert_eq!(header.tensor_count, 291);
        assert_eq!(header.metadata_count, 3);
        assert_eq!(header.architecture(), Some("llama"));
        assert!(header.metadata.len() < 3, "expected a partial map, got {:?}", header.metadata);
    }

    #[test]
    fn a_gguf_too_short_for_its_prologue_yields_no_header() {
        // Not even magic + version + counts fit: there is nothing to report.
        let got = inspect_prefix(b"GGUF\x03\x00\x00\x00", "model.gguf");
        assert_eq!(got.format, ArtifactFormat::Gguf);
        assert!(got.details.is_none());
        assert!(got.warning.is_some());
    }

    #[test]
    fn corrupt_gguf_string_length_does_not_panic() {
        let mut bytes = build_gguf();
        // Overwrite the first key's length prefix with a huge value.
        bytes[24..32].copy_from_slice(&u64::MAX.to_le_bytes());
        let got = inspect_prefix(&bytes, "model.gguf");
        assert_eq!(got.format, ArtifactFormat::Gguf);
        assert!(got.warning.is_some());
    }

    // -- GGML / whisper ---------------------------------------------------

    #[test]
    fn whisper_ggml_hyperparameters_are_decoded() {
        let got = inspect_prefix(&build_whisper_ggml(512, 51_864), "ggml-base.en.bin");
        assert_eq!(got.format, ArtifactFormat::Ggml);
        let Some(Details::Ggml(h)) = got.details else { panic!("no ggml details") };
        assert_eq!(h.variant, "base");
        assert!(h.english_only);
        assert_eq!(h.n_mels, 80);
        assert_eq!(h.n_audio_ctx, 1500);
    }

    #[test]
    fn multilingual_whisper_is_not_flagged_english_only() {
        let got = inspect_prefix(&build_whisper_ggml(1280, 51_865), "ggml-large-v3.bin");
        let Some(Details::Ggml(h)) = got.details else { panic!() };
        assert_eq!(h.variant, "large");
        assert!(!h.english_only);
    }

    #[test]
    fn non_whisper_ggml_container_is_rejected_with_a_warning() {
        let mut bytes = b"lmgg".to_vec();
        bytes.extend_from_slice(&[0_u8; 44]);
        let got = inspect_prefix(&bytes, "other.bin");
        assert_eq!(got.format, ArtifactFormat::Ggml);
        assert!(got.details.is_none());
        assert!(got.warning.unwrap().contains("not whisper-shaped"));
    }

    // -- safetensors ------------------------------------------------------

    #[test]
    fn safetensors_table_is_summarised() {
        let json = r#"{"__metadata__":{"format":"pt"},"a":{"dtype":"F32","shape":[2,3],"data_offsets":[0,24]},"b":{"dtype":"F16","shape":[4],"data_offsets":[24,32]}}"#;
        let got = inspect_prefix(&build_safetensors(json), "model.safetensors");
        assert_eq!(got.format, ArtifactFormat::Safetensors);
        let Some(Details::Safetensors(h)) = got.details else { panic!("no details") };
        assert_eq!(h.tensor_count, 2);
        assert_eq!(h.total_elements, 10); // 2*3 + 4
        assert_eq!(h.dtypes, vec!["F16".to_owned(), "F32".to_owned()]);
        assert_eq!(h.metadata.get("format").map(String::as_str), Some("pt"));
    }

    #[test]
    fn safetensors_header_beyond_the_prefix_is_flagged_not_dropped() {
        let json = r#"{"a":{"dtype":"F32","shape":[1],"data_offsets":[0,4]}}"#;
        let full = build_safetensors(json);
        let got = inspect_prefix(&full[..full.len() - 10], "model.safetensors");
        assert_eq!(got.format, ArtifactFormat::Safetensors);
        assert!(got.warning.unwrap().contains("beyond the"));
    }

    // -- ONNX -------------------------------------------------------------

    #[test]
    fn onnx_modelproto_top_level_is_decoded() {
        let got = inspect_prefix(&build_onnx(), "det.onnx");
        assert_eq!(got.format, ArtifactFormat::Onnx);
        let Some(Details::Onnx(h)) = got.details else { panic!("no onnx details") };
        assert_eq!(h.ir_version, 8);
        assert_eq!(h.producer_name, "paddle2onnx");
        assert_eq!(h.producer_version, "1.0.6");
        assert_eq!(h.graph_name, "paddle-ocr-det");
        assert_eq!(h.input_count, 1);
        assert_eq!(h.output_count, 1);
        assert_eq!(h.opsets, vec![":17".to_owned()]);
    }

    #[test]
    fn a_graph_larger_than_the_prefix_is_still_read_and_flagged() {
        // What every real ONNX file looks like through a prefix read: the
        // graph's declared length runs past the bytes we hold.
        let mut graph = pb_len_field(2, b"PaddlePaddle Graph in PIR mode");
        graph.extend(pb_len_field(11, b"\x0a\x01x"));

        let mut v = pb_tag(1, WIRE_VARINT);
        v.extend(pb_varint(6));
        v.extend(pb_tag(7, WIRE_LEN));
        // Claim a graph far larger than what follows.
        v.extend(pb_varint(50_000_000));
        v.extend_from_slice(&graph);

        let got = inspect_prefix(&v, "inference.onnx");
        assert_eq!(got.format, ArtifactFormat::Onnx);
        let Some(Details::Onnx(h)) = got.details else { panic!("no onnx details") };
        assert_eq!(h.ir_version, 6);
        assert_eq!(h.graph_name, "PaddlePaddle Graph in PIR mode");
        assert_eq!(h.input_count, 1);
        // The flag is what tells a reader that empty `opsets` means
        // "beyond the prefix", not "absent from the file".
        assert!(h.graph_truncated);
        assert!(h.opsets.is_empty());
    }

    #[test]
    fn a_complete_graph_is_not_flagged_as_truncated() {
        let got = inspect_prefix(&build_onnx(), "det.onnx");
        let Some(Details::Onnx(h)) = got.details else { panic!() };
        assert!(!h.graph_truncated);
        assert_eq!(h.opsets, vec![":17".to_owned()]);
    }

    #[test]
    fn truncated_onnx_still_yields_what_was_parsed() {
        let full = build_onnx();
        let got = inspect_prefix(&full[..12], "det.onnx");
        assert_eq!(got.format, ArtifactFormat::Onnx);
        let Some(Details::Onnx(h)) = got.details else { panic!() };
        assert_eq!(h.ir_version, 8);
    }

    // -- fallbacks --------------------------------------------------------

    #[test]
    fn pytorch_zip_archive_is_detected() {
        let got = inspect_prefix(b"PK\x03\x04rest-of-archive", "pytorch_model.bin");
        assert_eq!(got.format, ArtifactFormat::PyTorch);
    }

    #[test]
    fn json_sidecars_are_detected_by_content_and_by_name() {
        assert_eq!(inspect_prefix(b"  {\"a\":1}", "").format, ArtifactFormat::Json);
        assert_eq!(inspect_prefix(b"", "tokenizer.json").format, ArtifactFormat::Json);
    }

    #[test]
    fn random_bytes_are_unknown_and_not_loadable() {
        let got = inspect_prefix(&[0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x11], "blob");
        assert_eq!(got.format, ArtifactFormat::Unknown);
        assert!(!got.format.is_loadable());
    }

    #[test]
    fn empty_input_never_panics() {
        assert_eq!(inspect_prefix(&[], "").format, ArtifactFormat::Unknown);
    }

    #[test]
    fn loadable_formats_are_the_backed_ones() {
        for f in [
            ArtifactFormat::Gguf,
            ArtifactFormat::Ggml,
            ArtifactFormat::Onnx,
            ArtifactFormat::Safetensors,
        ] {
            assert!(f.is_loadable(), "{f} should be loadable");
        }
        for f in [ArtifactFormat::PyTorch, ArtifactFormat::Json, ArtifactFormat::Unknown] {
            assert!(!f.is_loadable(), "{f} should not be loadable");
        }
    }
}
