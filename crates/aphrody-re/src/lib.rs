// SPDX-License-Identifier: Apache-2.0
//! # aphrody-re — reverse engineering primitives
//!
//! Pure-Rust binary triage and disassembly built on top of [`goblin`] and
//! [`iced_x86`]. Exposes two observable features:
//!
//! - [`triage`] parses a byte slice and returns a fully populated
//!   [`TriageReport`] describing the detected format, entry point, sections
//!   with per-section Shannon entropy, imports, exports, an ASCII/UTF-16
//!   strings sample, and a SHA-256 of the whole input.
//!
//! - [`disasm`] decodes up to [`DISASM_LIMIT`] x86-64 instructions from a
//!   raw byte slice at a given `rip` base address using `iced-x86`'s
//!   `IntelFormatter`. Returns a `Vec<DisasmInsn>` ready for JSON
//!   serialization.
//!
//! Scope (Sprint R-E, 2026-05-19 / R5.4, 2026-05-21):
//!
//! - ✅ PE32 / PE64 (Windows)
//! - ✅ ELF32 / ELF64 (Linux + most Unixes)
//! - ✅ x86-64 disassembly via `iced-x86 1.21` (pure Rust, no C, no GPL)
//! - ❓ Mach-O — `goblin` exposes it under feature `mach{32,64}` which is
//!   off in the workspace pin; reports `Format::Unknown` for now.
//! - ❓ WebAssembly — not yet wired (`goblin` does not parse `.wasm`).
//!
//! YARA scanning is Phase 3 (`yara-x`, BSD-3).
//!
//! ## Quickstart
//!
//! ```
//! use aphrody_re::{triage, Format};
//!
//! // Empty bytes => Unknown format, no panic.
//! let report = triage(b"not a binary").expect("triage never panics on garbage");
//! assert_eq!(report.format, Format::Unknown);
//! assert_eq!(report.size, 12);
//! assert!(!report.sha256.is_empty());
//! ```
//!
//! ```
//! use aphrody_re::disasm;
//!
//! // `mov rbp, rsp` encoded as x86-64.
//! let insns = disasm(&[0x48, 0x89, 0xE5], 0x1000, 8);
//! assert_eq!(insns.len(), 1);
//! assert_eq!(insns[0].mnemonic, "mov");
//! ```
//!
//! ## Anti-goal
//!
//! `aphrody-re` is **not** a Ghidra/IDA reimplementation. It orchestrates
//! existing best-of-breed parsers (`goblin`, `iced-x86`) and surfaces stable
//! JSON reports for downstream consumers (CLI sub-command, MCP tool, audit
//! reports). Heavy lifting (decompilation, full disassembly, taint analysis)
//! is delegated.

#![forbid(unsafe_code)]

pub mod google;

/// Native Magika file-type classification (opt-in `magika` feature).
///
/// Host-only and excluded from default + wasm builds because it links the
/// ONNX Runtime via `ort`. Build with `cargo build -p aphrody --features
/// magika`. See the module docs for the rationale.
#[cfg(feature = "magika")]
pub mod magika;

use iced_x86::{
    Decoder, DecoderOptions, Formatter as _, Instruction, IntelFormatter,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// All errors returned by [`triage`] and helpers.
#[derive(Debug, Error)]
pub enum ReError {
    /// `goblin` rejected the bytes — typically a truncated header or
    /// internally inconsistent offsets. The format may still be detected
    /// by magic but section/import/export tables are unreliable.
    #[error("goblin parse error: {0}")]
    Parse(String),
}

impl From<goblin::error::Error> for ReError {
    fn from(e: goblin::error::Error) -> Self {
        Self::Parse(e.to_string())
    }
}

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

/// Detected binary format. Stable JSON serialisation: lowercase variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    /// 32-bit Windows Portable Executable.
    Pe32,
    /// 64-bit Windows Portable Executable.
    Pe64,
    /// 32-bit ELF (Linux / *BSD / etc.).
    Elf32,
    /// 64-bit ELF.
    Elf64,
    /// Bytes did not match any known magic.
    Unknown,
}

