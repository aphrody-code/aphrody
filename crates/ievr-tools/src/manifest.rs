// SPDX-License-Identifier: Apache-2.0
//! `cpk_list.cfg.bin` manifest probe — static analysis of the 12 MiB IEVR
//! master asset catalog without any format assumptions.
//!
//! The probe reads the whole file into a `Vec<u8>` (acceptable for ≤ 64 MiB
//! files — no mmap needed), then computes:
//!
//! * Shannon entropy over the full byte histogram.
//! * Hex dumps of the first and last 256 bytes.
//! * All hits of known four-byte magic signatures (CRI / CPK / PNG / RIFF …).
//! * Aligned offsets where a 4-byte big-endian word falls in `[1 MiB, 64 MiB]`, which is a
//!   heuristic for embedded file-size fields.

use std::{
    io::{self, BufWriter, Cursor, Write},
    path::Path,
};

use anyhow::Context as _;
use byteorder::{BigEndian, ReadBytesExt as _};

// ── Known four-byte magic signatures ──────────────────────────────────────

/// A magic-byte pattern together with a human-readable label.
struct KnownMagic {
    bytes: [u8; 4],
    label: &'static str,
}

/// All magic signatures we scan for.
///
/// The list covers CRIWARE container formats (CPK, USM, AWB, ACB, HCA) plus
/// common media-container magics likely to appear in a Level-5 asset catalog.
const KNOWN_MAGICS: &[KnownMagic] = &[
    KnownMagic { bytes: *b"@UTF", label: "@UTF (CRIWARE table)" },
    KnownMagic { bytes: *b"CPK ", label: "CPK  (CRIWARE CPK archive)" },
    KnownMagic { bytes: *b"CRID", label: "CRID (CRIWARE USM video)" },
    KnownMagic { bytes: *b"CRIF", label: "CRIF (CRIWARE generic)" },
    KnownMagic { bytes: *b"AFS2", label: "AFS2 (CRIWARE AWB audio bank)" },
    KnownMagic { bytes: *b"@ACB", label: "@ACB (CRIWARE audio cue bank)" },
    KnownMagic { bytes: *b"HCA\0", label: "HCA\\0 (CRIWARE HCA audio)" },
    KnownMagic { bytes: [0x89, b'P', b'N', b'G'], label: "\\x89PNG (PNG image)" },
    KnownMagic { bytes: *b"RIFF", label: "RIFF (WAV/AVI container)" },
    KnownMagic { bytes: *b"OggS", label: "OggS (Ogg bitstream)" },
    KnownMagic { bytes: *b"fLaC", label: "fLaC (FLAC audio)" },
    KnownMagic { bytes: [0xFF, 0xFE, 0x00, 0x00], label: "UTF-32 LE BOM" },
    KnownMagic { bytes: [0xFF, 0xFE, 0x00, 0x00], label: "UTF-32 LE BOM" },
    KnownMagic { bytes: [0xEF, 0xBB, 0xBF, 0x00], label: "UTF-8 BOM + null" },
    KnownMagic { bytes: [0x1F, 0x8B, 0x08, 0x00], label: "gzip stream (CM=8)" },
    KnownMagic { bytes: *b"PK\x03\x04", label: "PK\\x03\\x04 (ZIP local file)" },
    KnownMagic { bytes: [0x00, 0x00, 0x00, 0x00], label: "null-quad (padding / unset)" },
];

// ── Public types ───────────────────────────────────────────────────────────

/// A single magic-signature hit found during the scan.
#[derive(Debug, Clone)]
pub struct MagicHit {
    /// Byte offset of the first byte of the magic in the file.
    pub offset: u64,
    /// The four raw bytes at this offset.
    pub magic: [u8; 4],
    /// Printable ASCII representation of `magic` (non-printable bytes → `'.'`).
    pub ascii: String,
    /// Human-readable label for the matching known magic, if any.
    pub label: String,
}

