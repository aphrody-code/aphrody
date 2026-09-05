// SPDX-License-Identifier: Apache-2.0
//
// Criterion throughput benchmarks for the reverse-engineering triage pillar
// (PLAN.md R5 / R5.x). They quantify the headline metric:
//
//   "aphrody re triage p50 on a 5 MiB PE < 1 s"
//
// which was previously UNMEASURED. The benches exercise the real public API
// of `aphrody-re`:
//
//   1. triage(&[u8])                 — full pipeline: magic detect + goblin PE
//                                       parse + per-section Shannon entropy +
//                                       SHA-256 of the whole input + ASCII /
//                                       UTF-16LE strings sample. Run at 64 KiB,
//                                       1 MiB and 5 MiB to expose the size
//                                       scaling (entropy + hash are linear,
//                                       string scan is the other linear pass).
//   2. extract_strings(&[u8], 6, 64) — isolated strings pass at 1 MiB, so a
//                                       regression in the string scanner can be
//                                       told apart from one in goblin / entropy
//                                       / sha2.
//
// EVERYTHING IS GENERATED IN MEMORY. No real binary is embedded (anonymisation
// + bit-for-bit reproducibility) and nothing touches the filesystem. The PE is
// a hand-built, goblin-parseable PE32+ image whose section bodies are filled
// with deterministic content of three entropy regimes (zeros, ASCII text, and
// a xorshift64 pseudo-random block) so the Shannon-entropy loop and the string
// scanner both run on representative data rather than a flat constant buffer.
//
// `Throughput::Bytes` is set so criterion also reports MiB/s, which is the
// figure that travels into `docs/PERFORMANCE-HISTORY.md`.
//
// Reproduce:
//   cargo bench -p aphrody-re --bench triage

use std::hint::black_box;

use aphrody_re::{Format, extract_strings, triage};
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};

// ---------------------------------------------------------------------------
// Deterministic PRNG — xorshift64*, inline, zero external dependency.
//
// A flat or all-zero buffer would let Shannon entropy short-circuit on a tiny
// histogram and would give the string scanner nothing to chew on. A seeded
// xorshift gives a high-entropy, fully reproducible byte stream so the bench
// measures the worst-case entropy + scan cost without embedding real bytes.
// ---------------------------------------------------------------------------
struct XorShift64(u64);

