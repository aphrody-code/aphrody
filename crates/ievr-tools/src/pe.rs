// SPDX-License-Identifier: Apache-2.0
//! PE/COFF inspector for Windows binaries (`.exe`, `.dll`, `.sys`, …).
//!
//! Parses PE32 and PE32+ headers via [`goblin`] and returns a rich
//! [`PeReport`] that covers machine architecture, subsystem, link timestamp,
//! sections, imports, and exports.  The only I/O performed is a single
//! `std::fs::read` call; no memory-mapping is used so the API is
//! `#[cfg(target_os = …)]`-free and compiles on all tier-1 targets.

#![allow(clippy::module_name_repetitions)]

use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

use goblin::pe::PE;
use goblin::pe::header::{
    COFF_MACHINE_ARM, COFF_MACHINE_ARM64, COFF_MACHINE_ARMNT, COFF_MACHINE_IA64,
    COFF_MACHINE_X86, COFF_MACHINE_X86_64,
};
use goblin::pe::subsystem::{
    IMAGE_SUBSYSTEM_EFI_APPLICATION, IMAGE_SUBSYSTEM_EFI_BOOT_SERVICE_DRIVER,
    IMAGE_SUBSYSTEM_EFI_ROM, IMAGE_SUBSYSTEM_EFI_RUNTIME_DRIVER, IMAGE_SUBSYSTEM_NATIVE,
    IMAGE_SUBSYSTEM_WINDOWS_BOOT_APPLICATION, IMAGE_SUBSYSTEM_WINDOWS_CUI,
    IMAGE_SUBSYSTEM_WINDOWS_GUI, IMAGE_SUBSYSTEM_XBOX,
};

// ── public data types ──────────────────────────────────────────────────────

/// Complete PE/COFF inspection report for a single binary.
///
/// All string fields use human-readable representations (e.g. `"AMD64"`
/// rather than `"0x8664"`) so callers can display them without further
/// decoding.  The raw numeric values are preserved in [`SectionInfo`] and
/// other sub-structures for callers that need them.
#[derive(Debug, Clone)]
pub struct PeReport {
    /// Human-readable machine type, e.g. `"AMD64"`, `"I386"`, `"ARM64"`.
    pub machine: String,
    /// Human-readable subsystem, e.g. `"Windows GUI"`, `"Windows CUI"`.
    pub subsystem: String,
    /// COFF `TimeDateStamp` — seconds since Unix epoch (1970-01-01T00:00:00Z).
    pub timestamp: u32,
    /// All section table entries in header order.
    pub sections: Vec<SectionInfo>,
    /// Import descriptor table: one entry per DLL.
    pub imports: Vec<ImportInfo>,
    /// Exported symbol names (named exports only; ordinal-only exports are
    /// omitted because they carry no string name).
    pub exports: Vec<String>,
    /// `true` for PE32+ (64-bit), `false` for PE32 (32-bit).
    pub is_64bit: bool,
}

/// Metadata for one PE section.
#[derive(Debug, Clone)]
pub struct SectionInfo {
    /// Section name decoded from the 8-byte COFF name field (trailing NULs
    /// and spaces stripped).  Long names using the `/offset` COFF string-table
    /// convention are represented verbatim.
    pub name: String,
    /// `VirtualSize` from the section header (bytes when mapped into memory).
    pub virtual_size: u32,
    /// `VirtualAddress` (RVA) from the section header.
    pub virtual_address: u32,
    /// `SizeOfRawData` — size of the on-disk representation (may differ from
    /// `virtual_size` due to alignment padding or BSS).
    pub raw_size: u32,
    /// Raw `Characteristics` bitmask (e.g. `0x60000020` for `.text`).
    pub characteristics: u32,
}

/// Import information for a single source DLL.
#[derive(Debug, Clone)]
pub struct ImportInfo {
    /// DLL name exactly as recorded in the import descriptor (e.g.
    /// `"KERNEL32.dll"`).
    pub dll: String,
    /// Sorted, deduplicated list of imported symbol names from this DLL.
    /// Ordinal-only imports are represented as `"#<ordinal>"`.
    pub functions: Vec<String>,
}

// ── public API ─────────────────────────────────────────────────────────────

