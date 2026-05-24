<!-- SPDX-License-Identifier: Apache-2.0 -->
# aphrody — Source of Truth

> **Document unique de référence** consolidant l'état du workspace.
> Lire celui-ci en premier.
> Mis à jour : **2026-05-21** (workspace lean : 57 membres, suppression
> n2b/bxc/xtask, transport A2A 100 % gRPC).

---

## 0. TL;DR

| Quoi | Valeur |
|---|---|
| **Nom du projet** | `aphrody` |
| **Binaire distribué** | `aphrody` (cross-platform pur) |
| **Repo GitHub** | `https://github.com/aphrody-code/aphrody` (privé initialement) |
| **Stack** | Rust nightly (Edition 2024). 100 % Rust : pas de bun/node/python. |
| **Workspace** | 57 membres actifs (71 crates sur disque, 14 exclus) |
| **Plateformes** (ordre strict) | (1) Linux Ubuntu 26.04 → (2) Windows 11 Insider Canary → (3) WebAssembly → (4) macOS (best-effort) |
| **Licence** | Apache 2.0 |
| **Status** | `1.0.0-canary`, pre-LTS |
| **Pivot date** | 2026-05-17 (abandon "Google OS hybride", focus CLI portable) |

## 1. Mission & non-mission

### Mission

Livrer **le CLI ultime cross-platform** :

- Un binaire `aphrody` qui *fonctionne réellement* sur Linux Ubuntu 26.04,
  Windows 11 Insider Canary Build, et en lib WebAssembly (`wasm32-wasi` +
  `wasm32-unknown-unknown`).
- Une expérience CLI moderne (a2a agents intégrés, MCP server natif).
- Une supply-chain Google-grade (`cargo-vet` + `cargo-deny` + lockfile-only).
- Un workspace Rust hermétique reproductible bit-à-bit.

### Objectifs structurants

1. **Material Design 3 natif** : tokens (`m3-tokens`), icônes (`aphrody-icons`),
   renderer wgpu (`mui-rs*`, exclu par défaut) et intégration React
   (`aphrody-react-reconciler`).
2. **Plugin Claude Code aphrody** : serveur MCP natif unique `aphrody-mcp`
   (15 tools), commandes `/status` + `/docs`, catalogue d'agents/skills.
3. **Intégration de l'écosystème Rust Vercel** (swc-*, lightningcss, mdxjs, oxc)
   déclarée dans `Cargo.toml workspace.dependencies` pour les crates `tuono*`
   (exclues du build par défaut).

### Règles transversales

- **Web/UI** : tout nouveau projet web cible **WASM Rust natif** ou **WebGPU**.
  Pas de fallback JS/TS.
- **Zéro stub / zéro placeholder / zéro scaffolding** : toute feature commencée
  doit être finie. Toute fonction doit faire ce qu'elle prétend faire.
- **Linux Ubuntu 26.04 = cible #1 bloquante**. Ne compile pas Linux → ne merge pas.

### Non-mission

- **Pas** un OS, un kernel, ni un émulateur Windows-NT (le sous-projet
  `google_os` a été *archivé hors du repo* le 2026-05-17 — voir §7).
- **Pas** un fork de Windows Terminal ni un moteur de rendu Direct3D.
- **Pas** un wrapper node.js / npm.

## 2. Plateformes — priorités absolues

| Rang | Plateforme | Triple | Statut bloquant pour merge |
|---|---|---|---|
| **#1** | Linux Ubuntu 26.04 | `x86_64-unknown-linux-gnu` | **Oui** |
| **#2** | Windows 11 Insider Canary | `x86_64-pc-windows-msvc` | **Oui** |
| **#3** | WebAssembly (lib) | `wasm32-wasi`, `wasm32-unknown-unknown` | **Oui** |
| #4 | macOS | `x86_64-apple-darwin`, `aarch64-apple-darwin` | Best-effort |
| #5 | Android | `aarch64-linux-android`, `x86_64-linux-android` | Best-effort (CI active mais non bloquante) |

Toute introduction de code Windows-specific dans le binaire `cli` doit être
gated `#[cfg(target_os = "windows")]` *et* doit avoir un équivalent Linux
fonctionnel via `#[cfg(target_os = "linux")]`.

## 3. Architecture

> Détail complet : [`docs/ARCHITECTURE.md`](ARCHITECTURE.md),
> [`docs/cargo/WORKSPACE.md`](cargo/WORKSPACE.md),
> [`docs/cargo/CRATES.md`](cargo/CRATES.md).