/// Results of a static probe of a binary manifest file.
///
/// All fields are cheap to clone; the largest is `head_hex` / `tail_hex`
/// (512-character strings each) and the vectors (capped at 4 096 / 16 entries).
#[derive(Debug, Clone)]
pub struct ManifestProbe {
    /// Total file size in bytes.
    pub size_bytes: u64,
    /// Hex encoding of the first 256 bytes (512 hex chars, zero-padded if file
    /// is shorter than 256 bytes).
    pub head_hex: String,
    /// Hex encoding of the last 256 bytes.
    pub tail_hex: String,
    /// Shannon entropy H = −Σ p·log₂(p) in bits-per-byte over the full file.
    /// Range: [0.0, 8.0].  Values > 7.5 are consistent with encryption or
    /// strong compression; values < 2.0 suggest sparse / mostly-zero data.
    pub entropy_bits_per_byte: f64,
    /// All magic-signature hits found in the file, capped at 4 096.
    pub magic_candidates: Vec<MagicHit>,
    /// First 16 offsets (ascending) where the 4-byte big-endian word at that
    /// offset is in the range `[1 MiB, 64 MiB]` and the offset itself is
    /// 8-byte aligned — a heuristic for embedded file-size or offset fields.
    pub aligned_offsets_8: Vec<u64>,
}

// ── Core probe function ────────────────────────────────────────────────────

/// Read and analyse the binary file at `path`.
///
/// The entire file is loaded into a `Vec<u8>` once; subsequent analysis
/// passes operate on that in-memory buffer.
///
/// # Errors
///
/// * File I/O errors (not found, permission denied, …).
/// * Files larger than 256 MiB are rejected to prevent accidental exhaustion of process address
///   space.
pub fn probe(path: &Path) -> anyhow::Result<ManifestProbe> {
    const MAX_SIZE: u64 = 256 * 1024 * 1024; // 256 MiB hard cap
    const MAGIC_HIT_CAP: usize = 4_096;
    const ALIGNED_CAP: usize = 16;
    // Plausible file-size range for embedded 4-byte BE size fields (1 MiB..=64 MiB).
    const PLAUSIBLE_SIZE_MIN: u32 = 1 * 1024 * 1024; // 1 MiB
    const PLAUSIBLE_SIZE_MAX: u32 = 64 * 1024 * 1024; // 64 MiB

    let metadata = std::fs::metadata(path).with_context(|| format!("stat `{}`", path.display()))?;
    let size_bytes = metadata.len();
    anyhow::ensure!(
        size_bytes <= MAX_SIZE,
        "file `{}` is {size_bytes} bytes — exceeds 256 MiB safety cap",
        path.display()
    );

    let data: Vec<u8> =
        std::fs::read(path).with_context(|| format!("read `{}`", path.display()))?;

    // ── Shannon entropy ────────────────────────────────────────────────────
    let entropy_bits_per_byte = shannon_entropy(&data);

    // ── Head / tail hex ───────────────────────────────────────────────────
    let head_len = data.len().min(256);
    let head_hex = hex::encode(&data[..head_len]);

    let tail_start = data.len().saturating_sub(256);
    let tail_hex = hex::encode(&data[tail_start..]);

    // ── Magic scan ────────────────────────────────────────────────────────
    let mut magic_candidates: Vec<MagicHit> = Vec::new();
    if data.len() >= 4 {
        'outer: for offset in 0..=(data.len() - 4) {
            let window: [u8; 4] =
                [data[offset], data[offset + 1], data[offset + 2], data[offset + 3]];
            for km in KNOWN_MAGICS {
                if km.bytes == window {
                    magic_candidates.push(MagicHit {
                        offset: offset as u64,
                        magic: window,
                        ascii: bytes_to_printable_ascii(&window),
                        label: km.label.to_owned(),
                    });
                    if magic_candidates.len() >= MAGIC_HIT_CAP {
                        break 'outer;
                    }
                    // Only record the first matching label per offset to avoid
                    // duplicate entries from the null-quad having two entries.
                    break;
                }
            }
        }
    }

    // ── Aligned 8-byte offsets with plausible BE file-size values ─────────
    let mut aligned_offsets_8: Vec<u64> = Vec::new();
    if data.len() >= 4 {
        let aligned_end = data.len() - 3;
        let mut offset = 0usize;
        while offset < aligned_end && aligned_offsets_8.len() < ALIGNED_CAP {
            // Safety: we checked offset + 3 < data.len() via `aligned_end`.
            let mut cursor = Cursor::new(&data[offset..offset + 4]);
            // BigEndian::ReadBytesExt::read_u32 cannot fail on an in-memory
            // 4-byte slice — the only error would be EOF which is impossible here.
            let word = cursor.read_u32::<BigEndian>().unwrap_or(0);
            if word >= PLAUSIBLE_SIZE_MIN && word <= PLAUSIBLE_SIZE_MAX {
                aligned_offsets_8.push(offset as u64);
            }
            offset += 8;
        }
    }

    Ok(ManifestProbe {
        size_bytes,
        head_hex,
        tail_hex,
        entropy_bits_per_byte,
        magic_candidates,
        aligned_offsets_8,
    })
}

