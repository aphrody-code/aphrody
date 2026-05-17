# aphrody — Source of Truth

> **Document unique de référence** consolidant les anciens `CLAUDE.md`,
> `GEMINI.md`, `docs/PLAN.md`, `docs/DESIGN.md`. Lire celui-ci en premier.
> Mis à jour : **2026-05-17** (pivot CLI ultime cross-platform).

---

## 0. TL;DR

| Quoi | Valeur |
|---|---|
| **Nom du projet** | `aphrody` |
| **Binaire distribué** | `aphrody` (cross-platform pur) |
| **Repo GitHub** | `https://github.com/aphrody-code/aphrody` (privé initialement) |
| **Stack** | Rust nightly 1.97 + Edition 2024 + Bun (TypeScript) |
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

### Objectifs ultimes 2026-05-17 (pivot+1)

1. **Refactor Next.js via n2b** (aphrody-code/next.js@aphrody + aphrody-code/n2b@aphrody)
   en gardant **compatibilité COMPLETE avec upstream vercel/next.js**.
2. **Refactor shadcn-ui → Material Design 3 natif** (aphrody-code/ui@aphrody)
   via **bxc scraping** (Google Design / m3.material.io / material-web / CDN).
3. **Plugin Claude Code aphrody** : skill `pixel-perfect`, MCP `bxc-scrapper`,
   agent `n2b-ultra`, hook `oxclint`, pipeline `turbo` non bloquant.
4. **Intégration NATIVE de Turbopack + écosystème Rust Vercel** dans
   `Cargo.toml` workspace.dependencies (turbopack-*, swc-*, next-*, lightning-css,
   oxc, biome). Doc étudiée au même titre que les crates Rust internes.

### Règles transversales

- **Web/UI** : tout nouveau projet web cible **WASM Rust natif** ou **WebGPU**.
  Pas de fallback JS/TS. shadcn legacy → wrappers Material Web 3 natifs.
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

## 3. Architecture (post-pivot 2026-05-17)

### Upstreams aphrody-code (branche `aphrody` isolée d'upstream main)

| Repo | Type | Rôle | Intégration |
|---|---|---|---|
| `aphrody-code/n2b` | Rust workspace (11 crates) | Node→Bun linter, 68 rules | `Cargo.toml workspace.dependencies` git+branch="aphrody" |
| `aphrody-code/bxc` | Bun/TS package | Bun + Lightpanda browser engine | `packages/bxc/` placeholder + git clone OR github:aphrody-code/bxc#aphrody |
| `aphrody-code/ui` | pnpm workspace | Fork shadcn-ui/ui | `packages/ui/` placeholder + shadcn registry |
| `aphrody-code/next.js` | pnpm + Cargo (Turbopack) | Fork vercel/next.js canary | `packages/next.js/` placeholder + npm dep |

Chaque branche `aphrody` contient un fichier `.aphrody/INTEGRATION.md`
qui documente le contrat de divergence avec upstream.

### Workspace Rust (10 membres actifs)

| Crate | Rôle | Cross-platform | Statut |
|---|---|---|---|
| `cli` | **Binaire principal `aphrody`** | Pur | ✅ Stable |
| `base` | Primitives no_std partagées | Pur | ✅ Stable |
| `backend` | Forensics + network | Pur | ✅ Stable |
| `gui` | wry+tao desktop (exclu de `cli`) | Linux GTK3 / Win / macOS | ⚠ Migration GTK4 prévue |
| `a2a` | Protocole agent-to-agent | À adapter | 🔧 En cours |
| `a2a-client` | Client A2A | À adapter | 🔧 En cours |
| `a2a-server` | Serveur A2A | À adapter | 🔧 En cours |
| `a2a-pb` | Protobuf gen A2A | Pur | ✅ Stable |
| `a2a-grpc` | gRPC layer A2A | À adapter | 🔧 En cours |
| `google_mcp` | Serveur MCP (Model Context Protocol) | À adapter (était Windows-coupled) | 🔧 En cours |

**Notes** :
- `google_kv` archivé (orphan, aucun consumer dans le workspace).
- `python_ffi` archivé (orphan : 0 consumer, dépend de vendor/bun ; pour AI tuning / MD on utilise candle/comrak en Rust pur).

### Exclus du workspace

- `crates/coreutils/`, `crates/util-linux/` — userland GNU conservé en référence.
- `crates/a2a-slimrpc/` — bloqué par `agntcy-slim-mls` upstream (nightly lifetime issue).
- `crates/bun_ffi/fuzz/` — cargo-fuzz host-only.
- `vendor/` — sub-projets externes (bun, electron-prebuilt, coreutils, util-linux).