impl Format {
    /// Human-readable name (`"PE32+"`, `"ELF64"`, …). Used in CLI output.
    #[must_use]
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Pe32 => "PE32",
            Self::Pe64 => "PE32+",
            Self::Elf32 => "ELF32",
            Self::Elf64 => "ELF64",
            Self::Unknown => "Unknown",
        }
    }
}

/// One section row in the [`TriageReport`].
///
/// Entropy is the standard Shannon entropy of the section bytes, expressed
/// in bits per byte (i.e. in `[0.0, 8.0]`). High entropy (≥ 7.0) is a
/// strong indicator of compression or encryption (packed binaries, e.g.
/// UPX, almost always exceed 7.5 on their `.text` section).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    /// Section name as embedded in the binary (e.g. `".text"`, `".rdata"`).
    pub name: String,
    /// Virtual address (PE) / virtual address (ELF) of the section.
    pub vaddr: u64,
    /// Raw size on disk in bytes.
    pub size: u64,
    /// Shannon entropy of the section bytes, in bits/byte. `None` if the
    /// section has no on-disk content (e.g. `.bss` in ELF).
    pub entropy: Option<f64>,
}

/// Final triage report. Self-describing JSON via `serde`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriageReport {
    /// Detected format. Always populated — `Format::Unknown` if no magic
    /// matched.
    pub format: Format,
    /// Total input size in bytes.
    pub size: usize,
    /// SHA-256 hex digest of the entire input. Always populated, even for
    /// `Format::Unknown`.
    pub sha256: String,
    /// Architecture string when available (`"x86_64"`, `"aarch64"`,
    /// `"i386"`, …). `None` for `Format::Unknown`.
    pub arch: Option<String>,
    /// Entry-point virtual address. `None` for `Format::Unknown`.
    pub entry_point: Option<u64>,
    /// Sections found in the binary (PE/ELF). Empty for `Format::Unknown`.
    pub sections: Vec<Section>,
    /// Names of imported symbols (PE/ELF). Empty for `Format::Unknown`.
    pub imports: Vec<String>,
    /// Names of exported symbols (PE/ELF). Empty for `Format::Unknown`.
    pub exports: Vec<String>,
    /// Sample of ASCII + UTF-16 strings discovered (capped at
    /// [`STRINGS_SAMPLE_LIMIT`] entries, each ≥ [`STRINGS_MIN_LEN`]).
    pub strings_sample: Vec<String>,
}

/// Minimum length (in characters) for a string to appear in the sample.
pub const STRINGS_MIN_LEN: usize = 6;

/// Maximum number of strings included in [`TriageReport::strings_sample`].
/// Keeps the JSON report bounded for downstream consumers.
pub const STRINGS_SAMPLE_LIMIT: usize = 64;

/// Default maximum number of instructions returned by [`disasm`].
/// Callers may pass a smaller `limit`; values larger than this are honoured
/// but the default CLI surface caps at this constant.
pub const DISASM_LIMIT: usize = 256;

// ---------------------------------------------------------------------------
// Disassembly types
// ---------------------------------------------------------------------------

/// One decoded x86-64 instruction as returned by [`disasm`].
///
/// All fields are serializable to JSON. The `text` field contains the full
/// Intel-syntax disassembly line produced by `iced-x86`'s `IntelFormatter`
/// (e.g. `"mov rbp,rsp"`). `mnemonic` is the bare mnemonic extracted from
/// that line (everything before the first space or tab), convenient for
/// filtering without reparsing `text`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisasmInsn {
    /// Byte offset of this instruction from the start of the input slice.
    pub offset: u64,
    /// Virtual address of this instruction (`rip` base + `offset`).
    pub address: u64,
    /// Lowercase hex encoding of the raw instruction bytes (e.g. `"4889e5"`).
    pub bytes_hex: String,
    /// Bare mnemonic (e.g. `"mov"`, `"call"`, `"nop"`).
    pub mnemonic: String,
    /// Full Intel-syntax disassembly line (e.g. `"mov rbp,rsp"`).
    pub text: String,
}

// ---------------------------------------------------------------------------
// Disassembly public API
// ---------------------------------------------------------------------------

