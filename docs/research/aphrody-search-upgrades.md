<!--
SPDX-License-Identifier: Apache-2.0
SPDX-FileCopyrightText: 2026 aphrody contributors
-->

# aphrody search upgrades — local-filesystem index, magika-typed + store search

Concrete, prioritized upgrades to aphrody's search surface, grounded in the
reverse-engineering of the Google desktop app's `local_files.db` (see
[`google-local-install-map.md`](google-local-install-map.md) §3) and an audit
of the current aphrody search/RE/forensics code.

## 0. Current search surface (as built)

| Surface | Crate / path | What it does | Gap |
| --- | --- | --- | --- |
| `aphrody search <query>` | `cli` → `commands::SearchCommand` | Google **web** search (network) | not local, not offline |
| `aphrody-search` | `crates/aphrody-search` | In-memory BM25-lite inverted index over `Document{title,body,tags}`; JSON persist | only indexes memory/marketplace docs, never the filesystem |
| `aphrody forensics map` | `crates/cli/forensics_cmd.rs` | Parallel `walkdir` → `{path,size,ext}` JSON (now also `sha256`/`modified`/`mtime_unix`) | one-shot dump, no queryable index |
| `aphrody re classify` | `aphrody-re/magika` (feat `magika`) | Magika content type for one file | per-file, not corpus-wide |
| `aphrody re leveldb` / `forensics sqlite` | `aphrody-re/leveldb`, `forensics_cmd` | Read-only LevelDB enum / SQLite schema dump | no content search across stores |
| `dns_recon` / `advanced_recon` | `google_mcp` MCP tools | network recon | unrelated to local search |

**Headline gap:** aphrody can web-search and can map a directory once, but has
**no persistent, queryable local-filesystem index** — exactly the capability
the Google app ships as `local_files.db` (1.59M rows, FTS5). The
`aphrody-search` BM25 engine is the right scoring core but is fed only
in-memory docs.

## 1. Reference design lifted from `local_files.db`

The Google app's index is a clean, privacy-bounded template:

```sql
local_files(file_id, file_path UNIQUE NOCASE, parent_path, filename,
            extension, is_directory, is_users_directory, last_modified)
+ FTS5 virtual table over filename/path
+ idx_parent_path
+ after-insert/update/delete triggers keeping FTS in sync
```

Properties worth copying: **metadata-only** (no file contents → cheap +
privacy-bounded), `NOCASE` path uniqueness, `parent_path` index for prefix
queries, FTS5 for substring/prefix filename search, trigger-maintained sync.

## 2. Prioritized upgrades

### P0 — `aphrody index` : persistent local-file index (highest value, low risk)

A new `aphrody-fsindex` crate (or module under `aphrody-search`) that walks a
root and persists a SQLite FTS5 index mirroring the `local_files` schema.

- `aphrody index build --root <dir> [--db <path>]` — incremental walk
  (`walkdir` + `rayon`, already deps of `cli`), upsert by `file_path`, FTS5
  filename/path tokens, `last_modified` for incremental refresh.
- `aphrody index search <query> [--ext rs,toml] [--under <path>] [--limit N]`
  — FTS5 `MATCH` on filename/path + extension/parent_path filters; JSON out.
- `aphrody index refresh --db <path>` — re-walk, drop rows whose path vanished,
  upsert changed `mtime`.
- Reuses workspace `rusqlite` (bundled, already pinned, `links=sqlite3`) behind
  the existing `forensics` feature flag → no new supply-chain surface, no GPL,
  builds on Linux #1 / Windows #2 (host-only; wasm excluded like `forensics`).
- **Risk:** low. Self-contained crate; does not touch `gemini-web`,
  `aphrody-chat`, `google_mcp`, or `gui`.

### P1 — magika-typed search (`--type code|executable|document`)

Layer Magika (already wired: `aphrody-re/magika`, `aphrody re classify`) onto
the P0 index so files can be filtered by **content type**, not just extension.
Store a `magika_label` / `magika_group` column, populated lazily for files
under a size cap. Then `aphrody index search foo --group executable`. Gated
behind `--features magika` (ONNX, host-only) so the default build is untouched.

### P2 — corpus content search across leveldb / sqlite stores

Extend `aphrody re leveldb` + `forensics sqlite` into a **searchable** mode:
`aphrody re leveldb <dir> --grep <regex>` over keys + UTF-8 value previews
(value preview already capped in `LevelDbEntry`), and a schema-aware
`forensics sqlite --grep` over **column names / CREATE statements** (never row
values — preserves the no-secret-value contract). Useful for RE triage of
Chromium-family stores like those in §4 of the install map.

### P3 — unify ranking via `aphrody-search` BM25 over file metadata

Feed P0 index rows into the existing `aphrody-search` `InvertedIndex`
(`Document{id=path, title=filename, body=parent_path+ext, tags=[ext,group]}`)
so filename search gets BM25 relevance ranking + the existing snippet
machinery, instead of raw FTS5 rank only. Low effort (the engine exists);
optional second-pass re-rank over FTS5 candidates.

## 3. Recommended implementation plan

1. **(P0, this work — see §4)** Enhance `aphrody forensics map` to emit
   `sha256` (small files) + `modified` + `mtime_unix` — done, compiling, the
   minimal queryable-metadata foundation.
2. **(P0 next)** Add `aphrody-fsindex` crate with the `local_files`-shaped FTS5
   schema + `build`/`search`/`refresh`; wire `aphrody index` behind `forensics`
   feature. Smoke test: build over a temp tree, assert FTS5 `MATCH` hits.
3. **(P1)** Add `magika_label` column + `--group` filter under `magika` feature.
4. **(P2/P3)** Searchable store mode + BM25 re-rank, opportunistically.

Sequencing keeps every step self-contained, feature-gated, and off the
crates under concurrent edit.

## 4. Implemented in this work (small, self-contained, compiling)

`aphrody forensics map` (`crates/cli/forensics_cmd.rs`) now records, per file,
in addition to `path/size/ext`:

- `sha256` — SHA-256 hex of files at or below a size cap (default 1 MiB),
  `null` above the cap or on read error;
- `modified` — last-modified as RFC 3339 (UTC);
- `mtime_unix` — last-modified as Unix epoch seconds.

The map summary now reports `hashed_count`. This is the minimal persistent,
queryable metadata foundation the P0 index builds on, and it is the same field
set the PowerShell/bash reproduction scripts emit, so the Rust and script paths
produce equivalent `tree.json` shapes. The change is confined to one CLI module
behind the existing `forensics` feature — no new dependency, no GPL, no overlap
with crates under concurrent edit.
