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
Le code Windows-specific (NTDLL, IOCP, ConPTY, etc.) **ne doit jamais bloquer
la compilation sur Linux** : il est strictement gated `#[cfg(target_os = "windows")]`.

L'ancien sous-projet `google_os` (kernel emulator hybride Win-NT) a été **sorti
du workspace** (archivé sous `C:\google-os-archive\`). Ne pas le réintroduire.

## 0.3. Exception Rust-only : fusion bxc (2026-05-19)

`packages/bxc/` est la **fusion in-tree du projet `aphrody-code/bxc`** (browser
engine Bun + Lightpanda + curl-impersonate Chrome 131 + CDP-compat). Décision
utilisateur 2026-05-19 : on importe le projet bxc *tel quel*, **TS/Bun/Zig
inclus**, suspendant temporairement la règle [[feedback_aphrody_rust_only]]
pour ce sous-arbre uniquement.

Layout en place :

- `packages/bxc/` — mirror complet du repo `aphrody-code/bxc` (12 MB) : sous-workspace
  Bun avec `packages/{api,bxc-extension,llm-extract,omnistack}`, `src/` (cdp,
  ffi, mirror, profiles, scrapers, …), `rust-bridge/` (sous-workspace Rust FFI),
  `bin/bxc`, `Makefile`, `package.json`, `bun.lock` (excluded), `tsconfig.json`,
  `turbo.json`. Exclus du mirror : `.git`, `node_modules`, `target`, `dist`,
  `build`.
- `scripts/bxc/` — 20 scripts utilitaires bxc (`build-windows.{ps1,ts}`,
  `build-standalone.ts`, `cron-scheduler.ts`, `god-mode-executor.ts`,
  `apply-bxc-rebrand.ts`, `backup-bxc.sh`, …) exposés au niveau scripts/ pour
  découvrabilité.
- `docs/bxc/` — `docs/` complet de bxc + tous les .md racine (`CLAUDE.md`,
  `GEMINI.md`, `MEGA-PLAN.md`, `SKILLS.md`, `README.md`, `CHANGELOG.md`,
  `PUBLISHING.md`, `CONTRIBUTING.md`, `AGENTS-ARCHITECTURE.json`).

Conséquences :

- Le CLI `aphrody {tokens, scrape, bxc detect, bxc daemon}` peut désormais
  être branché soit sur `crates/bxc-engine/` (Rust workspace), soit sur
  `packages/bxc/bin/bxc` (Bun binary), au choix du runtime ; pas de duplication
  fonctionnelle — `crates/bxc-engine` reste l'implémentation Rust canonique
  (cf. memory [[project_aphrody_owned_tools]]), `packages/bxc/` est la
  référence amont avec features TS bleeding-edge (MCP server, GraphQL,
  extension VSCode).
- Toolchain ajouté requis pour build : `bun >= 1.3.14`, `zig` (pour
  `packages/omnistack/src/native/omni.zig`). N'affecte pas le build Rust
  workspace (les deux sont indépendants).
- `node_modules/` reste gitignored globalement (`.gitignore:21`).

## 0.4. État binaire installé (snapshot 2026-05-19)

`cargo build --release -p aphrody --locked` → `target/x86_64-pc-windows-msvc/release/aphrody.exe`
(8.3 MB, 3 min 28 s) copié dans `~/.local/bin/aphrody.exe` (PATH déjà résolu,
convention memory `feedback_clone_path_c_src`).
Smoke matrix complète des 27 sous-commandes dans
[`docs/PLAN.md` §P-Test](docs/PLAN.md#phase-p-test--validation-end-to-end-binaire-install%C3%A9-2026-05-19) :
**19 ✅ / 3 ⚠️ / 5 ❌**.

Gaps critiques à clore avant publish-ladder :

- **`bxc-engine` dep manquante** dans `aphrody self bootstrap` — sans ça,
  `aphrody {tokens, scrape, bxc detect, bxc daemon}` échouent out-of-the-box.
  Fix : `cargo install --locked --path crates/bxc-engine` automatisé dans
  bootstrap, ou copie `target/release/bxc-engine[.exe]` → `~/.local/bin/`.
- **`coreutils` / `util-linux` orphans** — `crates/coreutils/` et
  `crates/util-linux/` sont *hors workspace* (cf. §4 "Hors workspace") mais
  les commandes `aphrody coreutils|util-linux` restent wired dans
  `crates/cli/src/main.rs` et plantent `os error 267`. Fix : cfg-gate ou
  retrait des variants `Commands` enum, sinon binaires distincts.
- **`mirror`** silent exit 0 sans log — vérifier intention (no-op ou
  background spawn ?), ajouter `[ok] mirror started (…)` ou `[skip]`.
- **`search`** Google scraping retourne 0 (Google bloque) — ajouter fallback
  DuckDuckGo HTML / Brave Search API.
- **`aphrody version --json`** absent (parity manquante avec
  `aphrody doctor --json` qui marche déjà).

Outputs validés OK : `version`, `doctor` (+`--json`), `self bootstrap --check`,
`completions {5 shells}`, `scan {tree, manifests}`, `dns` (287 sous-domaines
sur google.com), `a2a "ping"` → **"pong"** via fallback Gemini CLI,
`notify` (erreur structurée propre sans creds),
`oc-{onboard, pairing, reset, uninstall, docs}` (incl. `--dry-run` chains),
`chromium sync` (7 profils + master key déchiffrée),
`term` (WS server `ws://127.0.0.1:18799` prêt), `gemini --version` (0.42.0),
`n2b scan` (0.6.0).

## 0.5. MISSION DU JOUR — clore PLAN.md ⏳ items (2026-05-18+)

`docs/PLAN.md` recense ~14 items `⏳` actionables sans intervention humaine.
Cap : crusher tout ce qui est techniquement faisable en un loop YOLO grind.
Liste prioritaire (ordonnée par leverage / verify-time court d'abord) :

| # | Item ⏳ | Cible | Fusion sources | Verify |
|---|---|---|---|---|
| 1 | T-2 VT extensions Ink/React | `crates/aphrody-terminal-vt` | `vte` crate + worktree `C:/worktree/wterm/packages/vt-decoder` + `C:/worktree/terminal/src/terminal/parser` | `cargo test -p aphrody-terminal-vt alt_screen mouse_sgr_1006 osc_52 bracketed_paste decstbm` |
| 2 | T-4 terminal-markdown | NEW `crates/aphrody-terminal-markdown` | `comrak` + `syntect` + OSC dispatch déjà dans `aphrody-terminal-llm/src/osc.rs` | `cargo test -p aphrody-terminal-markdown render_commonmark code_block_highlight osc_aphrody_md_emit` |
| 3 | T-5 terminal-json-out | NEW `crates/aphrody-terminal-json-out` | `serde_json` + framing pattern de `aphrody-terminal-backend/src/ws.rs` | `cargo test -p aphrody-terminal-json-out frame_stdout passthrough_app_json` |
| 4 | T-6 terminal-config | NEW `crates/aphrody-terminal-config` | loader partiel `aphrody-terminal-llm/src/mcp.rs:506` + `schemars` + JSON schema | `cargo test -p aphrody-terminal-config schema_validate import_claude_json import_mcp_json` |
| 5 | T-7 aphrody-terminal-demo.html | NEW `crates/aphrody-wasm/examples/aphrody-terminal-demo.html` | `gemini-clone-pixel-perfect.html` (734l) + M3 tokens via `m3-tokens` | `cargo run -p aphrody-wasm --example serve` + curl HTTP 200 + screenshot via bxc CDP |
| 6 | T-8 audit wterm vs ms-terminal vs aphrody | NEW `docs/audits/2026-05-18-wterm-vs-microsoft-terminal-vs-aphrody-terminal.md` | worktrees + `docs/research/BXC_CARTOGRAPHY.md` template | `wc -l` + `grep -c "^| "` rows ≥ 20 |
| 7 | T-10 aphrody-tui pure Rust | NEW `crates/aphrody-tui` | `ratatui` ref + pattern `a2a-ui/src` native backend | `cargo test -p aphrody-tui dsl_render layout_engine input_event` |
| 8 | A2A CLI agent autonome | `crates/cli/src/auto_command.rs` NEW | `AutoCommand` enum hook dans `Commands` + dispatch via `a2a-client::http_jsonrpc` | `aphrody "what is 2+2" → call A2A → reply` (smoke) |
| 9 | Trait `Transport` portable | `crates/a2a-client/src/transport.rs` refacto | déjà séparé HTTP/SSE/JSON-RPC, ajouter cfg(target_arch="wasm32") fetch path | `cargo check -p a2a-client --target wasm32-unknown-unknown` |
| 10 | Supply-chain `cargo vet suggest` | run + commit nouvelles entrées dans `supply-chain/audits.toml` | aucune fusion, juste run | `cargo vet suggest` + `cargo vet` exit 0 |
| 11 | Audit `safe-to-deploy` crates critiques | mêmes audits | `crates/cli` deps : `rustls`, `reqwest`, `tokio` | `cargo vet --locked` exit 0 |
| 12 | `cargo clippy + miri unsafe sweep` | grep tous `unsafe` workspace → annoter `#[allow(unsafe_op_in_unsafe_fn)]` ou justifier | `crates/aphrody-wasm`, `crates/base` | `cargo +nightly miri test --workspace --lib` exit 0 (best-effort) |
| 13 | Stress tests `cargo bench` | activer `bench` workflow + ajouter benchmark `crates/base/benches/` | `crates/backend/benches/backend_bench.rs` (existe déjà) | `cargo bench --workspace` exit 0 |
| 14 | T-8 demo gif Claude Code dans aphrody-terminal | `assets/aphrody-terminal-demo.gif` | `aphrody term` + Claude Code child process + asciinema → agg/vhs | `file assets/aphrody-terminal-demo.gif` mime=image/gif |

**Bloqués upstream / human-gated (ne pas tenter)** : PPA Launchpad, Homebrew tap publish, premier tag `v*` (requires human approval), a2a-slimrpc (upstream agntcy-slim-mls bug), path-bases RFC 3529 (Cargo 1.98), wry GTK4 (CVE pipeline), reqwest 0.13 aws-lc-sys, pyo3 0.22 PyString.

**Mode d'attaque par défaut** : `/aphrody-yolo-grind` (4 lanes parallèles par tick). Voir §8.

## 1. ZÉRO STUB, 100% PRODUCTION

L'architecture de base est en place. Mode "scaffolding" **interdit**.

- Toute fonction Rust ou C touchée contient sa logique métier complète et réelle.
- Pour le code Linux : appels `libc`, `nix`, `tokio`, `io_uring` (via `tokio-uring`
  ou `io-uring` crate) — pas d'émulation.
- Pour le code Windows : `windows-rs` direct, pas de wrapper artificiel.
- Jamais de `TODO: implement later`. Tu le fais maintenant ou tu ne l'écris pas.
- **Scaffold interdit** (cf. memory `feedback_no_scaffold`) : aucun package vide ni placeholder. Chaque nouveau fichier fusionne ≥3 ressources existantes du workspace ET ship une feature observable (HTTP 200, fichier généré, exit code attendu, NDJSON event émis) — pas juste `cargo check` / `tsc --noEmit`.

## 2. Politique de langages

> Toolchain pinned via `rust-toolchain.toml` to `nightly-2026-05-17`.
> Re-pin requires PR (audit trail).

**Règle absolue (2026-05-18) : aphrody est 100% Rust dans tout le repo.**
Le binaire, le workspace, les scripts, les skills, les MCP servers, le tooling
et la doc-build sont Rust. Aucune exception. Cf. memory
[[feedback_aphrody_rust_only]] (révoque la tolérance précédente accordée
à Bun/TS/Python pour scripting/MCP/tooling périphérique).

- **Tout code** : Rust nightly (Edition 2024). Sans exception.
- **C/C++** : interdit dans le code distribué (`crates/cli`, `crates/base`, etc.).
  Tolerable uniquement pour des wrappers FFI inévitables (`cxx::bridge`).
- **FFI / interop mémoire** : `mimalloc` allocator global, zero-copy via
  pointeurs bruts encapsulés (`crates/bun_ffi` a été archivé — cf. §4).
- **Bun / Node / TypeScript / JavaScript** : **BANNIS**. Tout fichier `.ts`,
  `.js`, `.mjs`, `.cjs`, `package.json`, `tsconfig.json`, `bun.lock`,
  `bunfig.toml`, `turbo.json`, `node_modules/`, `packages/` doit être migré
  vers Rust (`cargo xtask`, binaire dédié dans `crates/`, ou WASM) ou
  supprimé. Toute déclaration MCP `stdio: bun ...` doit être remplacée par
  un binaire Rust shippé. CI ne doit plus invoquer `bun`/`npm`/`node`/`tsc`/
  `turbo`. La memory [[feedback_bun_only]] (Bun préféré à Node) est devenue
  caduque côté repo aphrody : la directive est désormais "Rust préféré à
  tout le reste".
- **Python** : interdit. Les scripts `.py` existants à migrer vers Rust ou
  supprimer.
- **Shell (`.ps1`, `.sh`, `.cmd`, `.bat`)** : à éradiquer dès qu'un
  équivalent Rust existe. Préférer `cargo xtask <op>` ou un binaire
  dans `crates/aphrody-*-tools`. Les wrappers de bootstrap one-shot
  (ex. `dev-setup.sh` pour installer rustup) restent tolérés tant qu'aucun
  binaire Rust ne peut s'auto-installer.
- **Web / UI** (règle 2026-05-17, renforcée) : **WASM Rust natif
  (`wasm32-unknown-unknown` + `wasm-bindgen`) OU WebGPU (`wgpu` crate)** pour
  TOUT projet web. Aucun fallback JS/TS, aucun framework Node-based
  (Next.js inclus — la dep doit être en `[workspace.dependencies]` Rust-only
  via `next-rs` / `turbopack-*`, jamais via `bun install`). shadcn-ui legacy
  → réécrit en wrappers Material Web Components 3 natifs via bxc scraping.
- **Turbopack** + tout l'écosystème Rust Vercel (`turbopack-*`, `swc-*`,
  `next-*`, `lightning-css`, `oxc`) doit être déclaré en
  `[workspace.dependencies]` de `Cargo.toml` racine. Pas de re-vendoring.

## 2.5. Méthodologie docs / versions / fact-checking

**Avant** d'ajouter une dep à `[workspace.dependencies]`, d'écrire un appel
API non trivial, ou de prendre une décision basée sur la doc d'une lib :

- Utiliser le MCP **`context7`** (`resolve-library-id` puis `query-docs`)
  pour vérifier la version courante et l'API actuelle.
- **Combo recommandé** : `context7` pour **API surface / usage patterns** (méthodes,
  traits, signatures) + `WebSearch crates.io` pour **version numbers exacts** —
  context7 indexe les docs, pas toujours le dernier publish manifest. Validé sur
  capstone (0.13→0.14), quiche (0.24→0.28), curl-impersonate (chrome131→chrome146).
  Voir `docs/audits/2026-05-19-hermes-agent-vs-aphrody.md` pour exemples.
- Skip pour : refactoring local, scripts from scratch, debug business logic,
  concepts généraux.

Exemple : avant d'écrire `wgpu = { version = "23" }` → resolve-library-id
"wgpu" → voir versions disponibles (v26, v29) → utiliser la stable courante.

## 3. Commandes de validation (tolérance zéro)

```bash
# Build hermétique — alias définis dans .cargo/config.toml
cargo ci-offline           # = clippy --workspace --all-targets --locked --offline -- -D warnings
cargo xt-offline           # = nextest run --workspace --locked --offline

# Cross-platform (les 3 cibles prioritaires doivent passer)
cargo check -p aphrody --target x86_64-unknown-linux-gnu --locked   # cible #1
cargo check -p aphrody --target x86_64-pc-windows-msvc --locked     # cible #2
cargo check -p aphrody --target wasm32-unknown-unknown --locked     # cible #3

# Supply-chain (Google-grade)
cargo deny check           # CVE + licences + bans + sources
cargo vet                  # audits signés (Google / Mozilla / Fuchsia feeds)

# Compléments
cargo audit-machete        # unused deps
cargo audit-udeps          # nightly unused deps
```

> [!NOTE]
> **`cargo machete` false-positives** (cfg-gated transitive deps) → ajouter
> `[package.metadata.cargo-machete] ignored = ["wasm-bindgen", ...]` dans le
> `Cargo.toml` concerné. Exemples vivants : `aphrody-wasm`, `base`, `a2a-pb`.
>
> **`docs/SUMMARY.md`** est auto-généré par `cargo run -p aphrody-summary`.
> Ne PAS éditer à la main — re-run le script après tout ajout de doc.

## 4. Architecture (post-pivot)

Monorepo **100% Rust** (cf. §2 et memory [[feedback_aphrody_rust_only]]).
Toute trace de Bun/Node/TS/Python/turbo dans `packages/`, `node_modules/`,
`scripts/*.ts`, `bun.lock`, `package.json`, `tsconfig.json`, `turbo.json`
est destinée à être éradiquée (plan : `docs/PLAN.md`).

### Workspace (`Cargo.toml` root, 54 members)

- **CLI / cœur** : `cli` (binaire principal, **cross-platform pur**), `base`
  (no_std primitives), `backend` (forensics + network, cross-platform).
- **Kernel subcommands** (depuis 2026-05-18) :
  - `aphrody n2b [args]` — sous-commande Rust native (refactor en cours, ex-façade
    bun `packages/n2b/src/cli.ts` à supprimer). Cf. `docs/PLAN.md`.
  - `aphrody n2b watch --interval N --path P` — boucle infinie tokio (Ctrl-C trap).
  - `aphrody bxc {daemon,recon,scrape,detect,tokens}` — passthrough bxc-engine `:8765` via `crates/cli/src/scrape.rs::ScrapeClient`.
  - Install PATH : `aphrody self install-path` (binaire Rust natif, ex-`scripts/Install-AphrodyToPath.ps1` / `scripts/install-aphrody-path.sh`).
- **UI desktop** : `gui` (wry + tao) — desktop seulement, exclu du binaire CLI
  distribuable.
- **Agent / IA (A2A)** : `a2a`, `a2a-client`, `a2a-server`, `a2a-pb`, `a2a-grpc`.
  En cours d'adaptation cross-platform.
- **Bridges** : `google_mcp` (MCP server, en cours d'adaptation cross-platform).
- **Mapper (mrx)** : `mrx-core`, `mrx-detect`, `mrx-audit`, `mrx-watch`, `mrx-cli`
  (Monorepo Real-time X-platform mapper — migré 2026-05-17 depuis vps/packages/mrx).
- **Outils dev** : `aphrody-translate` (CLI traduction commentaires EN→FR + scrub AI
  + style Aphrody).

### Hors workspace (`exclude` du root)

- `crates/coreutils/`, `crates/util-linux/` : userland GNU, conservés en référence.
- `vendor/bun/` : runtime Bun fork (path deps depuis nos crates).
- `vendor/electron-prebuilt/` : binaires Electron.
- `crates/a2a-slimrpc/` : ré-intégration prévue (cf. PLAN).

### Archivé hors repo

- `crates/google_os/` → `C:\google-os-archive\20260517-*\`. NE PAS réintégrer
  sans accord explicite.
- `crates/bun_ffi/` → `C:\aphrody-archive\bun_ffi-20260517-*\`. FFI V8↔Rust
  archivé : pollue le workspace Rust pour zéro bénéfice côté cli pur.
- `crates/n2b/` → `C:\aphrody-archive\n2b-20260517-*\`. Migration tool Node→Bun
  trop spécialisé, deps lourdes (oxc_parser, fastembed). **Réintégré via
  upstream `aphrody-code/n2b` branche `aphrody`** (cf. Cargo.toml workspace.dependencies).
- `crates/google_kv/` → `C:\aphrody-archive\google_kv-*\`. Orphan, aucun consumer.
- `crates/python_ffi/` → `C:\aphrody-archive\python_ffi-*\`. Orphan, dépend
  de vendor/bun. Pour AI / MD : Rust pur via `candle`, `comrak`, etc.

## 4.1. Scripts d'automatisation haute-perf (`scripts/`)

Wrappers `aphrody n2b` / `aphrody bxc` parity bash↔pwsh (NDJSON streamable, p50/p95, SIGINT trap).

- `n2b-batch.{ps1,sh}` — migration parallèle (`ForEach-Object -Parallel` / `xargs -P`), NDJSON par target.
- `bxc-crawl.{ps1,sh}` — crawl parallèle URLs × actions (recon|detect|tokens), `--loop --interval N`, cache body-hash.
- `bxc-supervise.{ps1,sh}` — watchdog daemon bxc, NDJSON heartbeats, auto-restart cooldown.
- ~~`bunnize-gemini-cli.ts`~~ (déprécié) — l'outil de migration node→bun est sans objet depuis la politique [[feedback_aphrody_rust_only]] (2026-05-18). Tout fork JS/TS doit être migré vers Rust natif, pas vers Bun.
- **Gotcha pwsh** : pour trap Ctrl-C, jamais `[Console]::CancelKeyPress.Add({...})` — utiliser `[System.ConsoleCancelEventHandler]` delegate + `add_CancelKeyPress`. Pour here-strings sans expansion : single-quoted `@'..'@` (sinon backticks consommés).

## 5. Supply-chain (lire avant tout PR qui touche `Cargo.toml`)

- **Pas de `cargo vendor`** — repo lockfile-only (depuis 2026-05-16).
- **Toute nouvelle dep** doit passer `cargo deny check`.
- **Toute dep transitive non auditée** doit avoir un audit `cargo vet` ou une
  exemption justifiée dans `supply-chain/config.toml`.
- **Lints workspace** : voir `[workspace.lints]` dans `Cargo.toml`. Pedantic/
  nursery/style en `allow` workspace-wide, à activer per-crate hardenée via
  `#[warn(clippy::pedantic)]`.

## 6. Conventions de contribution

- Commits = Conventional Commits (`feat:`, `fix:`, `refactor:`, `build:`, ...).
  Pas de mock, pas de fake data.
- **Linux est la cible #1** : si ça ne compile pas sur Linux, ça ne mergeable pas.
- Process : lis `OpenProcess`+`NtQuerySystemInformation` (Win) **ET**
  `/proc/<pid>` (Linux). DNS : vraie résolution. IO : `io_uring` (Linux),
  `IOCP` (Windows).
- Avant push : `cargo ci-offline && cargo deny check` doit être vert sur
  Linux d'abord.
- `a2a-slimrpc` n'est pas dans `workspace.members` — ne pas l'y remettre tant
  qu'`agntcy-slim-mls` n'est pas fixé upstream.

## 6.1. A2A coordination cross-Claude (`ai.json` v1, in-tree depuis 2026-05-19)

Ce repo expose un manifest A2A AGNTCY a2a v1.0 (`lf.a2a.v1`, `ai.json` à la racine)
et un schéma channel-extension (`schemas/ai.json/v1.json`). Discovery thin via
`.well-known/ai.json` (HTTP-friendly).

**Source de vérité A2A in-tree** : `aphrody/ai/` (canonical depuis 2026-05-19,
ex-`C:\winclean\.coord\` qui reste la source de vérité côté peer winclean
uniquement). Déclarée par `ai.json` racine → `spec.coord_dir` + extension
`file-transport/v1.params`.

- **`ai/heartbeat.txt`** — proof-of-life ISO-8601 + résumé de session courante (écriture exclusive aphrody).
- **`ai/outbox.jsonl`** — NDJSON append-only des messages **émis** par aphrody (canonical sortant).
- **`ai/inbox.jsonl`** — NDJSON append-only des messages **reçus** par aphrody (canonical entrant).
- **`ai/peers/<name>.ai.json`** — snapshot local de chaque peer (refresh manuel via `cp` ou auto via futur `aphrody a2a sync-peers`).
- **`ai/README.md`** — documentation du protocole + convention envelope.

**Compatibilité legacy** : pendant la transition, aphrody continue d'écrire
ses messages sortants en **miroir** dans `C:\winclean\.coord\inbox-from-aphrody.jsonl`
pour back-compat avec le peer winclean. Le peer winclean est désormais
**attendu de lire `ai/outbox.jsonl`** comme canonical (cf. `peers[0].peer_outbox_path` et
`peers[0].local_mirror` dans `ai.json`).

**Channels secondaires** (toujours valides) :
- **HTTP listener** sur `:8788` (`bun run C:/winclean/.coord/listener.ts`)
  expose `/ping`, `/msg`, `/inbox`, `/ai.json` — côté winclean uniquement, pas in-tree aphrody.
- **Git tags** `aphrody-*` dans winclean repo pour signaux out-of-band.
- **`process_inspect`** (`ps -ef`) pour détecter activité live de l'autre.

**Workflow par tick** :
1. Lire `ai/peers/winclean.ai.json` (peer state) + `ai/inbox.jsonl` (peer asks).
2. Faire le travail.
3. Append fact/reply envelope à `ai/outbox.jsonl` (canonical).
4. Append mirror copy à `C:\winclean\.coord\inbox-from-aphrody.jsonl` (back-compat).
5. Bump `ai/heartbeat.txt` (ISO-8601 + 1-line summary).
6. Bump `C:\winclean\.coord\heartbeat-aphrody.txt` (back-compat mirror).

Toute écriture cross-repo doit être précédée d'un `fact` via `outbox.jsonl`.

**Politique gitignore** : la **structure** in-tree (`ai/README.md`, `ai/peers/.gitkeep`)
est trackée ; le **contenu transient** (`heartbeat.txt`, `inbox.jsonl`, `outbox.jsonl`,
`peers/*.ai.json`) est gitignored (cf. `.gitignore` §22).

- **`/a2a-duel-loop`** : 1 tick A2A par invocation, paire avec `/loop 60s /a2a-duel-loop`.
  Script `.claude/skills/a2a-duel-loop/scripts/duel-cycle.ts` (flags `--iteration --side --type --re --dry-run`).
- **Ievr ops aphrody-side** : ~~`scripts/ievr-serve.ps1`~~ / ~~`scripts/ievr-verify.ps1`~~
  (ps1 + bun) → à porter vers `aphrody ievr {serve,verify}` (sous-commande
  Rust native, voir `docs/PLAN.md`).
- **Concurrent peer Claude** dans le même repo : `git status` avant chaque edit ;
  ne jamais modifier les fichiers en cours d'édition uncommitted du peer
  (catastrophe garantie sur `Cargo.lock` et workspace `Cargo.toml`).

## 6.5. Skills & agents (`.claude/`)

Toute la surface skills est centralisée et documentée :

- **Inventaire + spec** → `docs/cargo/SKILLS.md` (format SKILL.md, runtime, ajout).
- **Index local** → `.claude/skills/README.md`.
- **Skills projet** : `start` (autonomous mode), `vps-commander` (SSH tunnel).
- **Agents projet** : `cargo-auditor`, `cpp-engineer`, `ffi-architect`,
  `rust-architect`, `rust-engineer`.
- **Runtime** : `skill` crate (workspace dep, lib) + binaires `skill-cli` /
  `agent-skills-cli` (validateur).
- **Sync catalogue externe** : `aphrody xtask skills-sync vercel-labs/agent-skills`
  ou `aphrody xtask skills-sync anthropics/skills` (sous-commande Rust native ;
  l'ancien `scripts/skills-sync.ts` est en cours de port — cf.
  `docs/PLAN.md`).

## 7. Pièges connus (mémoire institutionnelle)

- **aws-lc-sys** : pull via reqwest's `rustls-tls`. Sur Windows : compile via
  NASM prebuilt + Ninja (variables `AWS_LC_SYS_PREBUILT_NASM=1`,
  `CMAKE_GENERATOR=Ninja` dans `.cargo/config.toml`). Sur Linux : OpenSSL
  système (`apt install pkg-config libssl-dev` sur Ubuntu).
- **rustls 0.23 CryptoProvider** : le binaire panic `No provider set` au boot
  si `rustls::crypto::ring::default_provider().install_default()` n'est pas
  appelé AVANT le premier `reqwest::Client::new()`. Cf. `crates/cli/src/main.rs:160`.
- **cargo-zigbuild + `--icf=all`** : incompatible — zigcc rejette le flag.
  Retiré de `.cargo/config.toml` pour x86_64-unknown-linux-gnu. `--gc-sections` reste.
- **`a2a-pb` build.rs** : `src/gen/` est l'authoritative source ; codegen
  `tonic_prost_build` gated derrière `A2A_PB_REGEN=1` (sinon crates.io rejette :
  build scripts must only write to `$OUT_DIR`).
- **Package `cli` renommé `aphrody`** : utiliser `-p aphrody` partout (build,
  check, workflows, scripts). Le dir reste `crates/cli/` (historical, évite
  `git mv` churn).
- **tracing-subscriber** pinné à `0.3.22` (0.3.23+ a un bug `mod env` packaging).
- **`base = ...` (path-bases RFC 3529)** : feature instable nightly 1.97, à
  activer quand stable.
- **rand 0.8 imposé** (pas 0.9) par `denokv_proto`.
- **GTK3 CVE** (RUSTSEC-2024-04xx) : tirés par tao/wry sur Linux, ignorés dans
  `deny.toml` jusqu'à migration GTK4. Le binaire `cli` n'est PAS lié à GTK —
  seul `crates/gui` l'est, et `gui` n'est pas dans le pipeline `cli`.
- **wasm** : `tokio` ne compile pas tel quel — utiliser features sélectives
  (`tokio-stream` + `js-sys` + `wasm-bindgen-futures` pour le runtime web).
- **Steam download monitor** : `du steamapps/downloading/<id>/` ment — Steam pré-alloue
  les fichiers en sparse zeros dès l'event `preallocated N files (Y MB)` du
  `Steam/logs/content_log.txt`. Source vérité : parser `update started: download A/B,
  stage C/D` + `Current download rate: X Mbps`. Manifest `.acf BytesDownloaded` est
  stale (refresh rare).
- **mrx scan cwd** : écrit `path.json` + `monorepo-map.json` dans le cwd par défaut
  — gitignored au root (cf. `.gitignore` §20, commit `d89bcb8f3`).
- **Verify = observable, pas typecheck seul** : `cargo check` / `tsc --noEmit` / `bash -n` ne prouvent QUE la compilation. Toujours coupler avec un comportement vérifiable (curl HTTP 200, fichier généré, NDJSON event émis, exit code attendu, screenshot bxc CDP, audio bytes synth). Sinon FAIT = INCOMPLET déguisé.
- **Edge headless WebGPU** : `msedge --headless=new --enable-features=Vulkan,WebGPU
  --enable-unsafe-webgpu --virtual-time-budget=10000` insuffisant — `requestAdapter`
  reste pending au moment du screenshot. Gates 3-5 du 5-point UI gate exigent
  Playwright/chromedp CDP-driven ; `bxc` (peer côté winclean) est `gpu_capable=false`
  (HTML/DOM only via Lightpanda).
- **`npx` plante avec `EDUPLICATEWORKSPACE`** dans ce repo car `package.json` racine
  liste 2 fois `@google/gemini-cli` (`packages/gemini-cli/` + `packages/gemini-cli/packages/cli`).
  Tout MCP server stdio configuré en `command: "npx"` échoue silencieusement avec MCP
  error -32000 (`ConnectionClosed: initialize request`). Workaround validé : utiliser
  `bunx` (bun ignore les workspace package.json parents). Concerne `context7`,
  `playwright`, et tout futur MCP stdio npm-based. Fix permanent : dédoublonner les
  workspaces du `package.json` racine.
- **License GPL viral** : `unicorn-engine 2.x` (CPU emulator Rust) est **GPL-2.0**.
  aphrody est Apache-2.0 — tout `cargo add` d'un crate GPL contamine le binaire entier
  (linking-time viral). Vérifier license avant pin via `cargo info <crate>` ou
  `cargo deny check` (le `[licenses]` block dans `deny.toml` doit bloquer GPL/AGPL).
  Concerne surtout les crates reverse engineering, emu, crypto, vidéo.
- **context7 MCP — limite 3 query-docs par question** (documenté dans le tool description).
  Pour fact-check massif (audit de N libs), déléguer à un sub-agent général-purpose qui
  peut faire N questions séquentielles, pas appeler le tool N fois directement. Combo
  méthodologique : voir §2.5.
- **Sub-agents Explore peuvent halluciner sur le filesystem** : ils déclarent parfois
  des dossiers vides alors qu'ils contiennent des fichiers, ou des frontmatters
  "malformés" qui sont juste mojibake. Toujours `Bash ls` / `Read` direct pour verify
  avant action destructive (`rm`, `Write` overwrite, `git rm`).
- **BOM UTF-8 + mojibake double-encoding** dans certains `.md` legacy (caractères
  `Ã©` au lieu de `é`, `â€"` au lieu de `—`, BOM `﻿` invisible en début de fichier).
  Origine : Windows codepage conversion. Le `Write` tool produit UTF-8 propre sans
  BOM — préférer à `Edit` quand on touche l'encoding d'un fichier suspect.

## 7.5. aphrody-terminal — LLM-first terminal (pivot 2026-05-18)

`aphrody-terminal` n'est **pas** un wterm clone ni un Windows Terminal clone.
C'est un terminal **LLM-first** conçu pour sub-agents, skills, hooks, MCP,
Ink/React TUIs (Claude Code, Gemini CLI), avec **JSON output partout**,
**markdown rendu inline**, **config JSON full**, **bridge LLM↔DOM** natif via
bxc + agent-browser + edge headless fallback.

- **Spec normative** : [`docs/design/aphrody-terminal-spec.md`](docs/design/aphrody-terminal-spec.md)
- **Stack** : `crates/aphrody-terminal-{vt, wasm, backend, llm, browser, markdown, json-out, config}` (8 crates ciblés, 5 shippées au 2026-05-18).
- **Worktrees référence** : `C:/worktree/wterm` (vercel-labs, API surface),
  `C:/worktree/terminal` (microsoft, Buffer/Renderer/AtlasEngine/ConPTY/profiles.schema.json
  algorithmes — référence only, jamais lié sur Linux/WASM per
  memory `project_terminal_integration_policy`).
- **MCP bxc** wired dans `.mcp.json` (stdio, 7 tools : `tune_memory_sqlite`,
  `vision_analyze`, `start_scraping_subagent`, `auto_detect_skills`,
  `bxc_cdp_{snapshot,evaluate,logs}`). SQLite memory à `var/data/bxc-memory.sqlite`
  (gitignored).
- **OSC namespace** `aphrody-*` réservé pour extensions LLM (14 sequences :
  7 LLM event bus + 7 browser automation).
- **Ink/React TUI compat** : 22 séquences VT essentielles documentées (alt
  screen 1049h, mouse SGR 1006, true color 24-bit, cursor save/restore,
  bracketed paste, DECSTBM, OSC 0 title, OSC 52 clipboard, etc.).

## 7.6. Workflow YOLO grind par défaut (`/aphrody-yolo-grind`)

Mode d'attaque pour clore §0.5 (PLAN ⏳). 1 invocation = 1 tick = 4 agents background parallèles.

- Skill : `.claude/skills/aphrody-yolo-grind/SKILL.md`. Pair avec `/loop 30s /aphrody-yolo-grind` pour run continu.
- Par tick : rank top-4 ⏳ par leverage (mission-direct > publish-ladder > hygiène) puis dispatch 4 Agent calls `run_in_background: true` en **un seul message** (jamais sérialisé).
- Agents disponibles (`.claude/agents/`) : `yolo-prod-ready` (généraliste preferred), `rust-engineer`, `rust-architect`, `cpp-engineer`, `ffi-architect`, `cargo-auditor`.
- Prompt sub-agent : <300 mots, deliverable précis, verify command observable (cf. §7), footer honest-delivery obligatoire (FAIT/INCOMPLET/NON_FAIT), explicit "DO NOT commit" (orchestrateur batch-commit en fin de tick).
- Batch-commit en fin de tick : Conventional Commits, message scope groupé, footer per-deliverable, `Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>` autorisé (seul commit footer permis ; aucun fichier livré ne mentionne Claude/AI/Anthropic).
- Heartbeat A2A : bump `C:\winclean\.coord\heartbeat-aphrody.txt` + drop fact envelope `inbox-from-aphrody.jsonl` par tick.
- Break-loop : 3 ticks consécutifs avec 0 FAIT → PLAN exhausted, surface au user.

## 8. Source of Truth

Pour la vue d'ensemble consolidée (architecture, plateformes, livrables,
ressources), lire **[`docs/SOURCE_OF_TRUTH.md`](docs/SOURCE_OF_TRUTH.md)** —
fusion des anciens `CLAUDE.md` / `GEMINI.md` / `docs/PLAN.md` / `docs/DESIGN.md`.

Compléments connexes :

- [`docs/iecode-public-endpoints.md`](docs/iecode-public-endpoints.md) — catalog
  APIs publiques iecode/IEVR (azalee GraphQL, Steam Store/SteamSpy, inagle source).
- [`docs/WINCLEAN-AUDIT.md`](docs/WINCLEAN-AUDIT.md) — audit cross-repo C:/winclean
  (réutilisation, skills à importer, licences).
- [`docs/posts/2026-05-ai-json.md`](docs/posts/2026-05-ai-json.md) — dev journal du
  protocole A2A file-based (channels, envelope, 3-deep handshake).