// ── ManifestProbe display ─────────────────────────────────────────────────

impl ManifestProbe {
    /// Write a multi-line human-readable report to `out`.
    ///
    /// # Errors
    ///
    /// Propagates any `io::Error` returned by `out.write_all`.
    pub fn print_report(&self, out: &mut impl Write) -> io::Result<()> {
        let mut w = BufWriter::new(out);

        writeln!(w, "=== cpk_list.cfg.bin manifest probe ===")?;
        writeln!(
            w,
            "  size           : {} bytes ({:.3} MiB)",
            self.size_bytes,
            self.size_bytes as f64 / 1_048_576.0
        )?;
        writeln!(w, "  entropy        : {:.6} bits/byte", self.entropy_bits_per_byte)?;

        let entropy_label = if self.entropy_bits_per_byte > 7.5 {
            "HIGH — likely compressed or encrypted"
        } else if self.entropy_bits_per_byte > 5.0 {
            "MEDIUM — mixed binary data"
        } else if self.entropy_bits_per_byte > 2.0 {
            "LOW-MEDIUM — structured binary or mostly-ASCII"
        } else {
            "LOW — sparse / heavily zero-padded"
        };
        writeln!(w, "  entropy label  : {entropy_label}")?;

        writeln!(w, "\n--- head (first 256 bytes, hex) ---")?;
        write_hex_dump(&mut w, &self.head_hex, 32)?;

        writeln!(w, "\n--- tail (last 256 bytes, hex) ---")?;
        write_hex_dump(&mut w, &self.tail_hex, 32)?;

        writeln!(w, "\n--- magic hits ({} total, cap 4096) ---", self.magic_candidates.len())?;
        if self.magic_candidates.is_empty() {
            writeln!(w, "  (none)")?;
        } else {
            for hit in &self.magic_candidates {
                writeln!(
                    w,
                    "  +0x{:08X}  {:>8}  ascii={:?}  {}",
                    hit.offset,
                    hex::encode(hit.magic),
                    hit.ascii,
                    hit.label
                )?;
            }
        }

        writeln!(w, "\n--- aligned_offsets_8 (4-byte BE in [1 MiB, 64 MiB]) ---")?;
        if self.aligned_offsets_8.is_empty() {
            writeln!(w, "  (none)")?;
        } else {
            for &off in &self.aligned_offsets_8 {
                writeln!(w, "  +0x{off:08X}")?;
            }
        }

        writeln!(w, "\n=== end of report ===")?;
        w.flush()
    }
}

// ── Internal helpers ──────────────────────────────────────────────────────

/// Compute Shannon entropy H = −Σ p_i log₂(p_i) over `data`.
///
/// Returns 0.0 for empty or zero-length slices rather than NaN.
fn shannon_entropy(data: &[u8]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    let mut counts = [0u64; 256];
    for &b in data {
        counts[b as usize] += 1;
    }
    let len = data.len() as f64;
    let mut h = 0.0f64;
    for &c in &counts {
        if c > 0 {
            let p = c as f64 / len;
            h -= p * p.log2();
        }
    }
    h
}