/// Parse the PE/COFF headers of the file at `path` and return a [`PeReport`].
///
/// The entire file is read into a heap-allocated buffer before parsing.  For
/// large binaries (hundreds of MB) this is the dominant allocation; callers
/// that need streaming or mmap semantics should pre-read the bytes and call
/// [`inspect_bytes`] directly.
///
/// # Errors
///
/// Returns an error if the file cannot be read, if the bytes do not start
/// with a valid MZ/PE signature, or if a required header field is missing or
/// malformed.
pub fn inspect(path: &Path) -> anyhow::Result<PeReport> {
    let bytes = std::fs::read(path)
        .map_err(|e| anyhow::anyhow!("cannot read `{}`: {e}", path.display()))?;
    inspect_bytes(&bytes)
        .map_err(|e| anyhow::anyhow!("PE parse error for `{}`: {e}", path.display()))
}

/// Parse a PE/COFF binary from a byte slice and return a [`PeReport`].
///
/// Prefer [`inspect`] when reading from disk; this entry-point is useful for
/// in-memory buffers (e.g. downloaded payloads or test fixtures).
///
/// # Errors
///
/// Returns an error if the slice is not a valid PE32 or PE32+ image.
pub fn inspect_bytes(bytes: &[u8]) -> anyhow::Result<PeReport> {
    let pe = PE::parse(bytes)
        .map_err(|e| anyhow::anyhow!("goblin PE::parse failed: {e}"))?;

    let coff = &pe.header.coff_header;
    let machine = machine_name(coff.machine);

    let (subsystem, is_64bit) = match pe.header.optional_header {
        Some(ref opt) => {
            let sub = subsystem_name(opt.windows_fields.subsystem);
            // PE32+ magic = 0x20b; PE32 = 0x10b
            let is64 = opt.standard_fields.magic == 0x020b;
            (sub, is64)
        }
        None => (String::from("(no optional header)"), pe.is_64),
    };

    let timestamp = coff.time_date_stamp;

    // ── sections ────────────────────────────────────────────────────────────
    let sections: Vec<SectionInfo> = pe
        .sections
        .iter()
        .map(|s| {
            // The COFF name field is exactly 8 bytes; strip trailing NULs/spaces.
            let raw_name = std::str::from_utf8(&s.name)
                .unwrap_or("")
                .trim_end_matches('\0')
                .trim_end();
            // If goblin resolved a long name via the string table, prefer that.
            let name = s
                .real_name
                .as_deref()
                .unwrap_or(raw_name)
                .to_owned();
            SectionInfo {
                name,
                virtual_size: s.virtual_size,
                virtual_address: s.virtual_address,
                raw_size: s.size_of_raw_data,
                characteristics: s.characteristics,
            }
        })
        .collect();

    // ── imports ─────────────────────────────────────────────────────────────
    // goblin gives us a flat `Vec<Import>` where each entry carries the dll
    // name.  We need to group by dll preserving insertion order of DLLs while
    // deduplicating function names within each DLL.
    let mut dll_order: Vec<String> = Vec::new();
    let mut dll_funcs: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for imp in &pe.imports {
        let dll_key = imp.dll.to_owned();
        if !dll_funcs.contains_key(&dll_key) {
            dll_order.push(dll_key.clone());
            dll_funcs.insert(dll_key.clone(), Vec::new());
        }
        // Ordinal-only imports have a synthesised name like "#NNN" in goblin's
        // Cow<str> — we just store it verbatim; it clearly signals no symbol name.
        let func_name = imp.name.as_ref().to_owned();
        let funcs = dll_funcs
            .get_mut(&dll_key)
            .expect("just inserted above; invariant holds");
        if !funcs.contains(&func_name) {
            funcs.push(func_name);
        }
    }

    let imports: Vec<ImportInfo> = dll_order
        .into_iter()
        .filter_map(|dll| {
            dll_funcs.remove(&dll).map(|mut funcs| {
                funcs.sort_unstable();
                ImportInfo { dll, functions: funcs }
            })
        })
        .collect();

    // ── exports ──────────────────────────────────────────────────────────────
    // Only include exports that have a real symbol name; skip forwarders that
    // happen to be unnamed.
    let mut exports: Vec<String> = pe
        .exports
        .iter()
        .filter_map(|e| e.name.map(str::to_owned))
        .collect();
    exports.sort_unstable();
    exports.dedup();

    Ok(PeReport {
        machine,
        subsystem,
        timestamp,
        sections,
        imports,
        exports,
        is_64bit,
    })
}

