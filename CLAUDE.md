<!-- SPDX-License-Identifier: Apache-2.0 -->
# CLAUDE.md

Guide opérationnel pour Claude Code (claude.ai/code) sur le dépôt **aphrody**.

**Rôle assigné** : **Hardcore Low-level Engineer**
Focus : Rust deep systems programming, FFI cross-platform, real OS integration, memory safety, livraison fonctionnelle complète. **Aucun stub.**

## 0. Pivot 2026-05-17 — Nouveau cap

**Le projet est `aphrody`, le CLI ultime cross-platform.**

Priorités plateformes (ordre strict, non négociable) :
1. **Linux Ubuntu 26.04** — cible #1, build/test natif obligatoire.
2. **Windows 11 Insider Canary Build** — cible #2.
3. **WebAssembly (lib, `wasm32-unknown-unknown` + `wasm32-wasi`)** — cible #3.
4. macOS — best-effort, jamais bloquant pour merge.

Toute commande / API du binaire `aphrody` doit fonctionner sur (1) + (2) + (3).
Le code Windows-specific (NTDLL, IOCP, ConPTY, etc.) est gated `#[cfg(target_os = "windows")]` et ne doit jamais bloquer Linux.
L'ancien sous-projet `google_os` a été **sorti du workspace** (archivé sous `C:\google-os-archive\`).

## 0.1. Autonomie totale — règle inviolable, repo-wide (2026-05-19)

**aphrody n'a AUCUN humain dans la boucle, de A à Z, sur toutes les surfaces** (LLM, skills, CLI, sub-agents, Rust, GitHub, build, install, publish, evals, hooks). Décider seul.
- **`AskUserQuestion` / Commits** : Uniquement pour actions irréversibles destructives (`git push --force main`, drop DB, rm large scope). Tout le reste : direct.
- **Evals de skills** : Headless via script (regex, exit codes, JSON schemas). Pas de browser view/reviews.
- **CLI binary** : Scriptable, sans TTY. `bootstrap` 100% non-interactif.
- **GitHub** : Auto-merge via PRs/release-please/dependabot. Exception : premier tag `v*` reste human-gated.
- **Choix de design** : Trancher seul sur memories + audits + benchmarks. Priorités (§0, §2, §4).

Si une instruction prescrit un step human-in-loop, **cette règle prévaut** : décider seul et documenter.

## 0.3. Unification bxc Rust-only (2026-05-19 v2)

`aphrody bxc` est 100 % Rust (plus de Bun runtime). Conforme à [[feedback_aphrody_rust_only]].
- **Daemon HTTP** : `bxc-engine-daemon` (Rust, `crates/bxc-engine`) sert `/healthz` + `/api/{recon,scrape,detect,tokens}`.
- **Spawn** : `aphrody bxc daemon --port N` résout le binaire via `APHRODY_BXC_DAEMON_BIN` > `APHRODY_BXC_ENGINE_BIN` > `PATH` > `<exe_dir>/` > `<repo_root>/target/release/`.
- **Logs** : `var/run/bxc.log` (pas de redirect muet vers `/dev/null`).
- **`APHRODY_BXC_DRIVER`** : déprécié.
- **Amont** : `packages/bxc/` (Bun/TS) est conservé uniquement pour référence read-only.
- **Install** : `cargo install --locked --path crates/bxc-engine --bin bxc-engine-daemon` ou `cargo build --release --locked -p bxc-engine --bin bxc-engine-daemon` + cp.

## 0.4. État binaire installé (snapshot 2026-05-19)

`cargo build --release -p aphrody --locked` → `target/release/aphrody.exe` copié dans `~/.local/bin/aphrody.exe` (PATH déjà résolu, convention `feedback_clone_path_c_src`).
Smoke matrix : 19 ✅ / 3 ⚠️ / 5 ❌. Gaps critiques à clore :
- **`bxc-engine-daemon` dep manquante** dans `aphrody self bootstrap`. Fix : ajouter step d'install/copie.
- **`coreutils` / `util-linux` orphans** : hors workspace mais wired dans main.rs. Fix : cfg-gate ou variants Command enum retirés.
- **`mirror`** : silent exit 0 sans log. Fix : log au démarrage/skip.
- **`search`** : Google blocking. Fix : fallback DuckDuckGo HTML / Brave Search.
- **`aphrody version --json`** : absent (parity avec doctor --json).

