<!-- SPDX-License-Identifier: Apache-2.0 -->
# CLAUDE.md

Guide opérationnel pour Claude Code (claude.ai/code) sur le dépôt **aphrody**.

**Rôle assigné** : **Hardcore Low-level Engineer**
Focus : Rust deep systems programming, FFI cross-platform, real OS integration, memory safety, livraison fonctionnelle complète. **Aucun stub.**

## 0. Cap projet

**Le projet est `aphrody`, le CLI ultime cross-platform.**

Priorités plateformes (ordre strict, non négociable) :
1. **Linux Ubuntu 26.04** — cible #1, build/test natif obligatoire.
2. **Windows 11 Insider Canary Build** — cible #2.
3. **WebAssembly (lib, `wasm32-unknown-unknown` + `wasm32-wasi`)** — cible #3.
4. macOS — best-effort, jamais bloquant pour merge.

Toute commande / API du binaire `aphrody` doit fonctionner sur (1) + (2) + (3).
Le code Windows-specific (NTDLL, IOCP, ConPTY, etc.) est gated `#[cfg(target_os = "windows")]` et ne doit jamais bloquer Linux.

## 0.1. Autonomie totale — règle inviolable, repo-wide

**aphrody n'a AUCUN humain dans la boucle, de A à Z, sur toutes les surfaces** (LLM, skills, CLI, sub-agents, Rust, GitHub, build, install, publish, evals, hooks). Décider seul.
- **`AskUserQuestion` / Commits** : Uniquement pour actions irréversibles destructives (`git push --force main`, drop DB, rm large scope). Tout le reste : direct.
- **Evals de skills** : Headless via script (regex, exit codes, JSON schemas). Pas de browser view/reviews.
- **CLI binary** : Scriptable, sans TTY. `bootstrap` 100% non-interactif.
- **GitHub** : Auto-merge via PRs/release-please/dependabot. Exception : premier tag `v*` reste human-gated.
- **Choix de design** : Trancher seul sur memories + audits + benchmarks. Priorités (§0, §2, §4).

Si une instruction prescrit un step human-in-loop, **cette règle prévaut** : décider seul et documenter.

## 0.5. Mission — clore PLAN.md ⏳ items

`docs/PLAN.md` recense les items `⏳` actionables sans intervention humaine. Crusher tout ce qui est techniquement faisable.

**Bloqués upstream / human-gated (ne pas tenter)** : PPA Launchpad, Homebrew tap publish, premier tag `v*` (requires human approval), path-bases RFC 3529 (Cargo 1.98), wry GTK4 (CVE pipeline), reqwest 0.13 aws-lc-sys, pyo3 0.22 PyString.

## 1. ZÉRO STUB, 100% PRODUCTION

L'architecture de base est en place. Mode "scaffolding" **interdit**.
- Toute fonction Rust ou C touchée contient sa logique métier complète et réelle.
- Pour Linux : appels `libc`, `nix`, `tokio`, `io_uring` réels. Windows : `windows-rs` direct.
- Aucun `TODO: implement later`. Scaffold interdit : chaque fichier ship une feature observable.

## 2. Politique de langages — monorepo polyglotte

> Pivot 2026-05-21 : aphrody passe de « 100% Rust » à **monorepo polyglotte
> Rust + Bun + Python + Go**, **Rust restant le langage primaire** (cœur CLI,
> systems, FFI). Les trois autres toolchains sont des citoyens de première
> classe pour les surfaces où ils dominent (UI web, ML/data, bridges natifs).
> Rust toolchain pinned `nightly-2026-05-17` via `rust-toolchain.toml` ;
> Bun/Go/Python pinnés via `mise.toml`. Re-pin requires PR.

**Extraction 2026-05-23 : ce dépôt (`C:\src\aphrody`) ne garde QUE le Rust.**
Les trois autres surfaces langage vivent désormais dans des **dépôts FRÈRES
indépendants** (git séparés, pas de submodule) :

| Langage | Dépôt frère | Workspace / manifeste | Toolchain |
|---------|-------------|-----------------------|-----------|
| **Rust** (primaire, ici) | `C:\src\aphrody` | `Cargo.toml` (`crates/*`) | `rust-toolchain.toml` |
| **Bun** (TS/JS) | `C:\src\aphrody-ts` | `package.json` + `bunfig.toml` + `.oxlintrc.json` (`apps/*` + `packages/*`) | `mise.toml` (dans aphrody-ts) |
| **Python** | `C:\src\aphrody-py` | `pyproject.toml` (uv, membres à la racine) | `mise.toml` (dans aphrody-py) |
| **Go** | `C:\src\aphrody-go` | `go.work` | `mise.toml` (dans aphrody-go) |

