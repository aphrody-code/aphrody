// SPDX-License-Identifier: Apache-2.0
//! Zero-copy binary header inspection via zerocopy.
//!
//! This module provides safe, zero-copy access to the fixed-size headers that
//! appear at the start of PE32/PE32+ and ELF32/ELF64 binaries. The types
//! here are `FromBytes` (and where applicable `Immutable`) so that
//! `zerocopy::Ref::from_bytes` can transmute a raw `&[u8]` slice into a typed
//! header reference without any copy and without any `unsafe` block.
//!
//! ## Why zerocopy instead of goblin?
//!
//! `goblin` deserialises the full binary into owned Rust structs -- it is the
//! right tool for deep parsing (sections, imports, relocations). This module
//! targets triage: given a raw byte slice, answer "what magic does this start
//! with and what are the first few header fields?" in a single branch with no
//! allocation. That is exactly the zero-copy value proposition.
//!
//! ## Wasm compatibility
//!
//! All types derive `zerocopy::FromBytes` + `zerocopy::Immutable`; none of
//! them perform I/O or allocate, so this module compiles cleanly under
//! `wasm32-unknown-unknown` and `wasm32-wasi`.
//!
//! ## Layout guarantees
//!
//! Every PE field uses `zerocopy::little_endian::*` (LE) types so that the
//! interpretation is always little-endian regardless of host byte-order.
//! PE and ELF headers are defined as little-endian on-disk, and zerocopy
//! enforces the layout statically.

use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, little_endian as LE};

// ---------------------------------------------------------------------------
// ELF identity (first 16 bytes, byte-order agnostic)
// ---------------------------------------------------------------------------

/// The ELF identification array (the first 16 bytes of every ELF file).
/// Byte-order agnostic: all fields are individual bytes or byte arrays.
///
/// Reference: System V ABI section 4.1 ELF Identification.
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, KnownLayout, Immutable)]
#[repr(C)]
pub struct ElfIdent {
    /// Magic bytes: 0x7F followed by 'E', 'L', 'F'.
    pub magic: [u8; 4],
    /// ELF class: 1 = 32-bit, 2 = 64-bit.
    pub elf_class: u8,
    /// Data encoding: 1 = little-endian, 2 = big-endian.
    pub elf_data: u8,
    /// ELF version (must be 1 for EV_CURRENT).
    pub elf_version: u8,
    /// OS/ABI identification (0 = ELFOSABI_NONE / System V).
    pub elf_osabi: u8,
    /// ABI version.
    pub elf_abiversion: u8,
    /// Padding (7 bytes, must be zero).
    pub elf_pad: [u8; 7],
}

impl ElfIdent {
    /// ELF magic constant: 0x7F 'E' 'L' 'F'.
    pub const MAGIC: [u8; 4] = [0x7F, b'E', b'L', b'F'];
    /// EI_CLASS value for a 32-bit ELF.
    pub const CLASS_32: u8 = 1;
    /// EI_CLASS value for a 64-bit ELF.
    pub const CLASS_64: u8 = 2;

    /// Attempt to read an `ElfIdent` from the start of `bytes` without copying.
    ///
    /// Returns `None` if the slice is shorter than 16 bytes or if the magic
    /// bytes do not match the ELF magic (0x7F 'E' 'L' 'F').
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Option<&Self> {
        let ref_obj = zerocopy::Ref::<_, Self>::from_bytes(bytes.get(..16)?).ok()?;
        let ident = zerocopy::Ref::into_ref(ref_obj);
        if ident.magic == Self::MAGIC { Some(ident) } else { None }
    }

    /// Return `true` if this identifies a 64-bit ELF.
    #[must_use]
    pub fn is_64(&self) -> bool {
        self.elf_class == Self::CLASS_64
    }

    /// Return `true` if this identifies a 32-bit ELF.
    #[must_use]
    pub fn is_32(&self) -> bool {
        self.elf_class == Self::CLASS_32
    }
}

// ---------------------------------------------------------------------------
// DOS MZ stub (first 64 bytes of every PE file)
// ---------------------------------------------------------------------------