impl XorShift64 {
    fn new(seed: u64) -> Self {
        // Avoid the all-zero state, which is a fixed point for xorshift.
        Self(seed | 1)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        // The `*` (xorshift64*) scramble improves the low bits' distribution.
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Fill `dst` with pseudo-random bytes.
    fn fill(&mut self, dst: &mut [u8]) {
        let mut chunks = dst.chunks_exact_mut(8);
        for c in &mut chunks {
            c.copy_from_slice(&self.next_u64().to_le_bytes());
        }
        let rem = chunks.into_remainder();
        if !rem.is_empty() {
            let bytes = self.next_u64().to_le_bytes();
            rem.copy_from_slice(&bytes[..rem.len()]);
        }
    }
}

// ---------------------------------------------------------------------------
// Section body filler — three entropy regimes blended across the body so the
// per-section entropy figure is non-trivial and the string scanner finds a
// realistic density of printable runs.
//
// Layout per section body (deterministic, seeded by `seed`):
//   [0, 1/4)        zeros            (entropy ~0, e.g. real .bss-like padding)
//   [1/4, 1/2)      repeated ASCII   (printable runs for the string scanner)
//   [1/2, 1)        xorshift bytes   (entropy ~8, e.g. packed/compressed code)
// ---------------------------------------------------------------------------
fn fill_section_body(buf: &mut [u8], seed: u64) {
    let n = buf.len();
    if n == 0 {
        return;
    }
    let q = n / 4;
    // First quarter: already zero from vec![0u8; ..]; leave it.
    // Second quarter: printable ASCII with embedded NUL-separated tokens so
    // extract_strings yields many >= 6-char runs (both ASCII and, by luck of
    // the alignment, some UTF-16LE-looking pairs).
    let text = b"aphrody_re_bench_string_token_payload_GetProcAddress_LoadLibraryW\0";
    for (i, b) in buf[q..q * 2].iter_mut().enumerate() {
        *b = text[i % text.len()];
    }
    // Second half: high-entropy pseudo-random bytes.
    XorShift64::new(seed).fill(&mut buf[n / 2..]);
}

// ---------------------------------------------------------------------------
// Minimal goblin-parseable PE32+ (PE64) builder.
//
// We assemble, by hand, the smallest image goblin 0.10 will accept and parse
// into sections + entry point, then back it with `section_count` sections whose
// raw bodies are sized to bring the whole image to ~`target_len` bytes. goblin
// reads section data from file offset `PointerToRawData` for `SizeOfRawData`
// bytes, so filling those ranges makes `Section::entropy` run over real data.
//
// Header layout (offsets in the final image):
//   0x00  IMAGE_DOS_HEADER       (64 bytes; e_magic "MZ", e_lfanew -> PE)
//   0x40  PE signature "PE\0\0"  (4 bytes)
//   0x44  IMAGE_FILE_HEADER      (20 bytes; Machine=AMD64, NumberOfSections)
//   0x58  IMAGE_OPTIONAL_HEADER64 (240 bytes; Magic=0x20B, no data dirs used)
//   ...   section table          (40 bytes per section)
//   ...   section raw bodies      (file-aligned)
//
// FileAlignment is 0x200 (the canonical minimum). Section bodies are placed
// back-to-back, each padded up to FileAlignment.
// ---------------------------------------------------------------------------
const FILE_ALIGNMENT: usize = 0x200;
const SECTION_ALIGNMENT: u32 = 0x1000;

fn align_up(v: usize, a: usize) -> usize {
    v.div_ceil(a) * a
}

fn build_pe64(target_len: usize, section_count: usize) -> Vec<u8> {
    assert!(section_count >= 1, "need at least one section");

    let dos_len = 0x40usize; // DOS header, e_lfanew at 0x3C points to 0x40
    let pe_sig_len = 4usize;
    let file_hdr_len = 20usize;
    let opt_hdr_len = 240usize; // sizeof IMAGE_OPTIONAL_HEADER64 (no dirs filled)
    let sec_table_len = section_count * 40;

    let headers_raw = dos_len + pe_sig_len + file_hdr_len + opt_hdr_len + sec_table_len;
    let size_of_headers = align_up(headers_raw, FILE_ALIGNMENT);

    // Distribute the remaining bytes across the sections, each file-aligned.
    let body_budget = target_len.saturating_sub(size_of_headers);
    let per_section_raw = align_up((body_budget / section_count).max(FILE_ALIGNMENT), FILE_ALIGNMENT);

    let total_len = size_of_headers + per_section_raw * section_count;
    let mut img = vec![0u8; total_len];

    // ---- IMAGE_DOS_HEADER ----
    img[0] = b'M';
    img[1] = b'Z';
    // e_lfanew (offset 0x3C) -> start of PE signature (0x40).
    img[0x3C..0x40].copy_from_slice(&(dos_len as u32).to_le_bytes());

    // ---- PE signature "PE\0\0" ----
    let pe_off = dos_len;
    img[pe_off..pe_off + 4].copy_from_slice(b"PE\0\0");

    // ---- IMAGE_FILE_HEADER ----
    let fh = pe_off + 4;
    img[fh..fh + 2].copy_from_slice(&0x8664u16.to_le_bytes()); // Machine = AMD64
    img[fh + 2..fh + 4].copy_from_slice(&(section_count as u16).to_le_bytes()); // NumberOfSections
    // TimeDateStamp (fh+4), PointerToSymbolTable (fh+8), NumberOfSymbols (fh+12)
    // left zero. SizeOfOptionalHeader (fh+16):
    img[fh + 16..fh + 18].copy_from_slice(&(opt_hdr_len as u16).to_le_bytes());
    // Characteristics (fh+18): EXECUTABLE_IMAGE | LARGE_ADDRESS_AWARE.
    img[fh + 18..fh + 20].copy_from_slice(&0x0022u16.to_le_bytes());

    // ---- IMAGE_OPTIONAL_HEADER64 ----
    let oh = fh + file_hdr_len;
    img[oh..oh + 2].copy_from_slice(&0x020Bu16.to_le_bytes()); // Magic = PE32+
    img[oh + 2] = 14; // MajorLinkerVersion
    img[oh + 3] = 0; // MinorLinkerVersion
    // AddressOfEntryPoint (oh+16): point at the first section's RVA.
    let first_rva = SECTION_ALIGNMENT;
    img[oh + 16..oh + 20].copy_from_slice(&first_rva.to_le_bytes());
    // ImageBase (oh+24, u64) for PE32+:
    img[oh + 24..oh + 32].copy_from_slice(&0x0001_4000_0000u64.to_le_bytes());
    // SectionAlignment (oh+32), FileAlignment (oh+36):
    img[oh + 32..oh + 36].copy_from_slice(&SECTION_ALIGNMENT.to_le_bytes());
    img[oh + 36..oh + 40].copy_from_slice(&(FILE_ALIGNMENT as u32).to_le_bytes());
    // MajorOperatingSystemVersion (oh+40)=6, MajorSubsystemVersion (oh+48)=6.
    img[oh + 40..oh + 42].copy_from_slice(&6u16.to_le_bytes());
    img[oh + 48..oh + 50].copy_from_slice(&6u16.to_le_bytes());
    // SizeOfImage (oh+56): headers + section_count virtual slots, section-aligned.
    let size_of_image =
        align_up(size_of_headers, SECTION_ALIGNMENT as usize) as u32
            + section_count as u32 * SECTION_ALIGNMENT;
    img[oh + 56..oh + 60].copy_from_slice(&size_of_image.to_le_bytes());
    // SizeOfHeaders (oh+60):
    img[oh + 60..oh + 64].copy_from_slice(&(size_of_headers as u32).to_le_bytes());
    // Subsystem (oh+68): WINDOWS_CUI (3).
    img[oh + 68..oh + 70].copy_from_slice(&3u16.to_le_bytes());
    // NumberOfRvaAndSizes (oh+108): 16 standard data directories (all zero).
    img[oh + 108..oh + 112].copy_from_slice(&16u32.to_le_bytes());

    // ---- Section table + bodies ----
    let sec_table = oh + opt_hdr_len;
    for i in 0..section_count {
        let ent = sec_table + i * 40;
        // Name (8 bytes): ".text", ".rdata", ".data", then ".s<NN>".
        let name: [u8; 8] = match i {
            0 => *b".text\0\0\0",
            1 => *b".rdata\0\0",
            2 => *b".data\0\0\0",
            _ => {
                let mut n = *b".s00\0\0\0\0";
                n[2] = b'0' + ((i / 10) % 10) as u8;
                n[3] = b'0' + (i % 10) as u8;
                n
            },
        };
        img[ent..ent + 8].copy_from_slice(&name);

        let rva = SECTION_ALIGNMENT * (i as u32 + 1);
        let raw_ptr = (size_of_headers + i * per_section_raw) as u32;
        let raw_size = per_section_raw as u32;

        // VirtualSize (ent+8), VirtualAddress (ent+12):
        img[ent + 8..ent + 12].copy_from_slice(&raw_size.to_le_bytes());
        img[ent + 12..ent + 16].copy_from_slice(&rva.to_le_bytes());
        // SizeOfRawData (ent+16), PointerToRawData (ent+20):
        img[ent + 16..ent + 20].copy_from_slice(&raw_size.to_le_bytes());
        img[ent + 20..ent + 24].copy_from_slice(&raw_ptr.to_le_bytes());
        // Characteristics (ent+36): CODE | EXECUTE | READ for .text, else
        // INITIALIZED_DATA | READ | WRITE.
        let chars: u32 = if i == 0 { 0x6000_0020 } else { 0xC000_0040 };
        img[ent + 36..ent + 40].copy_from_slice(&chars.to_le_bytes());

        // Fill the raw body in place with mixed-entropy content.
        let start = raw_ptr as usize;
        let end = start + raw_size as usize;
        fill_section_body(&mut img[start..end], 0x9E37_79B9_7F4A_7C15 ^ (i as u64));
    }

    img
}

// ---------------------------------------------------------------------------
// Bench 1: full triage() at three input sizes.
// ---------------------------------------------------------------------------
fn bench_triage_sizes(c: &mut Criterion) {
    const KIB: usize = 1024;
    const MIB: usize = 1024 * 1024;
    let sizes: [(&str, usize); 3] = [("64KiB", 64 * KIB), ("1MiB", MIB), ("5MiB", 5 * MIB)];

    let mut group = c.benchmark_group("re_triage");
    // 5 MiB at criterion's default 100 samples is well under a second per
    // iteration on the reference machine, but we trim the sample count so the
    // 5 MiB row stays interactive on slower hosts. The estimate is statistically
    // sound at this size because per-iteration variance is low (pure CPU).
    group.sample_size(30);

    for (label, size) in sizes {
        // Build the synthetic PE once per size (outside the timed loop). Assert
        // the full triage path is taken (goblin parsed it into sections) so a
        // bench regression to the Format::Unknown fallback fails loudly instead
        // of silently measuring only SHA-256 + strings.
        let img = build_pe64(size, 3);
        let probe = triage(&img).expect("triage never errs on a well-formed PE");
        assert_eq!(
            probe.format,
            Format::Pe64,
            "synthetic PE must parse as PE64 (got {:?}); the bench would otherwise \
             measure the Unknown fallback, not the section/entropy hot path",
            probe.format
        );
        assert!(
            !probe.sections.is_empty(),
            "synthetic PE must expose sections for the entropy loop to run"
        );

        group.throughput(Throughput::Bytes(img.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(label), &img, |b, bytes| {
            b.iter(|| {
                let report = triage(black_box(bytes)).expect("triage ok");
                black_box(report)
            });
        });
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// Bench 2: isolated extract_strings() at 1 MiB.
// Separates the string-scan cost from goblin parse + entropy + sha2 so a
// regression can be localised.
// ---------------------------------------------------------------------------
fn bench_extract_strings(c: &mut Criterion) {
    const MIB: usize = 1024 * 1024;
    let img = build_pe64(MIB, 3);

    let mut group = c.benchmark_group("re_extract_strings");
    group.throughput(Throughput::Bytes(img.len() as u64));
    group.bench_function("1MiB_min6_limit64", |b| {
        b.iter(|| {
            let strings = extract_strings(black_box(&img), black_box(6), black_box(64));
            black_box(strings)
        });
    });
    group.finish();
}

criterion_group!(benches, bench_triage_sizes, bench_extract_strings);
criterion_main!(benches);
