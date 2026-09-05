// SPDX-License-Identifier: Apache-2.0
//! CRIWARE @UTF table reader and CPK archive header parser.
//!
//! # @UTF Format
//!
//! The @UTF (Universal Table Format) is a big-endian binary table structure
//! used throughout the CRIWARE middleware suite (CPK archives, USM video
//! streams, ACB/AWB audio banks).  The on-disk layout is:
//!
//! ```text
//! Offset  Size  Description
//! 0x00     4    Magic: 0x40 0x55 0x54 0x46 ("@UTF")
//! 0x04     4    Table byte-length (not including the 8-byte header above)
//! 0x08     4    Rows offset (relative to byte 8, i.e. start of table body)
//! 0x0C     4    String-pool offset (relative to byte 8)
//! 0x10     4    Data-pool offset (relative to byte 8)
//! 0x14     4    Table-name entry in the string pool (offset into string pool)
//! 0x18     2    Column count
//! 0x1A     2    Row stride (bytes per row)
//! 0x1C     4    Row count
//! 0x20     …    Schema: `column_count` column descriptors
//! …        …    Row data (starts at `rows_offset` relative to byte 8)
//! …        …    String pool (null-terminated UTF-8 strings)
//! …        …    Data pool (raw binary blobs)
//! ```
//!
//! References:
//! - <https://gitlab.com/Brandon-Wilson/criware/-/wikis/UTF>
//! - VGAudio source (UTF reader, MIT licensed)
//!
//! # CPK Format
//!
//! A CPK archive starts with the four-byte ASCII magic `CPK ` (0x43 0x50
//! 0x4B 0x20) at offset 0, followed by 12 bytes of unknown fields, with a
//! UTF table embedded at offset 0x10.  The table carries an `IsUtf=1` column
//! that signals a valid CPK envelope.
//!
//! # IEVR Encryption Note
//!
//! **All 921 CPK files shipped with Inazuma Eleven Victory Road are
//! envelope-encrypted** and do not present a plaintext `CPK ` magic at
//! offset 0 (see `var/docs/binaries-inventory.md` in the peer-winclean repo
//! for the binary survey).  [`read_cpk_header`] therefore works exclusively
//! on:
//!
//! 1. Unencrypted upstream CRIWARE samples (developer builds, sample packs).
//! 2. IEVR CPK files that have been decrypted in-memory by the appropriate IEVR decryption pass
//!    before being handed to this reader.
//!
//! Attempting to call [`read_cpk_header`] on a raw IEVR CPK will return an
//! `Err` with a descriptive message indicating the magic mismatch.

use std::io::Read;

use anyhow::{Context, anyhow, bail, ensure};
use byteorder::{BigEndian, ReadBytesExt};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A single cell value decoded from a @UTF table row.
#[derive(Debug, Clone, PartialEq)]
pub enum UtfValue {
    U8(u8),
    I8(i8),
    U16(u16),
    I16(i16),
    U32(u32),
    I32(i32),
    U64(u64),
    I64(i64),
    F32(f32),
    F64(f64),
    String(String),
    Bytes(Vec<u8>),
}

/// Descriptor for a single @UTF column (name + raw flags byte from schema).
#[derive(Debug, Clone, PartialEq)]
pub struct UtfColumn {
    /// Column name, resolved from the string pool.
    pub name: String,
    /// Raw flags byte as stored in the @UTF schema.
    ///
    /// The high nibble encodes storage class (0 = zero, 1 = constant, 2 = per-row).
    /// The low nibble encodes data type (see [`TypeId`] below).
    pub flags: u8,
}

/// A fully-decoded @UTF table.
#[derive(Debug, Clone)]
pub struct UtfTable {
    /// Table name, resolved from the string pool.
    pub name: String,
    /// Column descriptors in schema order.
    pub columns: Vec<UtfColumn>,
    /// Row data.  Each inner `Vec` has exactly `columns.len()` entries, in
    /// column order.
    pub rows: Vec<Vec<UtfValue>>,
}