### Archivé hors du repo (NE PAS réintégrer)

- `crates/google_os/` → `C:\google-os-archive\20260517-*\`.
  Kernel emulator hybride NT/POSIX, conservé pour référence historique.
  Le pivot 2026-05-17 abandonne cette trajectoire.
- `crates/bun_ffi/` → `C:\aphrody-archive\bun_ffi-20260517-*\`.
  FFI V8↔Rust (Bun bindings). Pollue le workspace pour zéro gain côté
  CLI portable. Bun reste utilisé comme runtime/CLI externe (scripting).
- `crates/n2b/` → `C:\aphrody-archive\n2b-20260517-*\`.
  Outil de migration Node.js → Bun (AST-driven via oxc_parser). Trop
  spécialisé, deps lourdes (oxc_*, fastembed, octocrab). **Réintégré via
  upstream `aphrody-code/n2b` branche `aphrody`** (`Cargo.toml workspace.dependencies`).
- `crates/google_kv/` → `C:\aphrody-archive\google_kv-*\`.
  Deno KV store SQLite-backed. Orphan (aucun consumer dans le workspace).
- `crates/python_ffi/` → `C:\aphrody-archive\python_ffi-*\`.
  PyO3 bridge + Bun JSC bindings. Orphan (0 consumer dans le workspace, dépend
  de `vendor/bun/src/jsc`). Pour AI tuning / MD rendering, l'écosystème Rust
  pur est suffisant : `candle` (ML), `llama-rs` (inference), `pulldown-cmark`
  / `comrak` (Markdown).

## 4. Politique de langages

| Langage | Usage |
|---|---|
| **Rust nightly + Edition 2024** | Tout le code distribué (binaires, libs, FFI). |
| **C/C++** | Interdit dans le code distribué. Tolérable uniquement via `cxx::bridge` pour wrappers FFI inévitables. |
| **Bun / TypeScript** | Scripting, MCP, tooling, build automation. **`node` interdit** (cf. [[feedback_bun_only]]). |
| **Python** | Tooling de build interne uniquement, jamais distribué. |
| **PowerShell 7+** | Scripts d'installation et de maintenance Windows. |
| **Bash** | Scripts d'installation et de maintenance Linux. |

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
- Tous les crates métier (`a2a*`, `google_mcp`, `google_kv`, `n2b`,
  `python_ffi`, `gui`, `backend`).
- Politique Bun + node interdit + PowerShell pour Windows.
- Stack 2026 (Rust nightly 1.97, Edition 2024, mimalloc, sccache).

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
| GTK3 CVE (RUSTSEC-2024-04xx) | Ignorés dans `deny.toml`. `cli` n'est PAS lié à GTK ; seul `gui` l'est. |
| `tokio` ne compile pas sur wasm | Utiliser features sélectives (`tokio-stream` + `js-sys` + `wasm-bindgen-futures`). |
| `node-pty` cassé sur Node v26 | Utiliser Bun à la place (cf. `docs/terminal/GEMINI_CLI.md`). |

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
| `GEMINI.md` | Directives Gemini (résumé stratégique). |
| `docs/PLAN.md` | Plan d'exécution détaillé. |
| `docs/DESIGN.md` | Décisions d'architecture historiques. |
| `docs/SUMMARY.md` | mdBook ToC global. |
| `docs/cargo/CROSS_PLATFORM.md` | Stratégie multi-target Cargo. |
| `docs/cargo/CHROMIUM_ANDROID_PATTERNS.md` | Patterns Google-grade. |
| `docs/cargo/SKILLS.md` | Catalogue agents + skills. |
| `docs/cargo/SUPPLY_CHAIN.md` | Détails cargo-vet / cargo-deny. |
| `docs/cargo/WORKSPACE.md` | Description fine du workspace. |
| `docs/cargo/FFI_POLICY.md` | Règles FFI strictes. |
| `docs/terminal/GEMINI_CLI.md` | Workarounds node-pty / Bun. |

## 11. Convention de contribution

- Conventional Commits (`feat:`, `fix:`, `refactor:`, `build:`, `docs:`).
- Pas de mock, pas de fake data, pas de stub.
- Linux est la cible #1 : **ça doit compiler et passer les tests sur Linux
  avant tout**.
- Avant push : `cargo ci-offline && cargo deny check` sur Linux d'abord.
- Cross-platform check : les 3 cibles prioritaires doivent passer `cargo check`.

---

*Cette source de vérité remplace les sections redondantes des anciens docs
(CLAUDE, GEMINI, PLAN, DESIGN). En cas de divergence, ce fichier prime.*
