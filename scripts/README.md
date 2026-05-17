<!-- SPDX-License-Identifier: Apache-2.0 -->

# `scripts/` — repo automation inventory

Quick reference for the scripts shipped with `aphrody`. Cross-platform parity
follows the project priority order (Linux #1, Windows #2, WASM #3, macOS
best-effort, cf. `CLAUDE.md` §0).

## Bootstrap (one-shot dev setup)

| Script                     | Platform        | Purpose                                                                                                       |
| -------------------------- | --------------- | ------------------------------------------------------------------------------------------------------------- |
| `dev-setup.sh`             | Linux / macOS   | Minimal one-shot bootstrap: bun + rustup nightly + targets + cargo extras + `bun install`. Mirrors devcontainer. |
| `dev-setup.cmd`            | Windows cmd     | Same as `dev-setup.sh`, for native Windows contributors (no Codespace). Uses `winget` hints, no admin needed. |
| `setup-linux.sh`           | Ubuntu 26.04    | Full Linux provisioning incl. `apt` system deps (libssl-dev, libgtk-3-dev, ninja, nasm, lld...).             |
| `setup-win.ps1`            | Windows 11      | Full Windows provisioning incl. winget catalog (VS 2026 Insiders, SDK 26100, etc.).                          |

## Build helpers

| Script                          | Purpose                                              |
| ------------------------------- | ---------------------------------------------------- |
| `build-linux.sh`                | Release build for `x86_64-unknown-linux-gnu`.        |
| `build-wasm.sh`                 | Build `aphrody-wasm` for `wasm32-unknown-unknown`.   |
| `install-wasm-bindgen-cli.sh`   | Pin-install the exact `wasm-bindgen-cli` version.    |

## Docs & maintenance

| Script                       | Purpose                                                            |
| ---------------------------- | ------------------------------------------------------------------ |
| `gen_summary.ts`             | Regenerate `docs/SUMMARY.md` (auto, do not edit by hand).          |
| `generate_summary.py`        | Legacy Python equivalent (kept for parity).                        |
| `skills-sync.ts`             | Sync external skill catalogues (Vercel labs, Anthropic official).  |
| `optimize-assets.ts`         | Lossless asset optimisation (PNG/SVG/etc.).                        |

## Forensics & ops (Windows-specific)

| Script                              | Purpose                                                   |
| ----------------------------------- | --------------------------------------------------------- |
| `Inject-Explorer.ps1`               | Explorer namespace extension probe.                       |
| `Invoke-DeepSearch.ps1`             | Deep filesystem / registry sweep.                         |
| `Invoke-NativeServiceControl.ps1`   | Native SCM API service control.                           |
| `Invoke-WindowsAutopsy.ps1`         | Combined autopsy snapshot.                                |
| `Test-ChromeDecryptorPerf.ps1`      | Benchmark `crates/forensics` Chrome decryptor.            |

## Verify & misc

| Script                       | Purpose                                                         |
| ---------------------------- | --------------------------------------------------------------- |
| `ievr-serve.ps1`             | Bootstrap bun :8787 (IEVR ops).                                 |
| `ievr-verify.ps1`            | Gates 1+2/5 — HTTP 200 + Edge screenshot.                       |
| `scan-manifests.ps1`         | Audit `*.toml` / `package.json` across workspace.               |
| `scan-tree.ps1`              | Filesystem tree snapshot for audit.                             |
| `archive-crates.ps1`         | Move a crate out of workspace into `C:\aphrody-archive\`.       |
| `archive-google-os.ps1`      | Specific archival for legacy `google_os` crate.                 |
| `drop-purged-dirs.ps1`       | Remove purged build artefact directories.                       |
| `wipe-artifacts.ps1`         | Clean `target/`, `node_modules/`, etc.                          |
| `rename-project.ps1`         | Mass-rename helper for project pivots.                          |
| `rename-to-aphrody.ps1`      | Final google-cli → aphrody local rename.                        |
| `fetch-vps-github-token.ps1` | Pull GH token from VPS secrets vault (read-only).               |
| `set-github-token.ps1`       | Local `gh auth` token plumbing.                                 |
| `move-mdi-residual.ps1`      | Clean MDI residual assets.                                      |
| `bunnize-gemini-cli.ts`      | Convert gemini-cli vendor patches to bun-friendly form.         |
| `refactor_n2b.py`            | Helper for `n2b` Node→Bun migration tool.                       |
| `merge_uv_deps.py`           | Merge Python `uv` lock deltas.                                  |
| `fetch_msys2_docs.py`        | Pull MSYS2 documentation snapshots.                             |
| `scrape-m3-tokens.ts`        | Scrape Material Design 3 token definitions.                     |
| `main.py`                    | Python entry stub (legacy).                                     |
| `pyproject.toml` / `uv.lock` | Pin Python tooling for the scripts in this directory.           |

## Subdirectories

| Path                | Contents                                                       |
| ------------------- | -------------------------------------------------------------- |
| `scripts/forensics/`| Forensics helpers split out by collector / parser.             |
| `scripts/scraper/`  | M3 / `bxc` doc scraping utilities.                             |
| `scripts/terminal/` | `microsoft/terminal` reference extraction helpers.             |
| `scripts/tools/`    | One-off ad-hoc utilities.                                      |
