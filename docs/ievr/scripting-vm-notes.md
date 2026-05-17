<!-- SPDX-License-Identifier: Apache-2.0 -->
# IEVR — scripting language and VM hypotheses

Working notes for **Inazuma Eleven Victory Road** (Level-5, 2026 PC).
Cross-links: [`cpk-extraction-workflow.md`](cpk-extraction-workflow.md),
[`static-analysis-log.md`](static-analysis-log.md),
[`level5-engine-notes.md`](level5-engine-notes.md),
[`legal-checklist.md`](legal-checklist.md).

## 1. Why this matters

IE logic is data-driven: every player, move, formation is content. The
interpreting code is either hardcoded C++ (rare at IE depth — hundreds of
edge cases per character) or scripted so designers iterate without
recompiling. If scripted, reversing the VM plus bytecode unlocks gameplay.
If hardcoded, only deep Ghidra analysis on `nie.exe` recovers it. `nie.exe`
≈ 31 MB for ≈ 60 GB install (see
[`level5-engine-notes.md`](level5-engine-notes.md)) strongly suggests
scripting is in play.

## 2. Hypothesis ranking (highest probability first)

### H1: Lua / LuaJIT — high probability
- Industry standard, well-tooled, MIT-licensed.
- Bytecode magic: `\x1BLua` (Lua 5.x), `\x1BLJ` (LuaJIT).
- Detection: `strings nie.exe | grep -E '^(lua_|luaopen_|luaL_|lj_)'`
  hits 50+ symbols if statically linked.
- Extensions: `.lua`, `.lub`, `.luac` in extracted CPKs.

### H2: Custom Level-5 VM — medium probability
- Level-5 ships bespoke engines since 3DS-era (see
  [`level5-engine-notes.md`](level5-engine-notes.md) §2); custom VM fits.
- Symptoms: large switch-table opcode dispatcher in `nie.exe`, no Lua
  symbols, opaque bytecode in CPKs.
- Speculative: `.lvs`, `.l5s`, `.nis`.

### H3: Squirrel / AngelScript / JavaScript — low probability
- Uncommon in Japanese retail. Prefixes: `sq_`, `asI`/`as_`, `JS_`, `duk_`.

### H4: Native-only, no script VM — lowest probability
- All gameplay in C++ with enormous switch statements.
- Would push `nie.exe` past 31 MB (expect 100+ MB); binary size argues
  against.

## 3. RE workflow

1. **Symbol scan**: `strings nie.exe | grep -iE '^(lua|lj_|sq_|as_|JS_|duk_)_'`
   bucketed per prefix; record in
   [`static-analysis-log.md`](static-analysis-log.md).
2. **Extension scan**: after extraction per
   [`cpk-extraction-workflow.md`](cpk-extraction-workflow.md), enumerate
   `*.lua`, `*.lub`, `*.luac`, `*.lvs`, `*.l5s`, `*.script`.
3. **Magic-byte scan**: search headers for `\x1BLua` / `\x1BLJ` and
   recurring 4-byte signatures not matching known formats.
4. **Lua branch**: decompile with `luadec` / `unluac` / `LJD`.
5. **Custom VM branch**: locate the interpreter loop in Ghidra (large
   switch on an opcode byte), reverse opcode-by-opcode.

## 4. Decompilation tools

- **luadec** (https://github.com/viruscamp/luadec) — Lua 5.1.
- **unluac** (https://sourceforge.net/projects/unluac/) — Lua 5.1–5.4.
- **LJD** (https://github.com/iliasGTI/ljd) — LuaJIT.
- **Ghidra Lua loader** — community scripts for bytecode disassembly.

## 5. Where scripts plausibly live

- Category-specific CPKs (speculative: `script.cpk`, `game.cpk`).
- Embedded in `data/<area>/*.bin`.
- Statically linked into `nie.exe` (worst case — interpreter recovery
  required first).

## 6. Cross-reference with prior IE titles

- IE GO (3DS): bespoke binary formats with ASCII config blocks, closer to
  data than full VM.
- IE Ares (cancelled, Switch): expected continuity of custom-engine pattern.
- IEVR (PC, 2026): may have adopted Lua for cross-platform parity
  (Switch + PC + PS4/5). Unconfirmed.

## 7. Open questions

- [ ] Lua/LuaJIT symbols present in `nie.exe`?
- [ ] Dominant script extension (`.lua` / `.lvs` / other)?
- [ ] Script blobs encrypted inside CPKs?
- [ ] Runtime script reload viable (modding feasibility)?

## 8. Mission alignment and legal posture

Script reversing is the highest-value lever for IE gameplay (Routine, AI,
formations). Work runs on a locally-owned copy. Private mods acceptable;
publishing decompiled source is not, per
[`legal-checklist.md`](legal-checklist.md). Findings land in
[`static-analysis-log.md`](static-analysis-log.md).
