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
> Bun/Python pinnés via `mise.toml`. Re-pin requires PR. (Go retiré 2026-05-31.)

**Réunification 2026-05-27 : retour au monorepo unique (`C:\src\aphrody`).**
L'extraction du 2026-05-23 vers des dépôts frères est **annulée** : les trois
autres surfaces langage sont **rapatriées en sous-dossiers de CE dépôt** (un
seul git, snapshot — l'historique propre des frères reste sur leurs remotes
GitHub) :

| Langage | Emplacement (in-tree) | Workspace / manifeste | Toolchain |
|---------|-----------------------|-----------------------|-----------|
| **Rust** (primaire) | racine `crates/*` | `Cargo.toml` | `rust-toolchain.toml` |
| **Bun** (TS/JS) | `ts/` (`ts/apps/*`) | `ts/package.json` + `ts/bunfig.toml` + `ts/.oxlintrc.json` | `ts/mise.toml` |
| **Python** | `py/` | `py/pyproject.toml` (uv) | `py/.python-version` |

> **Suppression Go (2026-05-31)** : la surface Go (`go/` — `gogcli` +
> `antigravity-langserver-re`) a été **entièrement supprimée** du dépôt. aphrody
> est désormais **Rust + Bun + Python**. Le tokenizer Go n'était pas câblé (aucun
> binaire construit) : `aphrody-context` utilise son fallback heuristique et peut
> consommer un binaire externe optionnel via `APHRODY_TOKENIZER_GO_BIN`.

Chaque sous-dossier garde son `CLAUDE.md`, son `.gitignore` (imbriqué — git le
respecte, donc `.venv`/`node_modules`/`target`/caches restent ignorés) et ses
runners natifs (`uv`/`ruff`/`pytest` dans `py/`, `bun`/`oxlint` dans `ts/`).
Le `justfile` racine pilote **le workspace Rust** ; pour python/ts, lancer les
runners dans le sous-dossier correspondant. Les remotes `aphrody-{py,ts}` sur
GitHub sont désormais des archives gelées.

> **Fusion `material-web` → aphrody (2026-06-01)** : le monorepo Material Design 3
> autonome a été **fusionné dans CE dépôt**. Les 9 bibliothèques `@aphrody-code/*`
> (`packages/{material-web,react,m3-tokens,m3-motion,m3-theme,m3-design,eslint-plugin-m3,doc-ai,bun-rs}`)
> sont désormais **membres du workspace bun** (`workspace:*`, plus de `.git`/npm/pnpm
> séparés) + `examples/showcase`. Outillage : **Bun + Turborepo** (`turbo.json`,
> `bun run build` = `turbo run build --filter=@aphrody-code/*`), catalog partagé +
> `patchedDependencies` (MCU 0.4.0, @webgpu/types) à la racine. `packages/bun-rs`
> (FFI Rust) est **exclu du workspace Cargo** (self-rooted `[package]`). Publication
> GitHub Packages : tag `m3-v*` → `.github/workflows/release-m3-packages.yml`
> (`bun publish` inline les `workspace:*`). Une app consommatrice : `apps/web`
> (client **grand public**, React + m3-react + TanStack, LLM custom shenron/rpbey). Le **cœur Rust reste 100 % Rust**.

- **Rust primaire** : tout code systems/CLI/FFI cross-platform reste Rust nightly (Edition 2024). Le binaire `aphrody` (crate `cli`) ne doit dépendre d'aucune autre toolchain au runtime.
- **Bun** (`@aphrody/ts`) : runtime/bundler/test pour TS/JS first-party sous `apps/*`, `packages/*` et `examples/*` (Turborepo). Lint = **oxlint** (oxc, `.oxlintrc.json`), format = **oxfmt** (oxc, `.oxfmtrc.json`). Web/UI = **Material Design 3** : la lib Lit `@aphrody-code/material-web` + les wrappers React `@aphrody-code/m3-react` et leurs sœurs (`m3-tokens`, `m3-motion`, `m3-theme`, `m3-design`, `eslint-plugin-m3`, `doc-ai`, `bun-rs`). Depuis la fusion 2026-06-01 ces packages sont **membres du workspace bun** (plus de `.git`/npm/pnpm séparés) et publiés sur GitHub Packages sous `@aphrody-code/*` (cf. note §2).
- **Python** (`aphrody-py`) : géré par **uv**, lint/format **ruff**, tests **pytest**.
- **C/C++** : toujours banni de la distribution, sauf wrappers FFI (`cxx::bridge`).
- **FFI** : `mimalloc` global côté Rust, zero-copy via pointeurs bruts encapsulés.
- **Shell** : wrappers de bootstrap one-shot tolérés ; logique réelle dans une des 3 toolchains.
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

Dépôt **monorepo polyglotte** (cf. §2) : Rust primaire (`crates/*`, le gros du workspace) + surfaces in-tree Bun/TS (`apps/*`, `examples/*`, `packages/*` — dont les 9 libs Material Design 3 `@aphrody-code/*` fusionnées le 2026-06-01, + `native/x/beyblade`) et Python (`py/`). L'extraction du 2026-05-23 vers des dépôts frères a été **réunifiée le 2026-05-27** ; la surface Go a été **supprimée le 2026-05-31** ; le monorepo `material-web` a été **fusionné le 2026-06-01**.
Détails complets dans [`docs/SOURCE_OF_TRUTH.md`](docs/SOURCE_OF_TRUTH.md).