// ── PeReport::print_summary ─────────────────────────────────────────────────

impl PeReport {
    /// Write a human-readable multi-line summary of the report to `out`.
    ///
    /// The output intentionally avoids terminal escape codes so it renders
    /// correctly in log files and CI pipelines.
    ///
    /// # Errors
    ///
    /// Propagates any I/O error returned by `out.write_all` or `writeln!`.
    pub fn print_summary(&self, out: &mut impl Write) -> std::io::Result<()> {
        writeln!(out, "=== PE Report ===")?;
        writeln!(out, "  Machine   : {}", self.machine)?;
        writeln!(out, "  Subsystem : {}", self.subsystem)?;
        writeln!(out, "  Bitness   : {}", if self.is_64bit { "64-bit (PE32+)" } else { "32-bit (PE32)" })?;
        writeln!(out, "  Timestamp : {} (0x{:08x})", self.timestamp, self.timestamp)?;

        writeln!(out)?;
        writeln!(out, "--- Sections ({}) ---", self.sections.len())?;
        writeln!(
            out,
            "  {:<16}  {:>10}  {:>10}  {:>10}  {}",
            "Name", "VirtSize", "RVA", "RawSize", "Characteristics"
        )?;
        for s in &self.sections {
            writeln!(
                out,
                "  {:<16}  {:>10}  0x{:08x}  {:>10}  0x{:08x}",
                s.name, s.virtual_size, s.virtual_address, s.raw_size, s.characteristics
            )?;
        }

        writeln!(out)?;
        writeln!(out, "--- Imports ({} DLLs) ---", self.imports.len())?;
        for imp in &self.imports {
            writeln!(out, "  {} ({} symbols)", imp.dll, imp.functions.len())?;
            for func in &imp.functions {
                writeln!(out, "    {func}")?;
            }
        }

        writeln!(out)?;
        writeln!(out, "--- Exports ({}) ---", self.exports.len())?;
        for exp in &self.exports {
            writeln!(out, "  {exp}")?;
        }

        Ok(())
    }
}

// ── private helpers ─────────────────────────────────────────────────────────

/// Map a COFF machine type u16 to a human-readable string.
///
/// All values are taken from the PE specification:
/// <https://learn.microsoft.com/en-us/windows/win32/debug/pe-format#machine-types>
fn machine_name(machine: u16) -> String {
    let s = match machine {
        COFF_MACHINE_X86 => "I386",
        COFF_MACHINE_X86_64 => "AMD64",
        COFF_MACHINE_ARM => "ARM",
        COFF_MACHINE_ARMNT => "ARMv7 Thumb-2",
        COFF_MACHINE_ARM64 => "ARM64",
        COFF_MACHINE_IA64 => "IA-64",
        0x0000 => "Unknown",
        0x01d3 => "AM33",
        0x0ebc => "EFI Byte Code",
        0x9041 => "M32R",
        0x0166 => "MIPS16",
        0x0366 => "MIPS with FPU",
        0x0466 => "MIPS16 with FPU",
        0x01f0 => "PowerPC",
        0x01f1 => "PowerPC with FP",
        0x0162 => "R4000",
        0x5032 => "RISC-V 32-bit",
        0x5064 => "RISC-V 64-bit",
        0x5128 => "RISC-V 128-bit",
        0x01a2 => "Hitachi SH3",
        0x01a3 => "Hitachi SH3 DSP",
        0x01a6 => "Hitachi SH4",
        0x01a8 => "Hitachi SH5",
        0x01c2 => "Thumb",
        0x0169 => "MIPS WCE v2",
        other => return format!("Unknown (0x{other:04x})"),
    };
    s.to_owned()
}

