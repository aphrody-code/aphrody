<!-- SPDX-License-Identifier: Apache-2.0 -->
<!--
SPDX-License-Identifier: Apache-2.0
SPDX-FileCopyrightText: 2026 aphrody contributors
-->

# IEVR — `cpk_list.cfg.bin` Master Manifest Anatomy

RE plan for `data/cpk_list.cfg.bin` (12.77 MB), the suspected pivot mapping
LOGICAL asset paths to MD5-hashed CPK filenames under
`data/packs/<hash>.cpk`. Decoding it is the prerequisite for any meaningful
taxonomy of the 921 CPK archives.

## 1. Why this file matters

All 921 CPK archives in `data/packs/` use MD5-style hash names. The engine
must internally hold pairs `(logical_path, cpk_hash)` so high-level asset
requests resolve to concrete archives. `cpk_list.cfg.bin` is by name and
size the most likely host. Decoding it FIRST collapses remaining RE work
by an order of magnitude: instead of opening each CPK and guessing its
content, the manifest yields the authoritative logical-to-physical map up
front.

## 2. Filename clues

- `.cfg` — config artefact, plausibly Level-5 internal convention.
- `.bin` — opaque binary, almost certainly custom packed serialisation.
- 12.77 MB for ~921 expected entries averages ~14 KB per entry, consistent
  with full Unicode paths plus per-entry metadata (hash, size, flags,
  optional CRC), or with a larger per-FILE entry count at smaller per-entry
  size.

## 3. Hypotheses for format

- **Option A** — flat record array. Layout
  `[count: u32][entry × count]` where each entry is
  `[hash: 16 bytes][size: u64][path: cstring]`. Plain and likely if Level-5
  ported a generic loader.
- **Option B** — protobuf or flatbuffers payload, with a schema embedded in
  `nie.exe`. Consistent with a modern engine refresh.
- **Option C** — bespoke Level-5 container with magic header plus section
  table, mirroring the CPK design (`@UTF`-like tables).
- **Option D** — precompiled Lua bytecode, if the engine embeds a Lua VM
  for config evaluation.
- **Option E** — encrypted blob, sharing a key with the CPK distribution
  layer or held title-specific inside `nie.exe`.

These options are mutually exclusive; the RE strategy in section 4
discriminates them in order of decreasing likelihood (A then B/C then D/E).

## 4. RE strategy (concrete steps)

1. Hex-dump the first 256 bytes (`hexdump -C`) and look for known magic
   (`PK`, `CRILAYLA`, `\x04\x22\x4D\x18` for LZ4, `\x28\xB5\x2F\xFD` for
   zstd, `LuaQ` for Lua bytecode).
2. Run `ent` for an entropy summary. Near-8.0 bits/byte points to
   encryption or strong compression; biased histograms point to a
   partially uncompressed structure.
3. If LZ4 magic appears, decode with `lz4 -d cpk_list.cfg.bin out.bin`
   and restart at step 1 on the output.
4. If text is reachable, `strings -n 8 cpk_list.cfg.bin | head -30`
   should surface logical paths such as `characters/players/...`,
   `textures/ui/...`. Presence or absence of readable paths is the
   cleanest discriminator between Option A/C and Option E.
5. If encrypted, switch to static analysis: load `nie.exe` in Ghidra,
   cross-reference the literal `cpk_list.cfg.bin` to locate the loader,
   then trace key derivation and primitive (plausibly AES-128 or a CRI
   variant).
6. Once a parser is workable, emit `var/data/cpk-logical-map.json` for
   downstream consumers (`cpk-extraction-workflow.md`, asset graph
   tooling).

## 5. Tooling

- `hexdump -C cpk_list.cfg.bin | head -20` for header inspection.
- `ent cpk_list.cfg.bin` for an entropy and chi-square report.
- `strings -n 8 cpk_list.cfg.bin | head -30` for ASCII fragments.
- Ghidra against `nie.exe` for cross-references to the literal filename.
- A custom parser in Rust (target crate `ievr-fmt`) or in Python for
  prototyping once the layout is pinned.

## 6. Expected output

Canonical artefact at `var/data/cpk-logical-map.json`:

```json
{
  "data/characters/players/inazuma_kishibe.bin": "000540c46ad1a58289d6064396b85202",
  "data/textures/ui/main_menu.dds": "027db50ba8c85d90345b7a11787427b7"
}
```

Combined with each CPK's ToC (`cpk-extraction-workflow.md`), this delivers
a fully resolved logical asset tree across all 921 archives.

## 7. Priority

- HIGH. A single decoded file unlocks the remaining asset taxonomy work.
- Estimate: one to two days for an operator already familiar with
  Level-5 internal formats, several weeks for a newcomer who must trace
  the loader from scratch.

## 8. Risks

- If encrypted with a title-specific key, `nie.exe` static analysis is on
  the critical path and must complete first.
- If a non-standard compression algorithm is used, several decompressors
  (LZ4, LZMA, zstd, snappy, CRILAYLA) must be trialled.
- If the container is a proprietary Level-5 binary with no public
  reference, the parser must be derived purely from observation.

## 9. Cross-links

- [`cpk-format.md`](cpk-format.md) — CPK container specification.
- `cpk-extraction-workflow.md` — extraction pipeline, downstream consumer
  of the logical map. _pending_
- [`level5-engine-notes.md`](level5-engine-notes.md) — engine observations
  feeding the RE hypotheses.
- `static-analysis-log.md` — log surface for live RE sessions on
  `nie.exe` and on this file. _pending_
