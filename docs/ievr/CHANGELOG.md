<!-- SPDX-License-Identifier: Apache-2.0 -->

# Changelog — IEVR documentation layer

[Keep-a-Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/) format. Tracks
evolution of `docs/ievr/` as real reverse-engineering findings refine the
format references and strategy. Semver-ish on the doc layer itself
(independent of aphrody crate versions).

## [Unreleased]

### Added — 2026-05-17, IEVR pivot session, P0 cartography phase

Initial doc layer landed in commit `d6a936f79` (`feat(ievr-pivot +
aphrody-final): IEVR doc layer + aphrody publish-ready`).

Strategy + planning:

- `PLAN.md` — 7-phase workplan (P0 inventory through P6 ML similarity).
- `INDEX.md` — sub-index, 16 docs catalogued with per-phase reading guidance.

Format references (landed):

- `cpk-format.md` — CRI Middleware CPK archive container spec.
- `cri-toolchain.md` — 15 CPK/USM/ADX/HCA tools surveyed.
- `usm-format.md` — CRI Sofdec video container.
- `adx-hca-format.md` — CRI ADX + HCA + AWB audio formats.
- `asset-formats.md` — generic game asset format reference.
- `level5-engine-notes.md` — Level-5 custom engine observations (NOT Unreal/Unity).
- `cpk-extraction-workflow.md` — extraction pipeline.
- `manifest-anatomy.md` — `cpk_list.cfg.bin` RE strategy.
- `cri-known-keys.md` — CRI encryption key reference scaffold.

Phase-specific + safety:

- `eac-considerations.md` — EAC anti-cheat impact, 8-row safety matrix.
- `ml-env-audit.md` — ML environment audit (Python 3.14, RTX 3050 4 GB, RE stack OK, ML stack missing).
- `static-analysis-log.md` — P2 phase log scaffold.
- `save-format-notes.md` — Level-5 save format hypotheses.
- `network-protocol-notes.md` — EOS + Steam observations + OUT-OF-SCOPE list.
- `scripting-vm-notes.md` — Lua / custom VM hypothesis ranking.
- `asset-classification-pipeline.md` — post-extract classification pipeline.

### Changed

- None yet — initial publish.

### Architecture facts established

- Install root: `C:/Program Files (x86)/Steam/steamapps/common/INAZUMA ELEVEN Victory Road` (60.36 GB).
- Engine: custom Level-5 + CRI Middleware (NOT Unreal, NOT Unity, DX11).
- Main exe: `nie.exe` (31 MB); bootstrapper: `GameBootstrapper.exe`.
- Anti-tamper: EAC + EOSSDK (online only; static disk-side analysis safe).
- Asset taxonomy: 921 CPKs named with 32-hex MD5 under `data/packs/<hash>.cpk`.
- Master manifest: `data/cpk_list.cfg.bin` (12.77 MB) — reverse to unlock taxonomy.

### Open follow-ups (next session)

- [ ] Land `binaries-inventory.md`, `re-toolchain.md`, `community-sources.md`, `glossary.md`, `legal-checklist.md` (referenced from `INDEX.md` as _in flight_).
- [ ] RE `cpk_list.cfg.bin` (P2 priority).
- [ ] Determine CPK encryption status (attempt unencrypted extract first).
- [ ] Lua signal search in `nie.exe` to confirm or rule out scripting VM.
- [ ] Stand up Ghidra headless pipeline per `ml-env-audit.md` action item.

## Format

- Entries dated and grouped by Keep-a-Changelog category (`Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, `Security`).
- New section `## [vX.Y.Z] - YYYY-MM-DD` per release of the doc layer.
- Cross-link to commit SHAs when applicable (`d6a936f79` is the seed commit).
