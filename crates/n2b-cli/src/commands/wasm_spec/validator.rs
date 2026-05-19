// Copyright 2026 Yohan Pierre
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Validation d'un binaire `.wasm` : détection des propositions WebAssembly
//! utilisées (bulk-memory, SIMD, GC, threads, memory64…).

use anyhow::{Context, Result};
use colored::Colorize;

pub use super::FeaturesOpts;

// ---------------------------------------------------------------------------
// Proposition détectée
// ---------------------------------------------------------------------------

/// Proposition WebAssembly détectée dans un binaire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedProposal {
    pub name: &'static str,
    pub used: bool,
    /// Opcodes ou éléments concrets qui ont déclenché la détection.
    pub evidence: Vec<String>,
}

// ---------------------------------------------------------------------------
// Point d'entrée
// ---------------------------------------------------------------------------

/// Analyse un binaire `.wasm` et retourne les propositions détectées.
pub fn run_features(opts: &FeaturesOpts) -> Result<()> {
    let path = &opts.path;
    if !path.exists() {
        anyhow::bail!("{} introuvable", path.display());
    }

    let bytes = std::fs::read(path).with_context(|| format!("lecture de {}", path.display()))?;

    // Vérification du magic number : 0x00 0x61 0x73 0x6d
    if bytes.len() < 8 || &bytes[0..4] != b"\x00asm" {
        anyhow::bail!(
            "{} n'est pas un binaire WebAssembly valide (magic manquant)",
            path.display()
        );
    }

    let version = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    if !opts.quiet {
        let size_kb = bytes.len() as f64 / 1024.0;
        println!("{} ({:.1} KB, wasm v{})", path.display(), size_kb, version);
    }

    let proposals = analyze_wasm_features(&bytes)?;

    if !opts.quiet {
        for p in &proposals {
            let mark = if p.used { "OK".green() } else { "  ".normal() };
            let name_col = format!("{:<22}", p.name);
            if p.used && !p.evidence.is_empty() {
                let ev = p.evidence.join(", ");
                println!("  {} {}  ({})", mark, name_col, ev.dimmed());
            } else {
                println!("  {} {}", mark, name_col);
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Analyse des sections wasm
// ---------------------------------------------------------------------------

/// Parse les sections wasm et détecte les propositions utilisées.
pub(super) fn analyze_wasm_features(bytes: &[u8]) -> Result<Vec<DetectedProposal>> {
    let mut proposals: Vec<DetectedProposal> = vec![
        DetectedProposal {
            name: "MVP (core)",
            used: true,
            evidence: vec![],
        },
        DetectedProposal {
            name: "bulk-memory",
            used: false,
            evidence: vec![],
        },
        DetectedProposal {
            name: "reference-types",
            used: false,
            evidence: vec![],
        },
        DetectedProposal {
            name: "tail-calls",
            used: false,
            evidence: vec![],
        },
        DetectedProposal {
            name: "exception-handling",
            used: false,
            evidence: vec![],
        },
        DetectedProposal {
            name: "SIMD (v128)",
            used: false,
            evidence: vec![],
        },
        DetectedProposal {
            name: "relaxed-SIMD",
            used: false,
            evidence: vec![],
        },
        DetectedProposal {
            name: "GC (structs/arrays)",
            used: false,
            evidence: vec![],
        },
        DetectedProposal {
            name: "multi-memory",
            used: false,
            evidence: vec![],
        },
        DetectedProposal {
            name: "memory64",
            used: false,
            evidence: vec![],
        },
        DetectedProposal {
            name: "multi-table",
            used: false,
            evidence: vec![],
        },
        DetectedProposal {
            name: "threads",
            used: false,
            evidence: vec![],
        },
        DetectedProposal {
            name: "custom-sections",
            used: false,
            evidence: vec![],
        },
    ];

    let mut cursor = 8usize; // saute le magic (4 bytes) + version (4 bytes)
    let len = bytes.len();

    // Compteurs pour détecter multi-memory / multi-table
    let mut memory_count = 0u32;
    let mut table_count = 0u32;

    while cursor < len {
        if cursor + 1 > len {
            break;
        }
        let section_id = bytes[cursor];
        cursor += 1;

        // Taille de la section (LEB128 u32)
        let (section_size, consumed) = read_leb128_u32(bytes, cursor)?;
        cursor += consumed;
        let section_start = cursor;
        let section_end = section_start + section_size as usize;
        if section_end > len {
            break;
        }

        match section_id {
            // Section 0 : custom
            0 => {
                // Lit le nom (LEB128 u32 + bytes)
                if let Ok((name_len, nc)) = read_leb128_u32(bytes, cursor) {
                    let name_start = cursor + nc;
                    let name_end = name_start + name_len as usize;
                    if name_end <= section_end {
                        let name = std::str::from_utf8(&bytes[name_start..name_end])
                            .unwrap_or("<invalid>");
                        mark_proposal(&mut proposals, "custom-sections", name.to_string());
                        // La section `target_features` indique explicitement les features
                        if name == "target_features" {
                            parse_target_features(bytes, name_end, section_end, &mut proposals);
                        }
                    }
                }
            }

            // Section 2 : imports — peut révéler threads (shared memory)
            2 => {
                scan_code_section(
                    bytes,
                    section_start,
                    section_end,
                    &mut proposals,
                    &mut memory_count,
                    &mut table_count,
                    true,
                );
            }

            // Section 3 : type — révèle GC (types récursifs, struct, array)
            1 => {
                detect_gc_types(bytes, section_start, section_end, &mut proposals);
            }

            // Section 4 : table — compte les tables
            4 => {
                if let Ok((count, _)) = read_leb128_u32(bytes, cursor) {
                    table_count += count;
                }
            }

            // Section 5 : memory — compte les mémoires, détecte memory64
            5 => {
                parse_memory_section(
                    bytes,
                    section_start,
                    section_end,
                    &mut proposals,
                    &mut memory_count,
                );
            }

            // Section 10 : code — scanne les opcodes
            10 => {
                scan_code_section(
                    bytes,
                    section_start,
                    section_end,
                    &mut proposals,
                    &mut memory_count,
                    &mut table_count,
                    false,
                );
            }

            _ => {}
        }

        cursor = section_end;
    }

    // Multi-memory / multi-table détecté après le parcours complet
    if memory_count > 1 {
        mark_proposal(
            &mut proposals,
            "multi-memory",
            format!("{memory_count} mémoires"),
        );
    }
    if table_count > 1 {
        mark_proposal(
            &mut proposals,
            "multi-table",
            format!("{table_count} tables"),
        );
    }

    Ok(proposals)
}

/// Marque une proposition comme utilisée et ajoute l'évidence.
pub(super) fn mark_proposal(proposals: &mut [DetectedProposal], name: &str, evidence: String) {
    if let Some(p) = proposals.iter_mut().find(|p| p.name.starts_with(name)) {
        p.used = true;
        if !evidence.is_empty() && !p.evidence.contains(&evidence) {
            p.evidence.push(evidence);
        }
    }
}

/// Scanne la section code pour les opcodes à propositions.
fn scan_code_section(
    bytes: &[u8],
    start: usize,
    end: usize,
    proposals: &mut [DetectedProposal],
    _memory_count: &mut u32,
    _table_count: &mut u32,
    _is_import: bool,
) {
    let mut i = start;
    while i < end {
        if i >= bytes.len() {
            break;
        }
        let op = bytes[i];
        i += 1;

        match op {
            // Tail calls (0x12 return_call, 0x13 return_call_indirect, 0x15 return_call_ref)
            0x12 => {
                mark_proposal(proposals, "tail-calls", "return_call".into());
                skip_leb(bytes, &mut i);
            }
            0x13 => {
                mark_proposal(proposals, "tail-calls", "return_call_indirect".into());
                skip_leb(bytes, &mut i);
                skip_leb(bytes, &mut i);
            }
            0x15 => {
                mark_proposal(proposals, "tail-calls", "return_call_ref".into());
                skip_leb(bytes, &mut i);
            }

            // Reference types (0x25 table.get, 0x26 table.set, 0xd0 ref.null, 0xd1 ref.is_null, 0xd2 ref.func)
            0x25 => {
                mark_proposal(proposals, "reference-types", "table.get".into());
                skip_leb(bytes, &mut i);
            }
            0x26 => {
                mark_proposal(proposals, "reference-types", "table.set".into());
                skip_leb(bytes, &mut i);
            }
            0xd0 => {
                mark_proposal(proposals, "reference-types", "ref.null".into());
                i += 1;
            }
            0xd1 => {
                mark_proposal(proposals, "reference-types", "ref.is_null".into());
            }
            0xd2 => {
                mark_proposal(proposals, "reference-types", "ref.func".into());
                skip_leb(bytes, &mut i);
            }
            0xd3 => {
                mark_proposal(proposals, "GC (structs/arrays)", "ref.eq".into());
            }
            0xd4 => {
                mark_proposal(proposals, "GC (structs/arrays)", "ref.as_non_null".into());
            }

            // Exception handling (0x06 try, 0x07 catch, 0x08 throw, 0x0a throw_ref, 0x19 try_table)
            0x06 => {
                mark_proposal(proposals, "exception-handling", "try".into());
            }
            0x07 => {
                mark_proposal(proposals, "exception-handling", "catch".into());
            }
            0x08 => {
                mark_proposal(proposals, "exception-handling", "throw".into());
                skip_leb(bytes, &mut i);
            }
            0x0a => {
                mark_proposal(proposals, "exception-handling", "throw_ref".into());
            }
            0x19 => {
                mark_proposal(proposals, "exception-handling", "try_table".into());
            }

            // call_ref (0x14) — reference types + GC
            0x14 => {
                mark_proposal(proposals, "reference-types", "call_ref".into());
                skip_leb(bytes, &mut i);
            }

            // Prefixed 0xFC — bulk-memory + misc
            0xfc => {
                if let Ok((subop, nc)) = read_leb128_u32(bytes, i) {
                    i += nc;
                    match subop {
                        0x00..=0x07 => {} // trunc_sat — no proposal
                        0x08 => {
                            mark_proposal(proposals, "bulk-memory", "memory.init".into());
                            skip_leb(bytes, &mut i);
                            skip_leb(bytes, &mut i);
                        }
                        0x09 => {
                            mark_proposal(proposals, "bulk-memory", "data.drop".into());
                            skip_leb(bytes, &mut i);
                        }
                        0x0a => {
                            mark_proposal(proposals, "bulk-memory", "memory.copy".into());
                            skip_leb(bytes, &mut i);
                            skip_leb(bytes, &mut i);
                        }
                        0x0b => {
                            mark_proposal(proposals, "bulk-memory", "memory.fill".into());
                            skip_leb(bytes, &mut i);
                        }
                        0x0c => {
                            mark_proposal(proposals, "bulk-memory", "table.init".into());
                            skip_leb(bytes, &mut i);
                            skip_leb(bytes, &mut i);
                        }
                        0x0d => {
                            mark_proposal(proposals, "bulk-memory", "elem.drop".into());
                            skip_leb(bytes, &mut i);
                        }
                        0x0e => {
                            mark_proposal(proposals, "bulk-memory", "table.copy".into());
                            skip_leb(bytes, &mut i);
                            skip_leb(bytes, &mut i);
                        }
                        0x0f..=0x11 => {
                            skip_leb(bytes, &mut i);
                        } // table.grow/size/fill
                        _ => {}
                    }
                }
            }

            // Prefixed 0xFD — SIMD v128
            0xfd => {
                if let Ok((subop, nc)) = read_leb128_u32(bytes, i) {
                    i += nc;
                    let mnemonic = simd_mnemonic(subop);
                    mark_proposal(proposals, "SIMD (v128)", mnemonic.to_string());
                    // Relaxed SIMD : subop >= 0x100
                    if subop >= 0x100 {
                        mark_proposal(proposals, "relaxed-SIMD", mnemonic.to_string());
                    }
                    // Saute les immediats des opcodes SIMD (approximatif)
                    // v128.load/store ont 2 LEB, v128.const a 16 bytes, shuffle a 16 bytes
                    match subop {
                        0x00..=0x0b => {
                            skip_leb(bytes, &mut i);
                            skip_leb(bytes, &mut i);
                        } // memop
                        0x0c => {
                            i += 16;
                        } // v128.const
                        0x0d => {
                            i += 16;
                        } // i8x16.shuffle lane indices
                        0x15..=0x22 => {
                            i += 1;
                        } // lane index
                        _ => {}
                    }
                }
            }

            // Prefixed 0xFE — threads/atomics
            0xfe => {
                if let Ok((subop, nc)) = read_leb128_u32(bytes, i) {
                    i += nc;
                    let mnemonic = atomic_mnemonic(subop);
                    mark_proposal(proposals, "threads", mnemonic.to_string());
                    skip_leb(bytes, &mut i); // alignment
                    skip_leb(bytes, &mut i); // offset
                }
            }

            // GC opcodes (0xfb prefix)
            0xfb => {
                if let Ok((subop, nc)) = read_leb128_u32(bytes, i) {
                    i += nc;
                    let mnemonic = gc_mnemonic(subop);
                    mark_proposal(proposals, "GC (structs/arrays)", mnemonic.to_string());
                    // Les immediats GC varient — on skip 1 ou 2 LEB pour les plus courants
                    match subop {
                        0x00..=0x03 => {
                            skip_leb(bytes, &mut i);
                        } // struct.new*
                        0x07..=0x0a => {
                            skip_leb(bytes, &mut i);
                        } // array.new*
                        0x0b => {
                            skip_leb(bytes, &mut i);
                            skip_leb(bytes, &mut i);
                        } // array.new_fixed
                        0x15..=0x1a => {
                            skip_leb(bytes, &mut i);
                            skip_leb(bytes, &mut i);
                        } // struct.get/set
                        0x1b..=0x1d => {
                            skip_leb(bytes, &mut i);
                            skip_leb(bytes, &mut i);
                        } // array.get/set
                        0x1e..=0x22 => {
                            skip_leb(bytes, &mut i);
                        } // array.len etc.
                        _ => {}
                    }
                }
            }

            // Instructions normales — LEB immediats à sauter (approximatif)
            // 0x0c (br), 0x0d (br_if), 0x10 (call), 0x20..=0x26 (local/global/table), 0x3f (memory.size), 0x40 (memory.grow)
            0x0c | 0x0d | 0x10 | 0x20..=0x26 | 0x3f | 0x40 => {
                skip_leb(bytes, &mut i);
            }
            // 0x11 (call_indirect) — 2 immediats
            0x11 => {
                skip_leb(bytes, &mut i);
                skip_leb(bytes, &mut i);
            }
            0x28..=0x3e => {
                skip_leb(bytes, &mut i);
                skip_leb(bytes, &mut i);
            } // memop
            0x41 | 0x42 => {
                skip_leb(bytes, &mut i);
            } // i32/i64.const
            0x43 => {
                i += 4;
            } // f32.const
            0x44 => {
                i += 8;
            } // f64.const
            _ => {}
        }
    }
}

/// Détecte les types GC dans la section de types.
fn detect_gc_types(bytes: &[u8], start: usize, end: usize, proposals: &mut [DetectedProposal]) {
    let mut i = start;
    while i < end {
        if i >= bytes.len() {
            break;
        }
        let b = bytes[i] as i8;
        // Les types GC utilisent des opcodes négatifs : -0x30 (rec), -0x31 (sub), -0x32 (sub final)
        // En LEB128 non-signé vu comme byte : 0x50 = rec, 0x4f = sub, 0x4e = sub final
        if b == -0x30_i8 || b == -0x31_i8 || b == -0x32_i8 {
            mark_proposal(proposals, "GC (structs/arrays)", "rec/sub type".into());
        }
        i += 1;
    }
}

/// Parse la section memory et détecte memory64.
fn parse_memory_section(
    bytes: &[u8],
    start: usize,
    end: usize,
    proposals: &mut [DetectedProposal],
    memory_count: &mut u32,
) {
    let mut i = start;
    if let Ok((count, nc)) = read_leb128_u32(bytes, i) {
        i += nc;
        *memory_count += count;
        for _ in 0..count {
            if i >= end || i >= bytes.len() {
                break;
            }
            let flags = bytes[i];
            i += 1;
            // bit 2 (0x04) = memory64
            if flags & 0x04 != 0 {
                mark_proposal(proposals, "memory64", "i64 limits".into());
            }
            skip_leb(bytes, &mut i);
            if flags & 0x01 != 0 {
                skip_leb(bytes, &mut i);
            } // max
        }
    }
}

/// Parse la section `target_features` pour des features explicites.
fn parse_target_features(
    bytes: &[u8],
    start: usize,
    end: usize,
    proposals: &mut [DetectedProposal],
) {
    // Format : u32 (count) + (u8 prefix + u32 len + bytes name)*
    let mut i = start;
    if let Ok((count, nc)) = read_leb128_u32(bytes, i) {
        i += nc;
        for _ in 0..count {
            if i >= end {
                break;
            }
            i += 1; // préfixe (+/-/=)
            if let Ok((name_len, nc)) = read_leb128_u32(bytes, i) {
                i += nc;
                let name_end = i + name_len as usize;
                if name_end <= end && name_end <= bytes.len() {
                    let feat = std::str::from_utf8(&bytes[i..name_end]).unwrap_or("");
                    match feat {
                        "bulk-memory" => {
                            mark_proposal(proposals, "bulk-memory", "target_features".into())
                        }
                        "simd128" => {
                            mark_proposal(proposals, "SIMD (v128)", "target_features".into())
                        }
                        "atomics" => mark_proposal(proposals, "threads", "target_features".into()),
                        "exception-handling" => {
                            mark_proposal(proposals, "exception-handling", "target_features".into())
                        }
                        "reference-types" => {
                            mark_proposal(proposals, "reference-types", "target_features".into())
                        }
                        "tail-call" => {
                            mark_proposal(proposals, "tail-calls", "target_features".into())
                        }
                        "gc" => mark_proposal(
                            proposals,
                            "GC (structs/arrays)",
                            "target_features".into(),
                        ),
                        "memory64" => {
                            mark_proposal(proposals, "memory64", "target_features".into())
                        }
                        "multi-memory" => {
                            mark_proposal(proposals, "multi-memory", "target_features".into())
                        }
                        _ => {}
                    }
                }
                i = name_end;
            } else {
                break;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// LEB128 + skip utilities
// ---------------------------------------------------------------------------

/// Saute un entier LEB128 sans le décoder (avance `i`).
pub(super) fn skip_leb(bytes: &[u8], i: &mut usize) {
    while *i < bytes.len() {
        let b = bytes[*i];
        *i += 1;
        if b & 0x80 == 0 {
            break;
        }
    }
}

/// Lit un entier u32 encodé en LEB128 non-signé.
/// Retourne `(valeur, octets_consommés)`.
pub(super) fn read_leb128_u32(bytes: &[u8], mut pos: usize) -> Result<(u32, usize)> {
    let start = pos;
    let mut result = 0u32;
    let mut shift = 0u32;
    loop {
        if pos >= bytes.len() {
            anyhow::bail!("LEB128 tronqué à l'offset {pos}");
        }
        let b = bytes[pos] as u32;
        pos += 1;
        result |= (b & 0x7f) << shift;
        if b & 0x80 == 0 {
            break;
        }
        shift += 7;
        if shift >= 35 {
            anyhow::bail!("LEB128 u32 trop long");
        }
    }
    Ok((result, pos - start))
}

// ---------------------------------------------------------------------------
// Mnémoniques
// ---------------------------------------------------------------------------

/// Nom court d'un opcode SIMD (0xFD prefix).
fn simd_mnemonic(subop: u32) -> &'static str {
    match subop {
        0x00..=0x0a => "v128.load*",
        0x0b => "v128.store",
        0x0c => "v128.const",
        0x0d => "i8x16.shuffle",
        0x0e => "i8x16.swizzle",
        0x0f => "i8x16.splat",
        0x10 => "i16x8.splat",
        0x11 => "i32x4.splat",
        0x12 => "i64x2.splat",
        0x13 => "f32x4.splat",
        0x14 => "f64x2.splat",
        0x23..=0x2a => "i8x16.*",
        0x2b..=0x32 => "i16x8.*",
        0x33..=0x3a => "i32x4.*",
        0x3b..=0x42 => "i64x2.*",
        0x43..=0x52 => "f32x4.*",
        0x53..=0x5f => "f64x2.*",
        0x60..=0x7f => "i8x16.*",
        0x80..=0xbf => "i16x8.*",
        0xc0..=0xdf => "i64x2.*",
        0xe0..=0xff => "f32x4/f64x2.*",
        0x100..=0x113 => "relaxed-simd.*",
        _ => "simd.*",
    }
}

/// Nom court d'un opcode atomique (0xFE prefix).
fn atomic_mnemonic(subop: u32) -> &'static str {
    match subop {
        0x00 => "memory.atomic.notify",
        0x01 => "memory.atomic.wait32",
        0x02 => "memory.atomic.wait64",
        0x10..=0x1f => "i32.atomic.*",
        0x20..=0x2f => "i64.atomic.*",
        0x30..=0x3f => "i32.atomic.rmw8.*",
        0x40..=0x4f => "i32.atomic.rmw16.*",
        0x50..=0x5f => "i64.atomic.rmw8.*",
        0x60..=0x6f => "i64.atomic.rmw16.*",
        0x70..=0x7f => "i64.atomic.rmw32.*",
        0xfe => "atomic.fence",
        _ => "atomic.*",
    }
}

/// Nom court d'un opcode GC (0xFB prefix).
fn gc_mnemonic(subop: u32) -> &'static str {
    match subop {
        0x00 => "struct.new",
        0x01 => "struct.new_default",
        0x02 => "struct.get",
        0x03 => "struct.get_s",
        0x04 => "struct.get_u",
        0x05 => "struct.set",
        0x07 => "array.new",
        0x08 => "array.new_default",
        0x09 => "array.new_fixed",
        0x0a => "array.new_data",
        0x0b => "array.new_elem",
        0x0c => "array.get",
        0x0d => "array.get_s",
        0x0e => "array.get_u",
        0x0f => "array.set",
        0x10 => "array.len",
        0x11 => "array.fill",
        0x12 => "array.copy",
        0x14 => "ref.test",
        0x15 => "ref.cast",
        0x16 => "br_on_cast",
        0x17 => "br_on_cast_fail",
        0x1c => "ref.i31",
        0x1d => "i31.get_s",
        0x1e => "i31.get_u",
        0x23 => "extern.internalize",
        0x24 => "extern.externalize",
        _ => "gc.*",
    }
}
