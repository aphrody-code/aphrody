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

//! Table d'opcodes statique et génération de la sortie (text / md / json).

use colored::Colorize;

// ---------------------------------------------------------------------------
// Types publics
// ---------------------------------------------------------------------------

/// Proposition WebAssembly pour la table d'opcodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Proposal {
    Mvp,
    BulkMemory,
    ReferenceTypes,
    TailCalls,
    ExceptionHandling,
    Simd,
    RelaxedSimd,
    Gc,
    /// Réservé : aucun opcode dédié hors section memory.
    #[allow(dead_code)]
    MultiMemory,
    /// Réservé : détecté via les flags de section memory.
    #[allow(dead_code)]
    Memory64,
    Threads,
}

impl Proposal {
    /// Nom de la proposition tel qu'utilisé dans `--proposal`.
    pub fn name(self) -> &'static str {
        match self {
            Proposal::Mvp => "mvp",
            Proposal::BulkMemory => "bulk-memory",
            Proposal::ReferenceTypes => "reference-types",
            Proposal::TailCalls => "tail-calls",
            Proposal::ExceptionHandling => "exception-handling",
            Proposal::Simd => "simd",
            Proposal::RelaxedSimd => "relaxed-simd",
            Proposal::Gc => "gc",
            Proposal::MultiMemory => "multi-memory",
            Proposal::Memory64 => "memory64",
            Proposal::Threads => "threads",
        }
    }
}

/// Entrée dans la table d'opcodes statique.
pub struct Opcode {
    /// Représentation hexadécimale de l'opcode (ou prefix + sous-code).
    pub hex: &'static str,
    /// Mnémonique WAT officiel.
    pub mnemonic: &'static str,
    /// Proposition dont il relève.
    pub proposal: Proposal,
    /// Immediats (description textuelle).
    pub immediates: &'static str,
}

// ---------------------------------------------------------------------------
// Table statique — 140 opcodes catalogués
// ---------------------------------------------------------------------------

