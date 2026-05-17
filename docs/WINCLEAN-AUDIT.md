# WinClean Audit — what's reusable for aphrody

Read-only audit of `C:\winclean\` (2026-05-17). The project is **single-target Windows 11 / Surface Laptop Studio** (per its README). Most of it is not portable to aphrody's Linux/Windows/WASM cross-platform stack, but several components are first-class extractions.

Total size : 8.2 GB (post-cache exclusion). Stack : Bun + C# NativeAOT + Python (uv) + Rust (n2b sub-workspace) + iecode C++20 submodule.

---

## 1. Reusability ranking

### 🟢 P1 — Direct import recommended

| Asset | Path | Why aphrody cares |
|---|---|---|
| **WASM docs corpus (9 files)** | `apps/mcp/docs/wasm/` | Adds OPFS, WebGPU-from-WASM pthread proxy, large-asset streaming, .NET 10 wasm, iecode pipeline. Complements aphrody's `docs/WASM/` (7 files) — zero overlap on content, just shared decision-matrix patterns. |
| **`bun-expert` skill** | `plugins/pwsh/skills/bun-expert/SKILL.md` | Pure Bun tooling, language-agnostic |
| **`microsoft-docs` skill** | `plugins/winclean/skills/microsoft-docs/` | Useful for `windows-rs` work in aphrody's Windows target |
| **`vercel-*` skills** (5 skills) | `plugins/winclean/skills/vercel-*` | Apply to `aphrody-code/next.js` fork : composition-patterns, react-best-practices, react-native, view-transitions, cli-with-tokens |
| **`shadcn-ui` skill** | `plugins/winclean/skills/shadcn-ui/` | Direct hit for `aphrody-code/ui` (shadcn → MD3 fork) |
| **`web-design-guidelines`, `platform-design` skills** | `plugins/winclean/skills/` | Cross-platform UI policy |
| **`uv-package-manager` skill** | `plugins/winclean/skills/uv-package-manager/` | If aphrody ever ships Python tooling |

### 🟡 P2 — Compare-then-merge

| Asset | Path | Action |
|---|---|---|
| **`packages/n2b/` (9 crates Rust workspace)** | `packages/n2b/{crates,skills}` | Compare with the upstream `aphrody-code/n2b` branch already referenced by aphrody. If winclean's local copy is ahead, propose an upstream PR on `aphrody-code/n2b`. Layout : `n2b-{types,util,rules,scanners,report,ai,github,core,cli,native}`. |
| **n2b skills (10)** | `packages/n2b/skills/{analyze,deploy,dream,gemini-cli-cli,gemini-cli-jsx,green-gate,move,n2b,regen-baseline,run}` | Some may already be in aphrody's plugin ; diff before copying. |
| **`iecode` submodule** (C# .NET 10 core + C++ overlay) | `packages/iecode/` | Original codebase is **C# .NET 10**, with a C++ overlay added later that exposes a C ABI (`cli/include/iecode/ffi.h`) — making the toolkit consumable from Rust via `bindgen`, from Bun via `bun:ffi`, and from WASM via Emscripten. NOT to migrate into aphrody — reference as an external dep if the game-asset pipeline ever lands. Suggest hosting at `aphrody-code/iecode`. |
| **`apps/iecode-web/`** | iecode browser host | Reference implementation for OPFS write-through + WebGPU 120 Hz loop + lazy WASM module load. Pattern transferable to `aphrody-code/ui`. |

### 🟠 P3 — Reference only

| Asset | Reason to skip direct copy |
|---|---|
| `src/Winclean.Core` (C# NativeAOT) | Workspace policy bans C++ ; C# was never accepted. Single-target Win11. |
| `src/Winclean.Mcp` (C# MCP server) | NativeAOT-only ; aphrody's `crates/google_mcp` is the Rust replacement. |
| `src/Winclean.{ExplorerHook, ImageConverter, ReverseEngine}` | Windows-only kernel hooks. |
| `apps/windows-mcp/` (Python venv 200+ MB) | Python only ; replaced by Rust crates in aphrody. |
| `bin/` (295 MB precompiled) | Binaries, not source. |
| `var/` (3.5 GB) | Runtime data. |
| `vendor/microsoft/WindowsAppSDK/` | Windows-only SDK. |

### ⚫ Hard-skip

| Asset | Why |
|---|---|
| `winclean.ps1` entry script | Wraps Windows-only orchestration. |
| `vcpkg.json` | C++ deps for Winclean.Core / iecode build ; iecode keeps its own vcpkg when imported separately. |
| `system_config.json`, `etc/`, `opt/`, `var/` | Win11 system tweaks specific to a single Surface laptop. |

---

## 2. WASM docs — gap analysis vs aphrody's `docs/WASM/`

| Topic | aphrody `docs/WASM/` | winclean `apps/mcp/docs/wasm/` | Gap |
|---|---|---|---|
| Index | `README.md` | `README.md` | Both have it ; merge decision matrices |
| Ecosystem snapshot | implied in versions table | `01-ecosystem-snapshot.md` | **Add** WASI 0.2/0.3 + browser support matrix |
| Rust fundamentals | `rust-wasm-fundamentals.md` | `03-rust-wasm.md` | **Merge** : add `cargo-component` (WASI 0.2) angle |
| wgpu / WebGPU | `wgpu-webgpu.md` | `06-webgpu-from-wasm.md` | **Add** : pthread proxy, SAB-aware queue submission |
| Next.js | `nextjs-integration.md` | — | aphrody already complete |
| Bun WASM | `bun-native-wasm.md` | `05-bun-wasm.md` | **Merge** : add `bun:ffi` vs `WebAssembly` vs `process.dlopen` comparison |
| Tooling | `tooling.md` | implied | aphrody already complete |
| Build targets | `build-targets.md` | implied | aphrody already complete |
| C++ / Emscripten | — | `02-cpp-emscripten.md` | Skip (aphrody is Rust-only) |
| .NET / WASM | — | `04-dotnet-wasm.md` | Skip (no .NET in aphrody) |
| OPFS large assets | — | `07-large-assets-opfs.md` | **Add** : standalone topic |
| iecode pipeline | — | `08-iecode-wasm-pipeline.md` | Skip unless aphrody-code/iecode lands |

Recommended : write `docs/WASM/ecosystem-snapshot.md` + `docs/WASM/opfs-large-assets.md` based on winclean's `01` and `07`, and **augment** `wgpu-webgpu.md` + `bun-native-wasm.md` with winclean's missing angles. Total estimated effort : 1 commit, ~20 KB added.

---

## 3. Skill import path — concrete

Skills live as `<name>/SKILL.md` (+ optional `references/`, `scripts/`). The aphrody plugin location is `.claude/plugins/aphrody/skills/<name>/`.

```bash
# From the aphrody repo root, copy a single skill (idempotent — skip if exists)
NAME=vercel-react-best-practices
SRC="C:/winclean/plugins/winclean/skills/$NAME"
DST=".claude/plugins/aphrody/skills/$NAME"
[ -d "$DST" ] || cp -r "$SRC" "$DST" && echo "imported $NAME"
```

P1 skill batch (immediate copy candidates) :

```
bun-expert
microsoft-docs
shadcn-ui
vercel-cli-with-tokens
vercel-composition-patterns
vercel-react-best-practices
vercel-react-native-skills
vercel-react-view-transitions
web-design-guidelines
platform-design
uv-package-manager
```

---

## 4. Action plan (gradual, no auto-import)

| # | Step | Effort | Risk |
|---|---|---|---|
| 1 | Compare n2b versions : `winclean/packages/n2b/` vs `aphrody-code/n2b@aphrody` branch. If forward, PR upstream. | low | none |
| 2 | Copy 11 P1 skills into `.claude/plugins/aphrody/skills/` (each is < 50 KB) | low | none |
| 3 | Write `docs/WASM/ecosystem-snapshot.md` and `docs/WASM/opfs-large-assets.md` adapted from winclean (paraphrase + verify versions vs 2026-05-17) | medium | none |
| 4 | Augment `docs/WASM/wgpu-webgpu.md` with pthread proxy + SAB section ; augment `docs/WASM/bun-native-wasm.md` with `bun:ffi` vs `WebAssembly` table | medium | none |
| 5 | If iecode pipeline becomes relevant (game-asset use case), publish iecode as `aphrody-code/iecode` and add a thin `crates/aphrody-iecode-ffi` wrapper. | high | scope creep |
| ⊘ | Do **not** import : C# NativeAOT sources, PowerShell entry script, Windows kernel hooks, Python MCP venv, precompiled bin/, runtime var/. |

---

## 5. License compatibility

| Source | License | aphrody (Apache-2.0) compat |
|---|---|---|
| `winclean` root | MIT | ✓ compatible (MIT → Apache-2.0 is fine for re-license on copy ; original notice retained) |
| `packages/n2b` | inherits aphrody-code/n2b | ✓ |
| `packages/iecode` | likely MIT (not verified — check before promotion) | ✓ pending verification |
| skills (per-file) | typically MIT | ✓ |

---

## 6. Files NOT read

Per the session security policy : no `.env`, no `var/`, no `.git/credentials`, no `etc/` (system config) were opened. The audit relied on top-level structure, Cargo.toml manifests, README files, and skill SKILL.md frontmatter only.