/// Render four bytes as printable ASCII; replace non-printable bytes with `'.'`.
fn bytes_to_printable_ascii(b: &[u8; 4]) -> String {
    b.iter().map(|&c| if c.is_ascii_graphic() { c as char } else { '.' }).collect()
}

/// Write `hex_str` as grouped rows of `cols` hex pairs (cols*2 hex chars per line),
/// indented by two spaces.
fn write_hex_dump(w: &mut impl Write, hex_str: &str, cols: usize) -> io::Result<()> {
    // Each byte is two hex chars.
    let chars_per_row = cols * 2;
    let mut pos = 0usize;
    let bytes_total = hex_str.len() / 2;
    while pos < hex_str.len() {
        let end = (pos + chars_per_row).min(hex_str.len());
        let byte_offset = pos / 2;
        write!(w, "  +{byte_offset:04X}  ")?;
        let row = &hex_str[pos..end];
        // Insert spaces between every pair for readability.
        for (i, ch) in row.chars().enumerate() {
            if i > 0 && i % 2 == 0 {
                write!(w, " ")?;
            }
            write!(w, "{ch}")?;
        }
        // Pad the last row with spaces so the ASCII column aligns.
        let written_pairs = (end - pos) / 2;
        if written_pairs < cols {
            let missing = cols - written_pairs;
            for _ in 0..missing {
                write!(w, "   ")?; // 2 hex + 1 space
            }
        }
        // ASCII side-bar.
        let hex_start = pos;
        let hex_end = end;
        write!(w, "  |")?;
        let mut hi = hex_start;
        while hi < hex_end {
            let pair = &hex_str[hi..hi + 2];
            let val = u8::from_str_radix(pair, 16).unwrap_or(b'.');
            let ch = if val.is_ascii_graphic() { val as char } else { '.' };
            write!(w, "{ch}")?;
            hi += 2;
        }
        writeln!(w, "|")?;
        pos = end;
        let _ = bytes_total; // suppress unused warning
    }
    Ok(())
}

// ── Unit tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: run entropy over a raw byte slice without writing a temp file.
    fn entropy_of(data: &[u8]) -> f64 {
        shannon_entropy(data)
    }

    #[test]
    fn entropy_of_zeros_is_zero() {
        let buf = vec![0u8; 1024];
        let h = entropy_of(&buf);
        assert_eq!(h, 0.0, "all-zero buffer must have entropy 0.0, got {h}");
    }

    #[test]
    fn entropy_of_random_is_near_8() {
        // Deterministic pseudo-random sequence using Knuth's multiplicative hash
        // constant.  The distribution approximates uniform over [0, 255].
        let buf: Vec<u8> =
            (0u32..1024).map(|i| (i.wrapping_mul(2_654_435_761) % 256) as u8).collect();
        let h = entropy_of(&buf);
        assert!(h > 7.0, "pseudo-random buffer should have entropy > 7.0, got {h:.6}");
    }

    #[test]
    fn magic_scan_finds_utf() {
        // 1024-byte buffer with @UTF injected at offset 512.
        let mut buf = vec![0u8; 1024];
        buf[512] = b'@';
        buf[513] = b'U';
        buf[514] = b'T';
        buf[515] = b'F';

        // Write to a temporary file so we can call the public `probe` API.
        let tmp_dir = std::env::temp_dir();
        let tmp_path = tmp_dir.join("ievr_manifest_test_utf.bin");
        std::fs::write(&tmp_path, &buf).expect("write temp file");

        let result = probe(&tmp_path).expect("probe should succeed on temp file");
        std::fs::remove_file(&tmp_path).ok();

        let utf_hits: Vec<_> = result.magic_candidates.iter().filter(|h| h.offset == 512).collect();

        assert!(
            !utf_hits.is_empty(),
            "expected at least one magic hit at offset 512, got: {:?}",
            result.magic_candidates
        );
        assert_eq!(utf_hits[0].magic, [b'@', b'U', b'T', b'F'], "hit magic bytes must be @UTF");
    }
}