### Cœur du workspace
- **CLI / cœur** : `cli` (binaire principal `aphrody`), `base` (primitives no_std), `backend` (network).
- **MCP** : `google_mcp` produit le binaire `aphrody-mcp` (`cargo build --release --bin aphrody-mcp`).
- **Term & Chat** : `aphrody-terminal-*` (8 crates), `aphrody-chat`, `aphrody-sdk`, `aphrody-tools`.
- **Modules unifiés** : `mrx`, `aphrody-voice`, `aphrody-design`, `aphrody-messaging`, `aphrody-llm-infra`, `aphrody-skills`, `aphrody-marketplace`.
- **Gemini** : crate Rust `gemini-runtime` (`gemini_runtime::resolve_bin()` résout `$APHRODY_GEMINI_BIN` > sibling de `current_exe()` > PATH).

### Déploiement

**Canonical : [`DEPLOY.md`](DEPLOY.md)** (VPS Linux, MCP, A2A, Python systemd `:8082`, bxc pair [`../bxc/DEPLOY.md`](../bxc/DEPLOY.md)).

- Linux : `CARGO_CONFIG=.cargo/config.linux-vps.toml` + `cargo build --release --target x86_64-unknown-linux-gnu -p aphrody -p google_mcp` → install depuis `target/x86_64-unknown-linux-gnu/x86_64-unknown-linux-gnu/release/`.
- Copie agents : `scripts/deploy.sh` ou `bash scripts/vps-deploy-bxc-aphrody.sh` + `scripts/vps-sync-agent-stack.sh`.
- Mémoire VPS : `~/.aphrody/workspace/MEMORY.md` · doc layout `docs/dot-aphrody/README.md` · Claude project memory `~/.claude/projects/-home-ubuntu-aphrody/memory/MEMORY.md`.
- **Ne pas confondre** Rust CLI `aphrody` et service Python `aphrody.service` (`/opt/aphrody`, port 8082).
- Un seul `cargo build --release` à la fois (LTO) ; tuer les builds dupliqués avant `cargo clean`.

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

- **aphrody doctor nécessite ai.json** : Bien que le transport file-based soit historiquement obsolète au profit de gRPC, la commande `doctor` exige la présence de `ai.json` et `.well-known/ai.json` à la racine du dépôt pour réussir. Sans eux, elle retourne un verdict `UNHEALTHY` (exit code 1).
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
- **mrx scan** : écrit `path.json`/`monorepo-map.json` dans `<root>` (= `--root`, défaut `$VPS_ROOT`/`$HOME/vps`), **pas le cwd** — vérifié 2026-06-04. Rediriger via `--out`/`--map` pour ne pas polluer un repo scanné.
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
- **UI / TS-JS / publish GitHub Packages (fusionné 2026-06-01)** : le monorepo Material Design 3 est **de retour dans CE dépôt** (`packages/*` + `apps/*` + `examples/*`). Gotchas vérifiés à la fusion : (a) bun ne **hoist pas** les packages workspace à la racine `node_modules` → un import bare *self* d'un package vers lui-même ne résout pas ; `packages/material-web/labs/gb` utilisait `@aphrody-code/material-web/…` au lieu d'un chemin relatif → corrigé en relatif (sinon `tsc` TS2307 à l'émission `.d.ts` de m3-react). (b) `sass-embedded` doit être déclaré explicitement (`packages/material-web` devDep) — il était hoisté ambigument dans l'ancien dépôt. (c) `examples/showcase` doit déclarer `@material/material-color-utilities` en dep directe. (d) `bun publish` inline les `workspace:*` depuis le champ `version` de `bun.lock` (cf. CLAUDE.md global). (e) `turbo.json` est ignoré par défaut dans `.gitignore` racine → `git add -f`. Publication : tag `m3-v*`.
- **Extraction `x-client` / `@aphrody-code/x` (2026-05-31)** : le module de crawling/RAG Bun `packages/x` a été extrait vers le dépôt autonome `/home/ubuntu/x-client` (package `@aphrody-code/x`, registre `npm.pkg.github.com`). Les dépendances/chemins dans les dépôts clients `rg` et `rpbey` doivent être mis à jour pour consommer le binaire compiled `dist/` ou pointer vers le nouveau chemin local.
- **Tests + `reqwest::Client` → panic `No provider set`** (rustls 0.23, providers ring+aws-lc ambigus) : installer le provider dans le `mod tests` via `static ONCE: Once` + `rustls::crypto::ring::default_provider().install_default()` (helper `install_rustls_provider()` ; modèle dans `a2a-client`/`aphrody-gateway`/`aphrody-messaging`) + `rustls = { workspace = true }` en `[dev-dependencies]`.
- **Imports Python / conflit de namespace** : l'exécution de `pytest` depuis le dossier `py/` résout par défaut le dossier de configuration `py/aphrody` comme un namespace package vide (avec `__file__ = None`), ce qui provoque l'échec de l'import de `__version__`. Toujours exécuter ou préfixer les tests avec la variable d'environnement `PYTHONPATH=aphrody` pour forcer la résolution vers le package source `py/aphrody/aphrody/`.

## 7.5. aphrody-terminal — LLM-first terminal

Conçu pour sub-agents/skills/MCP. Output JSON, markdown rendu inline, config JSON full, bridge LLM↔DOM via agent-browser.
- **Crates** : 8 terminal crates (`aphrody-terminal-*`).
- **OSC sequence** : `aphrody-*` namespace réservé pour extensions LLM (14 séquences).
- **Séquences VT** : 22 séquences VT essentielles gérées (alt screen, mouse SGR, true color...).

## 8. Source of Truth

Pour la vue d'ensemble consolidée (architecture, plateformes, livrables, ressources), lire **[`docs/SOURCE_OF_TRUTH.md`](docs/SOURCE_OF_TRUTH.md)** et [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).
