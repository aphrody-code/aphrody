# PLAN_RUST_ONLY.md — Migration aphrody 100% Rust

**Statut :** ACTIF (2026-05-18)
**Directive source :** memory `feedback_aphrody_rust_only` + CLAUDE.md §2 (révisé)
**Audit source :** scan exhaustif du 2026-05-18 (voir `docs/audits/2026-05-18-non-rust-surface.md` si archivé).

> **TL;DR.** aphrody est désormais 100% Rust dans tout le repo. Tout fichier
> `.ts`, `.js`, `.mjs`, `.py`, `.ps1`, `.sh`, `.cmd` doit disparaître ou être
> remplacé par un binaire / `cargo xtask` Rust. ~387 k lignes non-Rust /
> ~475 fichiers à traiter. Plan organisé par leverage : quick wins
> (delete-only) → ports tactiques → ports structurels → CI → archivages.

---

## 0. Surface non-Rust totale (audit 2026-05-18)

| Catégorie | Fichiers | Lignes | Action |
|-----------|---------:|-------:|--------|
| `packages/` TS (jsx, skills, ui) | 41 | 3 400 | Port Rust (sauf `ui` → delete) |
| `packages/next.js` JS (vendored fork) | 41 | 336 190 | Suppression JS / conserver crates Rust |
| `scripts/*.ts` (Bun) | 20 | 7 981 | → `cargo xtask <op>` |
| `scripts/*.ps1` | 34 | 3 116 | → `aphrody {self,win-tools,ievr,bxc}` |
| `scripts/*.sh` | 16 | 2 369 | → cargo aliases / CI Rust steps |
| `scripts/*.py` | 5 | 254 | Delete (4) + 1 port (`merge_uv_deps.py`) |
| `.claude/plugins/aphrody/**/*.ts` | 5 | 1 054 | Port Rust (hooks, MCP bxc, A2A) |
| Configs racine (package.json, bun.lock, tsconfig.json, turbo.json, bunfig.toml) | 5 | — | Delete après ports |
| `node_modules/` | — | 328 MB | Delete (post-purge) |
| Wrappers Rust invoquant `bun` (`crates/cli/src/commands.rs`) | 1 | 7 sites | Refacto Rust natif |
| `crates/gemini-runtime/src/lib.rs` (invoque external `gemini` Node) | 1 | 2 sites | Documenter blocker upstream |
| `.github/workflows/*.yml` (setup-bun, bun run) | 9 | ~70 sites | Drop bun, garder cargo |
| `vendor/bun/`, `vendor/electron-prebuilt/` | — | grosses bin | Archivage hors repo |

---

## 1. Phase 1 — Quick wins (delete-only, zéro port)

Aucun consumer interne ne dépend de ces artefacts. Suppression immédiate sans
remplacement. **Effet attendu : −340 k lignes en un commit.**

| # | Cible | Justification | Verify |
|---|-------|---------------|--------|
| 1.1 | `packages/ui/` (269 l TS, node_modules locaux) | « Standalone, no internal consumers » — fork shadcn non câblé | `grep -r "@aphrody/ui" crates/ packages/ .claude/` retourne 0 |
| 1.2 | `packages/next.js/{*.json,*.md,apps/,bench/,bun.lock}` (336 k l JS) | JS dev-only ; on conserve uniquement les crates Rust turbopack-* via git deps (déjà déclarées dans `Cargo.toml` ligne 388+) | `cargo build -p aphrody --target x86_64-pc-windows-msvc` toujours OK |
| 1.3 | `scripts/bunnize-gemini-cli.ts` (111 l), `scripts/n2b-batch.sh` (157 l), `scripts/refactor_n2b.py` (126 l), `scripts/main.py` (6 l) | Outils one-shot Node→Bun, archivés par essence | `ls scripts/ \| grep -E "bunnize\|n2b-batch\|refactor_n2b\|main.py"` retourne vide |
| 1.4 | `scripts/rename-project.ps1` (370 l), `scripts/rename-to-aphrody.ps1` (132 l) | One-off, refactor déjà appliqué | idem |
| 1.5 | `scripts/fetch_msys2_docs.py` (32 l), `scripts/generate_summary.py` (47 l) | Remplacé par `cargo run -p aphrody-summary` | `cargo run -p aphrody-summary --check` toujours OK |
| 1.6 | `tmp.jsonl` à la racine | Fichier temp non versionnable | Pas dans `git status` |

