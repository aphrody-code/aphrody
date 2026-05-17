# PLAN — aphrody

> Plan d'exécution stratégique. Révision : **2026-05-17 (pivot CLI cross-platform)**.
> Voir [`SOURCE_OF_TRUTH.md`](./SOURCE_OF_TRUTH.md) pour le contexte d'ensemble.

---

## 0. Phases livrées avant pivot (2026-05-16)

| Phase | Sortie | Statut |
|---|---|---|
| **P0 — Bring-up workspace** | resolver=3, rust-version=1.97, 80 deps centralisées, lints Google-grade, 5 profils release | ✅ |
| **P1 — Cleanup vendor** | `vendor/crates.io/` supprimé (1.2 Go libérés), `--locked` en CI | ✅ |
| **P2 — Supply-chain signée** | `cargo-vet` (7 feeds Google/Mozilla/Fuchsia/ChromeOS/BCA/Embark/Zcash) + `cargo-deny` | ✅ |
| **P3 — Path-bases (RFC 3529)** | Tentative, révoquée (feature instable Cargo 1.97) | ⏸ |
| **P4 — Validation hermétique** | `cargo ci-offline` + `cargo deny check` verts | ✅ |
| **P5 — Refresh docs + cleanup root** | README + CLAUDE + GEMINI + .gitignore alignés | ✅ |
| **P10 — Cross-platform Google-grade** | binaire `cli` cross-platform, `platform.rs`, aliases multi-target, lints `android-strict` | ✅ |
| **P11 — Alignement Google complet** | google.json sync, Android NDK targets, MUSL targets, CI matrix, `cargo-fuzz` skeleton, `cargo-auditable` alias, Dockerfile distroless | ✅ |
| **P13 — Skills ecosystem** | `skill` crate + binaires + 50+ skills documentés + sync upstream | ✅ |

## 1. Phases post-pivot (2026-05-17)

Le pivot abandonne la trajectoire "Google OS hybride Win-NT" et recentre
sur `aphrody`, le CLI ultime cross-platform. **Linux est désormais la
cible #1**.

### Phase P-A — Pivot mécanique (cette PR initiale)

