<!-- SPDX-License-Identifier: Apache-2.0 -->
# IEVR static analysis log

Chronological notes from disassembly/decompilation sessions on the IEVR
(Inazuma Eleven Victory Road) Windows shipping binaries. Add new entries at
the bottom. Each entry records: date, session focus, findings (with
`file:offset` anchors), open questions raised, questions resolved, tooling
used. Cross-reference [`cpk-format.md`](cpk-format.md),
[`usm-format.md`](usm-format.md), [`adx-hca-format.md`](adx-hca-format.md),
and [`level5-engine-notes.md`](level5-engine-notes.md) whenever a finding
touches a known format or engine subsystem.

Safety: every session that loads a binary into a debugger must first re-read
[`eac-considerations.md`](eac-considerations.md). Static analysis (Ghidra,
IDA, radare2, DIE, strings) is always safe; dynamic attach is not.

---

## 2026-05-17 — Pre-analysis observations

**Session focus**: Catalog what is already known from the P0 binary inventory
pass before opening a disassembler on `nie.exe`. No code is loaded yet; this
is a paper-only triage to focus the first real Ghidra session.

**File overview** (per [`binaries-inventory.md`](binaries-inventory.md)):

- `nie.exe` — 31.4 MB, assumed PE64, main game executable.
  - Name `nie` likely abbreviates **Inazuma Eleven** (English transliteration
    would be `ie`; the leading `n` suggests a Japanese romanisation or an
    internal Level-5 codename — `n` may stand for the project codename rather
    than the franchise initials).
  - 31 MB is small for a ~60 GB install, which confirms the engine is
    asset-heavy and code-light. Game logic almost certainly lives in
    data-driven scripts (Lua bytecode or a proprietary VM), packed into CPK
    archives — see [`cpk-extraction-workflow.md`](cpk-extraction-workflow.md).
- `GameBootstrapper.exe` — 2.6 MB launcher. Probably handles EAC kernel
  driver init, crash reporting handshake, and license check before spawning
  `nie.exe`. Static analysis of the bootstrapper is the safest first target.
- `libcurl.dll` — 543 KB, standard HTTP transport (telemetry, manifest pull,
  possibly EOS REST fallback).
- `EOSSDK-Win64-Shipping.dll` — 19 MB, Epic Online Services SDK (auth,
  presence, achievements, P2P, voice).
- `EACLauncher.exe` — 3.97 MB, Easy Anti-Cheat shim. Do not attach a debugger
  to anything spawned from this process tree.

**First static-analysis pass — predicted findings**:

- CRIWare imports — expect `cri_fs.dll`, `cri_movie.dll`, `cri_atom.dll`
  either statically linked into `nie.exe` or shipped as separate DLLs. Need
  an IAT dump to confirm. See [`cri-toolchain.md`](cri-toolchain.md).
- Lua VM — likely embedded (search for `lua_`/`luaL_` strings in `nie.exe`).
- Custom VM bytecode — possible (look for large switch tables on a single
  byte/u16 opcode register).
- Renderer — DirectX 11 confirmed by the `data/dx11/` shader directory shape;
  expect `d3d11.dll` + `dxgi.dll` imports plus `D3DCompiler_47.dll`.

**Open questions** (seed list — every future session should grow it):

- [ ] Is CRIWare statically linked into `nie.exe` or dynamically loaded?
- [ ] What scripting engine is used (Lua / proprietary VM / native-only)?
- [ ] Are CPK files encrypted? Try unencrypted first per
      [`cri-known-keys.md`](cri-known-keys.md).
- [ ] What is the asset taxonomy inside the CPKs? (Resolve via
      [`cpk-extraction-workflow.md`](cpk-extraction-workflow.md).)
- [ ] Where is the AES key for encrypted CPKs (if any) — string table,
      `.rdata`, derived from the executable hash, or fetched at runtime?
- [ ] Does `GameBootstrapper.exe` ever launch `nie.exe` without EAC, e.g. via
      an offline/repair flag?

**Tooling to use next**:

1. `pyghidra` headless on `nie.exe` (per the action item in
   [`ml-env-audit.md`](ml-env-audit.md) — install the Ghidra runtime first).
2. `Detect It Easy (DIE)` for a fast PE header scan + packer detection.
3. `strings nie.exe | rg -i 'cri_|criFs_|criAtom_|lua_|luaL_'` for a quick
   interesting-symbol grep.

**Definition of done for this session**: scaffold doc created, observations
captured, open-question list seeded. Next session: run the Ghidra headless
smoke test and dump the IAT of `nie.exe`.

---

## [TEMPLATE FOR NEXT SESSIONS]

## YYYY-MM-DD — <session title>

**Session focus**: <1-2 sentence goal>

**Findings**:

- `nie.exe:0xABC` — `<description of function>` — confidence: high/medium/low.
- ...

**Open questions added**:

- [ ] ...

**Resolved questions**:

- [x] <question from prior session> — answer: ...

**Tooling used**: <list>

---

(Add more sessions below as work progresses.)