Le `justfile` racine d'aphrody pilote **uniquement** le workspace Rust
(`just build|test|lint|fmt|ci`). Pour go/python/ts, lancer les runners dans le
dépôt frère correspondant.

- **Rust primaire** : tout code systems/CLI/FFI cross-platform reste Rust nightly (Edition 2024). Le binaire `aphrody` (crate `cli`) ne doit dépendre d'aucune autre toolchain au runtime.
- **Bun** (`aphrody-ts`) : runtime/bundler/test pour TS/JS first-party sous `apps/*`. Lint = **oxlint** (oxc, `.oxlintrc.json`), format = **oxfmt** (oxc, `.oxfmtrc.json`). Web/UI = **Material Web Components 3** (fork `packages/material-web`). Les forks `packages/*` gardent leur PM natif (npm/pnpm) et leur propre `.git`, hors workspace bun.
- **Python** (`aphrody-py`) : géré par **uv**, lint/format **ruff**, tests **pytest**.
- **Go** (`aphrody-go`) : modules agrégés par **go.work** ; `go vet`/`go test`.
- **C/C++** : toujours banni de la distribution, sauf wrappers FFI (`cxx::bridge`).
- **FFI** : `mimalloc` global côté Rust, zero-copy via pointeurs bruts encapsulés.
- **Shell** : wrappers de bootstrap one-shot tolérés ; logique réelle dans une des 4 toolchains.
- **Vercel Rust stack** (`turbopack-*`, `swc-*`, `next-*`, `lightning-css`, `oxc`) déclaré dans `Cargo.toml` racine. Pas de re-vendoring.
- **Supply-chain** : chaque toolchain passe son gate (`cargo deny`/`vet`, `bun`/`npm audit`, `uv`/`pip-audit`, `govulncheck`) — cf. §5.

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

## 4. Architecture

Dépôt **Rust uniquement** (cf. §2). Workspace 57 membres, 71 crates sur disque, 14 exclues. Les surfaces Go / Python / TS-JS (dont les forks UI `packages/*` et `apps/m3-react`) ont été extraites le 2026-05-23 vers les dépôts frères `C:\src\aphrody-{go,py,ts}`.
Détails complets dans [`docs/SOURCE_OF_TRUTH.md`](docs/SOURCE_OF_TRUTH.md).

### Cœur du workspace
- **CLI / cœur** : `cli` (binaire principal `aphrody`), `base` (primitives no_std), `backend` (network).
- **MCP** : `google_mcp` produit le binaire `aphrody-mcp` (`cargo build --release --bin aphrody-mcp`).
- **Term & Chat** : `aphrody-terminal-*` (8 crates), `aphrody-chat`, `aphrody-sdk`, `aphrody-tools`.
- **Modules unifiés** : `mrx`, `aphrody-voice`, `aphrody-design`, `aphrody-messaging`, `aphrody-llm-infra`, `aphrody-skills`, `aphrody-marketplace`.
- **Gemini** : crate Rust `gemini-runtime` (`gemini_runtime::resolve_bin()` résout `$APHRODY_GEMINI_BIN` > sibling de `current_exe()` > PATH).

### Déploiement
`cargo build --release -p aphrody --locked` → `target/release/aphrody.exe`. **Ce build ne met PAS à jour `~/.local/bin/aphrody.exe`** : déployer via `scripts/deploy.{ps1,sh}` (copie le binaire post-build).

### Hors workspace / Archivés
- Hors workspace : `coreutils`, `util-linux`, `vendor/bun/`, `vendor/electron-prebuilt/`.
- Archivés (NE PAS réintégrer) : `google_os`, `bun_ffi`, `google_kv`, `python_ffi`.

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

