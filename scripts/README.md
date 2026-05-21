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

## Build & deployment helpers

| Script                          | Purpose                                              |
| ------------------------------- | ---------------------------------------------------- |
| `deploy.ps1`                    | Windows PowerShell: cargo build --release + install to `~\.local\bin`. |
| `deploy.sh`                     | Linux/macOS bash: cargo build --release + install to `~/.local/bin`. |

## Maintenance helpers

All scripts have been migrated to **100% Rust native** or shell-only (bash/PowerShell).
Deprecated: Python (`.py`), TypeScript (`.ts`), Node/Bun runtimes.

## Forensics & ops (Windows-specific)

| Script                              | Purpose                                                   |
| ----------------------------------- | --------------------------------------------------------- |
| `Inject-Explorer.ps1`               | Explorer namespace extension probe.                       |
| `Invoke-DeepSearch.ps1`             | Deep filesystem / registry sweep.                         |
| `Invoke-NativeServiceControl.ps1`   | Native SCM API service control.                           |
| `Invoke-WindowsAutopsy.ps1`         | Combined autopsy snapshot.                                |
| `Test-ChromeDecryptorPerf.ps1`      | Benchmark `crates/forensics` Chrome decryptor.            |

## Utilities

| Script                       | Purpose                                                         |
| ---------------------------- | --------------------------------------------------------------- |
| `ievr-verify.ps1`            | Gates 1+2/5 — HTTP 200 + Edge screenshot (IEVR ops).            |
| `archive-crates.ps1`         | Move a crate out of workspace into `C:\aphrody-archive\`.       |
| `drop-purged-dirs.ps1`       | Remove purged build artefact directories.                       |
| `wipe-artifacts.ps1`         | Clean `target/`, build cache, etc.                              |
| `rename-project.ps1`         | Mass-rename helper for project pivots.                          |
| `fetch-vps-github-token.ps1` | Pull GH token from VPS secrets vault (read-only).               |
| `set-github-token.ps1`       | Local `gh auth` token plumbing.                                 |

## Subdirectories

| Path                | Contents                                                       |
| ------------------- | -------------------------------------------------------------- |
| `scripts/forensics/`| Forensics helpers split out by collector / parser.             |
| `scripts/scraper/`  | M3 / `bxc` doc scraping utilities.                             |
| `scripts/terminal/` | `microsoft/terminal` reference extraction helpers.             |
| `scripts/tools/`    | One-off ad-hoc utilities.                                      |
