<!-- SPDX-License-Identifier: Apache-2.0 -->
# mrx — Monorepo Real-time X-platform mapper

`mrx-cli` ships the **`mrx`** binary: a cross-platform scanner, auditor, and
watcher for polyglot monorepos. One static binary, zero runtime dependencies,
identical behaviour on Linux, Windows, and macOS.

## 1. What is mrx?

`mrx` is the **M**onorepo **R**eal-time **X**-platform mapper. It scans,
detects, audits, and watches monorepo structure across languages and build
systems (Bun, npm, pnpm, Cargo, Deno, Turbo, Nx, Lerna), classifies files by
language, fingerprints root configs with blake3, and emits JSON snapshots —
so CI gates, pre-commit hooks, AWS Lambda jobs, and developer daemons all
consume the same source of truth.

## 2. Install

```bash
# Crates.io (when published):
cargo install --locked mrx-cli

# Directly from the Aphrody monorepo (immediate):
cargo install --git https://github.com/aphrody-code/aphrody --bin mrx
```

The installed binary is named `mrx` (the crate is `mrx-cli`, the `[[bin]]`
target is `mrx`).

## 3. Quick start

```text
$ mrx --help
Usage: mrx [OPTIONS] <COMMAND>
Commands:
  scan   One-shot audit + map (serverless-friendly)
  watch  Long-running watcher (notify, debounced)
  check  Like `scan`, but exit non-zero if findings are detected

$ mrx --root . scan
2026-05-17T15:39:35Z  INFO scan complete status=ProductionReady \
    submodules=0 workspaces=0 duration_ms=1
```

`scan` always exits `0`. `check` exits `1` when audit `status` is anything
other than `ProductionReady` — the right tool for CI gating.

## 4. Subcommands

### `mrx scan`

```text
Usage: mrx scan [OPTIONS]
Options:
      --root <ROOT>  Root of the monorepo [env: VPS_ROOT=]
      --out  <OUT>   Default: <root>/path.json
      --map  <MAP>   Default: <root>/monorepo-map.json
      --log-json     Emit logs as JSON
```

Crawls the tree, classifies every file by language and content kind,
inventories workspaces and git submodules, and writes the two JSON artifacts.

```bash
$ mrx --root /srv/repo scan --out /tmp/path.json --map /tmp/map.json
```

> **Warning** — defaults land in **`<root>`**, i.e. usually the working
> directory when you run `mrx --root .`. Per `CLAUDE.md` §7 and `.gitignore`
> line 20, `path.json` and `monorepo-map.json` are git-ignored at the repo root
> so accidental commits are blocked.

### `mrx watch`

```text
Usage: mrx watch [OPTIONS]
      --debounce-ms <MS>   Debounce window [default: 1500]
```

Long-running daemon. Uses `notify` for FS events, coalesces bursts within the
debounce window, then re-runs `scan` and overwrites artifacts. Use it on a
workstation or VPS to keep the map live.

```bash
$ mrx --root . watch --debounce-ms 750
```

### `mrx check`

```text
Usage: mrx check [OPTIONS]
```

Same crawl as `scan`, but exits `1` whenever the audit `status` is not
`ProductionReady`. Wire it into pre-commit, pre-push, or CI as a hard gate.

```bash
$ mrx --root . check || exit 1
```

## 5. Output artifacts

| File | Contents |
| ---- | -------- |
| `<root>/path.json`         | Audit findings: status, submodules, workspaces, generator metadata. |
| `<root>/monorepo-map.json` | Full structural map: detected task runners, package managers, lockfiles, workspace flags. |

To redirect, pass `--out <path>` and `--map <path>` explicitly. There is no
`--output-dir` shortcut; each artifact is targeted individually so CI
pipelines can write them to different volumes.

## 6. Honest limitations

- Detection accuracy depends on a build-system signature matrix that ships
  hardcoded in `mrx-detect`. Exotic or non-conventional monorepos may not be
  auto-classified — open an issue with the root layout.
- `watch` is currently single-threaded. Large monorepos (>100k files) can
  saturate the event loop during burst rewrites; tune `--debounce-ms` upward.
- `--root` resolution falls back to `$VPS_ROOT`, then to `$HOME/vps`. Outside
  the original VPS deployment context this default is rarely useful — always
  pass `--root` explicitly in scripts.

## 7. Related

- [`../../BENCHMARKS.md`](../../BENCHMARKS.md) — section `mrx — monorepo
  scanner` for reproducible throughput numbers (~14,000 files/s, ~351 MB/s on
  a 19,213-file polyglot monorepo).
- `docs/audits/2026-05-17-mrx-aggressive.md` — in-flight mrx-stack audit
  (link valid _when published_).