**Commit suggéré :** `chore(repo): purge dead JS/PS/PY artifacts (~340k LOC)`

---

## 2. Phase 2 — `crates/aphrody-xtask` + ports `scripts/*.ts`

Création d'un crate workspace `crates/aphrody-xtask` (pattern Cargo officiel)
qui expose une sous-commande par script TS purgé. Invoqué via alias
`cargo xtask <op>` dans `.cargo/config.toml`.

### 2.1 Bootstrap `crates/aphrody-xtask`

- `Cargo.toml` : `[package] name = "aphrody-xtask"`, deps : `clap`, `tokio`,
  `reqwest`, `serde_json`, `walkdir`, `regex`, `comrak` (selon besoin).
- `src/main.rs` : dispatcher `clap` avec sous-commandes (cf. tableau).
- Alias `xtask = "run -p aphrody-xtask --release --"` dans `.cargo/config.toml`.
- Ajouter au workspace `members` du `Cargo.toml` racine.

### 2.2 Ports ordonnés par leverage (gros / consommés en premier)

| # | Source TS (lignes) | Cible | Tech Rust | Verify |
|---|---|---|---|---|
| 2.A | `scripts/m3-coverage-audit.ts` (753) | `xtask m3-audit` | `walkdir` + `regex` + `lightningcss` | `cargo xtask m3-audit --check` exit 0 |
| 2.B | `scripts/scrape-m3-tokens.ts` (679) | `xtask scrape-m3-tokens` | `reqwest` + `scraper` (HTML parser) | écrit `assets/m3-tokens.json` valide |
| 2.C | `scripts/aphrody-vs-open-design-openclaw.audit.ts` (700) | `xtask audit-openclaw` | `walkdir` + `serde_json` diff | rapport `docs/audits/openclaw-*.md` généré |
| 2.D | `scripts/skills-hot-reload.ts` (688) | `xtask skills-watch` | `notify` + `tokio::process::Command` | watch sur `.claude/skills/` re-load à la modif |
| 2.E | `scripts/design-templates-import.ts` (516) | `xtask design-import templates` | `reqwest` + `serde_json` | écrit `assets/design-templates/manifest.json` |
| 2.F | `scripts/skill-schema-align.ts` (481) | `xtask skill-schema-check` | `jsonschema` + `serde_json` | exit 0 sur tous les SKILL.md valides |
| 2.G | `scripts/design-systems-import.ts` (459) | `xtask design-import systems` | idem 2.E | idem |
| 2.H | `scripts/openclaw-extensions-audit.ts` (449) | `xtask audit-openclaw-ext` | `walkdir` + `regex` | rapport généré |
| 2.I | `scripts/setup-worktrees.ts` (421) | `xtask worktree-setup` | `git2` crate | `git worktree list` après run |
| 2.J | `scripts/bxc-mass-scrape.ts` (405) | `xtask bxc-mass-scrape` | `reqwest` + tokio + `crates/bxc-engine` direct | NDJSON émis sur stdout |
| 2.K | `scripts/design-google-curate.ts` (399) | `xtask design-google curate` | regex + serde | `assets/design-google/curated.json` |
| 2.L | `scripts/skills-harvest-open-design.ts` (381) | `xtask skills-harvest` | `octocrab` (GitHub API) + serde | écrit `assets/skills-harvest/*.json` |
| 2.M | `scripts/runtimes-detect.ts` (349) | `xtask runtimes-detect` | `which` crate | imprime JSON `{ rustc: X, cargo: Y }` (n'inclut PLUS bun/node) |
| 2.N | `scripts/edge-mass-scrape.ts` (344) | `xtask edge-scrape` | idem 2.J | idem |
| 2.O | `scripts/plugin-contract-port.ts` (255) | `xtask plugin-port` | regex + serde | écrit migration report |
| 2.P | `scripts/design-google-ingest.ts` (244) | `xtask design-google ingest` | idem 2.K | idem |
| 2.Q | `scripts/check-worktrees.ts` (139) | `xtask worktree-check` | `git2` | exit 0 / 1 selon état |
| 2.R | `scripts/skills-sync.ts` (122) | `xtask skills-sync <org>/<repo>` | `octocrab` + `tar` + `tokio::fs` | `aphrody xtask skills-sync vercel-labs/agent-skills` → repo populated |
| 2.S | `scripts/optimize-assets.ts` (86) | `xtask optimize-assets` | `oxipng`, `imagequant`, `lightningcss` | tailles `assets/` réduites mesurables |

**Verify global Phase 2 :** `find scripts -name "*.ts" -not -path "*/node_modules/*"` retourne vide.

**Commit suggéré :** `feat(xtask): port 20 bun scripts to cargo xtask subcommands`

---

## 3. Phase 3 — Ports `scripts/*.ps1` et `scripts/*.sh`

### 3.1 Sous-commande `aphrody self` (install / path / env)

| Source (lignes) | Cible Rust | Tech |
|---|---|---|
| `scripts/Install-AphrodyToPath.ps1` (64) + `scripts/install-aphrody-path.sh` (50) | `aphrody self install-path` | `winreg` (cfg windows) + `xdg` (cfg unix) |
| `scripts/setup-win.ps1` (214) + `scripts/setup-linux.sh` (199) + `scripts/setup-dev-env.{ps1,sh}` (124 + 76) + `scripts/dev-setup.{cmd,sh}` (130) | `aphrody self bootstrap` (auto-installe rustup / msvc / nasm / wasm targets) | `which` + `Command::new("rustup")` + `windows-rs` SetupAPI |

### 3.2 Sous-commande `aphrody win-tools` (Windows only, `#[cfg(target_os = "windows")]`)

| Source | Cible | Tech |
|---|---|---|
| `scripts/Inject-Explorer.ps1` | `aphrody win-tools explorer-inject` | `windows-rs` (CreateRemoteThread) |
| `scripts/Invoke-WindowsAutopsy.ps1` | `aphrody win-tools autopsy` | `windows-rs` (WMI, Get-Process, registre) |
| `scripts/Invoke-DeepSearch.ps1` | `aphrody win-tools deep-search` | `walkdir` + `regex` |
| `scripts/Invoke-NativeServiceControl.ps1` | `aphrody win-tools service` | `windows-service-rs` |
| `scripts/Test-ChromeDecryptorPerf.ps1` | `aphrody win-tools chrome-bench` | `criterion` + `aphrody-translate` |

### 3.3 Sous-commande `aphrody ievr` (Inazuma Eleven Victory Road CPK ops)

9 scripts `ievr-*.ps1` (verify, serve, inventory, poll, binaries, headers,
cpk-stats, nie-strings{,2,3}) → `aphrody ievr {verify,serve,inventory,poll,
binaries,headers,cpk-stats,strings}` (8 sous-commandes).

- **Tech :** `crates/ievr-tools` existe déjà (workspace member). Étendre.
- **Verify :** `aphrody ievr verify` reproduit gates 1+2/5 actuels (HTTP 200
  + Edge screenshot via `crates/obscura-cdp`).

### 3.4 Sous-commande `aphrody bxc` (Linux+Windows, parité parfaite)

| Source | Cible | Tech |
|---|---|---|
| `scripts/bxc-crawl.{ps1,sh}` (262 + 267) | `aphrody bxc crawl --urls X --actions Y` (déjà existe partiellement, étendre) | `crates/bxc-engine` direct |
| `scripts/bxc-mass-scrape.{ps1,sh}` (122 + déjà porté en 2.J) | déjà couvert | — |
| `scripts/bxc-supervise.{ps1,sh}` (124 + 128) | `aphrody bxc supervise` (watchdog daemon) | `tokio::time` + `crates/bxc-engine` |

### 3.5 Sous-commande `aphrody release` (CI helpers)

| Source | Cible | Tech |
|---|---|---|
| `scripts/release.sh` (239) | `aphrody release run` | `cargo_metadata` + `git2` + `Command::new("cargo")` |
| `scripts/publish-crates.sh` (89) | `aphrody release publish-crates` | idem |
| `scripts/verify-publish.sh` (248) | `aphrody release verify` | `reqwest` (registry crates.io API) |
| `scripts/changelog-since.sh` (184) | `aphrody release changelog --since <tag>` | `git2` + `git-cliff-core` crate |
| `scripts/fill-homebrew-shas.sh` (62) | `aphrody release homebrew-shas` | `sha256` + `reqwest` |
| `scripts/sbom-extract.sh` (319) | `aphrody release sbom` | `cargo_metadata` + `cyclonedx-rust-cargo` |
| `scripts/install-wasm-bindgen-cli.sh` (43) | Cargo alias `cargo install wasm-bindgen-cli --locked --version X` | direct |

### 3.6 Sous-commande `aphrody scan` (utilities)

| Source | Cible | Tech |
|---|---|---|
| `scripts/scan-tree.ps1` | `aphrody scan tree` | `walkdir` + `ignore` crate |
| `scripts/scan-manifests.ps1` | `aphrody scan manifests` | `walkdir` + `serde_json/toml` |
| `scripts/drop-purged-dirs.ps1` + `scripts/wipe-artifacts.ps1` | `aphrody scan clean --dry-run` | `walkdir` + `fs::remove_dir_all` |
| `scripts/archive-crates.ps1` + `scripts/archive-google-os.ps1` | `aphrody scan archive --dest <path>` | `walkdir` + `zip` ou `tar` |

### 3.7 Build helpers → cargo aliases

| Source | Cible | Tech |
|---|---|---|
| `scripts/build-linux.sh` (72) | Alias `cargo build-linux = "build -p aphrody --target x86_64-unknown-linux-gnu --release"` dans `.cargo/config.toml` | direct |
| `scripts/build-wasm.sh` (106) | Alias `cargo build-wasm = "build -p aphrody-wasm --target wasm32-unknown-unknown --release"` | direct |

**Verify global Phase 3 :** `find scripts -name "*.ps1" -o -name "*.sh" -o -name "*.cmd" -o -name "*.py"` retourne vide.

**Commit suggéré (par lot) :**
- `feat(cli): port windows installers + bootstrap to aphrody self`
- `feat(cli): port windows-tools ps1 to aphrody win-tools`
- `feat(ievr-tools): port 9 ps1 scripts to aphrody ievr subcommands`
- `feat(cli): port release helpers to aphrody release`

---

## 4. Phase 4 — Ports `packages/aphrody-*` et `.claude/plugins/`

### 4.1 `packages/aphrody-skills/` (1 169 l TS) → `crates/aphrody-skills-runtime`

Le crate `skill` existe déjà. Étendre pour :
- Charger `SKILL.md` (front-matter YAML) → `serde_yaml`
- Validation schema → `jsonschema`
- CLI `agent-skills` existe déjà (binaire workspace). Remplacer les call sites
  `bun run scripts/skills-*.ts` par `agent-skills ...`.

**Verify :** `agent-skills list ./.claude/skills` produit le même output que
`bun run scripts/skills-sync.ts list`.

### 4.2 `packages/aphrody-jsx/` (1 962 l TS) → `crates/aphrody-react-reconciler`

**C'est le port le plus structurant.** Consommateur : crates `aphrody-terminal-*`
(WASM). Le React reconciler TS doit être réécrit en Rust pur compilable
`wasm32-unknown-unknown`.

- Tech : `wasm-bindgen` + `js-sys` pour fenêtre d'interop côté JS host (Claude
  Code TUI / browser).