/// Disassemble up to `limit` x86-64 instructions from `bytes`.
///
/// `rip` is the virtual address of the first byte (used for RIP-relative
/// operand display and as the base for the `address` field of each
/// [`DisasmInsn`]). Pass `0` if the absolute address is unknown.
///
/// Uses `iced-x86`'s `IntelFormatter` with default options (no decorators,
/// uppercase mnemonics off). Stops at `limit` instructions, at the end of
/// the slice, or at the first invalid/incomplete encoding — whichever comes
/// first. Never panics on arbitrary input.
///
/// # Examples
///
/// ```
/// use aphrody_re::disasm;
///
/// // `mov rbp, rsp` — 3-byte encoding: REX.W + 0x89 /5
/// let insns = disasm(&[0x48, 0x89, 0xE5], 0x1000, 8);
/// assert_eq!(insns.len(), 1);
/// assert_eq!(insns[0].mnemonic, "mov");
/// assert_eq!(insns[0].address, 0x1000);
/// assert_eq!(insns[0].bytes_hex, "4889e5");
/// ```
#[must_use]
pub fn disasm(bytes: &[u8], rip: u64, limit: usize) -> Vec<DisasmInsn> {
    // `limit` is always respected exactly. `DISASM_LIMIT` is the default used
    // by the CLI when no `--limit` flag is given; the library itself does not
    // enforce a hard cap above the caller's request.
    let cap = limit;
    let mut out = Vec::with_capacity(cap.min(64));

    let mut decoder = Decoder::with_ip(64, bytes, rip, DecoderOptions::NONE);
    let mut formatter = IntelFormatter::new();
    // Suppress "db" invalid-byte fallback mnemonics — stop on first invalid.
    let mut insn = Instruction::default();
    let mut text_buf = String::with_capacity(64);

    while decoder.can_decode() && out.len() < cap {
        decoder.decode_out(&mut insn);

        // iced-x86 emits `db XX` for invalid byte sequences. We stop rather
        // than emit a misleading "instruction" because the remaining bytes
        // are uninterpretable as code.
        if insn.is_invalid() {
            break;
        }

        let offset = insn.ip().wrapping_sub(rip);
        let address = insn.ip();
        let len = insn.len();
        let start = offset as usize;
        let raw = bytes.get(start..start.saturating_add(len)).unwrap_or(&[]);
        let bytes_hex = hex::encode(raw);

        text_buf.clear();
        formatter.format(&insn, &mut text_buf);

        // Extract mnemonic: everything before the first ASCII whitespace.
        let mnemonic = text_buf
            .split_once(|c: char| c.is_ascii_whitespace())
            .map_or(text_buf.as_str(), |(m, _)| m)
            .to_owned();

        out.push(DisasmInsn {
            offset,
            address,
            bytes_hex,
            mnemonic,
            text: text_buf.clone(),
        });
    }

    out
}

// ---------------------------------------------------------------------------
// Public API (triage)
// ---------------------------------------------------------------------------

/// Triage a binary blob.
///
/// Returns a fully-populated [`TriageReport`]. **Never panics** on garbage
/// input — bytes that don't match any known magic produce
/// `Format::Unknown` with `size`, `sha256`, and `strings_sample` still
/// populated (the rest empty).
///
/// # Errors
///
/// Returns [`ReError::Parse`] only when `goblin` rejects a binary that
/// looked valid by magic but had internally inconsistent offsets. Truly
/// arbitrary garbage falls into the `Format::Unknown` path with `Ok`.
pub fn triage(bytes: &[u8]) -> Result<TriageReport, ReError> {
    let size = bytes.len();
    let sha256 = hex::encode(Sha256::digest(bytes));
    let strings_sample = extract_strings(bytes, STRINGS_MIN_LEN, STRINGS_SAMPLE_LIMIT);

    // `goblin::Object` requires the `te` feature which is intentionally
    // off in the workspace pin (Terse Executable is niche). We dispatch by
    // hand on the magic bytes and call the format-specific parsers
    // directly — saves enabling a dead feature and gives us explicit
    // control over the fallback path.
    match detect_magic(bytes) {
        MagicHit::Pe => match goblin::pe::PE::parse(bytes) {
            Ok(pe) => Ok(triage_pe(pe, bytes, size, sha256, strings_sample)),
            Err(_) => Ok(unknown_report(size, sha256, strings_sample)),
        },
        MagicHit::Elf => match goblin::elf::Elf::parse(bytes) {
            Ok(elf) => Ok(triage_elf(&elf, bytes, size, sha256, strings_sample)),
            Err(_) => Ok(unknown_report(size, sha256, strings_sample)),
        },
        MagicHit::None => Ok(unknown_report(size, sha256, strings_sample)),
    }
}