/// A parsed CPK archive header (the UTF table embedded at offset 0x10).
#[derive(Debug, Clone)]
pub struct CpkHeader {
    /// The @UTF table that carries CPK catalogue metadata.
    pub utf: UtfTable,
}

// ---------------------------------------------------------------------------
// Internal constants
// ---------------------------------------------------------------------------

const UTF_MAGIC: [u8; 4] = [0x40, 0x55, 0x54, 0x46]; // "@UTF"
const CPK_MAGIC: [u8; 4] = [b'C', b'P', b'K', b' ']; // "CPK "

/// Storage-class nibble values (high nibble of flags byte).
const STORAGE_ZERO: u8 = 0x00;
const STORAGE_CONST: u8 = 0x10;
const STORAGE_ROW: u8 = 0x20;

/// Data-type nibble values (low nibble of flags byte).
mod type_id {
    pub(super) const U8: u8 = 0x00;
    pub(super) const I8: u8 = 0x01;
    pub(super) const U16: u8 = 0x02;
    pub(super) const I16: u8 = 0x03;
    pub(super) const U32: u8 = 0x04;
    pub(super) const I32: u8 = 0x05;
    pub(super) const U64: u8 = 0x06;
    pub(super) const I64: u8 = 0x07;
    pub(super) const F32: u8 = 0x08;
    pub(super) const F64: u8 = 0x09;
    pub(super) const STRING: u8 = 0x0A;
    pub(super) const BYTES: u8 = 0x0B;
}

// ---------------------------------------------------------------------------
// Public entry points
// ---------------------------------------------------------------------------

/// Parse a @UTF table from `reader`.
///
/// The reader must be positioned at the start of the @UTF magic (`@UTF`).
/// The entire table is read into memory before decoding so that the string
/// pool and data pool can be accessed by offset without repeated seeks.  CPK
/// tables in the wild are a few kilobytes to a few hundred kilobytes; this is
/// not a concern in practice.
///
/// # Errors
///
/// Returns an error if:
/// - The magic bytes are not `@UTF`.
/// - Any read from the underlying stream fails.
/// - String-pool offsets point outside the allocated buffer.
/// - An unrecognised type-id nibble is encountered.
pub fn read_utf(reader: &mut impl Read) -> anyhow::Result<UtfTable> {
    // Read magic (4 bytes) + table length (4 bytes) up front.
    let mut magic = [0u8; 4];
    reader.read_exact(&mut magic).context("read @UTF magic")?;
    ensure!(
        magic == UTF_MAGIC,
        "invalid @UTF magic: expected {:02X?}, got {:02X?}",
        UTF_MAGIC,
        magic
    );

    let table_len = reader.read_u32::<BigEndian>().context("read @UTF table length")? as usize;

    // Slurp the rest of the table body (table_len bytes) into a Vec so we can
    // random-access the string pool and data pool without seeking.
    let mut body = vec![0u8; table_len];
    reader.read_exact(&mut body).context("read @UTF table body")?;

    decode_utf_body(&body)
}

/// Parse a CPK archive header from `reader`.
///
/// The reader must be positioned at byte 0 of the CPK file (or the start of
/// an in-memory decrypted CPK buffer).  The function checks the `CPK ` magic,
/// skips 12 bytes of internal CPK fields, then delegates to [`read_utf`] for
/// the embedded UTF table that starts at offset 0x10.
///
/// # Errors
///
/// Returns an error if the `CPK ` magic is absent or if the embedded UTF
/// table is malformed.  IEVR's encrypted CPK files will always fail the magic
/// check — see the module-level documentation.
pub fn read_cpk_header(reader: &mut impl Read) -> anyhow::Result<CpkHeader> {
    let mut magic = [0u8; 4];
    reader.read_exact(&mut magic).context("read CPK magic")?;
    ensure!(
        magic == CPK_MAGIC,
        "invalid CPK magic: expected {:02X?} (\"CPK \"), got {:02X?}; note — IEVR CPK files are \
         envelope-encrypted and must be decrypted before parsing",
        CPK_MAGIC,
        magic
    );

    // Skip 12 bytes of CPK internal header (revision, unknown fields).
    let mut _skip = [0u8; 12];
    reader.read_exact(&mut _skip).context("skip CPK internal header")?;

    // The UTF table starts at offset 0x10.
    let utf = read_utf(reader).context("read UTF table embedded in CPK header")?;
    Ok(CpkHeader { utf })
}

