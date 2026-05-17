<!-- SPDX-License-Identifier: Apache-2.0 -->

# IEVR — Reverse-Engineering Plan

Workplan for RE of **Inazuma Eleven Victory Road** (Level-5, 2026 PC). Sequences inventory into a multi-week pipeline, defines legal envelope, pins aphrody-workspace coordination. `docs/ievr/` is the **meta layer** — plans, inventories, audits, findings. RE work itself happens outside aphrody (see §5).

## 1. Mission

Reverse engineer IEVR game binaries for:

- **Educational study** — interoperability + format docs. Legal basis: operator owns a licensed retail copy.
- **Architecture documentation** — engine module map, asset pipeline, scripting surface, rendering layer.
- **Cross-reference with prior IE titles** — engine evolution across the franchise's 18-year history.
- **Reusable RE tooling** — format parsers extracted into Rust crates for future game-RE work.

Out of scope: asset distribution, cheat development, multiplayer protocol RE, anti-tamper circumvention.

## 2. Phases

### P0 — Inventory (in flight, today)

- **Inputs**: IEVR-#1 binaries, IEVR-#2 toolchain, IEVR-#3 ML env (sister agents).
- **Outputs**: [`binaries-inventory.md`](binaries-inventory.md) _pending_, [`re-toolchain.md`](re-toolchain.md) _pending_, [`ml-env-audit.md`](ml-env-audit.md) _pending_.
- **Done when**: 3 docs landed, binary count > 0, ≥ 1 disassembler confirmed locally.

### P1 — Setup (1-2 days)

- Install tools surfaced as missing by P0 (Ghidra, Frida via `pip install frida-tools`, radare2 as backup).
- Provision `C:\src\ievr-re\` — separate git repo, isolated from aphrody.
- Stand up Ghidra `analyzeHeadless` pipeline scaffold, output to `ievr-re/analysis/<binary>/`.
- Smoke test: disassemble the smallest IEVR `.dll`, confirm decompiled C output is readable.
- **Done when**: one binary fully analysed headlessly, output committed to `ievr-re`.

### P2 — Static analysis (1-2 weeks)

- Batch-run Ghidra headless across every `.exe` / `.dll` from P0.
- Export decompiled C pseudocode per function; persist for grep-ability.
- Identify main executable, engine modules, area DLLs (audio, render, scripting, networking, input).
- Function-name → behaviour map for the top-100 largest functions by instruction count.
- Continuous notes in [`static-analysis-log.md`](static-analysis-log.md) _pending_.
- **Done when**: ≥ 80 % of binaries analysed, top-100 function map landed.

### P3 — Asset extraction (parallel with P2)

- Unreal engine: drive **UModel** / **FModel** against `.pak` containers.
- Custom Level-5 format: identify magic bytes, document the container, write a Rust parser.
- Catalog: textures, models, audio, localisation strings, scripts, fonts.
- **Done when**: ≥ 50 % of assets are browsable from a tool; format note landed.

### P4 — Dynamic analysis (optional, gated on P2/P3)

- Frida script attached to a single-player session (operator's licensed copy, offline).
- Hook P2-identified functions to confirm behavioural hypotheses with live arguments.
- Capture memory snapshots at well-defined game states (main menu, in-match, save-load).
- **Done when**: ≥ 5 hypotheses from P2 confirmed or refuted in writing.

### P5 — Documentation + reusable tools (continuous)

- Architecture notes: rendering pipeline, scripting engine, asset container format.
- Extract reusable parsers into Rust crates — candidates `ievr-fmt`, `ievr-script`.
- Cross-reference findings with prior IE titles for engine-evolution tracking.
- **Done when**: ≥ 1 extracted parser crate compiles standalone outside the RE workspace.

### P6 — ML-assisted similarity (long-term, P2 outputs required)

- Train / fine-tune an asm2vec-style embedding on IEVR functions.
- Build a similarity index for cross-function navigation inside Ghidra.
- Apply it to locate where, e.g., dribble-physics logic from prior IE titles migrated inside IEVR.
- Pipeline wiring: see [`ml-env-audit.md`](ml-env-audit.md) _pending_.
- **Done when**: similarity queries return non-trivial matches across the top-100 function set.

## 3. Legal scope (explicit)

**Permitted:**

- RE for educational + interoperability purposes per **DMCA §1201(f)**, fair-use, EU Software Directive **2009/24/EC Art. 6**.
- All analysis against the operator's own purchased copy.

**Explicitly forbidden (out of scope):**

- Anti-tamper circumvention beyond what static analysis accommodates.
- Multiplayer protocol RE (likely ToS violation).
- Asset redistribution in any form (textures, audio, models, scripts).
- Anything enabling piracy, account compromise, or cheating against live opponents.
- Publication of decryption keys, signing keys, or other defeating material.

## 4. Cross-platform considerations

- IEVR ships on **Windows (Steam)**. Switch / PS5 binaries are ARM64 and structurally different; separate inventory + RE pass required if they ever land.
- The Ghidra toolchain in P1 already handles ARM64, so the workflow is portable — only the inputs change.

## 5. Coordination with `aphrody-yolo-grind`

- IEVR RE work lives in a **separate workspace** at `C:\src\ievr-re\` (proposed). aphrody never imports IEVR binaries or assets.
- `docs/ievr/` inside aphrody is the **meta layer** only: plans, inventories, audits, public findings notes.
- A post-v1.0 aphrody enhancement could expose `aphrody ievr scan` for re-running the pipeline — not a v1.0 deliverable, not on the current grind.

## 6. Schedule (target)

| Phase | Window | State |
|---|---|---|
| P0 | 2026-05-17 (today) | in flight |
| P1 | 2026-05-18 → 2026-05-19 | queued |
| P2 | 2026-05-20 → 2026-06-02 (~2 weeks) | queued |
| P3 | parallel with P2 | queued |
| P4 | 2026-06-03+ if P2 fruitful | conditional |
| P5 | continuous from P2 onward | n/a |
| P6 | 2026-06-15+ once P2 outputs are sufficient | conditional |

## 7. Risks

- **Anti-tamper** (Denuvo, EAC, Level-5-bespoke equivalent): may block dynamic analysis. Fallback is static-only — P4 becomes a no-op, P2/P5 deliver the value.
- **Unreal `.pak` encryption keys**: may require runtime key extraction, which itself triggers anti-tamper. If blocked, P3 downgrades to whatever is unencrypted.
- **Time vs. yield**: re-evaluate after P2 if findings are sparse — abort or refocus rather than sink unbounded hours.
- **Toolchain drift**: Ghidra/Frida/Unreal tools evolve quickly; pin versions in `re-toolchain.md`, refresh quarterly.

## 8. Next action

Wait for IEVR-#1/#2/#3. Then enter **P1 setup**: provision `C:\src\ievr-re\`, install missing tools, smoke-test the smallest IEVR `.dll`.
