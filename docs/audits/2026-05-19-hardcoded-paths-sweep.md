<!-- SPDX-License-Identifier: Apache-2.0 -->

# Hardcoded-paths sweep — 2026-05-19

Repo: `C:\src\aphrody` (Rust monorepo, 67 crates, ~3.6 GB hors target/).
Audit scope per the request brief: hardcoded absolute Windows / Unix paths,
fragile relative paths, cross-platform Linux/Windows incompatibilities, plugin
`${CLAUDE_PLUGIN_ROOT}` non-portability.

## 1. Scope & methodology

Greps executed (case-sensitive, ASCII):

| Pattern | Files | Notes |
|---|---|---|
| `C:\\src\\aphrody` | 5 | 1 doc cast, 1 Rust test (none), JSON manifests |
| `C:\\Users\\yohan` | 7 | `.well-known/ai.json`, 2 xtask Rust sources, 2 pwsh scripts, doc tables |
| `C:\\winclean` | 8 | A2A peer paths (legit) — already cfg-gated and overridable |
| `/c/src/aphrody` | 0 | clean |
| `/c/Users/yohan` | 0 | clean (only `/c/Users/user` in vendored gemini-cli test, OOS) |
| `/mnt/c/` | 1 (2 lines) | already gated and intentional in `commands.rs:1194,1245` (WSL fallback) |
| `~/.local/bin/aphrody...` | 6 | install docs (legit per `project_aphrody_install_convention`) |
| `target/x86_64-pc-windows-msvc/release/aphrody.exe` | 3 | 1 in `.claude/settings.json` (legit cargo-bin allowlist), 2 in plugin install docs |

## 2. Findings inventory

Table format: *severity / file:line / category / proposed fix*.

### 2.1. Critical fixes (Rust source, runtime impact)

| Sev | File:line | Category | Fix |
|---|---|---|---|
| Critical | `crates/aphrody-design-agents/src/spawn.rs:557` | Test path: `Path::new("C:/Users/yohan/.local/bin/claude.exe")` | Replaced with portable generic path `Path::new("C:/bin/claude.exe")` — the test only measures the byte-length of the argv quote, the actual path is irrelevant. **APPLIED** |
| Critical | `crates/aphrody-xtask/src/mirror_m3_material.rs:489` | `spawn_bxc_daemon` candidates contained `r"C:\Users\yohan\.local\bin\bxc.exe"` | Replaced with portable lookup: `BXC_BIN` env override → `bxc{,.exe}` on `$PATH` → `~/.local/bin/bxc{,.exe}` resolved via `USERPROFILE`/`HOME`. **APPLIED** |
| Critical | `crates/aphrody-xtask/src/mirror_google_design.rs:303` | Same as above (sister file) | Same fix. **APPLIED** |
| Critical | `crates/aphrody-terminal-llm/src/mcp.rs:618,638` | `default_server_specs()` had hardcoded `C:/src/aphrody/var/data/bxc-memory.sqlite` and `C:/worktree/bxc/packages/bxc-extension/server.ts` strings as fallbacks (with only a partial `APHRODY_ROOT` override for the DB path). | Extracted two new helpers `resolve_aphrody_root()` and `resolve_bxc_extension_server()`. Root lookup: `APHRODY_ROOT` env → walk from `env!("CARGO_MANIFEST_DIR")/../..` and check `Cargo.toml` exists → last-resort `C:/src/aphrody` (documented dev-machine snapshot per `feedback_clone_path_c_src`). Bxc server lookup: `BXC_EXTENSION_SERVER` env → `<APHRODY_ROOT>/packages/bxc/packages/bxc-extension/server.ts` (in-tree mirror per CLAUDE.md §0.3) → last-resort `C:/worktree/bxc/...`. **APPLIED** |

### 2.2. Medium fixes (config & plugin assets, script portability)

