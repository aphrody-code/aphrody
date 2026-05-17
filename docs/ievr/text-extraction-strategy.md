<!--
SPDX-License-Identifier: Apache-2.0
SPDX-FileCopyrightText: 2026 aphrody contributors
-->

# IEVR — Text Extraction Strategy

Strategy for recovering the localised text corpus of Inazuma Eleven Victory
Road — story dialog, UI labels, character and move names — across every
shipped language, then aligning entries one-to-one. Companion to
[`cpk-extraction-workflow.md`](cpk-extraction-workflow.md) (raw payload
source) and [`static-analysis-log.md`](static-analysis-log.md) (findings
log). All format claims are **HYPOTHESIS** until promoted by a P2 entry.

## 1. Likely storage locations (HYPOTHESIS)

- Per-language CPK shards (`text_jp.cpk`, `text_en.cpk`, `text_fr.cpk`, ...),
  likely MD5-named — resolution comes from the ToC map.
- Or a `localization/` subtree inside one larger CPK rather than shards.
- In-binary fallback strings (boot menu, fatal errors) inside `nie.exe`.

## 2. Detection passes

- Strings on the shipping binary: `strings -e l nie.exe | head -50` for
  UTF-16LE (Japanese games favour wide strings), plus `-e b` and `-e S`.
- Post-CPK extraction, sweep for `.txt`, `.json`, `.xml`, `.po`, `.csv`,
  `.tbl`, `.msg`.
- Flag unusually large `.bin` / `.dat` siblings of a localization directory
  — string tables tend to dominate that bucket.

## 3. Encoding identification (HYPOTHESIS)

Modern Level-5 titles typically pick UTF-8 or UTF-16LE; Shift-JIS lingers
in legacy assets. Order: `file <candidate>`, then `chardet` on ambiguous
files, then a BOM and byte-pattern sniff in Rust. Record encoding per file
in a sidecar so downstream tools never re-detect.

## 4. Extraction approach (HYPOTHESIS-gated)

- JSON / XML / PO: parse with the standard library, emit `{key: text}`.
- Proprietary Level-5 binary: identify magic, then RE the structure —
  expected shape is a fixed header, a `(count, offset)` index, a payload
  blob. Document the layout in
  [`level5-engine-notes.md`](level5-engine-notes.md) once confirmed.
- Tag every emitted record with source CPK hash and offset.

## 5. Cross-language alignment

- Per-language files are expected parallel: same key count, same order,
  same names. Any drift is a finding and goes into the static-analysis log.
- Build `lang_<code>_to_text.json` per language, fold into a single
  `aligned_strings.tsv` keyed by string ID with one column per language.
  Mismatches emit diagnostic rows, never silent drops.

## 6. Category separation (UI vs dialog vs proper noun)

- Heuristic: dialog entries carry sentence punctuation and length over
  ~20 characters; UI labels are short imperatives ("Save", "Continue");
  proper nouns (characters, moves, locations) usually live in dedicated
  files with a casing convention.
- Category flag stored per entry so downstream consumers can filter
  without re-classifying.

## 7. Tooling

- **Python** with `chardet` + `pandas` for cross-language joins and TSV.
- **strings** plus `iconv` for encoding conversion.
- **xxd** for binary inspection during proprietary-format RE.
- **Rust** binary (sibling of the CPK ToC parser) for production once a
  format is locked.
- **Aphrody bun script** for batch JSON when Python startup would dominate.
- **ripgrep** for cross-language key search and orphan detection.

## 8. Output layout

- `var/data/ievr-strings/<lang>.json` — one normalised file per language,
  key-sorted, UTF-8 on disk.
- `var/data/ievr-strings/aligned.tsv` — cross-language join, one row per
  string ID, one column per language plus a category column.
- `var/data/ievr-strings/manifest.json` — provenance: source CPK hash, path
  inside CPK, encoding, byte size, sha256.
