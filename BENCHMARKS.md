<!-- SPDX-License-Identifier: Apache-2.0 -->
# Benchmarks

Reproducible measurements of the binaries that ship in this repo. Run
locally on whatever hardware you have — the numbers below are starting
points, not absolute claims.

## mrx — monorepo scanner

`mrx` walks a polyglot monorepo, detects every workspace (Bun, npm, pnpm,
Cargo, Deno, Turbo, Nx, Lerna), classifies files by language, computes a
blake3 fingerprint of the root configs for downstream cache invalidation,
and writes two JSON snapshots. All in one shot, single static binary,
zero runtime deps.

### Setup

```bash
cargo build --release -p mrx-cli
MRX=./target/x86_64-pc-windows-msvc/release/mrx.exe   # or target/release/mrx on Linux/macOS

# Warm the filesystem cache once, then time three consecutive runs.
$MRX --root /path/to/monorepo scan --out /tmp/path.json --map /tmp/map.json
for i in 1 2 3; do
    /usr/bin/time -f '%e s' $MRX --root /path/to/monorepo scan \
        --out /tmp/path.json --map /tmp/map.json
done
```

### Results

Host: Surface Laptop Studio (i7-11370H, NVMe SSD), Windows 11 Insider
Canary 28020, Rust nightly 1.97 release profile (LTO=fat, codegen-units=1).

| Target monorepo | Files | Bytes scanned | Workspaces | Submodules | mrx scan (warm, run #3) |
|---|---:|---:|---:|---:|---:|
| Small Cargo workspace (this repo's `apps/` + `packages/`) | ~50 | ~2 MB | 0 | 0 | **63 ms** |
| Real polyglot monorepo (Bun + Cargo + 9 submodules) | **19,213** | **482 MB** | **53** | **9** | **1,375 ms** |

Cold-cache first run on the same 19,213-file repo: 2,572 ms.

Per-second throughput on the large repo: **~14,000 files/s** /
**~351 MB/s** including content classification by extension and the
blake3 root-config hash.

### What the run produces

```json
{
  "generated_at": "2026-05-17T13:40Z",
  "root": "...",
  "host": "...",
  "os": "windows",
  "root_kind": {
    "task_runners": ["turbo"],
    "package_managers": ["bun"],
    "lockfiles": ["bun.lock"],
    "has_bun_workspaces": true,
    "has_turbo": true
  },
  "content_hash": "8bd261bde888335bbf39c7243cf487baa6d83cbb7bed6203562cd71ecf32a30f",
  "stats": {
    "total_files": 19213,
    "total_workspaces": 53,
    "total_submodules": 9,
    "bytes_scanned": 482598305,
    "scan_duration_ms": 1326,
    "languages": {
      "HTML":       { "files": 8267, "bytes": 279940026 },
      "JSON":       { "files": 7698, "bytes": 164051726 },
      "JavaScript": { "files":   82, "bytes":  11201291 },
      "Markdown":   { "files": 1515, "bytes":  11047663 }
    }
  }
}
```

The `content_hash` is the blake3 of the concatenation of every root-level
manifest file (`package.json`, `bun.lock`, `Cargo.toml`, `Cargo.lock`,
`turbo.json`, etc.). It lets downstream tooling decide "the build graph
hasn't changed, my CI cache key is still valid" in ~30 µs.

### Why it's fast

- **`ignore` crate** (from ripgrep) walks the tree in parallel, honouring
  `.gitignore` + `.ignore` + per-directory `.gitignore` semantics for free.
- **Rayon work-stealing** across CPU cores — the walk and the per-file
  classification both saturate available threads.
- **blake3** for the content hash: SIMD-vectorised, 6+ GB/s/core, beats
  every SHA-2 derivative without giving up cryptographic strength.
- **Single static binary** — no Node/Python/Bun runtime to spin up, no
  module resolution cost. Cold-start under 5 ms.

### Comparison points (rough, not apples-to-apples)

For the same 19,213-file repo on the same hardware:

| Tool | Wall clock | Notes |
|---|---:|---|
| `mrx scan` | 1.4 s warm | full workspace + language + content-hash report |
| `find . \| wc -l` | 0.6 s warm | just counts entries, no parsing |
| `git ls-files \| wc -l` | 0.3 s warm | only tracked files, no untracked workspaces |
| `tokei` | 5–8 s | per-language LOC, ignores submodules by default |
| Custom Bun/Node walker | 4–12 s | varies widely with `fast-glob` config |

`mrx` does **more** than the fastest of these (workspace classification +
content hash, on top of the walk) while staying within 2.5× of raw
`find`. That gap is GC + parsing overhead that pure-Rust eats for free.

## Reproducing on your own monorepo

```bash
# Clone the binary (no source build needed once we ship to crates.io):
curl -sSf https://raw.githubusercontent.com/aphrody-code/aphrody/main/packaging/install.sh | sh

# Three warm runs, take the median:
for i in 1 2 3; do
    /usr/bin/time -f '%e s' mrx --root . scan --out path.json --map map.json
done
```

Send a PR adding your numbers to a third row of the results table — we
keep these benchmarks honest by collecting community data points.
