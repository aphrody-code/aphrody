<!-- SPDX-License-Identifier: Apache-2.0 -->
# IEVR — Level-5 custom engine notes

Engine notes for **Inazuma Eleven Victory Road** (Level-5, 2026 PC). Grounded
in the IEVR-#1 binary inventory; runtime-internal claims are flagged suspected
and gated on P2 ([`PLAN.md`](PLAN.md) §P2). No fabricated internals before
Ghidra confirms.

## 1. Engine identification

Evidence supporting "custom Level-5 engine" (not Unreal, not Unity):

- **`nie.exe` ≈ 31 MB** for a ≈ 60 GB install — code-light, asset-heavy is the
  canonical custom-engine signature (Unreal ships 80-200 MB; Unity is a small
  loader plus `Assembly-CSharp.dll`).
- **No `Engine/Binaries/Win64/`** — Unreal tree absent.
- **No `<Game>_Data/` and no `UnityPlayer.dll`** — Unity ruled out.
- **921 CPK containers + USM video** — CRI Middleware formats, never default
  Unreal/Unity tooling.
- **DX11 rendering** — `data/dx11/` layout.
- **Bootstrapper**: `GameBootstrapper.exe` → `nie.exe`. Almost certainly an
  EAC + integrity-check stub, not a third-party engine launcher.

## 2. Prior Level-5 engine evolution

- **Yokai Watch 4 (Switch, 2019)** — bespoke engine + CRIWare.
- **Inazuma Eleven 3 / GO (3DS, 2012-2015)** — extractable as SARC, ZIB,
  BCSAR via Sphida, Every File Explorer, 3dstool. 3DS-era containers won't
  survive into a 2026 PC build.
- **Inazuma Eleven Ares (cancelled)** — custom Level-5 engine, likely the
  direct ancestor of IEVR's pipeline.
- **IEVR** likely descends from the post-2018 Mixi-era refresh, retargeted
  for HD (Switch + PC + PS5).

## 3. Cross-references with prior IE community RE

- **Format heritage**: 3DS containers won't load as-is, but semantic naming
  (level IDs, character IDs, routine IDs) often survives engine migrations and
  is worth grepping in CPK TOC output. See
  [`community-sources.md`](community-sources.md) _pending_.
- **Routine (special move) system**: the scripted-animation bundle pattern
  (animation + camera + VFX + audio cue) is prior art IEVR likely inherits
  even with a reworked on-disk format.

## 4. Suspected runtime modules (verify in P2)

- **`cri_fs.dll` or equivalent** — CPK loading + virtual filesystem.
- **`cri_movie.dll` or similar** — USM video playback.
- **`cri_atom.dll` or similar** — ADX/HCA audio.
- **In-house DLLs** if split — rendering, scripting (Lua or proprietary VM),
  physics, AI/team-mate behaviours.
- **EAC bridge** — `EasyAntiCheat_x64.dll` linked into the bootstrapper.

Module inventory will be produced in P2 from `nie.exe` DLL imports.

## 5. Anti-tamper context

- **EAC presence** (`EACLauncher.exe` chain) confirms online-multiplayer
  infrastructure.
- **Single-player offline mode** likely bypasses EAC runtime checks — the
  licensed copy must launch offline for P4 dynamic analysis. Multiplayer
  protocol RE is out of scope ([`PLAN.md`](PLAN.md) §3).

## 6. RE strategy implications

- **Start with `nie.exe`** — 31 MB is small enough for a single Ghidra
  headless pass under an hour; whole-binary auto-analysis is tractable.
- **Map CRIWare entry points first** — `criFs_*`, `criMv_*`, `criAtom_*`
  exports give the asset-loading flow and unlock P3 extraction. Toolchain
  notes in [`cri-toolchain.md`](cri-toolchain.md) _pending_.
- **Cluster functions** by behaviour: rendering (DX11 wrappers), audio (CRI
  Atom callbacks), game logic (state machines + routine dispatch), network
  (EAC bridge + matchmaking stubs), scripting (VM dispatch loop).

## 7. Unknown / open questions

- **Scripting language** — embedded Lua, proprietary bytecode VM, or fully
  native? Look for `.lua` in CPK TOCs, then for jump-table-heavy dispatch
  loops in `nie.exe`.
- **Localisation** — per-region CPKs or single CPK with runtime switching?
  Resolved by inspecting CPK filenames for region tags
  ([`cpk-format.md`](cpk-format.md) _pending_).
- **Save format** — historic Level-5 patterns are XOR-obfuscated
  proto-buffers or JSON; sample the save directory after one in-game save.
- **GPU backend** — DX11 only or DX11 + DX12 + Vulkan fallbacks? Resolved
  in P2 via DLL import enumeration.

## 8. Coordination with peer Claude

- Peer in `C:\winclean\` runs the **ML pipeline** (MlCore agent #14) consuming
  the IEVR-#1 inventory JSON.
- **Aphrody-side**: docs + cross-references + community-source linking
  ([`community-sources.md`](community-sources.md) _pending_).
- **Winclean-side**: embedding pipeline + similarity index.
- Envelope: file-based A2A (`ai.json` + `.coord/` mailbox); both sides
  read-only on IEVR-#1.