### Workspace Rust — 57 membres actifs

Familles principales (inventaire exhaustif dans `CRATES.md`) :

| Famille | Crates clés |
|---|---|
| **Cœur** | `cli` (binaire `aphrody`), `base`, `backend`, `mrx` |
| **A2A** | `a2a` (`a2a-lf`), `a2a-client`, `a2a-server`, `a2a-pb`, `a2a-grpc`, `a2a-ui`, `google_mcp` |
| **LLM/agent** | `aphrody-llm-infra`, `aphrody-router`, `aphrody-providers`, `aphrody-gateway`, `aphrody-mcp`, `aphrody-chat`, `aphrody-sdk`, `aphrody-memory`, `gemini-runtime`, `notebooklm`, … |
| **Skills/orchestration** | `aphrody-skills`, `aphrody-skills-forge`, `aphrody-marketplace`, `aphrody-task-runner`, `aphrody-cron`, `aphrody-events` |
| **Système** | `aphrody-secrets`, `aphrody-settings`, `aphrody-telemetry`, `aphrody-search`, `aphrody-re`, `aphrody-messaging`, `aphrody-voice`, `ievr-tools`, `aphrody-translate`, `aphrody-summary` |
| **Design/terminal** | `aphrody-design`, `aphrody-design-agents`, `m3-tokens`, `aphrody-icons`, `aphrody-react-reconciler`, `aphrody-tui`, `aphrody-terminal-*` (8) |
| **WASM** | `aphrody-wasm`, `aphrody-terminal-wasm`, `a2a-ui` |

Le binaire **`aphrody-mcp`** (serveur MCP natif) est produit par le crate
`google_mcp`.

### Exclus du workspace

- `crates/aphrody-app` — coquille Tauri v2, workspace propre (voir §3.1).
- `aphrody-x-client`, `a2a-slimrpc` — bloqués upstream.
- `gui`, `agui-bridge`, `mui-rs*` (6), `tuono*` (4) — **extraits vers
  `C:\src\aphrody-ts` le 2026-05-23** ; ces chemins n'existent plus dans ce
  dépôt. `coreutils`/`util-linux` sont encore listés dans `exclude` mais
  n'existent plus sur disque.

### Supprimés (historique)