// ---------------------------------------------------------------------------
// Internal decoding
// ---------------------------------------------------------------------------

/// Decode the @UTF table body (the bytes after the 8-byte outer header).
///
/// `body` starts at what the spec calls "byte 8" — the first byte after the
/// 4-byte magic and 4-byte table-length.  All internal offsets in the header
/// fields are relative to this origin.
fn decode_utf_body(body: &[u8]) -> anyhow::Result<UtfTable> {
    // The body header is 0x18 bytes long before the schema entries begin.
    ensure!(body.len() >= 0x18, "@UTF body too short for header: {} bytes", body.len());

    let rows_offset = u32::from_be_bytes(body[0x00..0x04].try_into().unwrap()) as usize;
    let strings_offset = u32::from_be_bytes(body[0x04..0x08].try_into().unwrap()) as usize;
    let data_offset = u32::from_be_bytes(body[0x08..0x0C].try_into().unwrap()) as usize;
    let table_name_offset = u32::from_be_bytes(body[0x0C..0x10].try_into().unwrap()) as usize;
    let col_count = u16::from_be_bytes(body[0x10..0x12].try_into().unwrap()) as usize;
    let row_stride = u16::from_be_bytes(body[0x12..0x14].try_into().unwrap()) as usize;
    let row_count = u32::from_be_bytes(body[0x14..0x18].try_into().unwrap()) as usize;

    let table_name = read_cstring(body, strings_offset, table_name_offset)
        .context("read table name from string pool")?;

    // --- Parse schema (column descriptors) ---
    // Each descriptor is 5 bytes: 1 flags byte + 4 bytes name-pool offset.
    // They start at body[0x18].
    let schema_start = 0x18_usize;
    let schema_end = schema_start + col_count * 5;
    ensure!(
        body.len() >= schema_end,
        "@UTF body too short for schema: need {schema_end}, have {}",
        body.len()
    );

    let mut columns = Vec::with_capacity(col_count);
    // Alongside the schema we collect constant values so we can fill them
    // into every row without re-reading.
    let mut const_values: Vec<Option<UtfValue>> = Vec::with_capacity(col_count);

    // The constant-value data immediately follows the schema in the body at
    // `schema_end`, packed in column order.  We walk it with a cursor.
    let mut const_cursor = schema_end;

    for i in 0..col_count {
        let base = schema_start + i * 5;
        let flags = body[base];
        let name_pool_offset =
            u32::from_be_bytes(body[base + 1..base + 5].try_into().unwrap()) as usize;

        let name = read_cstring(body, strings_offset, name_pool_offset)
            .with_context(|| format!("read column {i} name from string pool"))?;

        columns.push(UtfColumn { name, flags });

        let storage = flags & 0xF0;
        let type_nibble = flags & 0x0F;

        let const_val = if storage == STORAGE_CONST {
            let (val, consumed) =
                read_typed_value(body, const_cursor, type_nibble, strings_offset, data_offset)
                    .with_context(|| format!("read constant value for column {i}"))?;
            const_cursor += consumed;
            Some(val)
        } else {
            None
        };
        const_values.push(const_val);
    }

    // --- Parse rows ---
    let mut rows = Vec::with_capacity(row_count);
    for r in 0..row_count {
        let row_base = rows_offset + r * row_stride;
        ensure!(
            body.len() >= row_base + row_stride || row_stride == 0,
            "row {r} out of bounds: body len={}, row_base={row_base}, stride={row_stride}",
            body.len()
        );

        let mut row = Vec::with_capacity(col_count);
        let mut row_cursor = row_base;

        for (c, col) in columns.iter().enumerate() {
            let storage = col.flags & 0xF0;
            let type_nibble = col.flags & 0x0F;

            let val = match storage {
                STORAGE_ZERO => zero_value(type_nibble)
                    .with_context(|| format!("build zero value for column {c}"))?,
                STORAGE_CONST => const_values[c]
                    .clone()
                    .ok_or_else(|| anyhow!("missing const value for column {c}"))?,
                STORAGE_ROW => {
                    let (val, consumed) = read_typed_value(
                        body,
                        row_cursor,
                        type_nibble,
                        strings_offset,
                        data_offset,
                    )
                    .with_context(|| format!("read row {r} column {c} value"))?;
                    row_cursor += consumed;
                    val
                },
                other => bail!("unknown storage class 0x{other:02X} at column {c}"),
            };
            row.push(val);
        }
        rows.push(row);
    }

    Ok(UtfTable { name: table_name, columns, rows })
}