/// Map a PE optional-header subsystem u16 to a human-readable string.
fn subsystem_name(subsystem: u16) -> String {
    let s = match subsystem {
        0 => "Unknown",
        IMAGE_SUBSYSTEM_NATIVE => "Native",
        IMAGE_SUBSYSTEM_WINDOWS_GUI => "Windows GUI",
        IMAGE_SUBSYSTEM_WINDOWS_CUI => "Windows CUI (console)",
        5 => "OS/2 CUI",
        7 => "POSIX CUI",
        8 => "Native Windows",
        9 => "Windows CE GUI",
        IMAGE_SUBSYSTEM_EFI_APPLICATION => "EFI Application",
        IMAGE_SUBSYSTEM_EFI_BOOT_SERVICE_DRIVER => "EFI Boot Service Driver",
        IMAGE_SUBSYSTEM_EFI_RUNTIME_DRIVER => "EFI Runtime Driver",
        IMAGE_SUBSYSTEM_EFI_ROM => "EFI ROM Image",
        IMAGE_SUBSYSTEM_XBOX => "Xbox",
        IMAGE_SUBSYSTEM_WINDOWS_BOOT_APPLICATION => "Windows Boot Application",
        other => return format!("Unknown (0x{other:04x})"),
    };
    s.to_owned()
}

// ── tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Default path for the smoke-test binary.
    const DEFAULT_NIE_EXE: &str =
        "C:/Program Files (x86)/Steam/steamapps/common/INAZUMA ELEVEN Victory Road/nie.exe";

    /// Smoke test against the real `nie.exe` binary.
    ///
    /// Skipped unless the `integration` feature is active **or** the env var
    /// `IEVR_NIE_EXE` is set to an existing path.  In CI, add
    /// `--features integration` to run it against a real binary.
    ///
    /// The test asserts:
    /// - The file parses without error.
    /// - At least one section is present.
    /// - At least one import DLL is listed.
    /// - The binary is AMD64 PE32+.
    #[cfg_attr(not(feature = "integration"), ignore = "requires IEVR_NIE_EXE or --features integration")]
    #[test]
    fn parses_nie_exe() {
        let path_str = std::env::var("IEVR_NIE_EXE")
            .unwrap_or_else(|_| DEFAULT_NIE_EXE.to_owned());
        let path = std::path::Path::new(&path_str);

        if !path.exists() {
            eprintln!("skip parses_nie_exe: path not found: {}", path.display());
            return;
        }

        let report = inspect(path).expect("inspect should succeed on nie.exe");

        assert!(
            !report.sections.is_empty(),
            "expected at least one section, got 0"
        );
        assert!(
            !report.imports.is_empty(),
            "expected at least one import DLL, got 0"
        );
        assert_eq!(report.machine, "AMD64", "nie.exe should be AMD64");
        assert!(report.is_64bit, "nie.exe should be PE32+");

        // Print summary to stderr so it shows up with `cargo test -- --nocapture`.
        let mut buf = Vec::<u8>::new();
        report
            .print_summary(&mut buf)
            .expect("print_summary should not fail");
        eprintln!("{}", String::from_utf8_lossy(&buf));
    }

    /// Unit test that validates the section-name trimming logic on a
    /// synthetic 8-byte name containing trailing NUL bytes.
    #[test]
    fn section_name_trim() {
        // Simulate a `.text\0\0\0` COFF section name.
        let raw: [u8; 8] = *b".text\x00\x00\x00";
        let trimmed = std::str::from_utf8(&raw)
            .unwrap_or("")
            .trim_end_matches('\0')
            .trim_end()
            .to_owned();
        assert_eq!(trimmed, ".text");
    }

    /// Confirms that [`machine_name`] produces the expected strings for the
    /// two most common architectures found in Windows game binaries.
    #[test]
    fn machine_name_known_values() {
        assert_eq!(machine_name(0x8664), "AMD64");
        assert_eq!(machine_name(0x014c), "I386");
        assert_eq!(machine_name(0xffff), "Unknown (0xffff)");
    }

    /// Confirms that [`subsystem_name`] correctly identifies GUI vs CUI.
    #[test]
    fn subsystem_name_known_values() {
        assert_eq!(subsystem_name(2), "Windows GUI");
        assert_eq!(subsystem_name(3), "Windows CUI (console)");
        assert_eq!(subsystem_name(0xbeef), "Unknown (0xbeef)");
    }
}