enum MagicHit {
    Pe,
    Elf,
    None,
}

fn detect_magic(bytes: &[u8]) -> MagicHit {
    // ELF: 0x7F 'E' 'L' 'F'
    if bytes.len() >= 4 && &bytes[..4] == b"\x7FELF" {
        return MagicHit::Elf;
    }
    // PE: 'M' 'Z' DOS stub. Strict check skips short MZ-only files which
    // goblin would reject anyway.
    if bytes.len() >= 64 && &bytes[..2] == b"MZ" {
        return MagicHit::Pe;
    }
    MagicHit::None
}

fn unknown_report(size: usize, sha256: String, strings_sample: Vec<String>) -> TriageReport {
    TriageReport {
        format: Format::Unknown,
        size,
        sha256,
        arch: None,
        entry_point: None,
        sections: Vec::new(),
        imports: Vec::new(),
        exports: Vec::new(),
        strings_sample,
    }
}

// ---------------------------------------------------------------------------
// Format-specific helpers
// ---------------------------------------------------------------------------

fn triage_pe(
    pe: goblin::pe::PE<'_>,
    bytes: &[u8],
    size: usize,
    sha256: String,
    strings_sample: Vec<String>,
) -> TriageReport {
    let format = if pe.is_64 { Format::Pe64 } else { Format::Pe32 };
    let arch = Some(if pe.is_64 { "x86_64".to_owned() } else { "i386".to_owned() });
    let entry_point = Some(pe.entry as u64);

    let sections = pe
        .sections
        .iter()
        .map(|s| Section {
            name: s.name().unwrap_or("<bad-utf8>").to_owned(),
            vaddr: u64::from(s.virtual_address),
            size: u64::from(s.size_of_raw_data),
            entropy: section_entropy(s.data(bytes).ok().flatten().as_deref()),
        })
        .collect();

    let imports = pe.imports.iter().map(|i| i.name.to_string()).collect();
    let exports = pe.exports.iter().filter_map(|e| e.name.map(|n| n.to_owned())).collect();

    TriageReport {
        format,
        size,
        sha256,
        arch,
        entry_point,
        sections,
        imports,
        exports,
        strings_sample,
    }
}

fn triage_elf(
    elf: &goblin::elf::Elf<'_>,
    bytes: &[u8],
    size: usize,
    sha256: String,
    strings_sample: Vec<String>,
) -> TriageReport {
    let format = if elf.is_64 { Format::Elf64 } else { Format::Elf32 };
    let arch = Some(elf_arch_name(elf.header.e_machine));
    let entry_point = Some(elf.entry);

    let sections = elf
        .section_headers
        .iter()
        .map(|sh| {
            let name = elf.shdr_strtab.get_at(sh.sh_name).unwrap_or("<bad-strtab>").to_owned();
            let offset = sh.sh_offset as usize;
            let len = sh.sh_size as usize;
            let entropy = bytes
                .get(offset..offset.saturating_add(len))
                .filter(|s| !s.is_empty())
                .map(shannon_entropy);
            Section { name, vaddr: sh.sh_addr, size: sh.sh_size, entropy }
        })
        .collect();

    let imports = elf.dynsyms.iter().filter(|s| s.is_import()).filter_map(|s| {
        elf.dynstrtab.get_at(s.st_name).map(std::borrow::ToOwned::to_owned)
    }).collect();

    let exports = elf
        .dynsyms
        .iter()
        .filter(|s| !s.is_import() && s.st_name != 0)
        .filter_map(|s| elf.dynstrtab.get_at(s.st_name).map(std::borrow::ToOwned::to_owned))
        .collect();

    TriageReport {
        format,
        size,
        sha256,
        arch,
        entry_point,
        sections,
        imports,
        exports,
        strings_sample,
    }
}

