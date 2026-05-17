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

## 1. ZÉRO STUB, 100% PRODUCTION

L'architecture de base est en place. Mode "scaffolding" **interdit**.

- Toute fonction Rust ou C touchée contient sa logique métier complète et réelle.
- Pour le code Linux : appels `libc`, `nix`, `tokio`, `io_uring` (via `tokio-uring`
  ou `io-uring` crate) — pas d'émulation.
- Pour le code Windows : `windows-rs` direct, pas de wrapper artificiel.
- Jamais de `TODO: implement later`. Tu le fais maintenant ou tu ne l'écris pas.

## 2. Politique de langages

- **Tout nouveau code** : Rust nightly (Edition 2024).
- **C/C++** : interdit dans le code distribué (`crates/cli`, `crates/base`, etc.).
  Tolerable uniquement pour des wrappers FFI inévitables (`cxx::bridge`).
- **FFI / interop mémoire** : `mimalloc` allocator global, zero-copy via
  pointeurs bruts encapsulés (`crates/bun_ffi` a été archivé — cf. §4).
- **Bun / TypeScript** : scripting, MCP, tooling périphérique uniquement.
  **node interdit** (cf. memory `feedback_bun_only`).
- **Python** : tooling de build interne, jamais pour générer du code distribué.
- **Web / UI** (règle 2026-05-17) : **WASM Rust natif (`wasm32-unknown-unknown`
  + `wasm-bindgen`) OU WebGPU (`wgpu` crate)** pour TOUT nouveau projet web.
  Pas de fallback JS/TS pour les nouveaux modules. shadcn-ui legacy → réécrit
  en wrappers Material Web Components 3 natifs via bxc scraping.
- **Turbopack** + tout l'écosystème Rust Vercel (`turbopack-*`, `swc-*`,
  `next-*`, `lightning-css`, `oxc`) doit être déclaré en
  `[workspace.dependencies]` de `Cargo.toml` racine. Pas de re-vendoring.

## 2.5. Méthodologie docs / versions / fact-checking

**Avant** d'ajouter une dep à `[workspace.dependencies]`, d'écrire un appel
API non trivial, ou de prendre une décision basée sur la doc d'une lib :

- Utiliser le MCP **`context7`** (`resolve-library-id` puis `query-docs`)
  pour vérifier la version courante et l'API actuelle.
- Préférer context7 à la mémoire ou à WebSearch pour la doc des libs.
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

## 4. Architecture (post-pivot)

Monorepo Rust + Bun.

### Workspace (`Cargo.toml` root, 16 members)

- **CLI / cœur** : `cli` (binaire principal, **cross-platform pur**), `base`
  (no_std primitives), `backend` (forensics + network, cross-platform).
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

## 6.5. Skills & agents (`.claude/`)

Toute la surface skills est centralisée et documentée :

- **Inventaire + spec** → `docs/cargo/SKILLS.md` (format SKILL.md, runtime, ajout).
- **Index local** → `.claude/skills/README.md`.
- **Skills projet** : `start` (autonomous mode), `vps-commander` (SSH tunnel).
- **Agents projet** : `cargo-auditor`, `cpp-engineer`, `ffi-architect`,
  `rust-architect`, `rust-engineer`.
- **Runtime** : `skill` crate (workspace dep, lib) + binaires `skill-cli` /
  `agent-skills-cli` (validateur).
- **Sync catalogue externe** : `bun run skills:sync:vercel`,
  `bun run skills:sync:claude-official`, ou `bun run scripts/skills-sync.ts
  <org>/<repo>`.

## 7. Pièges connus (mémoire institutionnelle)

- **aws-lc-sys** : pull via reqwest's `rustls-tls`. Sur Windows : compile via
  NASM prebuilt + Ninja (variables `AWS_LC_SYS_PREBUILT_NASM=1`,
  `CMAKE_GENERATOR=Ninja` dans `.cargo/config.toml`). Sur Linux : OpenSSL
  système (`apt install pkg-config libssl-dev` sur Ubuntu).
- **tracing-subscriber** pinné à `0.3.22` (0.3.23+ a un bug `mod env` packaging).
- **`base = ...` (path-bases RFC 3529)** : feature instable nightly 1.97, à
  activer quand stable.
- **rand 0.8 imposé** (pas 0.9) par `denokv_proto`.
- **GTK3 CVE** (RUSTSEC-2024-04xx) : tirés par tao/wry sur Linux, ignorés dans
  `deny.toml` jusqu'à migration GTK4. Le binaire `cli` n'est PAS lié à GTK —
  seul `crates/gui` l'est, et `gui` n'est pas dans le pipeline `cli`.
- **wasm** : `tokio` ne compile pas tel quel — utiliser features sélectives
  (`tokio-stream` + `js-sys` + `wasm-bindgen-futures` pour le runtime web).

## 8. Source of Truth

Pour la vue d'ensemble consolidée (architecture, plateformes, livrables,
ressources), lire **[`docs/SOURCE_OF_TRUTH.md`](docs/SOURCE_OF_TRUTH.md)** —
fusion des anciens `CLAUDE.md` / `GEMINI.md` / `docs/PLAN.md` / `docs/DESIGN.md`.