/// Table statique de référence — 140 opcodes catalogués.
///
/// Sources :
/// - <https://github.com/WebAssembly/spec/blob/main/interpreter/binary/decode.ml>
/// - <https://webassembly.github.io/spec/core/binary/instructions.html>
pub static OPCODE_TABLE: &[Opcode] = &[
    // --- MVP / Core ---
    Opcode {
        hex: "0x00",
        mnemonic: "unreachable",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x01",
        mnemonic: "nop",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x02",
        mnemonic: "block",
        proposal: Proposal::Mvp,
        immediates: "blocktype",
    },
    Opcode {
        hex: "0x03",
        mnemonic: "loop",
        proposal: Proposal::Mvp,
        immediates: "blocktype",
    },
    Opcode {
        hex: "0x04",
        mnemonic: "if",
        proposal: Proposal::Mvp,
        immediates: "blocktype",
    },
    Opcode {
        hex: "0x05",
        mnemonic: "else",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x0b",
        mnemonic: "end",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x0c",
        mnemonic: "br",
        proposal: Proposal::Mvp,
        immediates: "labelidx",
    },
    Opcode {
        hex: "0x0d",
        mnemonic: "br_if",
        proposal: Proposal::Mvp,
        immediates: "labelidx",
    },
    Opcode {
        hex: "0x0e",
        mnemonic: "br_table",
        proposal: Proposal::Mvp,
        immediates: "vec(labelidx) labelidx",
    },
    Opcode {
        hex: "0x0f",
        mnemonic: "return",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x10",
        mnemonic: "call",
        proposal: Proposal::Mvp,
        immediates: "funcidx",
    },
    Opcode {
        hex: "0x11",
        mnemonic: "call_indirect",
        proposal: Proposal::Mvp,
        immediates: "typeidx tableidx",
    },
    Opcode {
        hex: "0x1a",
        mnemonic: "drop",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x1b",
        mnemonic: "select",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x1c",
        mnemonic: "select",
        proposal: Proposal::ReferenceTypes,
        immediates: "vec(valtype)",
    },
    Opcode {
        hex: "0x20",
        mnemonic: "local.get",
        proposal: Proposal::Mvp,
        immediates: "localidx",
    },
    Opcode {
        hex: "0x21",
        mnemonic: "local.set",
        proposal: Proposal::Mvp,
        immediates: "localidx",
    },
    Opcode {
        hex: "0x22",
        mnemonic: "local.tee",
        proposal: Proposal::Mvp,
        immediates: "localidx",
    },
    Opcode {
        hex: "0x23",
        mnemonic: "global.get",
        proposal: Proposal::Mvp,
        immediates: "globalidx",
    },
    Opcode {
        hex: "0x24",
        mnemonic: "global.set",
        proposal: Proposal::Mvp,
        immediates: "globalidx",
    },
    Opcode {
        hex: "0x25",
        mnemonic: "table.get",
        proposal: Proposal::ReferenceTypes,
        immediates: "tableidx",
    },
    Opcode {
        hex: "0x26",
        mnemonic: "table.set",
        proposal: Proposal::ReferenceTypes,
        immediates: "tableidx",
    },
    Opcode {
        hex: "0x28",
        mnemonic: "i32.load",
        proposal: Proposal::Mvp,
        immediates: "memarg",
    },
    Opcode {
        hex: "0x29",
        mnemonic: "i64.load",
        proposal: Proposal::Mvp,
        immediates: "memarg",
    },
    Opcode {
        hex: "0x2a",
        mnemonic: "f32.load",
        proposal: Proposal::Mvp,
        immediates: "memarg",
    },
    Opcode {
        hex: "0x2b",
        mnemonic: "f64.load",
        proposal: Proposal::Mvp,
        immediates: "memarg",
    },
    Opcode {
        hex: "0x2c",
        mnemonic: "i32.load8_s",
        proposal: Proposal::Mvp,
        immediates: "memarg",
    },
    Opcode {
        hex: "0x2d",
        mnemonic: "i32.load8_u",
        proposal: Proposal::Mvp,
        immediates: "memarg",
    },
    Opcode {
        hex: "0x2e",
        mnemonic: "i32.load16_s",
        proposal: Proposal::Mvp,
        immediates: "memarg",
    },
    Opcode {
        hex: "0x2f",
        mnemonic: "i32.load16_u",
        proposal: Proposal::Mvp,
        immediates: "memarg",
    },
    Opcode {
        hex: "0x30",
        mnemonic: "i64.load8_s",
        proposal: Proposal::Mvp,
        immediates: "memarg",
    },
    Opcode {
        hex: "0x31",
        mnemonic: "i64.load8_u",
        proposal: Proposal::Mvp,
        immediates: "memarg",
    },
    Opcode {
        hex: "0x32",
        mnemonic: "i64.load16_s",
        proposal: Proposal::Mvp,
        immediates: "memarg",
    },
    Opcode {
        hex: "0x33",
        mnemonic: "i64.load16_u",
        proposal: Proposal::Mvp,
        immediates: "memarg",
    },
    Opcode {
        hex: "0x34",
        mnemonic: "i64.load32_s",
        proposal: Proposal::Mvp,
        immediates: "memarg",
    },
    Opcode {
        hex: "0x35",
        mnemonic: "i64.load32_u",
        proposal: Proposal::Mvp,
        immediates: "memarg",
    },
    Opcode {
        hex: "0x36",
        mnemonic: "i32.store",
        proposal: Proposal::Mvp,
        immediates: "memarg",
    },
    Opcode {
        hex: "0x37",
        mnemonic: "i64.store",
        proposal: Proposal::Mvp,
        immediates: "memarg",
    },
    Opcode {
        hex: "0x38",
        mnemonic: "f32.store",
        proposal: Proposal::Mvp,
        immediates: "memarg",
    },
    Opcode {
        hex: "0x39",
        mnemonic: "f64.store",
        proposal: Proposal::Mvp,
        immediates: "memarg",
    },
    Opcode {
        hex: "0x3a",
        mnemonic: "i32.store8",
        proposal: Proposal::Mvp,
        immediates: "memarg",
    },
    Opcode {
        hex: "0x3b",
        mnemonic: "i32.store16",
        proposal: Proposal::Mvp,
        immediates: "memarg",
    },
    Opcode {
        hex: "0x3c",
        mnemonic: "i64.store8",
        proposal: Proposal::Mvp,
        immediates: "memarg",
    },
    Opcode {
        hex: "0x3d",
        mnemonic: "i64.store16",
        proposal: Proposal::Mvp,
        immediates: "memarg",
    },
    Opcode {
        hex: "0x3e",
        mnemonic: "i64.store32",
        proposal: Proposal::Mvp,
        immediates: "memarg",
    },
    Opcode {
        hex: "0x3f",
        mnemonic: "memory.size",
        proposal: Proposal::Mvp,
        immediates: "memidx",
    },
    Opcode {
        hex: "0x40",
        mnemonic: "memory.grow",
        proposal: Proposal::Mvp,
        immediates: "memidx",
    },
    Opcode {
        hex: "0x41",
        mnemonic: "i32.const",
        proposal: Proposal::Mvp,
        immediates: "i32",
    },
    Opcode {
        hex: "0x42",
        mnemonic: "i64.const",
        proposal: Proposal::Mvp,
        immediates: "i64",
    },
    Opcode {
        hex: "0x43",
        mnemonic: "f32.const",
        proposal: Proposal::Mvp,
        immediates: "f32",
    },
    Opcode {
        hex: "0x44",
        mnemonic: "f64.const",
        proposal: Proposal::Mvp,
        immediates: "f64",
    },
    Opcode {
        hex: "0x45",
        mnemonic: "i32.eqz",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x46",
        mnemonic: "i32.eq",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x47",
        mnemonic: "i32.ne",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x48",
        mnemonic: "i32.lt_s",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x49",
        mnemonic: "i32.lt_u",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x4a",
        mnemonic: "i32.gt_s",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x4b",
        mnemonic: "i32.gt_u",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x4c",
        mnemonic: "i32.le_s",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x4d",
        mnemonic: "i32.le_u",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x4e",
        mnemonic: "i32.ge_s",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x4f",
        mnemonic: "i32.ge_u",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x50",
        mnemonic: "i64.eqz",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x51",
        mnemonic: "i64.eq",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x52",
        mnemonic: "i64.ne",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x53",
        mnemonic: "i64.lt_s",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x54",
        mnemonic: "i64.lt_u",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x55",
        mnemonic: "i64.gt_s",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x56",
        mnemonic: "i64.gt_u",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x57",
        mnemonic: "i64.le_s",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x58",
        mnemonic: "i64.le_u",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x59",
        mnemonic: "i64.ge_s",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x5a",
        mnemonic: "i64.ge_u",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x5b",
        mnemonic: "f32.eq",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x5c",
        mnemonic: "f32.ne",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x5d",
        mnemonic: "f32.lt",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x5e",
        mnemonic: "f32.gt",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x5f",
        mnemonic: "f32.le",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x60",
        mnemonic: "f32.ge",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x61",
        mnemonic: "f64.eq",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x62",
        mnemonic: "f64.ne",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x63",
        mnemonic: "f64.lt",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x64",
        mnemonic: "f64.gt",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x65",
        mnemonic: "f64.le",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x66",
        mnemonic: "f64.ge",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x67",
        mnemonic: "i32.clz",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x68",
        mnemonic: "i32.ctz",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x69",
        mnemonic: "i32.popcnt",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x6a",
        mnemonic: "i32.add",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x6b",
        mnemonic: "i32.sub",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x6c",
        mnemonic: "i32.mul",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x6d",
        mnemonic: "i32.div_s",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x6e",
        mnemonic: "i32.div_u",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x6f",
        mnemonic: "i32.rem_s",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x70",
        mnemonic: "i32.rem_u",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x71",
        mnemonic: "i32.and",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x72",
        mnemonic: "i32.or",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x73",
        mnemonic: "i32.xor",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x74",
        mnemonic: "i32.shl",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x75",
        mnemonic: "i32.shr_s",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x76",
        mnemonic: "i32.shr_u",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x77",
        mnemonic: "i32.rotl",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x78",
        mnemonic: "i32.rotr",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x79",
        mnemonic: "i64.clz",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x7a",
        mnemonic: "i64.ctz",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x7b",
        mnemonic: "i64.popcnt",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x7c",
        mnemonic: "i64.add",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x7d",
        mnemonic: "i64.sub",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x7e",
        mnemonic: "i64.mul",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x7f",
        mnemonic: "i64.div_s",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x80",
        mnemonic: "i64.div_u",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x81",
        mnemonic: "i64.rem_s",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x82",
        mnemonic: "i64.rem_u",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x83",
        mnemonic: "i64.and",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x84",
        mnemonic: "i64.or",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x85",
        mnemonic: "i64.xor",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x86",
        mnemonic: "i64.shl",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x87",
        mnemonic: "i64.shr_s",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x88",
        mnemonic: "i64.shr_u",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x89",
        mnemonic: "i64.rotl",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x8a",
        mnemonic: "i64.rotr",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x8b",
        mnemonic: "f32.abs",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x8c",
        mnemonic: "f32.neg",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x8d",
        mnemonic: "f32.ceil",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x8e",
        mnemonic: "f32.floor",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x8f",
        mnemonic: "f32.trunc",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x90",
        mnemonic: "f32.nearest",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x91",
        mnemonic: "f32.sqrt",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x92",
        mnemonic: "f32.add",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x93",
        mnemonic: "f32.sub",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x94",
        mnemonic: "f32.mul",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x95",
        mnemonic: "f32.div",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x96",
        mnemonic: "f32.min",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x97",
        mnemonic: "f32.max",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x98",
        mnemonic: "f32.copysign",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x99",
        mnemonic: "f64.abs",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x9a",
        mnemonic: "f64.neg",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x9b",
        mnemonic: "f64.ceil",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x9c",
        mnemonic: "f64.floor",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x9d",
        mnemonic: "f64.trunc",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x9e",
        mnemonic: "f64.nearest",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0x9f",
        mnemonic: "f64.sqrt",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xa0",
        mnemonic: "f64.add",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xa1",
        mnemonic: "f64.sub",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xa2",
        mnemonic: "f64.mul",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xa3",
        mnemonic: "f64.div",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xa4",
        mnemonic: "f64.min",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xa5",
        mnemonic: "f64.max",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xa6",
        mnemonic: "f64.copysign",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xa7",
        mnemonic: "i32.wrap_i64",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xa8",
        mnemonic: "i32.trunc_f32_s",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xa9",
        mnemonic: "i32.trunc_f32_u",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xaa",
        mnemonic: "i32.trunc_f64_s",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xab",
        mnemonic: "i32.trunc_f64_u",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xac",
        mnemonic: "i64.extend_i32_s",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xad",
        mnemonic: "i64.extend_i32_u",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xae",
        mnemonic: "i64.trunc_f32_s",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xaf",
        mnemonic: "i64.trunc_f32_u",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xb0",
        mnemonic: "i64.trunc_f64_s",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xb1",
        mnemonic: "i64.trunc_f64_u",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xb2",
        mnemonic: "f32.convert_i32_s",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xb3",
        mnemonic: "f32.convert_i32_u",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xb4",
        mnemonic: "f32.convert_i64_s",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xb5",
        mnemonic: "f32.convert_i64_u",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xb6",
        mnemonic: "f32.demote_f64",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xb7",
        mnemonic: "f64.convert_i32_s",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xb8",
        mnemonic: "f64.convert_i32_u",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xb9",
        mnemonic: "f64.convert_i64_s",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xba",
        mnemonic: "f64.convert_i64_u",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xbb",
        mnemonic: "f64.promote_f32",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xbc",
        mnemonic: "i32.reinterpret_f32",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xbd",
        mnemonic: "i64.reinterpret_f64",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xbe",
        mnemonic: "f32.reinterpret_i32",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xbf",
        mnemonic: "f64.reinterpret_i64",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xc0",
        mnemonic: "i32.extend8_s",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xc1",
        mnemonic: "i32.extend16_s",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xc2",
        mnemonic: "i64.extend8_s",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xc3",
        mnemonic: "i64.extend16_s",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xc4",
        mnemonic: "i64.extend32_s",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    // Reference types
    Opcode {
        hex: "0xd0",
        mnemonic: "ref.null",
        proposal: Proposal::ReferenceTypes,
        immediates: "heaptype",
    },
    Opcode {
        hex: "0xd1",
        mnemonic: "ref.is_null",
        proposal: Proposal::ReferenceTypes,
        immediates: "",
    },
    Opcode {
        hex: "0xd2",
        mnemonic: "ref.func",
        proposal: Proposal::ReferenceTypes,
        immediates: "funcidx",
    },
    Opcode {
        hex: "0xd3",
        mnemonic: "ref.eq",
        proposal: Proposal::Gc,
        immediates: "",
    },
    Opcode {
        hex: "0xd4",
        mnemonic: "ref.as_non_null",
        proposal: Proposal::Gc,
        immediates: "",
    },
    Opcode {
        hex: "0xd5",
        mnemonic: "br_on_null",
        proposal: Proposal::Gc,
        immediates: "labelidx",
    },
    Opcode {
        hex: "0xd6",
        mnemonic: "br_on_non_null",
        proposal: Proposal::Gc,
        immediates: "labelidx",
    },
    // Tail calls
    Opcode {
        hex: "0x12",
        mnemonic: "return_call",
        proposal: Proposal::TailCalls,
        immediates: "funcidx",
    },
    Opcode {
        hex: "0x13",
        mnemonic: "return_call_indirect",
        proposal: Proposal::TailCalls,
        immediates: "typeidx tableidx",
    },
    Opcode {
        hex: "0x14",
        mnemonic: "call_ref",
        proposal: Proposal::Gc,
        immediates: "typeidx",
    },
    Opcode {
        hex: "0x15",
        mnemonic: "return_call_ref",
        proposal: Proposal::TailCalls,
        immediates: "typeidx",
    },
    // Exception handling
    Opcode {
        hex: "0x06",
        mnemonic: "try",
        proposal: Proposal::ExceptionHandling,
        immediates: "blocktype",
    },
    Opcode {
        hex: "0x07",
        mnemonic: "catch",
        proposal: Proposal::ExceptionHandling,
        immediates: "tagidx",
    },
    Opcode {
        hex: "0x08",
        mnemonic: "throw",
        proposal: Proposal::ExceptionHandling,
        immediates: "tagidx",
    },
    Opcode {
        hex: "0x0a",
        mnemonic: "throw_ref",
        proposal: Proposal::ExceptionHandling,
        immediates: "",
    },
    Opcode {
        hex: "0x18",
        mnemonic: "delegate",
        proposal: Proposal::ExceptionHandling,
        immediates: "labelidx",
    },
    Opcode {
        hex: "0x19",
        mnemonic: "catch_all",
        proposal: Proposal::ExceptionHandling,
        immediates: "",
    },
    // Bulk memory (0xFC prefix)
    Opcode {
        hex: "0xFC:0x08",
        mnemonic: "memory.init",
        proposal: Proposal::BulkMemory,
        immediates: "dataidx memidx",
    },
    Opcode {
        hex: "0xFC:0x09",
        mnemonic: "data.drop",
        proposal: Proposal::BulkMemory,
        immediates: "dataidx",
    },
    Opcode {
        hex: "0xFC:0x0a",
        mnemonic: "memory.copy",
        proposal: Proposal::BulkMemory,
        immediates: "memidx memidx",
    },
    Opcode {
        hex: "0xFC:0x0b",
        mnemonic: "memory.fill",
        proposal: Proposal::BulkMemory,
        immediates: "memidx",
    },
    Opcode {
        hex: "0xFC:0x0c",
        mnemonic: "table.init",
        proposal: Proposal::BulkMemory,
        immediates: "elemidx tableidx",
    },
    Opcode {
        hex: "0xFC:0x0d",
        mnemonic: "elem.drop",
        proposal: Proposal::BulkMemory,
        immediates: "elemidx",
    },
    Opcode {
        hex: "0xFC:0x0e",
        mnemonic: "table.copy",
        proposal: Proposal::BulkMemory,
        immediates: "tableidx tableidx",
    },
    Opcode {
        hex: "0xFC:0x0f",
        mnemonic: "table.grow",
        proposal: Proposal::ReferenceTypes,
        immediates: "tableidx",
    },
    Opcode {
        hex: "0xFC:0x10",
        mnemonic: "table.size",
        proposal: Proposal::ReferenceTypes,
        immediates: "tableidx",
    },
    Opcode {
        hex: "0xFC:0x11",
        mnemonic: "table.fill",
        proposal: Proposal::ReferenceTypes,
        immediates: "tableidx",
    },
    // Trunc sat (0xFC prefix, non-trapping)
    Opcode {
        hex: "0xFC:0x00",
        mnemonic: "i32.trunc_sat_f32_s",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xFC:0x01",
        mnemonic: "i32.trunc_sat_f32_u",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xFC:0x02",
        mnemonic: "i32.trunc_sat_f64_s",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xFC:0x03",
        mnemonic: "i32.trunc_sat_f64_u",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xFC:0x04",
        mnemonic: "i64.trunc_sat_f32_s",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xFC:0x05",
        mnemonic: "i64.trunc_sat_f32_u",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xFC:0x06",
        mnemonic: "i64.trunc_sat_f64_s",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    Opcode {
        hex: "0xFC:0x07",
        mnemonic: "i64.trunc_sat_f64_u",
        proposal: Proposal::Mvp,
        immediates: "",
    },
    // SIMD (0xFD prefix — sélection représentative)
    Opcode {
        hex: "0xFD:0x00",
        mnemonic: "v128.load",
        proposal: Proposal::Simd,
        immediates: "memarg",
    },
    Opcode {
        hex: "0xFD:0x0b",
        mnemonic: "v128.store",
        proposal: Proposal::Simd,
        immediates: "memarg",
    },
    Opcode {
        hex: "0xFD:0x0c",
        mnemonic: "v128.const",
        proposal: Proposal::Simd,
        immediates: "16 bytes",
    },
    Opcode {
        hex: "0xFD:0x0d",
        mnemonic: "i8x16.shuffle",
        proposal: Proposal::Simd,
        immediates: "16 lane indices",
    },
    Opcode {
        hex: "0xFD:0x0e",
        mnemonic: "i8x16.swizzle",
        proposal: Proposal::Simd,
        immediates: "",
    },
    Opcode {
        hex: "0xFD:0x60",
        mnemonic: "i8x16.add",
        proposal: Proposal::Simd,
        immediates: "",
    },
    Opcode {
        hex: "0xFD:0x6e",
        mnemonic: "i8x16.mul",
        proposal: Proposal::Simd,
        immediates: "",
    },
    Opcode {
        hex: "0xFD:0xd6",
        mnemonic: "i64x2.eq",
        proposal: Proposal::Simd,
        immediates: "",
    },
    Opcode {
        hex: "0xFD:0xe4",
        mnemonic: "f32x4.add",
        proposal: Proposal::Simd,
        immediates: "",
    },
    Opcode {
        hex: "0xFD:0xe7",
        mnemonic: "f32x4.div",
        proposal: Proposal::Simd,
        immediates: "",
    },
    Opcode {
        hex: "0xFD:0xf0",
        mnemonic: "f64x2.add",
        proposal: Proposal::Simd,
        immediates: "",
    },
    // Relaxed SIMD (0xFD:0x100+)
    Opcode {
        hex: "0xFD:0x100",
        mnemonic: "i8x16.relaxed_swizzle",
        proposal: Proposal::RelaxedSimd,
        immediates: "",
    },
    Opcode {
        hex: "0xFD:0x105",
        mnemonic: "f32x4.relaxed_madd",
        proposal: Proposal::RelaxedSimd,
        immediates: "",
    },
    Opcode {
        hex: "0xFD:0x10d",
        mnemonic: "f32x4.relaxed_min",
        proposal: Proposal::RelaxedSimd,
        immediates: "",
    },
    // GC (0xFB prefix)
    Opcode {
        hex: "0xFB:0x00",
        mnemonic: "struct.new",
        proposal: Proposal::Gc,
        immediates: "typeidx",
    },
    Opcode {
        hex: "0xFB:0x01",
        mnemonic: "struct.new_default",
        proposal: Proposal::Gc,
        immediates: "typeidx",
    },
    Opcode {
        hex: "0xFB:0x02",
        mnemonic: "struct.get",
        proposal: Proposal::Gc,
        immediates: "typeidx fieldidx",
    },
    Opcode {
        hex: "0xFB:0x05",
        mnemonic: "struct.set",
        proposal: Proposal::Gc,
        immediates: "typeidx fieldidx",
    },
    Opcode {
        hex: "0xFB:0x07",
        mnemonic: "array.new",
        proposal: Proposal::Gc,
        immediates: "typeidx",
    },
    Opcode {
        hex: "0xFB:0x08",
        mnemonic: "array.new_default",
        proposal: Proposal::Gc,
        immediates: "typeidx",
    },
    Opcode {
        hex: "0xFB:0x0c",
        mnemonic: "array.get",
        proposal: Proposal::Gc,
        immediates: "typeidx",
    },
    Opcode {
        hex: "0xFB:0x0f",
        mnemonic: "array.set",
        proposal: Proposal::Gc,
        immediates: "typeidx",
    },
    Opcode {
        hex: "0xFB:0x10",
        mnemonic: "array.len",
        proposal: Proposal::Gc,
        immediates: "",
    },
    Opcode {
        hex: "0xFB:0x1c",
        mnemonic: "ref.i31",
        proposal: Proposal::Gc,
        immediates: "",
    },
    Opcode {
        hex: "0xFB:0x1d",
        mnemonic: "i31.get_s",
        proposal: Proposal::Gc,
        immediates: "",
    },
    Opcode {
        hex: "0xFB:0x1e",
        mnemonic: "i31.get_u",
        proposal: Proposal::Gc,
        immediates: "",
    },
    // Threads / atomics (0xFE prefix)
    Opcode {
        hex: "0xFE:0x00",
        mnemonic: "memory.atomic.notify",
        proposal: Proposal::Threads,
        immediates: "memarg",
    },
    Opcode {
        hex: "0xFE:0x01",
        mnemonic: "memory.atomic.wait32",
        proposal: Proposal::Threads,
        immediates: "memarg",
    },
    Opcode {
        hex: "0xFE:0x02",
        mnemonic: "memory.atomic.wait64",
        proposal: Proposal::Threads,
        immediates: "memarg",
    },
    Opcode {
        hex: "0xFE:0x10",
        mnemonic: "i32.atomic.load",
        proposal: Proposal::Threads,
        immediates: "memarg",
    },
    Opcode {
        hex: "0xFE:0x17",
        mnemonic: "i32.atomic.store",
        proposal: Proposal::Threads,
        immediates: "memarg",
    },
    Opcode {
        hex: "0xFE:0x1e",
        mnemonic: "i32.atomic.rmw.add",
        proposal: Proposal::Threads,
        immediates: "memarg",
    },
    Opcode {
        hex: "0xFE:0xfe",
        mnemonic: "atomic.fence",
        proposal: Proposal::Threads,
        immediates: "0x00",
    },
];

// ---------------------------------------------------------------------------
// Point d'entrée
// ---------------------------------------------------------------------------

pub use super::OpcodesOpts;

/// Affiche la table d'opcodes filtrée par proposition.
pub fn run_opcodes(opts: &OpcodesOpts) -> anyhow::Result<()> {
    let filter = opts.proposal.as_deref().map(|s| s.to_ascii_lowercase());

    let opcodes: Vec<&Opcode> = OPCODE_TABLE
        .iter()
        .filter(|op| match &filter {
            None => true,
            Some(f) => {
                op.proposal.name().eq_ignore_ascii_case(f)
                    || (f == "all")
                    || (f == "core" && op.proposal == Proposal::Mvp)
            }
        })
        .collect();

    if opcodes.is_empty() {
        let known: Vec<&str> = [
            "mvp",
            "bulk-memory",
            "reference-types",
            "tail-calls",
            "exception-handling",
            "simd",
            "relaxed-simd",
            "gc",
            "multi-memory",
            "memory64",
            "threads",
        ]
        .to_vec();
        anyhow::bail!(
            "Proposition inconnue `{}`. Valeurs : {}",
            filter.as_deref().unwrap_or(""),
            known.join(", ")
        );
    }

    match opts.report.as_str() {
        "json" => print_opcodes_json(&opcodes),
        "md" | "markdown" => print_opcodes_md(&opcodes),
        _ => print_opcodes_text(&opcodes, opts.quiet),
    }

    Ok(())
}

fn print_opcodes_text(opcodes: &[&Opcode], _quiet: bool) {
    println!(
        "{:<12} {:<32} {:<22} {}",
        "Hex".bold(),
        "Mnemonic".bold(),
        "Proposal".bold(),
        "Immediats".bold()
    );
    println!("{}", "-".repeat(82));
    for op in opcodes {
        println!(
            "{:<12} {:<32} {:<22} {}",
            op.hex.cyan(),
            op.mnemonic,
            op.proposal.name().dimmed(),
            op.immediates
        );
    }
    println!("\n{} opcodes listés", opcodes.len());
}

fn print_opcodes_md(opcodes: &[&Opcode]) {
    println!("| Hex | Mnemonic | Proposal | Immediats |");
    println!("|-----|----------|----------|-----------|");
    for op in opcodes {
        println!(
            "| `{}` | `{}` | {} | {} |",
            op.hex,
            op.mnemonic,
            op.proposal.name(),
            op.immediates
        );
    }
}

fn print_opcodes_json(opcodes: &[&Opcode]) {
    println!("[");
    let last = opcodes.len().saturating_sub(1);
    for (i, op) in opcodes.iter().enumerate() {
        let comma = if i < last { "," } else { "" };
        println!(
            r#"  {{"hex":"{}","mnemonic":"{}","proposal":"{}","immediates":"{}"}}{}"#,
            op.hex,
            op.mnemonic,
            op.proposal.name(),
            op.immediates,
            comma
        );
    }
    println!("]");
}