**`C:\src\winclean\`** est la **spécialisation Windows-only** d'aphrody (C# NativeAOT/C++20, 176 tools P/Invoke `[McpServerTool]`, IEVR).
aphrody est la version **cross-platform** (Linux #1, Rust).
- **Skills/agents** : Génériques (reverse engineering, deep analysis, protocol RE) dans aphrody. Windows-only (Winclean.Mcp, IEVR) dans winclean.
- **Logique** : Si cross-platform, pure Rust dans aphrody. Si Win32 spécifique complexe (DWM, ConPTY), dans winclean.
- **A2A** : le bridge C# `Winclean.A2a.exe` héberge un serveur A2A 1.0 HTTP/JSON-RPC sur `127.0.0.1:5151` (env `WINCLEAN_A2A_PORT`) et spawn `Winclean.Mcp.exe` en stdio. Coordination cross-repo = mailbox fichier `C:\src\winclean\.coord\` (`inbox-from-{aphrody,winclean}.jsonl` + `heartbeat-aphrody.txt`). `:8788` = listener côté aphrody, distinct du `:5151` C#. Détails : [`docs/peer-a2a-mcp-csharp.md`](docs/peer-a2a-mcp-csharp.md). Ne pas toucher aux fichiers uncommitted du peer (`git -C C:\src\winclean status` d'abord).

## 6.5. Skills & agents

Surface skills exposée via le plugin `aphrody` (`.claude/plugins/aphrody/`).
- **Specs / index** : [`docs/cargo/SKILLS.md`](docs/cargo/SKILLS.md).
- **Skills clés** : `start` (mode autonome), `vps-commander` (SSH), `rust-best-practices-2026`, `best-stack-2026` (policy awesome-rust).
- **Agents** : `rust-engineer`, `rust-architect`, `cpp-engineer`, `ffi-architect`, `cargo-auditor`.

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
- **mrx scan** : écrit `path.json`/`monorepo-map.json` dans cwd (gitignored).
- **Verify strictly** : `cargo check` ne suffit pas, tester le comportement réel (curl, exit codes, etc.).
- **Edge headless WebGPU** : requestAdapter pending en headless. Utiliser CDP/Chromedp.
- **Licence GPL** : `unicorn-engine` est GPL-2.0, banni d'aphrody (Apache-2.0) pour éviter contamination.
- **context7 MCP** : max 3 `query-docs` par tour. Déléguer aux sub-agents pour audits massifs.
- **Hallucinations filesystem** : sub-agents peuvent mal lire les dossiers/fichiers. Toujours verify via ls/read.
- **BOM UTF-8 / Mojibake** : caractères corrompus. Préférer `Write` à `Edit` pour fixer les encodages.
- **Pipes shell exit code** : un pipe vers `tail`/`head` masque l'exit code d'origine. Utiliser `; echo "===EXIT=$?==="`.
- **`skill-creator` frontmatter** : `source/version` dans `metadata`, pas de `<` / `>` dans description.
- **`octocrab` Search API** : `.sort("stars")` prend `&str`. Feature-gate `github` active pour isolation CI.
- **Agents parallèles `isolation: "worktree"`** : non fiable (agents partagent parfois le main tree → races HEAD-switch). Intégrer par *harvest* (`git checkout <branch> -- <path>`) + hand-merge des fichiers partagés, jamais `git merge` de bases enchevêtrées. Nettoyer : `git worktree unlock` avant `git worktree remove --force`.
- **Peer agy ↔ main concurrent** : le peer committe sur `main` avec l'identité git `aphrody-code` (identique à la nôtre). Re-vérifier `git log --oneline main..HEAD` avant tout fast-forward ; ne pas supposer la base de branche stable.
- **UI / TS-JS / publish GitHub Packages** : surfaces **extraites vers `C:\src\aphrody-ts`** (2026-05-23). Les gotchas Lit autonome, Bun bundler, collisions `@property`↔Element, build wireit, et `publish-github-packages.ts`/`.npmrc`/`PUBLISHING.md` y vivent désormais. Ce dépôt = **Rust only**.
- **Tests + `reqwest::Client` → panic `No provider set`** (rustls 0.23, providers ring+aws-lc ambigus) : installer le provider dans le `mod tests` via `static ONCE: Once` + `rustls::crypto::ring::default_provider().install_default()` (helper `install_rustls_provider()` ; modèle dans `a2a-client`/`aphrody-gateway`/`aphrody-messaging`) + `rustls = { workspace = true }` en `[dev-dependencies]`.

## 7.5. aphrody-terminal — LLM-first terminal

Conçu pour sub-agents/skills/MCP. Output JSON, markdown rendu inline, config JSON full, bridge LLM↔DOM via agent-browser.
- **Crates** : 8 terminal crates (`aphrody-terminal-*`).
- **OSC sequence** : `aphrody-*` namespace réservé pour extensions LLM (14 séquences).
- **Séquences VT** : 22 séquences VT essentielles gérées (alt screen, mouse SGR, true color...).

## 8. Source of Truth

Pour la vue d'ensemble consolidée (architecture, plateformes, livrables, ressources), lire **[`docs/SOURCE_OF_TRUTH.md`](docs/SOURCE_OF_TRUTH.md)** et [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).