fn elf_arch_name(e_machine: u16) -> String {
    use goblin::elf::header;
    match e_machine {
        header::EM_X86_64 => "x86_64".to_owned(),
        header::EM_386 => "i386".to_owned(),
        header::EM_AARCH64 => "aarch64".to_owned(),
        header::EM_ARM => "arm".to_owned(),
        header::EM_RISCV => "riscv".to_owned(),
        header::EM_PPC64 => "ppc64".to_owned(),
        other => format!("EM_{other}"),
    }
}

// ---------------------------------------------------------------------------
// Entropy + strings
// ---------------------------------------------------------------------------

fn section_entropy(data: Option<&[u8]>) -> Option<f64> {
    data.filter(|d| !d.is_empty()).map(shannon_entropy)
}

/// Standard Shannon entropy in bits/byte. Range `[0.0, 8.0]`.
#[must_use]
pub fn shannon_entropy(data: &[u8]) -> f64 {
    let mut counts = [0u64; 256];
    for &b in data {
        counts[b as usize] += 1;
    }
    let len = data.len() as f64;
    counts
        .iter()
        .filter(|&&c| c > 0)
        .map(|&c| {
            let p = c as f64 / len;
            -p * p.log2()
        })
        .sum()
}

/// Extract ASCII + UTF-16LE strings of length `>= min_len` (in chars).
/// Returns at most `limit` strings, in order of first appearance, with
/// duplicates collapsed.
#[must_use]
pub fn extract_strings(bytes: &[u8], min_len: usize, limit: usize) -> Vec<String> {
    let mut out = Vec::with_capacity(limit);
    let mut seen: std::collections::HashSet<String> =
        std::collections::HashSet::with_capacity(limit);

    // ASCII pass: contiguous runs of 0x20..=0x7E.
    let mut start: Option<usize> = None;
    for (i, &b) in bytes.iter().enumerate() {
        let printable = (0x20..=0x7E).contains(&b);
        match (printable, start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                let len = i - s;
                if len >= min_len {
                    if let Ok(text) = std::str::from_utf8(&bytes[s..i]) {
                        push_unique(&mut out, &mut seen, text.to_owned(), limit);
                        if out.len() >= limit {
                            return out;
                        }
                    }
                }
                start = None;
            },
            _ => {},
        }
    }
    if let Some(s) = start {
        let len = bytes.len() - s;
        if len >= min_len {
            if let Ok(text) = std::str::from_utf8(&bytes[s..]) {
                push_unique(&mut out, &mut seen, text.to_owned(), limit);
            }
        }
    }

    // UTF-16LE pass: pairs of (printable_ascii, 0x00).
    if out.len() < limit {
        let mut buf = String::with_capacity(64);
        let mut i = 0;
        while i + 1 < bytes.len() && out.len() < limit {
            let lo = bytes[i];
            let hi = bytes[i + 1];
            if hi == 0 && (0x20..=0x7E).contains(&lo) {
                buf.push(lo as char);
            } else {
                if buf.len() >= min_len {
                    push_unique(&mut out, &mut seen, std::mem::take(&mut buf), limit);
                }
                buf.clear();
            }
            i += 2;
        }
        if buf.len() >= min_len {
            push_unique(&mut out, &mut seen, buf, limit);
        }
    }

    out
}