- Étapes :
  1. Définir le trait `Reconciler` (fibers, commits, effects).
  2. Implémenter en Rust no_std + `alloc`.
  3. WASM bindings pour `createElement`, `useState`, `useEffect`.
  4. Tests parité avec React 19 (snapshot tests).
- **Verify :** `cargo test -p aphrody-react-reconciler` (snapshot tests JSX →
  fiber tree) ET `crates/aphrody-terminal-wasm/examples/demo.html` toujours
  fonctionnel sans charger React JS.

### 4.3 `.claude/plugins/aphrody/mcp/bxc-scrapper/server.ts` (475 l) → `crates/bxc-engine` (binaire MCP)

Le crate `bxc-engine` existe déjà. Ajouter un sous-binaire `bxc-mcp` qui
expose le protocole MCP stdio (déjà implémenté pour `aphrody-mcp` /
`google_mcp` / `obscura-mcp`).

- Tech : réutiliser le code MCP de `crates/aphrody-mcp/src/server.rs` ou
  `crates/google_mcp/` (déjà sur la spec MCP 1.0).
- **Verify :** `echo '{"jsonrpc":"2.0","method":"tools/list","id":1}' |
  bxc-mcp` répond avec la liste des tools (snapshot identique à
  `server.ts`).

### 4.4 `.claude/plugins/aphrody/hooks/*.ts` (340 l) → binaires Rust