/// The MS-DOS stub header that starts every Portable Executable.
///
/// The minimum useful portion is the first 64 bytes: the MZ magic (`e_magic`)
/// and `e_lfanew` at offset 60, which gives the byte offset of the PE
/// signature (`IMAGE_NT_HEADERS`).
///
/// Reference: PE/COFF specification, section 2.1 MS-DOS Stub.
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, KnownLayout, Immutable)]
#[repr(C)]
pub struct DosHeader {
    /// Magic: 'M' 'Z' (0x4D 0x5A).
    pub e_magic: [u8; 2],
    /// Bytes on last page of file.
    pub e_cblp: LE::U16,
    /// Pages in file.
    pub e_cp: LE::U16,
    /// Relocations.
    pub e_crlc: LE::U16,
    /// Size of header in paragraphs.
    pub e_cparhdr: LE::U16,
    /// Minimum extra paragraphs needed.
    pub e_minalloc: LE::U16,
    /// Maximum extra paragraphs needed.
    pub e_maxalloc: LE::U16,
    /// Initial (relative) SS value.
    pub e_ss: LE::U16,
    /// Initial SP value.
    pub e_sp: LE::U16,
    /// Checksum.
    pub e_csum: LE::U16,
    /// Initial IP value.
    pub e_ip: LE::U16,
    /// Initial (relative) CS value.
    pub e_cs: LE::U16,
    /// File address of relocation table.
    pub e_lfarlc: LE::U16,
    /// Overlay number.
    pub e_ovno: LE::U16,
    /// Reserved (4 x u16).
    pub e_res: [LE::U16; 4],
    /// OEM identifier.
    pub e_oemid: LE::U16,
    /// OEM information.
    pub e_oeminfo: LE::U16,
    /// Reserved (10 x u16).
    pub e_res2: [LE::U16; 10],
    /// File address of the PE header (`IMAGE_NT_HEADERS`). Little-endian signed.
    pub e_lfanew: LE::I32,
}

impl DosHeader {
    /// DOS MZ magic bytes.
    pub const MAGIC: [u8; 2] = [b'M', b'Z'];

    /// Attempt to read a `DosHeader` from the start of `bytes` without copying.
    ///
    /// Returns `None` if the slice is shorter than 64 bytes or if the magic
    /// bytes are not 'M' 'Z'.
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Option<&Self> {
        let ref_obj = zerocopy::Ref::<_, Self>::from_bytes(bytes.get(..64)?).ok()?;
        let hdr = zerocopy::Ref::into_ref(ref_obj);
        if hdr.e_magic == Self::MAGIC { Some(hdr) } else { None }
    }

    /// Byte offset of the PE signature (`IMAGE_NT_HEADERS`) within the file.
    ///
    /// Returns `None` when the offset is negative or zero (malformed header).
    #[must_use]
    pub fn pe_offset(&self) -> Option<usize> {
        let v = self.e_lfanew.get();
        if v > 0 { Some(v as usize) } else { None }
    }
}

// ---------------------------------------------------------------------------
// PE Optional-header magic (identifies PE32 vs PE32+)
// ---------------------------------------------------------------------------

/// The `Magic` field at the start of the PE Optional Header.
///
/// Immediately follows the 20-byte COFF file header (which in turn follows
/// the 4-byte PE signature `PE\0\0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeMagic {
    /// `IMAGE_NT_OPTIONAL_HDR32_MAGIC` -- 32-bit PE (0x010B).
    Pe32,
    /// `IMAGE_NT_OPTIONAL_HDR64_MAGIC` -- 64-bit PE, PE32+ (0x020B).
    Pe64,
    /// Unrecognised magic value.
    Unknown,
}

impl PeMagic {
    /// Read the PE optional-header magic from a raw byte slice.
    ///
    /// `bytes` must start at byte 0 of the file. The method locates the
    /// optional-header via `e_lfanew`, skips the 4-byte PE signature and the
    /// 20-byte COFF header, and reads the 2-byte magic.
    /// Returns `PeMagic::Unknown` for any malformed structure.
    #[must_use]
    pub fn from_file_bytes(bytes: &[u8]) -> Self {
        let dos = match DosHeader::from_bytes(bytes) {
            Some(d) => d,
            None => return Self::Unknown,
        };
        let pe_off = match dos.pe_offset() {
            Some(o) => o,
            None => return Self::Unknown,
        };
        // 4-byte PE signature + 20-byte COFF header = 24 bytes before the
        // optional header magic field.
        let opt_off = pe_off.saturating_add(24);
        let magic_bytes = match bytes.get(opt_off..opt_off + 2) {
            Some(b) => b,
            None => return Self::Unknown,
        };
        match u16::from_le_bytes([magic_bytes[0], magic_bytes[1]]) {
            0x010B => Self::Pe32,
            0x020B => Self::Pe64,
            _ => Self::Unknown,
        }
    }
}

// ---------------------------------------------------------------------------
// Public facade: HeaderProbe
// ---------------------------------------------------------------------------

/// Lightweight, zero-copy result of probing the first few header bytes of a
/// binary.
///
/// Constructed via [`HeaderProbe::from_bytes`], which reads at most the first
/// 64 bytes and performs no allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderProbe {
    /// Valid ELF identity detected; carries the bitness.
    Elf {
        /// `true` for ELF64, `false` for ELF32.
        is_64: bool,
    },
    /// Valid DOS/MZ magic detected; PE magic resolved to bitness.
    Pe {
        /// `true` for PE32+ (64-bit), `false` for PE32 (32-bit).
        is_64: bool,
    },
    /// No recognised magic in the first bytes.
    Unknown,
}