**Résolution Gemini binary** : `gemini_runtime::resolve_bin()` résout `$APHRODY_GEMINI_BIN` > sibling de `current_exe()` > fork in-tree `packages/gemini-cli/bundle/` > PATH `which("gemini")`.
**Auth Gemini end-to-end** : requis pour inférence Gemini :
1. **`~/.gemini/oauth_creds.json`** (OAuth credentials, share possible avec Antigravity).
2. **`~/.gemini/settings.json`** (`{ "security": { "auth": { "selectedType": "oauth-personal" } } }`).
3. **`GEMINI_CLI_TRUST_WORKSPACE=true`** (auto-set par `aphrody a2a`/`gemini` si absent).
4. **Bundle/fork EDUPLICATEWORKSPACE** : le fork in-tree `packages/gemini-cli/` est gitignored. Pour build local, renommer son root `package.json` en `@google/gemini-cli-workspace` et créer le patch placeholder pour `@types/node`.

Outputs validés OK : version, doctor (+--json), self bootstrap --check, completions, scan, dns, a2a "ping" → "pong", notify, oc-*, chromium sync, term, gemini, n2b scan.

## 0.45. Crates livrés (post-Bun, full Rust)

Migration **gemini-cli + bxc + n2b → Rust natif** :
- `aphrody-chat` : Turn-loop orchestrator. Backend `ModelBackend`. Wired via `aphrody chat --stub --prompt "X"`.
- `aphrody-sdk` : Façade publique programatique stable v1.0.
- `aphrody-tools` : Port des 9 builtin tools (read_file, write_file, edit, glob, grep, ls, run_shell, web_fetch, web_search).
- `aphrody-shell` : Cross-platform shell exec.
- `aphrody-sandbox` : Multi-backend isolated exec (seccomp/Job/WASI).
- `a2a-server` : Serveur unifié.
- `bxc-engine::google` : Module Google de bxc (dns, detector, client, search, cache, cache_limit, strategy, etc.).

**Gotcha déploiement** : `cargo build -p aphrody` met à jour `target/release/aphrody.exe` mais **PAS** `~/.local/bin/aphrody.exe`. Toujours copier le binaire post-build.

## 0.5. MISSION DU JOUR — clore PLAN.md ⏳ items

`docs/PLAN.md` recense les items `⏳` actionables sans intervention humaine.
Cap : crusher tout ce qui est techniquement faisable en un loop YOLO grind.
Voir la liste prioritaire à jour dans [`docs/PLAN.md`](docs/PLAN.md).

**Bloqués upstream / human-gated (ne pas tenter)** : PPA Launchpad, Homebrew tap publish, premier tag `v*` (requires human approval), a2a-slimrpc (upstream agntcy-slim-mls bug), path-bases RFC 3529 (Cargo 1.98), wry GTK4 (CVE pipeline), reqwest 0.13 aws-lc-sys, pyo3 0.22 PyString.

**Mode d'attaque par défaut** : `/aphrody-yolo-grind` (4 lanes parallèles par tick). Voir §8.

## 1. ZÉRO STUB, 100% PRODUCTION

L'architecture de base est en place. Mode "scaffolding" **interdit**.
- Toute fonction Rust ou C touchée contient sa logique métier complète et réelle.
- Pour Linux : appels `libc`, `nix`, `tokio`, `io_uring` réels. Windows : `windows-rs` direct.
- Aucun `TODO: implement later`. Scaffold interdit : chaque fichier fusionne ≥3 ressources existantes et ship une feature observable.

## 2. Politique de langages

> Toolchain pinned to `nightly-2026-05-17` via `rust-toolchain.toml`. Re-pin requires PR.