fn push_unique(
    out: &mut Vec<String>,
    seen: &mut std::collections::HashSet<String>,
    s: String,
    limit: usize,
) {
    if out.len() >= limit {
        return;
    }
    if seen.insert(s.clone()) {
        out.push(s);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn garbage_input_returns_unknown_without_panicking() {
        let input: &[u8] = b"clearly not a binary, just random bytes here";
        let r = triage(input).expect("never errs");
        assert_eq!(r.format, Format::Unknown);
        assert_eq!(r.size, input.len());
        assert_eq!(r.sha256.len(), 64);
        assert!(r.entry_point.is_none());
        assert!(r.sections.is_empty());
    }

    #[test]
    fn empty_input_is_unknown_with_known_hash() {
        let r = triage(b"").expect("never errs");
        assert_eq!(r.format, Format::Unknown);
        assert_eq!(r.size, 0);
        // SHA-256 of the empty string is a well-known constant.
        assert_eq!(
            r.sha256,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn shannon_entropy_uniform_bytes_close_to_8() {
        // 256 distinct bytes, equiprobable => entropy == 8.0.
        let uniform: Vec<u8> = (0u8..=255).collect();
        let h = shannon_entropy(&uniform);
        assert!((h - 8.0).abs() < 1e-9, "uniform entropy must be 8.0, got {h}");
    }

    #[test]
    fn shannon_entropy_constant_bytes_is_zero() {
        let zeros = vec![0u8; 1024];
        let h = shannon_entropy(&zeros);
        assert_eq!(h, 0.0);
    }

    #[test]
    fn extract_strings_finds_ascii_runs() {
        let bytes = b"\x00\x00hello world\x00\x01\x02foobar\xffmidstream short\x00THIS_IS_AN_API";
        let s = extract_strings(bytes, 6, 16);
        assert!(s.iter().any(|x| x == "hello world"), "missing 'hello world' in {s:?}");
        assert!(s.iter().any(|x| x == "foobar"), "missing 'foobar' in {s:?}");
        assert!(
            s.iter().any(|x| x.contains("THIS_IS_AN_API")),
            "missing 'THIS_IS_AN_API' in {s:?}"
        );
        // 'short' is 5 chars, below min_len=6 → excluded.
        assert!(!s.iter().any(|x| x == "short"));
    }

    #[test]
    fn extract_strings_finds_utf16le_runs() {
        // "API_KEY" encoded as UTF-16LE: byte order is (low, high) per char,
        // so 'A' (0x41) becomes 0x41 0x00, 'P' (0x50) becomes 0x50 0x00, …
        // Prefix with a few non-string bytes that the UTF-16 pass should
        // skip (\xFF\x01 is not (printable, 0)).
        let bytes: &[u8] = b"\xFF\x01A\x00P\x00I\x00_\x00K\x00E\x00Y\x00\xFF\x01";
        let s = extract_strings(bytes, 6, 8);
        assert!(s.iter().any(|x| x == "API_KEY"), "missing UTF-16LE 'API_KEY' in {s:?}");
    }

    #[test]
    fn extract_strings_respects_limit() {
        // 100 distinct 8-char runs separated by null bytes.
        let mut bytes = Vec::new();
        for i in 0..100u32 {
            bytes.extend_from_slice(format!("STRNG{:03}", i).as_bytes());
            bytes.push(0);
        }
        let s = extract_strings(&bytes, 6, 10);
        assert_eq!(s.len(), 10);
    }

    #[test]
    fn format_display_names_stable() {
        assert_eq!(Format::Pe32.display_name(), "PE32");
        assert_eq!(Format::Pe64.display_name(), "PE32+");
        assert_eq!(Format::Elf32.display_name(), "ELF32");
        assert_eq!(Format::Elf64.display_name(), "ELF64");
        assert_eq!(Format::Unknown.display_name(), "Unknown");
    }

    #[test]
    fn triage_report_serializes_to_stable_json_shape() {
        let r = triage(b"unknown").expect("ok");
        let json = serde_json::to_value(&r).expect("serialize");
        // Lowercase variant per #[serde(rename_all = "lowercase")].
        assert_eq!(json["format"], "unknown");
        assert_eq!(json["size"], 7);
        assert!(json["sha256"].as_str().unwrap().len() == 64);
        // Sections/imports/exports must always be arrays (never null) for
        // downstream consumers that index without null-checking.
        assert!(json["sections"].is_array());
        assert!(json["imports"].is_array());
        assert!(json["exports"].is_array());
        assert!(json["strings_sample"].is_array());
    }

    #[test]
    fn minimal_pe_magic_is_detected_or_falls_back_gracefully() {
        // Bare MZ header (DOS stub start) without a valid PE32+ header.
        // goblin should either parse partially or error — either way we
        // must not panic and the report must be well-formed.
        let mut bytes = b"MZ".to_vec();
        bytes.extend_from_slice(&[0u8; 62]); // pad DOS header
        let r = triage(&bytes).expect("never panics");
        // Format is either Pe* (if goblin accepts) or Unknown (if it
        // rejects). Both are acceptable — what matters is the API contract.
        assert!(matches!(r.format, Format::Pe32 | Format::Pe64 | Format::Unknown));
        assert_eq!(r.size, 64);
    }

    // -----------------------------------------------------------------------
    // disasm tests
    // -----------------------------------------------------------------------

    /// `48 89 E5` = `mov rbp, rsp` (REX.W + MOV r/m64,r64, ModRM 0xE5).
    #[test]
    fn disasm_mov_rbp_rsp() {
        let bytes: &[u8] = &[0x48, 0x89, 0xE5];
        let insns = disasm(bytes, 0x1000, 8);
        assert_eq!(insns.len(), 1, "expected exactly 1 instruction, got {insns:?}");
        assert_eq!(insns[0].mnemonic, "mov");
        assert_eq!(insns[0].address, 0x1000);
        assert_eq!(insns[0].offset, 0);
        assert_eq!(insns[0].bytes_hex, "4889e5");
        // Full text must contain both operands.
        assert!(
            insns[0].text.contains("rbp") && insns[0].text.contains("rsp"),
            "unexpected text: {}",
            insns[0].text
        );
    }

    /// Multi-instruction sequence: `push rbp` + `mov rbp, rsp` + `nop`.
    /// Canonical function prologue found in almost every x86-64 binary.
    #[test]
    fn disasm_function_prologue_three_insns() {
        // 55            push rbp
        // 48 89 E5      mov rbp, rsp
        // 90            nop
        let bytes: &[u8] = &[0x55, 0x48, 0x89, 0xE5, 0x90];
        let insns = disasm(bytes, 0x0, 16);
        assert_eq!(insns.len(), 3, "expected 3 instructions, got {insns:?}");
        assert_eq!(insns[0].mnemonic, "push");
        assert_eq!(insns[1].mnemonic, "mov");
        assert_eq!(insns[2].mnemonic, "nop");
        // Offsets must be monotonically increasing and match byte lengths.
        assert_eq!(insns[0].offset, 0);
        assert_eq!(insns[1].offset, 1); // `push rbp` is 1 byte
        assert_eq!(insns[2].offset, 4); // `mov rbp, rsp` is 3 bytes
    }

    /// `limit` must be respected: decode at most N instructions even if more
    /// valid bytes remain.
    #[test]
    fn disasm_respects_limit() {
        // 10 consecutive `nop` bytes (0x90).
        let bytes: Vec<u8> = vec![0x90; 10];
        let insns = disasm(&bytes, 0x0, 3);
        assert_eq!(insns.len(), 3, "limit=3 must cap at 3, got {insns:?}");
    }

    /// Empty slice must produce an empty result without panicking.
    #[test]
    fn disasm_empty_bytes_returns_empty() {
        let insns = disasm(&[], 0x0, DISASM_LIMIT);
        assert!(insns.is_empty());
    }

    /// Garbage bytes must not panic; the decoder may emit 0 valid instructions
    /// or stop at the first invalid encoding.
    #[test]
    fn disasm_invalid_bytes_does_not_panic() {
        // 0xFF 0xFF … is an invalid encoding in x86-64 (no valid 64-bit opcode
        // starts with FF FF for GRPFF /7 + invalid ModRM).
        let bytes: &[u8] = &[0xFF, 0xFF, 0xFF, 0xFF];
        // Must not panic regardless of the output.
        let _insns = disasm(bytes, 0x4000, DISASM_LIMIT);
    }

    /// Verify `DisasmInsn` round-trips through JSON without losing fields.
    #[test]
    fn disasm_insn_serializes_to_stable_json_shape() {
        let bytes: &[u8] = &[0x90]; // nop
        let insns = disasm(bytes, 0x2000, 1);
        assert_eq!(insns.len(), 1);
        let json = serde_json::to_value(&insns[0]).expect("serialize");
        assert!(json["offset"].is_number());
        assert!(json["address"].is_number());
        assert!(json["bytes_hex"].is_string());
        assert!(json["mnemonic"].is_string());
        assert!(json["text"].is_string());
        assert_eq!(json["address"], 0x2000u64);
        assert_eq!(json["mnemonic"], "nop");
    }
}