/// Read a null-terminated UTF-8 string from the string pool.
///
/// `strings_offset` is the byte offset within `body` where the string pool
/// starts.  `name_offset` is the offset of the desired string within the pool
/// (i.e. the final byte position is `strings_offset + name_offset`).
fn read_cstring(body: &[u8], strings_offset: usize, name_offset: usize) -> anyhow::Result<String> {
    let start = strings_offset
        .checked_add(name_offset)
        .ok_or_else(|| anyhow!("string offset arithmetic overflow"))?;
    ensure!(
        start < body.len(),
        "string pool offset 0x{start:X} out of bounds (body len=0x{:X})",
        body.len()
    );
    let slice = &body[start..];
    let nul = slice
        .iter()
        .position(|&b| b == 0)
        .ok_or_else(|| anyhow!("string at offset 0x{start:X} has no null terminator"))?;
    let s = std::str::from_utf8(&slice[..nul])
        .with_context(|| format!("string at 0x{start:X} is not valid UTF-8"))?
        .to_owned();
    Ok(s)
}

/// Return the zero/default value for a given type nibble.
fn zero_value(type_nibble: u8) -> anyhow::Result<UtfValue> {
    Ok(match type_nibble {
        type_id::U8 => UtfValue::U8(0),
        type_id::I8 => UtfValue::I8(0),
        type_id::U16 => UtfValue::U16(0),
        type_id::I16 => UtfValue::I16(0),
        type_id::U32 => UtfValue::U32(0),
        type_id::I32 => UtfValue::I32(0),
        type_id::U64 => UtfValue::U64(0),
        type_id::I64 => UtfValue::I64(0),
        type_id::F32 => UtfValue::F32(0.0),
        type_id::F64 => UtfValue::F64(0.0),
        type_id::STRING => UtfValue::String(String::new()),
        type_id::BYTES => UtfValue::Bytes(Vec::new()),
        other => bail!("unknown type nibble 0x{other:X}"),
    })
}