| Source | Cible | Tech |
|---|---|---|
| `hooks/cargo-check.ts` (124 l) | `aphrody hook cargo-check` (réutilise déjà le wrapper Rust) | `Command::new("cargo")` + parse JSON output |
| `hooks/cargo-toml-validate.ts` (87 l) | `aphrody hook toml-validate` | `toml` crate + custom rules |
| `hooks/oxclint.ts` (129 l) | Supprimer (le linter TS n'a plus d'objet dans un repo 100% Rust). Si linter Rust nécessaire : `cargo clippy --all -- -D warnings`. | — |

Update `.claude/plugins/aphrody/hooks/hooks.json` pour pointer vers les
binaires Rust.

### 4.5 `.claude/plugins/aphrody/skills/a2a-duel-loop/scripts/duel-cycle.ts` (239 l) → `crates/a2a-client` binaire

Le code de coordination A2A existe déjà en Rust dans `crates/a2a-*`. Ajouter
un binaire `a2a-duel-loop` (ou sous-commande `aphrody a2a duel-loop --iteration
N --side {aphrody,winclean}`) qui reproduit le comportement TS.

- **Verify :** `aphrody a2a duel-loop --iteration 1 --side aphrody --dry-run`
  produit le même JSONL que `bun run duel-cycle.ts --iteration 1 --side
  aphrody --dry-run`.