impl HeaderProbe {
    /// Probe `bytes` and return the detected header type.
    ///
    /// Performs zero allocations and reads at most 64 bytes from the front
    /// of the slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use aphrody_re::headers::HeaderProbe;
    ///
    /// // ELF64 probe.
    /// let mut elf_bytes = [0u8; 16];
    /// elf_bytes[0..4].copy_from_slice(b"\x7FELF");
    /// elf_bytes[4] = 2; // EI_CLASS = 64-bit
    /// assert_eq!(HeaderProbe::from_bytes(&elf_bytes), HeaderProbe::Elf { is_64: true });
    ///
    /// // Unknown: too short.
    /// assert_eq!(HeaderProbe::from_bytes(b""), HeaderProbe::Unknown);
    /// ```
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Self {
        if let Some(ident) = ElfIdent::from_bytes(bytes) {
            return Self::Elf { is_64: ident.is_64() };
        }
        if DosHeader::from_bytes(bytes).is_some() {
            let is_64 = matches!(PeMagic::from_file_bytes(bytes), PeMagic::Pe64);
            return Self::Pe { is_64 };
        }
        Self::Unknown
    }

    /// `true` if the probe identified any recognised format.
    #[must_use]
    pub fn is_known(&self) -> bool {
        !matches!(self, Self::Unknown)
    }