/// Decode one typed value from `body` at `offset`, returning `(value, bytes_consumed)`.
///
/// For `STRING` the value is resolved from the string pool at `strings_offset`.
/// For `BYTES` the blob header (offset + length) is read inline and the blob
/// itself is copied from `data_offset + blob_offset`.
fn read_typed_value(
    body: &[u8],
    offset: usize,
    type_nibble: u8,
    strings_offset: usize,
    data_offset: usize,
) -> anyhow::Result<(UtfValue, usize)> {
    // Helper: read a big-endian slice of N bytes from body at a given position.
    macro_rules! be_slice {
        ($pos:expr, $n:expr) => {{
            let end = $pos + $n;
            ensure!(
                body.len() >= end,
                "read past end of body: need byte {end}, have {}",
                body.len()
            );
            &body[$pos..end]
        }};
    }

    let result = match type_nibble {
        type_id::U8 => {
            let v = be_slice!(offset, 1)[0];
            (UtfValue::U8(v), 1)
        },
        type_id::I8 => {
            let v = be_slice!(offset, 1)[0] as i8;
            (UtfValue::I8(v), 1)
        },
        type_id::U16 => {
            let v = u16::from_be_bytes(be_slice!(offset, 2).try_into().unwrap());
            (UtfValue::U16(v), 2)
        },
        type_id::I16 => {
            let v = i16::from_be_bytes(be_slice!(offset, 2).try_into().unwrap());
            (UtfValue::I16(v), 2)
        },
        type_id::U32 => {
            let v = u32::from_be_bytes(be_slice!(offset, 4).try_into().unwrap());
            (UtfValue::U32(v), 4)
        },
        type_id::I32 => {
            let v = i32::from_be_bytes(be_slice!(offset, 4).try_into().unwrap());
            (UtfValue::I32(v), 4)
        },
        type_id::U64 => {
            let v = u64::from_be_bytes(be_slice!(offset, 8).try_into().unwrap());
            (UtfValue::U64(v), 8)
        },
        type_id::I64 => {
            let v = i64::from_be_bytes(be_slice!(offset, 8).try_into().unwrap());
            (UtfValue::I64(v), 8)
        },
        type_id::F32 => {
            let bits = u32::from_be_bytes(be_slice!(offset, 4).try_into().unwrap());
            (UtfValue::F32(f32::from_bits(bits)), 4)
        },
        type_id::F64 => {
            let bits = u64::from_be_bytes(be_slice!(offset, 8).try_into().unwrap());
            (UtfValue::F64(f64::from_bits(bits)), 8)
        },
        type_id::STRING => {
            // 4-byte offset into the string pool.
            let pool_off = u32::from_be_bytes(be_slice!(offset, 4).try_into().unwrap()) as usize;
            let s = read_cstring(body, strings_offset, pool_off)
                .context("resolve STRING value from pool")?;
            (UtfValue::String(s), 4)
        },
        type_id::BYTES => {
            // 4-byte data-pool offset + 4-byte byte length.
            let blob_off = u32::from_be_bytes(be_slice!(offset, 4).try_into().unwrap()) as usize;
            let blob_len =
                u32::from_be_bytes(be_slice!(offset + 4, 4).try_into().unwrap()) as usize;
            let blob_start = data_offset
                .checked_add(blob_off)
                .ok_or_else(|| anyhow!("data pool offset overflow"))?;
            let blob_end = blob_start
                .checked_add(blob_len)
                .ok_or_else(|| anyhow!("data blob end overflow"))?;
            ensure!(
                body.len() >= blob_end,
                "data blob [{blob_start}..{blob_end}) out of body bounds (len={})",
                body.len()
            );
            let bytes = body[blob_start..blob_end].to_vec();
            (UtfValue::Bytes(bytes), 8)
        },
        other => bail!("unknown type nibble 0x{other:X}"),
    };
    Ok(result)
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    // -----------------------------------------------------------------------
    // Fixture builder — construct a minimal, valid @UTF table in memory.
    //
    // The table we build has:
    //   - name: "TestTable"
    //   - 2 columns: 0: "ColA"  flags=0x24 (STORAGE_ROW | type U32 = 0x20|0x04) 1: "ColB"
    //     flags=0x2A (STORAGE_ROW | type STRING = 0x20|0x0A)
    //   - 2 rows: row 0: (ColA=42u32, ColB="hello") row 1: (ColA=99u32, ColB="world")
    // -----------------------------------------------------------------------

    /// Build the raw bytes for the @UTF fixture and return them (including the
    /// 8-byte outer header: magic + table_length).
    fn build_utf_fixture() -> Vec<u8> {
        // String pool contents (null-terminated, tightly packed):
        //   offset 0 : "TestTable\0"  (10 bytes)
        //   offset 10: "ColA\0"       (5 bytes)
        //   offset 15: "ColB\0"       (5 bytes)
        //   offset 20: "hello\0"      (6 bytes)
        //   offset 26: "world\0"      (6 bytes)
        // total string pool = 32 bytes
        let string_pool: Vec<u8> = {
            let mut v = Vec::new();
            v.extend_from_slice(b"TestTable\0"); // offset 0  → name
            v.extend_from_slice(b"ColA\0"); // offset 10 → col 0
            v.extend_from_slice(b"ColB\0"); // offset 15 → col 1
            v.extend_from_slice(b"hello\0"); // offset 20 → row 0 ColB
            v.extend_from_slice(b"world\0"); // offset 26 → row 1 ColB
            v
        };
        let strings_len = string_pool.len(); // 32

        // Data pool: empty (no BYTES columns).
        let data_pool: Vec<u8> = Vec::new();

        // Schema: 2 columns × 5 bytes each = 10 bytes.
        // Column 0: flags=0x24, name_offset=10 (0x0000_000A)
        // Column 1: flags=0x2A, name_offset=15 (0x0000_000F)
        let schema: Vec<u8> = {
            let mut v = Vec::new();
            // col 0
            v.push(0x24u8); // STORAGE_ROW(0x20) | U32(0x04)
            v.extend_from_slice(&10u32.to_be_bytes());
            // col 1
            v.push(0x2Au8); // STORAGE_ROW(0x20) | STRING(0x0A)
            v.extend_from_slice(&15u32.to_be_bytes());
            v
        };

        // No constant-value block (no STORAGE_CONST columns).

        // Row data: 2 rows × row_stride bytes.
        // ColA = U32 (4 bytes) + ColB = STRING ptr (4 bytes) → stride = 8.
        let row_stride: u16 = 8;
        let row_data: Vec<u8> = {
            let mut v = Vec::new();
            // row 0: ColA=42, ColB → string pool offset 20 ("hello")
            v.extend_from_slice(&42u32.to_be_bytes()); // ColA
            v.extend_from_slice(&20u32.to_be_bytes()); // ColB → pool[20] = "hello"
            // row 1: ColA=99, ColB → string pool offset 26 ("world")
            v.extend_from_slice(&99u32.to_be_bytes()); // ColA
            v.extend_from_slice(&26u32.to_be_bytes()); // ColB → pool[26] = "world"
            v
        };

        // Now we can compute the body layout.  Offsets are relative to the
        // start of the body (i.e. byte 8 of the full stream).
        //
        // Body layout:
        //   [0x00..0x18)  — body header (24 bytes, fixed)
        //   [0x18..0x22)  — schema (10 bytes, 2 cols × 5)
        //   no const block
        //   [rows_offset] — row data
        //   [strings_offset] — string pool
        //   [data_offset]  — data pool (empty)
        //
        // rows_offset = 0x18 + schema.len() = 0x18 + 10 = 0x22
        // strings_offset = rows_offset + row_data.len() = 0x22 + 16 = 0x32
        // data_offset = strings_offset + string_pool.len() = 0x32 + 32 = 0x52

        let rows_offset: u32 = 0x18 + schema.len() as u32; // 0x22
        let strings_offset: u32 = rows_offset + row_data.len() as u32; // 0x32
        let data_offset: u32 = strings_offset + strings_len as u32; // 0x52

        // table_name_offset = 0 (first string in the pool = "TestTable")
        let table_name_offset: u32 = 0;
        let col_count: u16 = 2;
        let row_count: u32 = 2;

        // Body header (24 bytes).
        let mut body = Vec::new();
        body.extend_from_slice(&rows_offset.to_be_bytes()); // 0x00
        body.extend_from_slice(&strings_offset.to_be_bytes()); // 0x04
        body.extend_from_slice(&data_offset.to_be_bytes()); // 0x08
        body.extend_from_slice(&table_name_offset.to_be_bytes()); // 0x0C
        body.extend_from_slice(&col_count.to_be_bytes()); // 0x10
        body.extend_from_slice(&row_stride.to_be_bytes()); // 0x12
        body.extend_from_slice(&row_count.to_be_bytes()); // 0x14

        // Schema.
        body.extend_from_slice(&schema);
        // Row data.
        body.extend_from_slice(&row_data);
        // String pool.
        body.extend_from_slice(&string_pool);
        // Data pool (empty).
        body.extend_from_slice(&data_pool);

        // Outer header: magic (4 bytes) + table_length (4 bytes).
        let table_len = body.len() as u32;
        let mut out = Vec::new();
        out.extend_from_slice(&UTF_MAGIC);
        out.extend_from_slice(&table_len.to_be_bytes());
        out.extend_from_slice(&body);
        out
    }

    // -----------------------------------------------------------------------
    // Test: parse the synthetic UTF fixture
    // -----------------------------------------------------------------------

    #[test]
    fn utf_fixture_parses_correctly() {
        let fixture = build_utf_fixture();
        let mut cursor = Cursor::new(&fixture);
        let table = read_utf(&mut cursor).expect("read_utf should succeed on valid fixture");

        // Table name.
        assert_eq!(table.name, "TestTable");

        // Column count and names.
        assert_eq!(table.columns.len(), 2);
        assert_eq!(table.columns[0].name, "ColA");
        assert_eq!(table.columns[0].flags, 0x24);
        assert_eq!(table.columns[1].name, "ColB");
        assert_eq!(table.columns[1].flags, 0x2A);

        // Row count.
        assert_eq!(table.rows.len(), 2);

        // Row 0 values.
        assert_eq!(table.rows[0][0], UtfValue::U32(42));
        assert_eq!(table.rows[0][1], UtfValue::String("hello".to_owned()));

        // Row 1 values.
        assert_eq!(table.rows[1][0], UtfValue::U32(99));
        assert_eq!(table.rows[1][1], UtfValue::String("world".to_owned()));
    }

    // -----------------------------------------------------------------------
    // Test: bad magic is rejected with an informative error
    // -----------------------------------------------------------------------

    #[test]
    fn utf_bad_magic_rejected() {
        let mut bad = build_utf_fixture();
        bad[0] = b'X'; // corrupt the first magic byte
        let mut cursor = Cursor::new(&bad);
        let err = read_utf(&mut cursor).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("invalid @UTF magic"), "expected magic error, got: {msg}");
    }

    // -----------------------------------------------------------------------
    // Test: CPK magic accepted / rejected
    // -----------------------------------------------------------------------

    /// Build a minimal valid CPK byte stream whose embedded UTF table is the
    /// synthetic fixture above.
    fn build_cpk_fixture() -> Vec<u8> {
        let utf_bytes = build_utf_fixture();

        // CPK stream layout:
        //   [0x00..0x04) — "CPK " magic
        //   [0x04..0x10) — 12 bytes of internal CPK header (all zeroes here)
        //   [0x10..)     — @UTF table bytes
        let mut out = Vec::new();
        out.extend_from_slice(&CPK_MAGIC);
        out.extend_from_slice(&[0u8; 12]); // internal header placeholder
        out.extend_from_slice(&utf_bytes);
        out
    }

    #[test]
    fn cpk_magic_check_accepted() {
        let fixture = build_cpk_fixture();
        let mut cursor = Cursor::new(&fixture);
        let header = read_cpk_header(&mut cursor).expect("valid CPK fixture should parse");
        // The embedded UTF table should have the same structure as the fixture.
        assert_eq!(header.utf.name, "TestTable");
        assert_eq!(header.utf.rows.len(), 2);
    }

    #[test]
    fn cpk_magic_check_rejected() {
        let mut bad = build_cpk_fixture();
        bad[0] = b'X'; // break the CPK magic
        let mut cursor = Cursor::new(&bad);
        let err = read_cpk_header(&mut cursor).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("invalid CPK magic"), "expected CPK magic error, got: {msg}");
    }

    // -----------------------------------------------------------------------
    // Test: truncated body is rejected gracefully (no panic)
    // -----------------------------------------------------------------------

    #[test]
    fn utf_truncated_body_rejected() {
        let full = build_utf_fixture();
        // Keep only the first 20 bytes — enough magic+len, body truncated.
        let truncated = &full[..20];
        let mut cursor = Cursor::new(truncated);
        assert!(read_utf(&mut cursor).is_err(), "truncated stream must not parse successfully");
    }
}