**Verify global Phase 4 :** `find packages .claude/plugins -name "*.ts" -o
-name "*.js"` retourne vide.

**Commit suggéré :** `feat: port packages/aphrody-* and .claude/plugins to Rust`

---

## 5. Phase 5 — Refactor `crates/cli/src/commands.rs` (drop tous les `Command::new("bun")`)

Audit : 7 sites invoquent `bun` dans `crates/cli/src/commands.rs` (lignes
481, 499, 510, 531, 551, 646, 1437, 1639).

Pour chaque site :
- Identifier le script TS appelé (paramètre `script_path` / `entry`).
- Le script TS doit déjà être porté en `cargo xtask <op>` ou sous-commande
  Rust (Phases 2-4).
- Remplacer `Command::new("bun").arg("run").arg(path)` par appel direct au
  binaire Rust (via `crates::xtask::run(op)` ou sous-commande).

**Verify :** `grep -n '"bun"\|"npm"\|"node"\|"npx"' crates/cli/src/*.rs`
retourne 0 résultat.

**Commit suggéré :** `refactor(cli): drop all bun/node spawn sites`

---

## 6. Phase 6 — Purge finale racine (configs + node_modules)

Après Phases 1-5 réussies (toutes les sources TS/JS/PS/SH/PY purgées ou
portées) :

