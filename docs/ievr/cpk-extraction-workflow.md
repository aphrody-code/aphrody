<!-- SPDX-License-Identifier: Apache-2.0 -->

# IEVR — CPK Extraction Workflow

End-to-end pipeline that turns 921 opaque, MD5-named CPK archives into a fully mapped asset tree. Companion to [`cpk-format.md`](cpk-format.md) (container internals) and [`cri-toolchain.md`](cri-toolchain.md) (tooling matrix). Slots into [`PLAN.md`](PLAN.md) **P3 asset extraction** as the first executable step.

## 1. Goal

Convert `data/packs/<32-hex>.cpk` x921 (cumulative ~60 GB) into:

- A **hash to logical-path map** (`{md5_hash: [internal_path, ...]}`) resolving each CPK back to the named files it ships.
- An **asset-type histogram** to direct the rest of P3 toward the highest-value containers.
- A **failure list** isolating encrypted or non-standard CPKs that need targeted RE before they can be parsed.

Hash-named CPKs come from Steam's content-delivery layer; the logical name is only visible after the ToC is decoded.

## 2. Pipeline overview

ToC-only: never materialises 60 GB of inner files. Five idempotent stages:

- **(a) Walk inventory** — read `var/data/ievr-hashes-cpk.json` for the 921 `(hash, rel_path, size_bytes)` tuples.
- **(b) Per-CPK open** — Rust parser first; QuickBMS with `cpk.bms` as fallback for containers the Rust path rejects (legacy CRILAYLA quirks, exotic codecs).
- **(c) Extract ToC** — read the `CPK ` header, decode the `@UTF` table, walk `TOC` / `ITOC` rows for filename, offset, compressed and extract sizes. No payload reads.
- **(d) Aggregate ToCs** — fold each per-CPK ToC into `var/data/cpk-toc-map.json` shaped `{cpk_hash: [internal_paths, ...]}` via streamed writes.
- **(e) Classify** — dispatch on extension first, magic bytes on ambiguity. Emit a per-type tally.

## 3. Implementation outline

Tool is a small Rust binary. Sketch:

```rust
let inventory: Inventory = load_json("var/data/ievr-hashes-cpk.json")?;
let mut toc_map = BTreeMap::<Md5Hash, Vec<InternalPath>>::new();
let mut histogram = BTreeMap::<AssetType, u64>::new();
let mut failures = Vec::<CpkFailure>::new();

for cpk in inventory.entries {
    let path = install_root.join(&cpk.rel_path);
    match parse_cpk_toc(&path) {
        Ok(toc) => {
            for entry in &toc.files {
                let kind = classify(entry);
                *histogram.entry(kind).or_default() += 1;
            }
            toc_map.insert(cpk.hash, toc.files.into_iter().map(|f| f.path).collect());
        }
        Err(err) => failures.push(CpkFailure { hash: cpk.hash, reason: err.to_string() }),
    }
}
```

`parse_cpk_toc` uses `binrw` for the fixed header plus a hand-rolled `@UTF` reader; `goblin` stays available for inner-file magic sniffing. Parallelise with `rayon` over the inventory vector.

## 4. Suggested location

Out-of-tree, alongside the rest of the IEVR RE workspace:

- Path: `C:/src/ievr-re/cpk-walker/` — standalone Cargo binary, no link to aphrody workspace (see PLAN §5).
- Outputs land in `C:/src/ievr-re/var/cpk/`; JSON artifacts are mirrored read-only into `aphrody/var/data/` for cross-referencing.

## 5. Performance considerations

ToC parsing reads at most ~64 KB per CPK (header plus `@UTF` tables). On a modern NVMe SSD, 921 archives times tens of KB stays well under one second of physical IO; wall time is dominated by JSON serialisation. Target: **under 60 seconds end-to-end** with `rayon` defaults. Anything slower means the parser is accidentally reading payload.

## 6. Storage budget

- Full extract (rejected): ~60 GB inner files, transient value, redistribution risk.
- ToC-only map: **10 to 50 MB** JSON; gitignored under `var/`.
- Histogram and failure list: a few KB each.

The ToC-only stance is what makes this workflow legal and replayable.

## 7. Classification heuristics

Extension-first, magic-bytes when ambiguous:

- `.usm` -> video (CRI Sofdec2).
- `.awb`, `.acb`, `.hca`, `.adx`, `.aax` -> audio (CRI).
- `.lua`, `.lub` -> script.
- `.dat`, `.bin` -> binary asset; magic-byte sniff over the first 16 bytes for sub-classification.
- Magic `PNG\r\n`, `DDS `, `KTX ` -> texture.
- Magic `RIFF` -> generic container (wav, webp, anim).
- Unknown extension and unknown magic -> bucket as `opaque` for manual review.

Heuristics live in a single `classify(entry)` function so they stay unit-testable and easy to extend.

## 8. Validation gates

After a run the operator confirms:

- All 921 CPKs parsed, or every failure recorded in `cpk-extract-failures.json` with a reason.
- Total internal-file count strictly greater than zero.
- At least **five** distinct asset-type categories present in the histogram.
- `cpk-toc-map.json` round-trips through `serde` (load, re-serialise, byte-identical).
- Spot-check: five random CPK hashes have inner filenames plausible against `nie.exe` strings.

## 9. Output artifacts

Written under `C:/src/ievr-re/var/cpk/`, mirrored read-only into aphrody:

- `cpk-toc-map.json` — master map `{cpk_hash: [internal_path, ...]}`.
- `asset-type-histogram.json` — `{asset_type: count}` rollup.
- `cpk-extract-failures.json` — `[{hash, rel_path, reason}, ...]`; most failures are expected to be encrypted CPKs needing the key-extraction path from [`cri-toolchain.md`](cri-toolchain.md) §5.
- `cpk-walker.log` — human-readable run log with timing and counts.
