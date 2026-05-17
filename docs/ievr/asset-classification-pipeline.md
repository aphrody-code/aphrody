<!-- SPDX-License-Identifier: Apache-2.0 -->

# IEVR — Asset Classification Pipeline

Post-extraction workflow that turns the raw tree produced by [`cpk-extraction-workflow.md`](cpk-extraction-workflow.md) into a typed, searchable catalog. Slots into [`PLAN.md`](PLAN.md) **P3** as the second executable step.

## 1. Goal

Convert raw extracted files into a categorised, deduplicated, queryable asset catalog. Canonical output: `var/data/ievr-asset-catalog.json`. Answers: "given a category or magic signature, list every internal path across all 921 CPKs."

## 2. Pipeline stages

### Stage 1: Magic byte fingerprinting

For each file under `<extract_root>/`, read the first 16 bytes and lookup in the magic byte table from `asset-formats.md` §1. Tag with one of: `archive`, `video`, `audio`, `texture`, `model`, `script`, `data`, `unknown`. Magic is authoritative — extensions lie.

### Stage 2: Extension cross-check

If the extension is known (`.usm`, `.hca`, `.awb`, `.cpk`), verify it matches the magic-derived category. On mismatch, set `flag = "ext_mismatch"` and route to manual review.

### Stage 3: Inner-archive recursion

If `category == archive` (CPK inside CPK, AWB inside CPK, ZIP inside blob), recurse: extract, classify children, link them via `parent_sha256` so the catalog stays a forest, not a flat list.

### Stage 4: Size and entropy heuristics

Compute Shannon entropy on a 64 KB sample:

- Size > 1 MB, low entropy (< 4.0) — likely uncompressed asset (mesh, raw audio).
- Size < 1 KB — metadata, index, or pointer table.
- Entropy > 7.5 — already compressed or encrypted (LZ4, deflate, AES).

These hints disambiguate `category` when magic is unclear.

### Stage 5: Catalog emission

One JSON line per file:

```json
{
  "cpk_hash": "<32-hex>",
  "internal_path": "sound/voice/chr_001.awb",
  "size": 4823104,
  "category": "audio",
  "magic_bytes_hex": "41465332",
  "sha256": "<64-hex>",
  "entropy": 6.42,
  "parent_sha256": null,
  "flag": null
}
```

## 3. Implementation plan (Rust, pseudo-code)

Implementation lives outside this repo, in `C:/src/ievr-re/asset-classifier/`.

```rust
struct AssetEntry {
    cpk_hash: String,
    internal_path: PathBuf,
    size: u64,
    category: AssetCategory,
    magic_bytes_hex: String,
    sha256: String,
    entropy: f32,
    parent_sha256: Option<String>,
    flag: Option<String>,
}

enum AssetCategory {
    Archive, Video, Audio, Texture, Model, Script, Data, Unknown,
}

fn classify(path: &Path) -> Result<AssetEntry> {
    let header = read_first_n_bytes(path, 16)?;
    let category = match_magic(&header);
    // sample entropy, compute sha256, cross-check extension, emit row
}
```

## 4. Performance

- Per-file budget: < 1 ms (16-byte read, table lookup, SHA-256 over 64 KB sample).
- 921 CPKs times ~100 inner files equals ~92 000 entries in under 90 seconds single-threaded.
- Parallelise via `rayon::par_iter` over the walker — 4-8x speedup on 8 cores; I/O-bound after that.

## 5. Output artifact

`var/data/ievr-asset-catalog.json` (newline-delimited JSON, gzipped at rest). Size 5-50 MB. Path is gitignored; rebuild from extraction snapshot is reproducible.

## 6. Visualization

- `mrx` (aphrody's monorepo mapper) ingests the catalog as a synthetic file tree for terminal browsing.
- D3-backed static HTML emitting an asset-taxonomy sunburst, under `docs/ievr/visualizations/`.

## 7. Anomaly detection

Manual-review queue accepts:

- `ext_mismatch` rows from Stage 2.
- `unknown` magic bytes (undocumented Level-5 format candidates).
- Files > 100 MB (probable hero asset, worth targeted RE).
- Files < 100 bytes (metadata, padding, dangling link).

## 8. Connection to ML pipeline

Per [`ml-env-audit.md`](ml-env-audit.md), `script`-tagged and unknown-high-instruction-density entries feed an `asm2vec` embedding job. Resulting vectors back a similarity index for cross-asset navigation.

## 9. Validation

- Coverage: at least 90 percent of extracted files in a non-`unknown` category.
- Recursion correctness: every `archive` row has at least one child with matching `parent_sha256`.
- Uniqueness: `(cpk_hash, internal_path)` is a primary key.

## 10. Manual review queue

Top 100 `unknown` rows by frequency are inspected; each new signature is documented in `asset-formats.md` §1 and the classifier is re-run until coverage clears 90 percent.

## 11. Cross-links

- [`cpk-extraction-workflow.md`](cpk-extraction-workflow.md) — upstream input producer.
- `asset-formats.md` — magic byte lookup table.
- [`ml-env-audit.md`](ml-env-audit.md) — downstream consumer.