| Action | Justification | Verify |
|---|---|---|
| `rm -rf node_modules/ packages/ui/ packages/aphrody-jsx/ packages/aphrody-skills/` | Sources TS portées | `ls packages/` ne contient plus que `next.js/` (Rust crates) |
| `rm package.json bun.lock bunfig.toml tsconfig.json turbo.json opencode.json` | Configs Bun/TS plus utilisées | `cargo build --workspace --locked` toujours OK |
| `rm packages/next.js/{package.json,bun.lock,apps/,bench/,*.md sauf README,LICENSE}` | Sub-fork next.js : ne garder que la surface Rust (turbopack-* crates) | `cargo tree -i turbo-tasks` montre toujours la dep |
| Mettre à jour `.gitignore` (drop `node_modules`, `bun.lock`, `.turbo/`, etc.) | Hygiène | git status propre |
| Update `CLAUDE.md` §4.1 (suppression mention scripts shell) | Cohérence doc | — |

**Verify global :** `git ls-files | grep -E '\.(ts\|js\|mjs\|cjs\|py\|ps1\|sh\|cmd\|bat)$' | grep -v '^packages/next\.js'` retourne **0 ligne**.

**Commit suggéré :** `chore(repo): purge Bun/Node/TS root configs — 100% Rust`

---

## 7. Phase 7 — CI / GitHub Actions (drop bun, garder cargo)

9 workflows touchent bun (audit : 70+ invocations). Refacto :

| Workflow | Action |
|---|---|
| `build.yml` (13 sites) | Supprimer `setup-bun`, `bun lint`, garder `cargo clippy --workspace --locked --offline -- -D warnings` |
| `cross-platform.yml` (22 sites) | Supprimer toutes les steps bun, garder matrix `cargo build --target {linux-gnu,pc-windows-msvc,wasm32-unknown-unknown}` |
| `docs.yml` (10 sites) | Remplacer par `cargo doc --workspace --no-deps` + `mdbook build` (déjà dans book.toml) |
| `release.yml` (9 sites) | Remplacer par `aphrody release run` (Phase 3.5) |
| `codeql.yml` (2 sites) | Drop bun setup ; garder language: rust |
| `security.yml` (4 sites) | Drop, remplacer par `cargo deny check + cargo vet` |
| `coverage.yml` (1 site) | Remplacer par `cargo llvm-cov` |
| `dependabot-auto-merge.yml` (2 sites) | Drop bun-specific rules ; garder cargo ecosystem |
| `release-please.yml` (1 site) | Garder (release-please est repo-agnostic, pas bun-specific) |

**Verify global :** `grep -rn 'bun\|npm\|node-version\|setup-bun\|setup-node' .github/workflows/` retourne 0.

**Commit suggéré :** `ci: drop bun/node from all workflows — cargo-only pipelines`

---

## 8. Phase 8 — Archivage vendor