- Pivot 2026-05-17 : `google_os` (archivé `C:\google-os-archive\`), `bun_ffi`,
  `google_kv`, `python_ffi`.
- 2026-05-21 : les 11 `n2b-*`, `bxc-engine`, `aphrody-xtask`, et 18 doublons
  fusionnés (`aphrody-{cache,cost,rateguard,retry}`→`aphrody-llm-infra` ;
  `aphrody-channels`→`aphrody-messaging` ;
  `aphrody-{hooks,permissions,skills-runtime}`→`aphrody-skills` ;
  `aphrody-design-{daemon,sidecar}`→`aphrody-design` ;
  `aphrody-voice-stt`→`aphrody-voice` ; `mrx-{core,detect,audit,watch,cli}`→
  `mrx` ; orphelins `aphrody-shell`, `aphrody-sandbox`).
- `vendor/` retiré (ne contenait que des stubs Bun/uv).

### 3.1 Surface GUI desktop (cross-repo)

L'application desktop graphique d'aphrody s'étend sur deux repos :

- **Backend Rust** — `crates/aphrody-app` (ce repo, exclu du workspace core).
  Expose la commande Tauri `aphrody_exec` qui appelle `aphrody::run_captured`
  en in-process (Rust vers Rust, sans saut FFI) : une action GUI emprunte
  exactement le chemin de code du terminal.
- **Frontend Angular 21.2 + Angular Material 21.2** —
  `C:\src\aphrody-ts\apps\desktop` (repo frère). UI style Gemini pixel-fidele,
  polices vendorisees hors-CDN. Committe sur `aphrody-ts` `main`.

Positionnement : **assistant IA autonome grand public propulse par Gemini** —
pas un outil de reverse engineering. Surfaces : chat Assistant (`aphrody chat`),
Accueil/Dashboard, Skills, MCP (`aphrody mcp list`), Commandes (surface CLI
complete), Settings multi-onglets (Compte via `aphrody antigravity whoami`,
Apparence, Backend conversation agy/web/stub, Memoire, Ame/soul, Identite,
Canaux, Actions, Agents, A propos), voice-to-voice, pieces jointes.

Build : `scripts/tauri.{ps1,sh}` (depuis la racine de ce repo, apres
`cd C:\src\aphrody-ts\apps\desktop && bun install`).

## 4. Politique de langages

| Langage | Usage |
|---|---|
| **Rust nightly + Edition 2024** | Tout le code (binaires, libs, FFI, tooling, MCP, scripts portés en Rust). |
| **C/C++** | Interdit dans le code distribué. Tolérable uniquement via `cxx::bridge` pour wrappers FFI inévitables. |
| **JS/TS/Node/Bun** | **Bannis** (policy 100 % Rust). Plus aucune invocation `bun`/`node`/`npm`/`tsc` dans la CI. |
| **Python** | **Banni** (scripts `.py` migrés en Rust ou supprimés). |
| **PowerShell 7+ / Bash** | Wrappers d'install/déploiement uniquement (`scripts/deploy.{ps1,sh}`). |

## 5. Commandes critiques

### Build local

```bash
# Linux (cible #1)
cargo build --release -p aphrody                          # natif
cargo build --release -p aphrody --target x86_64-unknown-linux-gnu

# Windows (cible #2)
cargo build --release -p aphrody                          # depuis Windows
cargo build --release -p aphrody --target x86_64-pc-windows-msvc

# WebAssembly (cible #3)
cargo build --release -p aphrody --target wasm32-wasi
cargo build --release -p aphrody --target wasm32-unknown-unknown
```

### Validation (zéro tolérance warnings)

```bash
cargo ci-offline       # clippy + --locked + --offline + -D warnings
cargo xt-offline       # nextest + --locked + --offline
cargo deny check       # CVE + licences + bans + sources
cargo vet              # audits signés (Google/Mozilla/Fuchsia/ChromeOS)
cargo audit-machete    # unused deps detector
```

### Cross-platform check (avant chaque PR)

```bash
cargo check -p aphrody --target x86_64-unknown-linux-gnu --locked   # bloquant
cargo check -p aphrody --target x86_64-pc-windows-msvc --locked     # bloquant
cargo check -p aphrody --target wasm32-unknown-unknown --locked     # bloquant
```

## 6. Supply-chain (Google-grade)

- **Lockfile-only** depuis 2026-05-16 (pas de `cargo vendor`).
- **Sparse registry** (10-100× plus rapide que git).
- **`cargo-vet` audits** importés depuis 7 feeds : Google, Mozilla, Fuchsia,
  ChromeOS, Bytecode Alliance, Embark Studios, Zcash.
- **`cargo-deny`** : CVE RustSec DB + licences + bans + sources.
- **CI hermétique** : `--locked --offline -D warnings`.

## 7. Pivot 2026-05-17 — décisions structurelles

### Ce qui change

- ✅ Repo renommé `google-cli` → **`aphrody`** (script `scripts/rename-project.ps1`).
- ✅ Crate `google_os` **sortie hors du workspace**, archivée sous
  `C:\google-os-archive\`. Ne plus importer.
- ✅ `crates/google_mcp` dépendance vers `google_os` **retirée**.
- ✅ Metadata workspace : `authors`, `homepage`, `repository`, `keywords`,
  `categories` mis à jour pour aphrody.
- 🔧 CI matrix `.github/workflows/cross-platform.yml` priorisée Linux d'abord.
- 🔧 `crates/a2a*` et `crates/google_mcp` à adapter pour Linux pur.

### Ce qui reste

- Architecture workspace + supply-chain + FFI zero-copy.
- Les crates métier conservés (`a2a*`, `google_mcp`, `backend`, `mrx`, …).
- Politique 100 % Rust (node/bun/python bannis).
- Stack 2026 (Rust nightly, Edition 2024, mimalloc, sccache).

### Ce qui est abandonné

- ❌ Émulation Windows NT kernel (`google_os`).
- ❌ Hybride Windows Terminal C++ fork (Pilier II historique).
- ❌ DxEngine custom (Direct2D/D3D11 textures).
- ❌ Architecture "Material Design 3" comme priorité (move to optional GUI).

## 8. Pièges connus (mémoire institutionnelle)

| Piège | Mitigation |
|---|---|
| `aws-lc-sys` build cassé sur MSVC | NASM prebuilt + Ninja generator (`.cargo/config.toml`). Sur Linux : `apt install libssl-dev`. |
| `tracing-subscriber 0.3.23+` | Pinné à `0.3.22` (bug `mod env` packaging). |
| `path-bases` (RFC 3529) | Instable nightly 1.97, à activer quand stable. |
| `rand 0.8` imposé par `denokv_proto` | Ne pas migrer vers 0.9 avant que `denokv_proto` accepte. |
| GTK3 CVE (RUSTSEC-2024-04xx) | Ignorés dans `deny.toml`. `cli` n'est PAS lié à GTK. `gui` a été extrait vers `aphrody-ts` (2026-05-23) ; seul `aphrody-app` (Tauri, exclu) pull wry/webkit2gtk. |
| `tokio` ne compile pas sur wasm | Utiliser features sélectives (`tokio-stream` + `js-sys` + `wasm-bindgen-futures`). |
| pty cross-platform | `portable-pty` (ConPTY Windows / openpty Unix) dans `aphrody-terminal-backend`. Pas de `node-pty`. |
| `aphrody chat` + token agy expiré | `classify_agy_error` dans `crates/cli/src/agy_backend.rs` intercepte `SdkError::OAuthServer{401|403}` et retourne un message court actionnable (re-auth `aphrody antigravity login`, ou `--web` / `--stub`) au lieu de dumper le JSON Google. Commit : `e5a932da2`. |
| `aphrody antigravity refresh` cassé | Google retourne `400 client_secret is missing` : le client OAuth public Antigravity ne fournit pas de `client_secret` pour le refresh grant. Workaround : relancer `agy` en arriere-plan (re-mint le token). Re-auth : `aphrody antigravity login`. |

## 9. Roadmap (post-pivot)

### Phase P-Linux (PRIORITÉ ABSOLUE)

- [ ] Validation `cargo build --release -p aphrody` sur Ubuntu 26.04 natif.
- [ ] Validation `cargo nextest run -p aphrody` sur Ubuntu 26.04.
- [ ] CI runner `ubuntu-26.04` (sinon `ubuntu-latest`).
- [ ] Package `apt` / PPA pour distribution Ubuntu.

### Phase P-Win11

- [ ] Validation `cargo build --release -p aphrody` sur Win11 Insider Canary.
- [ ] Package `scoop` + `winget` manifest.

### Phase P-Wasm

- [ ] `cli` compilable sur `wasm32-wasi` (CLI lib).
- [ ] `cli` compilable sur `wasm32-unknown-unknown` (web lib).
- [ ] `wasm-pack publish` sur npm en tant que `@aphrody-code/aphrody-wasm`.

### Phase P-A2A-Adapt

- [ ] Auditer `a2a*` pour code Windows-only.
- [ ] Gater en `#[cfg(target_os = "windows")]` ce qui doit l'être.
- [ ] Implémenter équivalents Linux (epoll, io_uring) là où nécessaire.

### Phase P-MCP-Adapt

- [ ] Idem pour `google_mcp`.
- [ ] Renommer éventuellement `google_mcp` → `aphrody_mcp` (décision séparée).

### Phase P-Distribution

- [ ] crates.io publication (`aphrody`).
- [ ] Homebrew tap `aphrody-code/tap` → `brew install aphrody`.
- [ ] Releases GitHub avec binaires Linux + Windows + wasm.

## 10. Ressources documentaires

| Doc | Contenu |
|---|---|
| `CLAUDE.md` | Directives Claude Code (résumé opérationnel). |
| `docs/ARCHITECTURE.md` | Carte du workspace (57 membres) + diagrammes. |
| `docs/PLAN.md` | Plan d'exécution détaillé. |
| `docs/SUMMARY.md` | mdBook ToC global (auto-généré par `aphrody-summary`). |
| `docs/cargo/CRATES.md` | Inventaire par crate. |
| `docs/cargo/CROSS_PLATFORM.md` | Stratégie multi-target Cargo. |
| `docs/cargo/CHROMIUM_ANDROID_PATTERNS.md` | Patterns Google-grade. |
| `docs/cargo/SKILLS.md` | Catalogue agents + skills. |
| `docs/cargo/SUPPLY_CHAIN.md` | Détails cargo-vet / cargo-deny. |
| `docs/cargo/WORKSPACE.md` | Description fine du workspace. |
| `docs/cargo/FFI_POLICY.md` | Règles FFI strictes. |

## 11. Convention de contribution

- Conventional Commits (`feat:`, `fix:`, `refactor:`, `build:`, `docs:`).
- Pas de mock, pas de fake data, pas de stub.
- Linux est la cible #1 : **ça doit compiler et passer les tests sur Linux
  avant tout**.
- Avant push : `cargo ci-offline && cargo deny check` sur Linux d'abord.
- Cross-platform check : les 3 cibles prioritaires doivent passer `cargo check`.

---

*Cette source de vérité remplace les sections redondantes des autres docs.
En cas de divergence, ce fichier prime.*