**aphrody est 100% Rust dans tout le repo (binaire, workspace, scripts, skills, MCP, tooling, doc-build).**
- **Tout code** : Rust nightly (Edition 2024).
- **C/C++** : Banni de la distribution, sauf wrappers FFI (`cxx::bridge`).
- **FFI** : `mimalloc` global, zero-copy via pointeurs bruts encapsulés.
- **JS/TS/Node/Bun** : **BANNIS**. Plus d'invocation `bun`/`node`/`npm`/`tsc`/`turbo` dans la CI. Tout code/script `.ts`/`.js` et config associée doivent être migrés en Rust ou supprimés. MCP stdio `stdio: bun ...` migré en binaires Rust. `packages/bxc/` est conservé uniquement pour référence read-only.
- **Python** : Banni (scripts `.py` à migrer en Rust ou supprimer).
- **Shell** : Préférer `cargo xtask` ou binaires Rust. Wrappers de bootstrap one-shot (`dev-setup.sh`) tolérés si nécessaire.
- **Web / UI** : **WASM Rust natif** (`wasm-bindgen`) ou **WebGPU** (`wgpu` crate) pour tout projet web. shadcn-ui legacy réécrit en wrappers Material Web Components 3 natifs via bxc.
- **Vercel Rust stack** (`turbopack-*`, `swc-*`, `next-*`, `lightning-css`, `oxc`) déclaré dans `Cargo.toml` racine. Pas de re-vendoring.

## 2.5. Méthodologie docs / versions / fact-checking

**Avant** d'ajouter une dep ou d'utiliser une API complexe :
- Utiliser le MCP **`context7`** (`resolve-library-id` puis `query-docs`) pour valider la surface d'API / patterns.
- Utiliser `WebSearch crates.io` pour les numéros de version exacts stables.
- S'applique aux setup, configurations, appels d'API non triviaux. Skip pour refactoring local/debug.

## 3. Commandes de validation (tolérance zéro)

```bash
# Build hermétique
cargo ci-offline           # clippy --workspace --all-targets --locked --offline -- -D warnings
cargo xt-offline           # nextest run --workspace --locked --offline

# Cross-platform targets obligatoires
cargo check -p aphrody --target x86_64-unknown-linux-gnu --locked   # cible #1
cargo check -p aphrody --target x86_64-pc-windows-msvc --locked     # cible #2
cargo check -p aphrody --target wasm32-unknown-unknown --locked     # cible #3

# Supply-chain
cargo deny check           # CVE + licences + bans + sources
cargo vet                  # audits signés (feeds Google / Mozilla / Fuchsia)
cargo audit-machete        # unused deps detector
```
*Note* : Machete false-positives contournés via `[package.metadata.cargo-machete]` ignored. `docs/SUMMARY.md` est auto-généré via `cargo run -p aphrody-summary` (ne pas éditer à la main).

## 4. Architecture (post-pivot)

Monorepo **100% Rust** (cf. §2 et memory [[feedback_aphrody_rust_only]]).
Détails complets de l'architecture et du workspace dans [`docs/SOURCE_OF_TRUTH.md`](docs/SOURCE_OF_TRUTH.md).

### Cœur du workspace
- **CLI / cœur** : `cli` (binaire principal), `base` (primitives no_std), `backend` (network).
- **Sous-commandes** : `aphrody n2b`, `aphrody bxc {daemon...}`.
- **A2A / MCP** : `a2a*` crates, `google_mcp`.
- **Term & Chat** : `aphrody-terminal-*` (8 crates), `aphrody-chat`, `aphrody-sdk`.
- **Modules unifiés** : `mrx`, `aphrody-voice`, `aphrody-design`, `aphrody-messaging`, `aphrody-llm-infra`, `aphrody-skills`, `aphrody-marketplace`.

### Hors workspace / Archivés
- Hors workspace : `coreutils`, `util-linux`, `vendor/bun/`, `vendor/electron-prebuilt/`, `a2a-slimrpc`.
- Archivés (NE PAS réintégrer) : `google_os`, `bun_ffi`, `google_kv`, `python_ffi`, `n2b` legacy.

## 4.1. Scripts d'automatisation haute-perf (`scripts/`)

Wrappers `aphrody n2b` / `aphrody bxc` parity bash↔pwsh (NDJSON, p50/p95, SIGINT trap).
- `n2b-batch.{ps1,sh}` — migration parallèle (`xargs -P` / `ForEach-Object -Parallel`).
- `bxc-crawl.{ps1,sh}` — crawl URLs × actions, cache body-hash.
- `bxc-supervise.{ps1,sh}` — watchdog bxc, auto-restart.
- **Gotcha pwsh** : pour trap, utiliser `add_CancelKeyPress` delegate. Single-quoted `@'..'@` pour here-strings brutes.