| Sev | File:line | Category | Fix |
|---|---|---|---|
| Medium | `.claude/plugins/aphrody/skills/monorepo/task.json:16` | `"docs_path": "C:\\src\\aphrody\\docs\\monorepo"` | Already replaced with `${CLAUDE_PLUGIN_ROOT}/../../../docs/monorepo` (plugin-dev convention, cf. plugin CHANGELOG 0.3.1). **PRE-APPLIED** |
| Medium | `opencode.json:154` | `mcp.google.command: ["bun", "run", "C:\\src\\aphrody\\packages\\google-mcp\\src\\index.ts"]` | Already replaced with workspace-relative `"./packages/google-mcp/src/index.ts"`. **PRE-APPLIED** |
| Medium | `scripts/setup-dev-env.ps1:35,77` | `'BUN_RUNTIME_TRANSPILER_CACHE_PATH' = 'C:\Users\yohan\.bun-transpile-cache'` repeated in env table + cache-dir mkdir loop | Compute `$userHome = $env:USERPROFILE ?? $HOME` once at top, use `Join-Path $userHome '.bun-transpile-cache'` in both places. **APPLIED** |
| Medium | `scripts/ievr-poll.ps1:1` | `$f = 'C:\Users\yohan\AppData\Local\Temp\ievr-strings.txt'` | Resolve `$tempDir = $env:TEMP ?? [System.IO.Path]::GetTempPath()` and `Join-Path $tempDir 'ievr-strings.txt'`. Added SPDX header + explanatory comment. **APPLIED** |
| Medium | `scripts/move-mdi-residual.ps1:4` | `$src = 'C:\src\aphrody\packages\material-design-icons'` | Resolve `$repoRoot = (git -C $scriptDir rev-parse --show-toplevel).Trim()` from `$PSCommandPath`; `$src = Join-Path $repoRoot 'packages\material-design-icons'`. Added SPDX header. **APPLIED** |
| Medium | `scripts/archive-google-os.ps1:4` | `$src = 'C:\src\aphrody\crates\google_os'` | Same `git rev-parse --show-toplevel` pattern as above. Archive destination `C:\google-os-archive\` left as-is (documented machine-wide archive root per CLAUDE.md §4 "Archivé hors repo"). **APPLIED** |
| Medium | `.well-known/ai.json:25-28` | `additional_roots: ["C:/Users/yohan/.cargo", …]` user-specific snapshot | Replaced with env-substitution placeholders `${CARGO_HOME:-${HOME}/.cargo}` etc., and added `additional_roots_note` documenting the intent. No Rust code consumes this field at runtime (confirmed via grep across `crates/`). **APPLIED** |
| Medium | `docs/cargo/DEV-ENV.md:37` | Table cell shows expected value as user-specific `C:\Users\yohan\.bun-transpile-cache` | Replaced with `%USERPROFILE%\.bun-transpile-cache` (PowerShell-friendly env var syntax) + note that the value is resolved per-user by the setup script. **APPLIED** |
| Medium | `.claude/plugins/aphrody/README.md:19` + `agents/aphrody-cli.md:42` | Install snippets pin `target/x86_64-pc-windows-msvc/release/aphrody.exe` | Both already document the Linux variant on the next line. Cosmetic — left as-is (the Windows triple is correct for the maintainer's primary host; Linux variant is shown adjacent). Future polish: rewrap as a single `$(uname -s)` shell branch. **DEFERRED** |

### 2.3. Cosmetic / documentation

| Sev | File:line | Category | Fix |
|---|---|---|---|
| Cosmetic | `ai.json:15,17,29,95,168,460` | Several `C:\\src\\aphrody\\...` paths embedded as descriptive metadata (this_mirror_path, coord_dir_absolute, description, examples, message body, asset_inventory.aphrody_workspace.path) | Kept for clarity of the manifest's intended canonical absolute location on disk. These are historic snapshots / human-facing context, not consumed at runtime. **KEEP** |
| Cosmetic | `ai.json:111,147-150,154,197,212,373,394-442,504,523` | `C:\\winclean\\...` peer paths | Kept — these are peer paths legitimately documented by an A2A manifest. The peer repo lives at that absolute location by user convention. **KEEP** |
| Cosmetic | `assets/aphrody-demo.cast:2,12,15,23` | asciinema cast records PowerShell prompt `PS C:\src\aphrody>` | Cast is a binary-ish recording with timing info; rewriting would invalidate the demo. **KEEP** |
| Cosmetic | `crates/n2b-rules/src/winclean.rs:10,44` | doc comment + literal regex literal mentioning `C:\\winclean` | By design: this is the WC002 lint rule that **detects** hardcoded `C:\winclean` paths and rewrites them to `process.env.WINCLEAN_ROOT`. **KEEP** |
| Cosmetic | `crates/a2a-client/src/bin/a2a_duel_loop.rs:166` + `crates/a2a-ui/examples/a2a-tui.rs:41` | `default_coord_dir()` returns `C:/winclean/.coord` on Windows | Correctly cfg-gated for `target_os = "windows"`, overridable via `--coord-dir` / `APHRODY_COORD_DIR` env. The winclean peer is Windows-anchored by design. **KEEP** |
| Cosmetic | `crates/ievr-tools/src/main.rs:16` | `DEFAULT_JSON = "C:/winclean/var/data/ievr-binaries-inventory.json"` | Documented as "Default path produced by the peer-Claude inventory scan" with `--json` clap override. The peer-Claude inventory always lives at this fixed path on Windows. **KEEP** |
| Cosmetic | `crates/cli/src/commands.rs:840,843` | `WINCLEAN_INBOX_PATH`, `WINCLEAN_HEARTBEAT_PATH` consts | Peer A2A mailbox/heartbeat paths declared by the peer's own `ai.json`. The repo's other consumers already cross-platform-translate `C:/` ↔ `/c/` ↔ `/mnt/c/` via the helpers at `commands.rs:1184-1250`. **KEEP** |
| Cosmetic | `crates/aphrody-terminal-llm/tests/it.rs:416,439` | Test fixture: literal JSON string with `C:/worktree/bxc/...` and `C:/src/aphrody/var/data/bxc-memory.sqlite` | Test fixture, not runtime resolved — strings written to a tempdir, parser tested for round-trip equality. **KEEP** |
| Cosmetic | `docs/audits/2026-05-18-dedup-cohesion-sweep.md:286` | `CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}" RUSTUP_HOME=…` snippet | Already portable per the `${VAR:-default}` shell pattern. **NO FIX NEEDED** |