    /// `true` if this is a 64-bit binary (ELF64 or PE32+).
    #[must_use]
    pub fn is_64(&self) -> bool {
        matches!(self, Self::Elf { is_64: true } | Self::Pe { is_64: true })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use googletest::prelude::*;

    // ElfIdent tests

    #[gtest]
    fn elf_ident_parses_elf64_magic() {
        let mut buf = [0u8; 16];
        buf[0..4].copy_from_slice(b"\x7FELF");
        buf[4] = ElfIdent::CLASS_64;
        buf[5] = 1; // EI_DATA = little-endian
        buf[6] = 1; // EI_VERSION = current

        let ident = ElfIdent::from_bytes(&buf).expect("must parse valid ELF ident");
        expect_that!(ident.magic, eq(ElfIdent::MAGIC));
        expect_that!(ident.is_64(), eq(true));
        expect_that!(ident.is_32(), eq(false));
    }

    #[gtest]
    fn elf_ident_parses_elf32_magic() {
        let mut buf = [0u8; 16];
        buf[0..4].copy_from_slice(b"\x7FELF");
        buf[4] = ElfIdent::CLASS_32;

        let ident = ElfIdent::from_bytes(&buf).expect("must parse valid ELF32 ident");
        expect_that!(ident.is_32(), eq(true));
        expect_that!(ident.is_64(), eq(false));
    }

    #[gtest]
    fn elf_ident_rejects_wrong_magic() {
        // Second byte is 'G' not 'E' -- invalid ELF magic.
        let mut buf = [0u8; 16];
        buf[0] = 0x7F;
        buf[1] = b'G';
        buf[2] = b'L';
        buf[3] = b'F';
        expect_that!(ElfIdent::from_bytes(&buf), none());
    }

    #[gtest]
    fn elf_ident_rejects_short_slice() {
        // 15 bytes -- one too short for a valid ELF ident.
        let buf = [0x7F, b'E', b'L', b'F', 2u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        expect_that!(ElfIdent::from_bytes(&buf), none());
    }

    // DosHeader tests

    #[gtest]
    fn dos_header_parses_mz_prefix() {
        let mut buf = [0u8; 64];
        buf[0] = b'M';
        buf[1] = b'Z';
        // e_lfanew at offset 60, LE value 0x80 = 128.
        buf[60] = 0x80;

        let dos = DosHeader::from_bytes(&buf).expect("must parse valid DOS header");
        expect_that!(dos.e_magic, eq(DosHeader::MAGIC));
        expect_that!(dos.pe_offset(), some(eq(0x80_usize)));
    }

    #[gtest]
    fn dos_header_rejects_wrong_magic() {
        let mut buf = [0u8; 64];
        buf[0] = b'Z';
        buf[1] = b'M';
        expect_that!(DosHeader::from_bytes(&buf), none());
    }

    #[gtest]
    fn dos_header_rejects_short_slice() {
        let buf = [b'M', b'Z', 0u8, 0];
        expect_that!(DosHeader::from_bytes(&buf), none());
    }

    #[gtest]
    fn dos_header_pe_offset_zero_returns_none() {
        let mut buf = [0u8; 64];
        buf[0] = b'M';
        buf[1] = b'Z';
        // e_lfanew stays zero -- pe_offset() must return None.
        let dos = DosHeader::from_bytes(&buf).expect("parses");
        expect_that!(dos.pe_offset(), none());
    }

    // PeMagic tests

    #[gtest]
    fn pe_magic_unknown_for_short_slice() {
        expect_that!(PeMagic::from_file_bytes(b"MZ"), eq(PeMagic::Unknown));
    }

    #[gtest]
    fn pe_magic_unknown_for_non_pe_bytes() {
        let buf: Vec<u8> = b"\x7FELF".iter().chain([0u8; 60].iter()).copied().collect();
        expect_that!(PeMagic::from_file_bytes(&buf), eq(PeMagic::Unknown));
    }

    /// Craft a minimal DOS stub + PE optional-header to verify Pe64 resolution.
    ///
    /// Layout: 64-byte DOS header, PE header at offset 64.
    /// e_lfanew (offset 60) = 64.
    /// PE sig at 64 = "PE\0\0".
    /// COFF header: 20 bytes zeroed (offsets 68..88).
    /// Optional header magic at offset 64+4+20 = 88: 0x0B 0x02 = PE32+.
    #[gtest]
    fn pe_magic_resolves_pe32plus() {
        let mut buf = vec![0u8; 200];
        buf[0] = b'M';
        buf[1] = b'Z';
        buf[60] = 64; // e_lfanew = 64 (LE i32)
        buf[64] = b'P';
        buf[65] = b'E';
        buf[66] = 0;
        buf[67] = 0;
        buf[88] = 0x0B;
        buf[89] = 0x02;
        expect_that!(PeMagic::from_file_bytes(&buf), eq(PeMagic::Pe64));
    }

    // HeaderProbe integration tests

    #[gtest]
    fn header_probe_elf64() {
        let mut buf = [0u8; 16];
        buf[0..4].copy_from_slice(b"\x7FELF");
        buf[4] = ElfIdent::CLASS_64;
        expect_that!(HeaderProbe::from_bytes(&buf), eq(HeaderProbe::Elf { is_64: true }));
    }

    #[gtest]
    fn header_probe_elf32() {
        let mut buf = [0u8; 16];
        buf[0..4].copy_from_slice(b"\x7FELF");
        buf[4] = ElfIdent::CLASS_32;
        expect_that!(HeaderProbe::from_bytes(&buf), eq(HeaderProbe::Elf { is_64: false }));
    }

    #[gtest]
    fn header_probe_pe_minimal_mz() {
        // Minimal MZ with e_lfanew=0: pe_offset() returns None, so
        // PeMagic resolves to Unknown, meaning is_64 = false.
        let mut buf = [0u8; 64];
        buf[0] = b'M';
        buf[1] = b'Z';
        let probe = HeaderProbe::from_bytes(&buf);
        expect_that!(probe, eq(HeaderProbe::Pe { is_64: false }));
        expect_that!(probe.is_known(), eq(true));
    }

    #[gtest]
    fn header_probe_unknown_for_empty() {
        expect_that!(HeaderProbe::from_bytes(b""), eq(HeaderProbe::Unknown));
        expect_that!(HeaderProbe::from_bytes(b"").is_known(), eq(false));
    }

    #[gtest]
    fn header_probe_unknown_for_garbage() {
        expect_that!(
            HeaderProbe::from_bytes(b"this is clearly not a binary file at all"),
            eq(HeaderProbe::Unknown)
        );
    }

    #[gtest]
    fn header_probe_is_64_consistency() {
        let mut elf64 = [0u8; 16];
        elf64[0..4].copy_from_slice(b"\x7FELF");
        elf64[4] = 2;
        expect_that!(HeaderProbe::from_bytes(&elf64).is_64(), eq(true));

        let mut elf32 = [0u8; 16];
        elf32[0..4].copy_from_slice(b"\x7FELF");
        elf32[4] = 1;
        expect_that!(HeaderProbe::from_bytes(&elf32).is_64(), eq(false));

        expect_that!(HeaderProbe::from_bytes(b"garbage data here!!").is_64(), eq(false));
    }

    /// Regression guard: ElfIdent must be exactly 16 bytes (ELF spec requirement).
    #[gtest]
    fn elf_ident_size_is_exactly_16_bytes() {
        expect_that!(std::mem::size_of::<ElfIdent>(), eq(16));
    }

    /// Regression guard: DosHeader must be exactly 64 bytes.
    #[gtest]
    fn dos_header_size_is_exactly_64_bytes() {
        expect_that!(std::mem::size_of::<DosHeader>(), eq(64));
    }
}