## 5. Supply-chain (lire avant tout PR qui touche `Cargo.toml`)

- **Pas de `cargo vendor`** : repo lockfile-only (sparse registry).
- **Toute dep** : doit passer `cargo deny check` et avoir un audit `cargo vet` (ou exemption config.toml).
- **Lints** : configurés dans `[workspace.lints]`. Activer pedantic per-crate via `#[warn(clippy::pedantic)]`.

## 6. Conventions de contribution

- Commits = Conventional Commits. Pas de mock ni de stubs.
- **Linux Ubuntu 26.04 est la cible #1** : validation obligatoire.
- Vrais appels système (procfs sur Linux, NT/Win32 sur Windows, io_uring/epoll vs IOCP).
- Avant push : `cargo ci-offline && cargo deny check` sur Linux.

## 6.0. Relation aphrody ↔ winclean

**`C:\src\winclean\`** est la **spécialisation Windows-only** d'aphrody (C# NativeAOT/C++20, 146 tools P/Invoke, IEVR).
aphrody est la version **cross-platform** (Linux #1, Rust, a2a in-tree).
- **Skills/agents** : Génériques (reverse engineering, deep analysis, protocol RE) dans aphrody. Windows-only (Winclean.Mcp, IEVR) dans winclean.
- **Logique** : Si cross-platform, pure Rust dans aphrody. Si Win32 spécifique complexe (DWM, ConPTY), dans winclean.
- **A2A** : `aphrody/ai/outbox.jsonl` ↔ `C:\winclean\.coord\inbox-from-aphrody.jsonl` (miroir de compatibilité).

## 6.1. A2A coordination cross-Claude (ai.json v1, in-tree depuis 2026-05-19)

Ce repo expose un manifest A2A v1.0 (`ai.json` au root). Source de vérité in-tree : `aphrody/ai/`.
- **`ai/heartbeat.txt`** — ISO-8601 + résumé session (écriture exclusive).
- **`ai/outbox.jsonl`** / **`ai/inbox.jsonl`** — NDJSON messages émis/reçus.
- **`ai/peers/<name>.ai.json`** — Snapshots des peers.
- **`ai/README.md`** — Documentation protocole.

**Compatibilité legacy & winclean** : Miroir dans `C:\winclean\.coord\inbox-from-aphrody.jsonl` et `heartbeat-aphrody.txt`. winclean écoute sur `:8788`.
**Workflow par tick** :
1. Lire `ai/peers/winclean.ai.json` + `ai/inbox.jsonl`.
2. Exécuter le travail.
3. Append outbox + mirror outbox.
4. Bump local heartbeat + mirror heartbeat.

**Politique gitignore** : structure trackée (`ai/README.md`, `peers/.gitkeep`), contenu transient gitignored.
- **`/a2a-duel-loop`** : Tick A2A unitaire, appairé avec `.claude/skills/a2a-duel-loop/scripts/duel-cycle.ts`.
- **Concurrent peer** : Toujours `git status` ; ne pas toucher aux fichiers en cours d'édition uncommitted du peer.

## 6.5. Skills & agents (`.claude/`)

Surface skills centralisée :
- **Specs / index** : `docs/cargo/SKILLS.md` et `.claude/skills/README.md`.
- **Skills** : `start` (mode autonome), `vps-commander` (SSH), `rust-best-practices-2026`, `best-stack-2026` (policy awesome-rust).
- **Agents** : `rust-engineer`, `rust-architect`, `cpp-engineer`, `ffi-architect`, `cargo-auditor`.
- **Sync** : `aphrody xtask skills-sync <org>/<repo>` (binaire native).

## 7. Pièges connus (mémoire institutionnelle)

- **aws-lc-sys** : MSVC require NASM prebuilt + Ninja via `.cargo/config.toml`. Linux require `libssl-dev`.
- **rustls 0.23 CryptoProvider** : appeler `rustls::crypto::ring::default_provider().install_default()` avant premier `reqwest::Client`.
- **cargo-zigbuild + `--icf=all`** : incompatible, retiré de `.cargo/config.toml`.
- **`a2a-pb` build.rs** : code source `src/gen/` fait autorité. Codegen gated sous `A2A_PB_REGEN=1`.
- **Crate name `aphrody`** : utiliser `-p aphrody` pour builds/checks. Le dossier reste `crates/cli/`.
- **tracing-subscriber** : pinné à `0.3.22` due to packaging bug in 0.3.23+.
- **`base` package** : `path-bases` (RFC 3529) instable nightly 1.97, désactivé.
- **rand** : version 0.8 imposée par `denokv_proto` (pas de 0.9).
- **GTK3 CVE** : ignorés dans `deny.toml` (wry/tao Linux).
- **wasm** : `tokio` require features sélectives (`tokio-stream`, `js-sys`, `wasm-bindgen-futures`).
- **Steam download monitor** : `du` ment (sparse files). Parser logs + status bytes.
- **mrx scan** : écrit `path.json`/`monorepo-map.json` dans cwd (gitignored).
- **Verify strictly** : `cargo check`/`tsc --noEmit` ne suffisent pas, tester le comportement réel (curl, exit codes, etc.).
- **Edge headless WebGPU** : requestAdapter pending en headless. Utiliser CDP/Chromedp.
- **`npx` et EDUPLICATEWORKSPACE** : workspace dupliqué, MCP `npx` échoue. Utiliser `bunx` ou dédoublonner.
- **Licence GPL** : `unicorn-engine` est GPL-2.0, banni d'aphrody (Apache-2.0) pour éviter contamination.
- **context7 MCP** : max 3 `query-docs` par tour. Déléguer aux sub-agents pour audits massifs.
- **Hallucinations filesystem** : sub-agents peuvent mal lire les dossiers/fichiers. Toujours verify via ls/read.
- **BOM UTF-8 / Mojibake** : caractères corrompus. Préférer `Write` à `Edit` pour fixer les encodages.
- **Pipes shell exit code** : un pipe vers `tail`/`head` masque l'exit code d'origine. Utiliser `; echo "===EXIT=$?==="`.
- **`skill-creator` frontmatter** : `source/version` dans `metadata`, pas de `<` / `>` dans description.
- **`octocrab` Search API** : `.sort("stars")` prend `&str`. Feature-gate `github` active pour isolation CI.

## 7.5. aphrody-terminal — LLM-first terminal (pivot 2026-05-18)

Conçu pour sub-agents/skills/MCP. Output JSON, markdown rendu inline, config JSON full, bridge LLM↔DOM via bxc/agent-browser.
- **Specs** : [`docs/design/aphrody-terminal-spec.md`](docs/design/aphrody-terminal-spec.md).
- **Crates** : 8 terminal crates (`aphrody-terminal-*`).
- **Worktrees** : Reference `wterm` (Vercel-labs) et `terminal` (Microsoft).
- **OSC sequence** : `aphrody-*` namespace réservé pour extensions LLM (14 séquences).
- **Séquences VT** : 22 séquences VT essentielles gérées (alt screen, mouse SGR, true color...).

## 7.6. Workflow YOLO grind par défaut (`/aphrody-yolo-grind`)

1 tick = 4 agents background. Skill : `.claude/skills/aphrody-yolo-grind/SKILL.md`.
- **Process** : rank top-4 ⏳, dispatch agent calls `run_in_background: true` en 1 message.
- **Commit** : orchestrateur commit en batch (Conventional Commits, message groupé, co-authored trailer `Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>` autorisé).
- **A2A** : bump local + winclean heartbeats et outboxes.
- **Break** : 3 ticks consécutifs sans FAIT -> fin du loop.

## 8. Source of Truth

Pour la vue d'ensemble consolidée (architecture, plateformes, livrables, ressources), lire **[`docs/SOURCE_OF_TRUTH.md`](docs/SOURCE_OF_TRUTH.md)**.
Documents connexes :
- [`docs/iecode-public-endpoints.md`](docs/iecode-public-endpoints.md) — APIs publiques.
- [`docs/WINCLEAN-AUDIT.md`](docs/WINCLEAN-AUDIT.md) — audit cross-repo C:/winclean.
- [`docs/posts/2026-05-ai-json.md`](docs/posts/2026-05-ai-json.md) — dev journal A2A file-based.