| Cible | Destination | Justification |
|---|---|---|
| `vendor/bun/` | `C:\aphrody-archive\vendor-bun-20260518-*\` | Bun runtime fork — plus aucun consumer Rust depuis purge bun_ffi |
| `vendor/electron-prebuilt/` | `C:\aphrody-archive\vendor-electron-20260518-*\` | Plus aucun consumer ; UI desktop = `crates/gui` (wry/tao Rust pur) |

Retirer les path deps correspondantes dans `Cargo.toml` (root + crates).

**Verify :** `cargo build --workspace --locked` après archivage.

**Commit suggéré :** `chore(vendor): archive bun + electron-prebuilt (no Rust consumers)`

---

## 9. Bloqués upstream (à documenter, pas à fixer)

| Item | Blocker | Workaround |
|---|---|---|
| `crates/gemini-runtime` invoque external `gemini` Node CLI (`Command::new("gemini")`) | Le binaire `gemini` est un produit Google Node-only | Documenter dans CLAUDE.md §7. Long-terme : remplacer par `crates/aphrody-tui` natif (Phase 10 future). |
| `packages/gemini-cli/` (si présent) | Upstream Google, Ink/React | Ne pas réintégrer dans le workspace `bun.lock`. Garder hors `members` Cargo. |
| `packages/next.js/` JS layer (apps/, bench/) | Upstream Vercel ne builde pas sans Node | OK : on n'utilise que les crates Rust turbopack-* via git deps. JS upstream peut rester intouché dans le fork mais doit être git-ignored pour le repo aphrody. |

---

## 10. Ordre d'exécution recommandé (YOLO grind)

Chaque tick `/aphrody-yolo-grind` dispatch 4 agents background sur 4 items
non-conflictuels en parallèle. Ordre suggéré :

| Tick | Lane 1 | Lane 2 | Lane 3 | Lane 4 |
|------|--------|--------|--------|--------|
| 1 | Phase 1 (deletes) | Bootstrap `aphrody-xtask` crate | Audit `commands.rs` (préparer mapping bun→Rust) | Préparer CI workflow diff |
| 2 | Phase 2.A-2.E (top 5 ports TS) | Phase 3.1 (aphrody self) | Phase 4.4 (hooks) | Phase 8 (vendor archivage) |
| 3 | Phase 2.F-2.L | Phase 3.2 (win-tools) | Phase 4.3 (bxc-mcp binaire) | — |
| 4 | Phase 2.M-2.S | Phase 3.3 (ievr) | Phase 4.5 (a2a duel-loop) | Phase 7 (CI) |
| 5 | Phase 4.1 (skills-runtime) | Phase 3.4 (bxc) | Phase 5 (commands.rs cleanup) | — |
| 6 | Phase 4.2 (jsx → reconciler Rust) — gros morceau | Phase 3.5 (release) | Phase 3.6 (scan) | — |
| 7 | Phase 6 (purge finale racine) | — | — | — |

---

## 11. Verify final repo

```bash
# Aucun source non-Rust (sauf packages/next.js qui héberge des crates Rust upstream)
git ls-files | grep -E '\.(ts|js|mjs|cjs|py|ps1|sh|cmd|bat)$' | grep -v '^packages/next\.js'
# → doit retourner 0 ligne

# Aucun Cargo.toml ne référence bun/node/turbo
grep -rn 'bun\|node\|turbo\|next-rs' Cargo.toml crates/*/Cargo.toml | grep -v "^.*#" | grep -v "turbopack-\|turbo-tasks\|next-build\|next-core"
# → doit retourner 0 ligne (sauf crates Rust turbopack-* / next-* explicitement Rust)

# CI ne mentionne aucun bun/node setup
grep -rn 'bun\|setup-bun\|setup-node\|node-version\|npm\|npx' .github/workflows/
# → doit retourner 0 ligne

# Build de référence cross-platform
cargo build -p aphrody --target x86_64-unknown-linux-gnu --locked
cargo build -p aphrody --target x86_64-pc-windows-msvc --locked
cargo build -p aphrody --target wasm32-unknown-unknown --locked
# → tous OK

# Supply-chain toujours vert
cargo deny check && cargo vet
# → exit 0
```

---

## 12. Métriques de progression

À ajouter au footer de chaque commit Conventional :

```
Refs: PLAN_RUST_ONLY.md Phase X.Y
LOC removed: N (TS/JS/PS/SH/PY)
LOC added: M (Rust)
Verify: <observable result>
```

Score-board global cible :
- **Avant :** ~387 k l non-Rust, 475 fichiers
- **Après :** 0 l non-Rust dans le repo (sauf `packages/next.js/` héritage Rust)
- **Effort estimé :** 6-7 ticks `/aphrody-yolo-grind` (24-28 agent-runs)