| Tâche | Statut |
|---|---|
| Repo `google-cli` → `aphrody` (rename script ~78 fichiers) | ✅ |
| `crates/google_os` archivé hors du repo (`C:\google-os-archive\`) | ✅ |
| `[workspace.members]` mis à jour (google_os retiré) | ✅ |
| `[workspace.package]` metadata (authors, homepage, repository, keywords) | ✅ |
| `crates/google_mcp` : retrait dep `google_os` | ✅ |
| `packages/material-design-icons/` purgé (4.6 GB, .gitignore + README stub) | ✅ |
| `docs/SOURCE_OF_TRUTH.md` créé (fusion CLAUDE/GEMINI/PLAN/DESIGN) | ✅ |
| `CLAUDE.md`, `README.md`, `docs/PLAN.md` réécrits en UTF-8 propre | ✅ |
| Anonymisation : tracked files (1 leak path, déjà fix) | ✅ |
| Premier commit + push vers `aphrody-code/aphrody` (privé) | 🔧 |
| Rename dossier racine `C:\src\google-cli` → `C:\src\aphrody` | ⏳ |

### Phase P-Linux — Validation Linux Ubuntu 26.04 (PRIORITÉ #1)

| Tâche | Statut |
|---|---|
| `cargo check -p cli --target x86_64-unknown-linux-gnu` vert | ⏳ |
| `cargo build --release -p cli` natif sur Ubuntu 26.04 | ⏳ |
| `cargo nextest run -p cli` vert sur Linux | ⏳ |
| Adapter `crates/a2a*` pour Linux (retirer Windows-only) | ⏳ |
| Adapter `crates/google_mcp` pour Linux | ⏳ |
| CI runner `ubuntu-26.04` (ou `ubuntu-latest` en fallback) | ⏳ |
| Package `apt`/`deb` PPA pour distribution Ubuntu | ⏳ |

### Phase P-Win11 — Validation Windows 11 Insider Canary (PRIORITÉ #2)

| Tâche | Statut |
|---|---|
| `cargo build --release -p cli` sur Win11 Insider Canary | ⏳ |
| `cargo nextest run -p cli` vert sur Windows | ⏳ |
| Package `scoop` + `winget` manifest | ⏳ |
| Profil Windows Terminal pour `aphrody` | ⏳ |

### Phase P-Wasm — WebAssembly lib (PRIORITÉ #3)

Matrice validée 2026-05-17 (host : Windows 11) :

| Crate           | `wasm32-unknown-unknown` | `wasm32-wasip1` |
|-----------------|:------------------------:|:---------------:|
| `base`          | ✅ (getrandom "js" gated)| ✅              |
| `mrx-core`      | n/a (chrono)             | ✅              |
| `aphrody-translate` | ❌ (tokio "full")    | ❌              |
| `cli` (binary)  | ❌ (tokio "full" + mio)  | ❌              |
| `backend`/`a2a*`| ❌                       | ❌              |

Sous-tâches :

| Tâche | Statut |
|---|---|
| `base` : feature `js` getrandom gated wasm32-unknown-unknown | ✅ |
| `base` : compile `wasm32-unknown-unknown` + `wasm32-wasip1` | ✅ |
| `mrx-core` : compile `wasm32-wasip1` | ✅ |
| `aphrody-translate` : retirer tokio `full` (idéalement tokio-rt minimal) | ⏳ |
| `cli` : refactor tokio + cfg-gate commandes OS-bound pour wasm | ⏳ (P-Wasm-CLI) |
| `crates/aphrody-wasm` : wrapper `base` exposé via `wasm-bindgen` | ⏳ |
| `wasm-pack publish` sur npm `@aphrody-code/aphrody-wasm` | ⏳ |

### Phase P-Wasm-CLI — Port cli binaire vers wasm32

Le cli pull tokio (full features) + reqwest + mimalloc + rustls + ring via
backend/a2a-client. Refactor requis :

| Tâche | Statut |
|---|---|
| `cli/Cargo.toml` : `[target.'cfg(not(target_arch = "wasm32"))'.dependencies]` pour mimalloc/backend/a2a-client/reqwest/rustls | ⏳ |
| `cli/Cargo.toml` : `[target.'cfg(target_arch = "wasm32")'.dependencies]` avec tokio minimal (sync,macros,io-util,rt,time) | ⏳ |
| `cli/src/main.rs` : `#[cfg(not(target_arch = "wasm32"))]` sur les commandes OS-bound | ⏳ |
| `cli/src/main.rs` : stub wasm minimal (Version + help) | ⏳ |
| `aphrody-translate/Cargo.toml` : tokio minimal pour wasm (translate API HTTP via reqwest wasm) | ⏳ |

### Phase P-Distribution

| Tâche | Statut |
|---|---|
| crates.io publication (`aphrody`) | ⏳ |
| Homebrew tap `aphrody-code/tap` | ⏳ |
| GitHub Releases avec binaires Linux + Windows + wasm | ⏳ |
| `cargo install aphrody` documenté | ⏳ |

## 2. Tâches de fond (continues)

### A2A native integration

- [ ] CLI comme agent autonome : prompts NL interceptés par `AutoCommand`,
  routés vers le moteur natif `a2a` avec streaming zero-buffering.
- [ ] Cross-platform : trait `Transport` portable (HTTP/2 Linux, IOCP Win, fetch wasm).

### Supply-chain real audits

- [ ] `cargo vet suggest` → liste des audits manquants.
- [ ] Pour chaque crate critique (crypto, net, ffi), audit `safe-to-deploy`.
- [ ] Publier nos audits sur `aphrody-code/rust-crate-audits` pour partage.

### Stabilité / Release 1.0.0-LTS

- [ ] Pression `cargo clippy` + `miri` sur tous les `unsafe`.
- [ ] Audit FFI bloc par bloc (`python_ffi` — `bun_ffi` archivé hors workspace).
- [ ] Stress tests : `cargo bench` sur benchmarks critiques.
- [ ] Cross-compile validation : 3 cibles prioritaires + macOS + Android.
- [ ] Tag `v1.0.0-LTS`.

### Upstream alignment

- [ ] **a2a-slimrpc** : ré-inclure quand `agntcy-slim-mls` fixe lifetime/async-trait.
- [ ] **path-bases (RFC 3529)** : activer workspace-wide quand stable Cargo 1.98+.
- [ ] **wry → GTK4** : retirer les CVE GTK3 (RUSTSEC-2024-04xx).
- [ ] **reqwest 0.13** : drop `aws-lc-sys` propre (retirer 4 CVE ignorés).
- [ ] **pyo3 0.22** : fix CVE PyString (RUSTSEC-2025-0020).

## 3. Règles de collaboration inter-AI

Les IAs (Gemini, Claude, OpenCode) opèrent avec consigne commune :
**la qualité prime sur la vitesse**.

Toute tâche trop vaste pour une passe est divisée en sous-tâches **chacune
100% exécutable + testée**.

Avant tout push : `cargo ci-offline && cargo deny check` doit être vert
**sur Linux d'abord**.

## 4. Métriques de santé (snapshot 2026-05-17)

| Métrique | Valeur | Cible |
|---|---|---|
| Workspace members | 14 | 14 (post-pivot, google_os retiré) |
| `cargo check --locked` (Linux) | ⏳ à valider | < 1 min |
| `cargo clippy -- -D warnings` | ⏳ à valider | 0 erreur |
| `cargo deny check` | ✅ ok×4 | ok×4 |
| Repo size (sans target, sans mdi) | ~7 Go (vendor/bun dominant) | optimisé |
| Disque libéré (P1+pivot) | 1.2 Go (vendor/crates.io) + 4.6 Go (material-design-icons) | n/a |
| CVE ignorés (justifiés) | 12 | < 5 (après upstream alignment) |
| Cibles cross-platform bloquantes | 3 (Linux/Win/wasm) | 3 |

---

*Pour la vue d'ensemble exécutive, lire [`SOURCE_OF_TRUTH.md`](./SOURCE_OF_TRUTH.md).*
