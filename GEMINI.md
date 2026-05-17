# GEMINI.md

**Rôle** : **Strategic Lead & Uncompromising Engineer**
Focus : architecture cross-platform production-grade, code complet et profond,
zéro stub, tests rigoureux, supply-chain hygiénique.

## 0. Pivot 2026-05-17

Le projet est `aphrody`, **le CLI ultime cross-platform**.

Priorités plateformes (ordre strict) :
1. **Linux Ubuntu 26.04** (cible #1 bloquante)
2. **Windows 11 Insider Canary Build** (cible #2 bloquante)
3. **WebAssembly** (cible #3 bloquante, lib distribuable)
4. macOS (best-effort, non bloquant)

L'ancien sous-projet `google_os` (kernel emulator Windows-NT) a été
**archivé hors du repo**. Ne pas réintégrer.

Voir [`docs/SOURCE_OF_TRUTH.md`](docs/SOURCE_OF_TRUTH.md) pour la vue d'ensemble.

## Directives

- **PRODUCTION-READY ONLY** : pas de prototyping, pas de stubs. Code final,
  robuste, exhaustif.
- **No Stubs** : code fonctionnel uniquement. Appels réseau natifs, OS thread
  APIs, jamais `// To be implemented`. Implémenter des portions plus petites
  parfaitement si le temps manque.
- **Langues** : Rust uniquement pour le code distribué. C/C++ interdit dans
  les binaires (tolérable via `cxx::bridge` pour FFI inévitables).
  Bun/TypeScript pour scripting (`node` interdit). Python pour build tooling
  uniquement.
- **Mémoire** : `mimalloc` global. Chaque `unsafe` justifié.
  (`bun_ffi` archivé hors workspace — pollue le Rust pour zéro gain CLI.)
- **Tests** : PRs require `cargo nextest` (alias `cargo xt`) vert sur Linux
  d'abord, Windows ensuite.
- **Native Linux / Windows / Wasm** : abstractions portable par défaut, code
  OS-specific gated par `#[cfg(target_os = "...")]`. Pas d'émulation
  artificielle.

## Workspace (10 membres, post-pivot)

- **CLI / cœur** : `cli`, `base`, `backend`
- **UI desktop** : `gui` (exclu de `cli`)
- **Agent / IA (A2A)** : `a2a`, `a2a-client`, `a2a-server`, `a2a-pb`, `a2a-grpc`
- **Bridges** : `google_mcp`

**Exclus** : `crates/coreutils/`, `crates/util-linux/`, `crates/a2a-slimrpc/`,
`vendor/`.

**Archivé hors repo** :
- `crates/google_os/`  → `C:\google-os-archive\`
- `crates/bun_ffi/`    → `C:\aphrody-archive\` (FFI Bun, pollue le Rust)
- `crates/n2b/`        → `C:\aphrody-archive\` (réintégré via upstream branche aphrody)
- `crates/google_kv/`  → `C:\aphrody-archive\` (orphan)
- `crates/python_ffi/` → `C:\aphrody-archive\` (orphan, dépend de vendor/bun)

## Validation pipeline (hermetic, Linux-first)

```bash
# Linux d'abord (cible #1)
cargo check -p cli --target x86_64-unknown-linux-gnu --locked
cargo nextest run -p cli --target x86_64-unknown-linux-gnu --locked

# Puis Windows (cible #2)
cargo check -p cli --target x86_64-pc-windows-msvc --locked

# Puis wasm (cible #3)
cargo check -p cli --target wasm32-unknown-unknown --locked

# Workspace-wide
cargo ci-offline      # clippy --workspace --all-targets --locked --offline -- -D warnings
cargo xt-offline      # nextest run --workspace --locked --offline
cargo deny check      # CVE + licences + bans + sources
cargo vet             # signed audits (Google/Mozilla/Fuchsia/ChromeOS)
```

## A2A Native Integration

- CLI comme agent autonome : prompts NL interceptés par `AutoCommand`,
  routés vers le moteur natif `a2a` avec streaming zero-buffering.
- A2A crates en cours d'adaptation cross-platform — retirer dépendances
  Windows-only, ajouter équivalents Linux (epoll, io_uring).

## Supply-chain (lockfile-only, depuis 2026-05-16)

- **Pas de `cargo vendor`** — remplacé par `Cargo.lock` SHA-256 pins +
  sparse registry + sccache.
- Toutes les deps doivent passer `cargo deny check` avant merge.
- Trusted audit feeds : Google, Mozilla, Fuchsia, ChromeOS, Bytecode
  Alliance, Embark, Zcash (cf. `supply-chain/config.toml`).
- New deps without an existing audit trigger `cargo vet suggest` → require
  explicit exemption.

## Docs

- [`docs/SOURCE_OF_TRUTH.md`](docs/SOURCE_OF_TRUTH.md) — **Source de vérité unifiée**.
- [`CLAUDE.md`](./CLAUDE.md) — Directives low-level engineering.
- [`docs/PLAN.md`](docs/PLAN.md) — Plan d'exécution post-pivot.
- [`docs/cargo/`](docs/cargo/) — Workspace, FFI policy, cross-platform.