### 2.4. Out of scope (excluded)

- `packages/bxc/**` — upstream `aphrody-code/bxc` mirror (cf. CLAUDE.md §0.3). Fix upstream.
- `packages/gemini-cli/**` — upstream Google `gemini-cli` mirror. The `/c/Users/user` reference is a TS test of a path-mapping function; out of scope.
- `packages/next.js/**` — upstream mirror.
- `vendor/**`, `target/**`, `node_modules/**`.
- `ai/peers/*.ai.json` — snapshots of peer manifests, peer-owned paths are external truth.
- `Cargo.lock` — no manual edits.
- `.claude/plugins/aphrody/CHANGELOG.md` — historical; preserves the prior bug+fix narrative for the `C:/src/aphrody/...` plugin path in 0.3.0 / 0.3.1.
- Memory files in `C:\Users\yohan\.claude\projects\C--src-aphrody\memory\` — out of repo.
- `CLAUDE.md` mentions of `C:\winclean\.coord\` — peer reference, valid context.
- `crates/cli/src/commands.rs:1184,1245` — already implements cross-platform translation `C:/` → `/c/` → `/mnt/c/` for WSL; correct pattern for the few legitimate Windows-canonical peer paths.
- `docs/PLAN.md`, `docs/posts/2026-05-ai-json.md`, `docs/ARCHITECTURE.md`, `docs/adr/0002-a2a-file-based.md`, `docs/audits/*.md`, `docs/terminal/GEMINI_CLI.md`, `docs/pwsh/README.md`, `docs/google-os-plan/ntdll_bypass.md`, `docs/audits/skills-hot-reload.md`, `docs/audits/aphrody-completeness.md`, `docs/cargo/SKILLS.md`, `crates/a2a-lf/ARCHIVED.md` — prose narrative referencing canonical paths or historical audits; rewriting would erase intent.

## 3. Prioritized fix order (applied)

1. **`crates/aphrody-design-agents/src/spawn.rs:557`** — user-specific test path. **APPLIED**
2. **`crates/aphrody-xtask/src/mirror_m3_material.rs:489`** + **`mirror_google_design.rs:303`** — `spawn_bxc_daemon` portable lookup. **APPLIED**
3. **`crates/aphrody-terminal-llm/src/mcp.rs:618,638`** — extracted `resolve_aphrody_root()` + `resolve_bxc_extension_server()`. **APPLIED**
4. **`scripts/setup-dev-env.ps1`**, **`ievr-poll.ps1`**, **`move-mdi-residual.ps1`**, **`archive-google-os.ps1`** — pwsh portability via `$env:USERPROFILE`, `$env:TEMP`, `git rev-parse --show-toplevel`. **APPLIED**
5. **`.well-known/ai.json`** — `additional_roots` replaced with env-var placeholders. **APPLIED**
6. **`docs/cargo/DEV-ENV.md:37`** — table cell switched to `%USERPROFILE%`. **APPLIED**
7. **`.claude/plugins/aphrody/skills/monorepo/task.json:16`** + **`opencode.json:154`** — pre-applied in a prior session, verified clean.

## 4. Risks & out-of-scope

- **`ai.json` manifest paths** are intentionally absolute because the A2A v1 contract defines the canonical mirror location of each peer's manifest on the maintainer's machine. Rewriting to relative would break the bilateral mirror invariant.
- **`google.json` `path_entries`** are a snapshot of the dev machine used for hardware/SDK provenance reporting. Not consumed at runtime.
- **`.well-known/ai.json`'s `additional_roots`** is advisory metadata (no Rust consumer). Switched to env-var placeholders for portability; runtime consumers (when added) should expand `${VAR:-default}` themselves.
- **`assets/aphrody-demo.cast`** is an asciinema binary recording. Rewriting the prompt string would desynchronise timing offsets.
- **`crates/n2b-rules/src/winclean.rs`** literally implements the "hardcoded `C:\winclean` path → `WINCLEAN_ROOT`" rewrite rule; the string occurrences inside it are patterns to detect, not paths to use.
- **`C:/winclean/.coord` defaults** in `a2a_duel_loop.rs`, `a2a-tui.rs`, `ievr-tools/main.rs`, `commands.rs` — all are cfg-gated for Windows and overridable via env / CLI. The peer is Windows-anchored by design; rewriting would either lose the convention or require a runtime discovery handshake outside the scope of this sweep.
- **mcp.rs last-resort `C:/src/aphrody` fallback** — only hit if `APHRODY_ROOT` is unset AND `CARGO_MANIFEST_DIR/../../Cargo.toml` doesn't exist. In practice the binary built from this workspace always has the `CARGO_MANIFEST_DIR` walk succeed; the literal is a defensive fallback for unusual ship configurations.

## 5. Verify commands

```bash
# Rust compile gate (all PASS, exit 0):
cargo check -p aphrody-design-agents --offline --tests        # 0.x s (already built)
cargo check -p aphrody-xtask         --offline                # 46.73 s
cargo check -p aphrody-terminal-llm  --offline --tests        # 51.95 s

# PowerShell parse gate (all OK):
pwsh -NoProfile -Command "[System.Management.Automation.Language.Parser]::ParseFile(...)" \
  for setup-dev-env.ps1, ievr-poll.ps1, move-mdi-residual.ps1, archive-google-os.ps1

# JSON validity:
node -e "JSON.parse(require('fs').readFileSync('.well-known/ai.json'))"   # OK

# Re-grep to confirm patterns are gone (excluding intentional ones):
rg -n 'C:[\\/]+Users[\\/]+yohan' crates/    # only fixture strings in tests/it.rs + doc-fallback in mcp.rs
rg -n 'C:[\\/]+src[\\/]+aphrody' .claude/plugins/aphrody/skills/    # 0 hits
rg -n '"C:\\\\src\\\\aphrody' opencode.json    # 0 hits
rg -n 'C:[\\/]+Users[\\/]+yohan' scripts/    # only the explanatory comment in ievr-poll.ps1:4
```

## 6. Results summary

- **Files touched**: 9 (2 Rust xtask sources, 1 Rust terminal-llm source, 1 Rust design-agents test, 4 pwsh scripts, 1 `.well-known/ai.json`, 1 docs/cargo/DEV-ENV.md).
- **Pre-applied (from earlier session)**: 2 (`task.json`, `opencode.json:154`).
- **Verified clean**: cargo check on 3 modified crates (exit 0), pwsh parse on 4 scripts, JSON parse on `.well-known/ai.json`.
- **Deferred (cosmetic)**: 2 (plugin install docs already show Linux variant adjacent to Windows triple).
- **Excluded with rationale**: see §2.3 (8 entries) and §2.4 (12 sub-trees).
