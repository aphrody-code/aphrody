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
| Rename dossier racine `C:\src\google-cli` → `C:\src\aphrody` | ✅ |

### Phase P-Linux — Validation Linux Ubuntu 26.04 (PRIORITÉ #1)

| Tâche | Statut |
|---|---|
| `cargo check -p cli --target x86_64-unknown-linux-gnu` vert | ✅ (WSL local, 8.28 s, zéro warning) |
| `cargo build --release -p aphrody` natif sur Ubuntu 26.04 | ✅ (job `linux-native` ubuntu-latest commit 96ae82e7 ; cross-compile zigbuild Windows → Linux ✅) |
| `cargo nextest run -p aphrody` vert sur Linux | ✅ (job `linux-native` commit 96ae82e7 — nextest sur workspace `--locked`) |
| Adapter `crates/a2a*` pour Linux (retirer Windows-only) | ✅ (cli chromium gated, voir fix d222d0061) |
| Adapter `crates/google_mcp` pour Linux | ✅ (YOLO #1 — cargo check native + zigbuild x86_64-unknown-linux-gnu exit 0) |
| CI runner `ubuntu-26.04` (ou `ubuntu-latest` en fallback) | ✅ (ubuntu-latest job `linux-native` commit 96ae82e7) |
| Package `.deb` template (`packaging/deb/` — control + postinst + prerm + cargo-deb snippet + README) | ✅ |
| Publication PPA `aphrody-code/aphrody` sur Launchpad | ⏳ |

### Phase P-Win11 — Validation Windows 11 Insider Canary (PRIORITÉ #2)

| Tâche | Statut |
|---|---|
| `cargo build --release -p aphrody` sur Win11 Insider Canary | ✅ (local Win11 28020) |
| `cargo nextest run` workspace vert sur Windows | ✅ (**387/387** en 2,914 s — 2026-05-17) |
| Package `scoop` manifest | ✅ (`packaging/scoop/aphrody.json`) |
| Package `winget` manifest | ✅ (`packaging/winget/manifests/a/aphrody-code/aphrody/__VERSION__/`) |
| Profil Windows Terminal pour `aphrody` | ✅ (`packaging/windows-terminal/aphrody.profile.json`) |

### Phase P-Wasm — WebAssembly lib (PRIORITÉ #3)

Matrice validée 2026-05-17 (host : Windows 11) :

| Crate           | `wasm32-unknown-unknown` | `wasm32-wasip1` |
|-----------------|:------------------------:|:---------------:|
| `base`          | ✅ (getrandom "js" gated)| ✅              |
| `mrx-core`      | n/a (chrono)             | ✅              |
| `aphrody-translate` | ✅ (wasm stub)       | ✅              |
| `cli` (binary)  | ✅ (stub minimal)        | ✅ (stub minimal)|
| `a2a-client`    | ✅ (JSON-RPC+REST+SSE via browser fetch; async-trait ?Send) | ✅ (traits + types only; reqwest/HTTP modules cfg-stripped — WASI p1 has no sockets) |
| `backend`       | ❌                       | ❌              |

`a2a-client` notes :
- `wasm32-unknown-unknown` : `JsonRpcTransport`, `RestTransport`, `AgentCardResolver` compilent. `reqwest` utilise browser `fetch`. Streaming SSE via `bytes_stream()`. `BoxStream` = `LocalBoxStream` (futures !Send sur wasm). `async-trait` avec `?Send`.
- `wasm32-wasip1` : reqwest 0.13 utilise le stack natif (hyper/mio/socket2) sur `target_os = "wasi"` — les syscalls socket ne sont pas exposés par WASI p1. Modules `jsonrpc`, `rest`, `agent_card`, `factory` cfg-strippés. Traits (`Transport`, `TransportFactory`), types de données, `auth`, `middleware` compilent. Déblocage possible avec WASI p2 + `wasi-http` crate.
- `backend` reste ❌ sur les deux cibles : `tokio::fs`, `fs_extra`, `tracing-subscriber`, `base::Vfs`, et DNS OSINT sont tous des OS-primitives sans équivalent WASM. Port non tractable sans réécriture complète.

Sous-tâches :

| Tâche | Statut |
|---|---|
| `base` : feature `js` getrandom gated wasm32-unknown-unknown | ✅ |
| `base` : compile `wasm32-unknown-unknown` + `wasm32-wasip1` | ✅ |
| `mrx-core` : compile `wasm32-wasip1` | ✅ |
| `aphrody-translate` : retirer tokio `full` (idéalement tokio-rt minimal) | ✅ |
| `cli` : refactor tokio + cfg-gate commandes OS-bound pour wasm | ✅ (cli/src/main.rs cfg-gates mimalloc + tokio + reqwest + backend + a2a) |
| `crates/aphrody-wasm` : wrapper `base` exposé via `wasm-bindgen` | ✅ |
| `wasm-pack publish` sur npm `@aphrody-code/aphrody-wasm` (89 KB wasm + 12 KB JS, metadata.wasm-pack profile.release SIMD+bulk-memory, README + publish doc) | ✅ scaffolded — `wasm-pack publish --access public` awaits `npm login` |
| `a2a-client` : port wasm32-unknown-unknown (browser fetch transport) | ✅ |
| `a2a-client` : port wasm32-wasip1 (traits/types ; HTTP cfg-strippé) | ✅ |
| `a2a-pb` : gate tonic/proto/pbconv non-wasm ; protojson_conv disponible sur wasm | ✅ |
| `backend` : port wasm (tokio::fs + VFS + DNS = OS-only, pas tractable) | ❌ bloqué |

### Phase P-Wasm-CLI — Port cli binaire vers wasm32

Le cli pull tokio (full features) + reqwest + mimalloc + rustls + ring via
backend/a2a-client. Refactor requis :

| Tâche | Statut |
|---|---|
| `cli/Cargo.toml` : `[target.'cfg(not(target_arch = "wasm32"))'.dependencies]` pour mimalloc/backend/a2a-client/reqwest/rustls | ✅ |
| `cli/Cargo.toml` : `[target.'cfg(target_arch = "wasm32")'.dependencies]` avec tokio minimal (sync,macros,io-util,rt,time) | ✅ |
| `cli/src/main.rs` : `#[cfg(not(target_arch = "wasm32"))]` sur les commandes OS-bound | ✅ |
| `cli/src/main.rs` : stub wasm minimal (Version + help) | ✅ |
| `aphrody-translate/Cargo.toml` : tokio minimal pour wasm (translate API HTTP via reqwest wasm) | ✅ |

### Phase P-A2A — Bilateral Claude-to-Claude coordination (2026-05-17)

| Tâche | Statut |
|---|---|
| `ai.json` v1 schema (channel-extension layer) | ✅ (`schemas/ai.json/v1.json`) |
| `ai.json` AGNTCY a2a/v0.4 CollaborationManifest (peer-authored) | ✅ (`ai.json` racine) |
| `.well-known/ai.json` HTTP-discoverable subset | ✅ (`canonical_manifest` field pointe vers racine) |
| Bilateral handshake apx-handshake-1 → wc-ack-1 → apx-ack-of-ack-1 | ✅ (3-deep loop, 6 canaux : file_jsonl, http, markdown, heartbeat, process_inspect, git_tag) |
| Live HTTP listener (`/ping`, `/msg`, `/inbox`, `/ai.json`) sur `:8788` | ✅ (`C:\winclean\.coord\listener.ts`) |
| Asks queue + facts journal en JSONL | ✅ (`C:\winclean\.coord\inbox-*.jsonl`) |
| Peer mirror `C:\winclean\ai.json` publié | ✅ (winclean Claude — 16:00Z 2026-05-17) |
| Technical post D+14 (per `/start` skill arc) — 2076 mots sur ai.json | ✅ (`docs/posts/2026-05-ai-json.md` — picked up by mdBook SUMMARY.md regen) |
| GitHub Pages workflow (`docs.yml`) triggers main branch + deploys mdBook | ✅ (fix branch trigger `master` → `main` 2026-05-17) |
| CI hardening — machete fail-fast + binary smoke (--version/--help) + vet fail-fast | ✅ (commit 0372fe57c) |

### Phase P-Distribution

| Tâche | Statut |
|---|---|
| crates.io publication (`aphrody`) | ⏳ |
| Homebrew formula template (`packaging/homebrew/aphrody.rb`) | ✅ |
| Homebrew tap `aphrody-code/tap` publié | ⏳ |
| GitHub Releases workflow (`.github/workflows/release.yml`, 8 targets, SHA-256, SBOM) | ✅ |
| Premier tag `v*` poussé qui produit un release | ⏳ |
| Package workspace `cli` → `aphrody` | ✅ (commit 2026-05-17, dir conservé `crates/cli/`) |
| `cargo install aphrody` documenté | ⏳ (besoin de publier `base`/`backend`/`a2a-*` à crates.io d'abord) |
| One-line install (`install.sh` / `install.ps1`) avec vérif SHA-256 | ✅ |
| Cleanup `cargo machete` — 11 dead deps retirées (cli/google_mcp/gui/mrx-{audit,core,detect}) | ✅ |
| `aphrody --version` runtime bug — rustls 0.23 CryptoProvider install au boot | ✅ (commit pending) |

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

### Phase Q — Mission D+7 → D+15 polish (génération auto tick 7)

Mission 100k stars / 30 jours — actions mission-direct entre D+7 (demo) et
D+15 (Show HN), tous techniquement actionables sans autorisation utilisateur.

| Tâche | Statut |
|---|---|
| Re-rendre asciinema cast `assets/aphrody-demo.cast` avec `aphrody doctor` (+ couleurs) | ⏳ (cast actuel précède doctor subcommand) |
| Fresh clippy `-D warnings` audit + fix résidu Oracle gate 2 `double_ended_iterator_last` | ⏳ (oracle #20 rapport sale ; vérifier état réel) |
| Deuxième post technique `docs/posts/2026-05-yolo-grind-loop.md` (architecture 4-agent parallel grind) | ⏳ (D+14 milestone arc, complète post a2a-ai-json) |
| Show HN launch package `docs/launch/SHOW-HN.md` (title + body + comment templates draft) | ⏳ (D+15 milestone arc ; draft seul, pas de post) |
| `n2b` aggressive scan + auto-migrate `scripts/**/*.{ts,mjs,js}` node→bun | ⏳ (mémoire feedback_bun_only — node interdit, scan + apply tout fixable) |
| `mrx` aggressive `scan/audit/detect` du workspace → rapport `docs/audits/2026-05-17-mrx-aggressive.md` | ⏳ (5 crates mrx-* shippées, drogger usage interne) |
| `bxc` scrape via A2A ask peer winclean → AGNTCY spec page + M3 tokens, mirror aphrody side | ⏳ (bxc lives in C:/winclean, request via inbox-from-aphrody.jsonl) |
| `aphrody completions {bash,zsh,fish,pwsh,elvish}` subcommand via `clap_complete` | ⏳ (30-second test win — engineers expect this from any modern CLI) |
| Integration test `crates/cli/tests/doctor.rs` — `assert_cmd` driven smoke of `aphrody doctor` + `--json` | ⏳ (no test coverage on doctor surface) |
| Supply-chain `audits.toml` + `config.toml` formatting drift fix (Oracle gate 5 partial) | ⏳ (cargo vet fmt needed but careful — agent #21 noted side-effects) |
| Marketing `docs/COMPARISON.md` — aphrody vs just/taskfile/gh/devcontainer/asdf | ⏳ (mission D+15 Show HN traction — 30-sec engineer skim differentiator) |
| ADRs `docs/adr/{0001-cross-platform-rust,0002-a2a-file-based,0003-yolo-parallel-grind}.md` | ⏳ (engineering-credibility marker — engineers expect ADRs in serious repos) |
| WASM browser playground `crates/aphrody-wasm/examples/browser-playground.html` + crate README upgrade | ⏳ (D+7 demo extension — hands-on wasm in 3 clicks) |
| `.devcontainer/devcontainer.json` one-click Codespace setup (rust nightly + bun + cargo extras) | ⏳ (lowers contribution barrier — 30-sec engineer-onboarding win) |
| `SECURITY.md` verify + strengthen — concrete report process, supported versions, scope | ⏳ (OSS hygiene + GitHub security-policy badge) |
| `packaging/snap/snapcraft.yaml` + `packaging/arch/PKGBUILD` — Ubuntu Snap + Arch AUR distribution expansion | ⏳ (D+18 distribution arc — covers 2 major Linux distros) |
| `crates/backend/benches/backend_bench.rs` — criterion benchmark suite | ⏳ (BENCHMARKS.md credibility — engineers expect criterion not hand-roll) |
| `docs/FAQ.md` + `docs/ROADMAP.md` — anticipated Q + public 90-day roadmap | ⏳ (D+15 Show HN preparation — reduces noise in comment thread) |
| `crates/mrx-cli/README.md` — usage doc with scan/detect/audit/watch examples | ⏳ (mrx is undocumented despite being shipped — engineering credibility gap) |
| `crates/aphrody-translate/README.md` — translate CLI usage doc | ⏳ (publish ladder readiness — undocumented = unpublishable) |
| `.github/workflows/codeql.yml` — GitHub CodeQL security scanning | ⏳ (Security policy badge + Rust + Bun coverage) |
| `docs/extensions/` — a2a extension specs (file-transport, honest-delivery, context7-version-pinning) | ⏳ (URLs in ai.json point at aphrody.dev/a2a-extensions/* — make them publishable docs) |
| `crates/base/README.md` — base crate docs (publish-ready) | ⏳ (publish ladder leaf — first crate to ship, docs are user's first impression) |
| `crates/a2a-pb/README.md` + `crates/backend/README.md` — publish-ladder docs | ⏳ (undocumented = unpublishable) |
| `crates/a2a-{client,server,grpc}/README.md` — 3 publish-ladder docs | ⏳ (publish ladder middle tier — full coverage required) |
| `docs/ARCHITECTURE.md` — workspace overview + ASCII module dep graph | ⏳ (30-sec engineer skim: "what's the shape of this thing?") |
| `mrx-core/src/scan.rs` `workspace_key()` Windows path-sep bug fix per #30 audit | ⏳ (mrx ships broken on Windows — file_count=0 bytes=0 per workspace) |
| `.github/workflows/release-please.yml` — Google release automation via Conventional Commits | ⏳ (D+15 Show HN — automated release notes + version PR) |
| `crates/google_mcp/README.md` + `crates/a2a/README.md` + `crates/a2a-lf/README.md` | ⏳ (publish-ladder completion — last 3 a2a-family crates without docs) |
| `crates/mrx-{core,detect,audit,watch}/README.md` — 4 mrx lib crate docs | ⏳ (publish-ladder + engineering credibility) |
| Root `README.md` polish — sweep new docs into doc-tree TOC (COMPARISON/FAQ/ROADMAP/ADRs/extensions) | ⏳ (D+15 first-impression for HN landing — must reflect actual content) |
| `.github/ISSUE_TEMPLATE/*` + `PULL_REQUEST_TEMPLATE.md` + `FUNDING.yml` — OSS template bundle | ⏳ (GitHub community-health badge — engineers expect templates) |
| `CHANGELOG.md` sweep — log 2026-05-17 mega-batch (30+ commits today) | ⏳ (Keep-a-Changelog drift — last entry pre-tick-7) |
| `packaging/nix/flake.nix` — Nix flake for Nix/NixOS users | ⏳ (high-engineer-cred niche distribution channel) |
| `packaging/flatpak/com.aphrody.aphrody.json` — Flatpak manifest | ⏳ (cross-distro Linux desktop installation) |
| `.github/dependabot.yml` — monthly cargo + GH actions dep updates | ⏳ (OSS hygiene + GH community-health badge) |
| `packaging/{scoop,winget,homebrew,deb}/*` version+arch sweep | ⏳ (manifests must claim 1.0.0-canary + x86_64+arm64 coverage) |
| `docs/PROTOCOL.md` — definitive a2a/v0.4 protocol description for impl reference | ⏳ (post is narrative ; need a normative protocol doc) |
| `docs/SECURITY-MODEL.md` — threat model + trust boundaries for the A2A protocol | ⏳ (security engineers want STRIDE/asset list, not just SECURITY.md report process) |
| `assets/aphrody-logo.svg` — vector logo for README badges + favicons | ⏳ (visual identity = first impression on HN landing) |
| `docs/MIGRATION.md` — from just/taskfile/gh to aphrody (adoption path) | ⏳ (D+21 user-feature post-launch — reduces "ok but how do I switch?" friction) |
| `scripts/install.{sh,ps1}` audit + curl-bash one-liner doc | ⏳ (`curl -fsSL https://aphrody.dev/install.sh | sh` = frictionless first-try) |
| `docs/cargo/SECURITY-DEEP.md` — extended supply-chain doc | ⏳ (audit-stage transparency for security-aware engineers) |
| `docs/PERFORMANCE.md` — bench claims with reproducible recipes | ⏳ (BENCHMARKS.md is short ; engineers want recipes) |
| `packaging/chocolatey/aphrody.nuspec` + install.ps1 | ⏳ (Chocolatey alongside scoop/winget — wider Windows reach) |
| `packaging/aur-bin/PKGBUILD` — pre-built binary AUR variant | ⏳ (alongside source AUR ; faster install for users) |
| `rust-toolchain.toml` audit + explicit nightly pin with date | ⏳ (hermetic build — reproducibility for security audit) |
| `docs/COMMUNITY.md` — community guidelines + future Discord/Matrix invite | ⏳ (engagement scaffold for post-Show HN influx) |
| `docs/PRIVACY.md` — telemetry-zero policy | ⏳ (legal hygiene + engineering trust) |
| `docs/cargo/PUBLISH-LADDER.md` — explicit topological publish order doc | ⏳ (publish ladder operational runbook) |
| `.github/workflows/security.yml` — gitleaks + trufflehog secret scan | ⏳ (3rd security layer after CodeQL + cargo-deny) |
| `AGENTS.md` — agent-facing onboarding for AI assistants working in repo | ⏳ (aphrody IS about agent coord — meta-document own usage by agents) |
| `scripts/dev-setup.{sh,cmd}` — single-script environment bootstrap | ⏳ (devcontainer handles it ; native dev setup script for non-codespace users) |
| `.github/workflows/bench.yml` — criterion CI gate with summary comment | ⏳ (perf regression detection ; backend bench shipped via #41) |
| `CONTRIBUTORS.md` — auto-generated contributor recognition stub | ⏳ (honor commit authors + AGENTS.md cross-link) |

## 4. Métriques de santé (snapshot 2026-05-17)

| Métrique | Valeur | Cible |
|---|---|---|
| Workspace members | 17 | 17 (post-pivot : 10 core + 5 mrx + aphrody-translate + aphrody-wasm ; google_os exclu) |
| `cargo check --locked` (Linux) | ✅ (cross x86_64-unknown-linux-gnu, exit 0) | < 1 min |
| `cargo clippy -- -D warnings` | ✅ (workspace --all-targets, 16,07 s, exit 0) | 0 erreur |
| `cargo deny check` | ✅ ok×4 | ok×4 |
| Repo size (sans target, sans mdi) | ~7 Go (vendor/bun dominant) | optimisé |
| Disque libéré (P1+pivot) | 1.2 Go (vendor/crates.io) + 4.6 Go (material-design-icons) | n/a |
| CVE ignorés (justifiés) | 12 | < 5 (après upstream alignment) |
| Cibles cross-platform bloquantes | 3 (Linux/Win/wasm) | 3 |

---

*Pour la vue d'ensemble exécutive, lire [`SOURCE_OF_TRUTH.md`](./SOURCE_OF_TRUTH.md).*
