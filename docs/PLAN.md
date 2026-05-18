<!-- SPDX-License-Identifier: Apache-2.0 -->
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
| `cargo install aphrody` documenté | ✅ (README + `docs/launch/SHOW-HN.md` mentionnent la commande ; publication crates.io toujours pendante cf. ligne 134) |
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
- [x] **reqwest 0.13** : drop `aws-lc-sys` propre (retirer 4 CVE ignorés).
- [x] **pyo3 0.22** : fix CVE PyString (RUSTSEC-2025-0020).

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
| Re-rendre asciinema cast `assets/aphrody-demo.cast` avec `aphrody doctor` (+ couleurs) | ✅ (`assets/aphrody-doctor-demo.cast` 111 l. — companion cast doctor-focused, demo cast préservé) |
| Fresh clippy `-D warnings` audit + fix résidu Oracle gate 2 `double_ended_iterator_last` | ✅ (`cargo clippy --workspace --all-targets --locked --offline -- -D warnings` exit 0, no warnings) |
| Deuxième post technique `docs/posts/2026-05-yolo-grind-loop.md` (architecture 4-agent parallel grind) | ✅ (297 l., D+14 milestone arc, complète post a2a-ai-json) |
| Show HN launch package `docs/launch/SHOW-HN.md` (title + body + comment templates draft) | ✅ (112 l. draft — title + body + comment templates) |
| `n2b` aggressive scan + auto-migrate `scripts/**/*.{ts,mjs,js}` node→bun | ✅ (`docs/audits/2026-05-17-n2b-scan.md`, 6 findings, 2 auto-migrated) |
| `mrx` aggressive `scan/audit/detect` du workspace → rapport `docs/audits/2026-05-17-mrx-aggressive.md` | ✅ (rapport shippé, 5 crates mrx-* couvertes) |
| `bxc` scrape via A2A ask peer winclean → AGNTCY spec page + M3 tokens, mirror aphrody side | ✅ (`docs/audits/2026-05-17-bxc-scrape-request.md` — envelope `apx-ask-bxc-scrape-1` shipped via http_jsonrpc + file_jsonl) |
| `aphrody completions {bash,zsh,fish,pwsh,elvish}` subcommand via `clap_complete` | ✅ (`crates/cli/src/main.rs:140` `Commands::Completions { shell }` via `clap_complete::Shell`) |
| Integration test `crates/cli/tests/doctor.rs` — `assert_cmd` driven smoke of `aphrody doctor` + `--json` | ✅ (150 l. assert_cmd-driven smoke + --json) |
| Supply-chain `audits.toml` + `config.toml` formatting drift fix (Oracle gate 5 partial) | ✅ (cargo vet fmt appliqué sans effet de bord) |
| Marketing `docs/COMPARISON.md` — aphrody vs just/taskfile/gh/devcontainer/asdf | ✅ (94 l. — vs just/taskfile/gh/devcontainer/asdf) |
| ADRs `docs/adr/{0001-cross-platform-rust,0002-a2a-file-based,0003-yolo-parallel-grind}.md` | ✅ (86 + 94 + 95 l., template 0000 inclus) |
| WASM browser playground `crates/aphrody-wasm/examples/browser-playground.html` + crate README upgrade | ✅ (584 l. HTML playground + 62 l. crate README) |
| `.devcontainer/devcontainer.json` one-click Codespace setup (rust nightly + bun + cargo extras) | ✅ (60 l. devcontainer manifest) |
| `SECURITY.md` verify + strengthen — concrete report process, supported versions, scope | ✅ (81 l. — report process + supported versions + scope) |
| `packaging/snap/snapcraft.yaml` + `packaging/arch/PKGBUILD` — Ubuntu Snap + Arch AUR distribution expansion | ✅ (50 l. + 65 l. — Snap manifest + Arch PKGBUILD) |
| `crates/backend/benches/backend_bench.rs` — criterion benchmark suite | ✅ (182 l. criterion benchmark suite) |
| `docs/FAQ.md` + `docs/ROADMAP.md` — anticipated Q + public 90-day roadmap | ✅ (101 l. + 68 l.) |
| `crates/mrx-cli/README.md` — usage doc with scan/detect/audit/watch examples | ✅ (129 l. — usage doc, scan/check/watch examples) |
| `crates/aphrody-translate/README.md` — translate CLI usage doc | ✅ (121 l. — CLI usage doc) |
| `.github/workflows/codeql.yml` — GitHub CodeQL security scanning | ✅ (70 l. CodeQL workflow) |
| `docs/extensions/` — a2a extension specs (file-transport, honest-delivery, context7-version-pinning) | ✅ (74 + 82 + 78 l. + index.md — 3 extension specs publishable) |
| `crates/base/README.md` — base crate docs (publish-ready) | ✅ (127 l. — publish-ready) |
| `crates/a2a-pb/README.md` + `crates/backend/README.md` — publish-ladder docs | ✅ (95 l. + 135 l.) |
| `crates/a2a-{client,server,grpc}/README.md` — 3 publish-ladder docs | ✅ (76 + 79 + 80 l.) |
| `docs/ARCHITECTURE.md` — workspace overview + ASCII module dep graph | ✅ (155 l. — workspace overview + dep graph) |
| `mrx-core/src/scan.rs` `workspace_key()` Windows path-sep bug fix per #30 audit | ✅ (fix lives in `crates/mrx-audit/src/lib.rs:362` — `replace('\\', "/")` + tests `workspace_key_normalises_windows_paths`) |
| `.github/workflows/release-please.yml` — Google release automation via Conventional Commits | ✅ (28 l. — release-please workflow scaffolded) |
| `crates/google_mcp/README.md` + `crates/a2a/README.md` + `crates/a2a-lf/README.md` | ✅ (85 + 80 + 90 l.) |
| `crates/mrx-{core,detect,audit,watch}/README.md` — 4 mrx lib crate docs | ✅ (75 + 77 + 85 + 81 l.) |
| Root `README.md` polish — sweep new docs into doc-tree TOC (COMPARISON/FAQ/ROADMAP/ADRs/extensions) | ✅ (README §Documentation now references COMPARISON/FAQ/ROADMAP/ADRs/extensions/SHOW-HN/posts/IEVR docs/WASM playground) |
| `.github/ISSUE_TEMPLATE/*` + `PULL_REQUEST_TEMPLATE.md` + `FUNDING.yml` — OSS template bundle | ✅ (bug_report + feature_request + question + config + PR template + FUNDING.yml) |
| `CHANGELOG.md` sweep — log 2026-05-17 mega-batch (30+ commits today) | ✅ (251 l. — Keep-a-Changelog updated with 2026-05-17 batch) |
| `packaging/nix/flake.nix` — Nix flake for Nix/NixOS users | ✅ (102 l. flake + README) |
| `packaging/flatpak/com.aphrody.aphrody.json` — Flatpak manifest | ✅ (36 l. Flatpak manifest + README + LICENSE.SPDX) |
| `.github/dependabot.yml` — monthly cargo + GH actions dep updates | ✅ (68 l. — cargo + actions update schedule) |
| `packaging/{scoop,winget,homebrew,deb}/*` version+arch sweep | ✅ (scoop 64bit+arm64, winget x64+arm64, homebrew on_macos+on_linux arm/intel, deb amd64+arm64 — all `1.0.0-canary` aligned to workspace.package.version, asset pattern `aphrody-v1.0.0-canary-<triple>.{zip,tar.gz}`) |
| `docs/PROTOCOL.md` — definitive a2a/v0.4 protocol description for impl reference | ✅ (190 l. — normative a2a/v0.4 protocol doc) |
| `docs/SECURITY-MODEL.md` — threat model + trust boundaries for the A2A protocol | ✅ (139 l. — threat model + trust boundaries) |
| `assets/aphrody-logo.svg` — vector logo for README badges + favicons | ✅ (`assets/aphrody-logo.svg` + `aphrody-mark.svg` SVG vectors) |
| `docs/MIGRATION.md` — from just/taskfile/gh to aphrody (adoption path) | ✅ (157 l. — adoption path from just/taskfile/gh) |
| `scripts/install.{sh,ps1}` audit + curl-bash one-liner doc | ✅ (`packaging/install.sh` 121 l. + `packaging/install.ps1` 108 l. + `packaging/INSTALL-ONELINER.md`) |
| `docs/cargo/SECURITY-DEEP.md` — extended supply-chain doc | ✅ (200 l. — extended supply-chain doc) |
| `docs/PERFORMANCE.md` — bench claims with reproducible recipes | ✅ (185 l. — reproducible bench recipes) |
| `packaging/chocolatey/aphrody.nuspec` + install.ps1 | ✅ (`aphrody.nuspec` 27 l. + `tools/chocolateyinstall.ps1` 20 l. + README) |
| `packaging/aur-bin/PKGBUILD` — pre-built binary AUR variant | ✅ (57 l. binary AUR PKGBUILD + README) |
| `rust-toolchain.toml` audit + explicit nightly pin with date | ✅ (24 l. — channel `nightly-2026-05-17`, 6 targets, hermetic) |
| `docs/COMMUNITY.md` — community guidelines + future Discord/Matrix invite | ✅ (109 l. — community guidelines) |
| `docs/PRIVACY.md` — telemetry-zero policy | ✅ (103 l. — telemetry-zero policy) |
| `docs/cargo/PUBLISH-LADDER.md` — explicit topological publish order doc | ✅ (126 l. — topological publish order runbook) |
| `.github/workflows/security.yml` — gitleaks + trufflehog secret scan | ✅ (98 l. — gitleaks + trufflehog secret scan workflow) |
| `AGENTS.md` — agent-facing onboarding for AI assistants working in repo | ✅ (144 l. — agent-facing onboarding) |
| `scripts/dev-setup.{sh,cmd}` — single-script environment bootstrap | ✅ (`dev-setup.sh` 130 l. + `dev-setup.cmd` 124 l.) |
| `.github/workflows/bench.yml` — criterion CI gate with summary comment | ✅ (75 l. — criterion CI gate) |
| `CONTRIBUTORS.md` — auto-generated contributor recognition stub | ✅ (49 l. — contributor recognition + AGENTS.md cross-link) |
| `.github/DISCUSSION_TEMPLATE/*.yml` — Discussion templates (Q+A, ideas, show-and-tell) | ✅ (qna.yml + ideas.yml + show-and-tell.yml) |
| `scripts/release.sh` — operational release helper (tag + push + release-please trigger) | ✅ (239 l. — operational release helper) |
| `crates/aphrody-wasm/tests/wasm_smoke.rs` — wasm-bindgen integration test | ✅ (67 l. — wasm-bindgen integration test) |
| `docs/POST-LAUNCH.md` — Show HN +24h/+72h/+7d engagement protocol | ✅ (107 l. — Show HN engagement protocol) |
| `docs/EXAMPLES.md` — recipe collection (curl bash, doctor outputs, A2A samples) | ✅ (249 l. — recipe collection) |
| `docs/posts/2026-05-cross-platform-rust.md` — 3rd technical post on cross-platform Rust | ✅ (405 l. — 3rd technical post on cross-platform Rust) |
| `scripts/verify-publish.sh` — pre-publish dry-run gate sweeper | ✅ (248 l. — pre-publish dry-run gate sweeper) |
| `crates/cli/examples/doctor_consumer.rs` — example consuming doctor --json output | ✅ (62 l. — example consuming doctor --json) |
| `docs/CI-CD.md` — overview of all 10+ GitHub Actions workflows + roles | ✅ (164 l. — overview of 10 GitHub Actions workflows) |
| `crates/aphrody-wasm/src/lib.rs` — `encrypt_aes_gcm` companion to existing decrypt | ✅ (`crates/aphrody-wasm/src/lib.rs:95` `pub fn encrypt_aes_gcm` + round-trip test) |
| `scripts/changelog-since.sh` — Conventional Commits since last tag (release prep) | ✅ (184 l. — Conventional Commits since last tag) |
| `docs/RELEASE-CHECKLIST.md` — per-release maintainer checklist before tag | ✅ (108 l. — per-release maintainer checklist) |
| `docs/INDEX.md` — master index of all 60+ docs (auto-generated by script ideally) | ✅ (94 l. — master docs index) |
| `packaging/rpm/aphrody.spec` — Fedora/RHEL RPM spec | ✅ (91 l. — RPM spec + README) |
| `packaging/desktop/aphrody.desktop` — XDG .desktop file for Linux DEs | ✅ (26 l. .desktop entry + README) |
| `docs/SUMMARY.md` regenerated via `cargo run -p aphrody-summary` | ✅ (531 l. — mdBook SUMMARY regenerated) |
| `assets/aphrody-social-preview.svg` — GitHub OG card 1280×640 SVG | ✅ (`assets/aphrody-social-preview.svg` SVG card shipped) |
| `docs/cargo/PROFILES.md` — Cargo workspace profile reference | ✅ (110 l. — workspace profile reference) |
| `scripts/sbom-extract.sh` — extract auditable SBOM from built binary | ✅ (319 l. — SBOM extraction helper) |
| `docs/PERFORMANCE-HISTORY.md` — bench ledger / regression tracking | ✅ (91 l. — bench ledger / regression tracking) |

### Phase T — Terminal LLM-first WASM+M3 (génération tick post-PLAN-MOONSHOT)

**Pivot 2026-05-17 soir** : aphrody-terminal n'est PAS un wterm/Windows-Terminal
clone. C'est le **terminal LLM-first** — conçu spécifiquement pour sub-agents,
skills, hooks, MCP servers, Ink/React TUIs (Claude Code + Gemini CLI), avec
**JSON output partout**, **markdown rendu inline**, **config JSON full**.
Spec normative : [`docs/design/aphrody-terminal-spec.md`](design/aphrody-terminal-spec.md).

Worktrees référence : `vercel-labs/wterm` (API surface, TS+Zig WASM Apache-2.0)
+ `microsoft/terminal` (algorithmes Buffer/Renderer/AtlasEngine/ConPTY MIT).
Politique : §2 CLAUDE.md "WASM Rust natif", memory `project_terminal_integration_policy`.

| Tick | Tâche | Statut |
|---|---|---|
| T-1 | Worktrees `vercel-labs/wterm` + `microsoft/terminal` (15 entries, ~1095 MB) | ✅ |
| T-1 | `docs/design/aphrody-terminal-spec.md` — spec normative LLM-first | ✅ |
| T-1 | `crates/aphrody-terminal-vt` — VT base (vte + ScreenBuffer + SGR 16-color) | ✅ (`crates/aphrody-terminal-vt/src/lib.rs`, 703 l.) |
| T-1 | `crates/aphrody-terminal-wasm` — wasm-bindgen DOM + M3 + keyboard ANSI | ✅ (`crates/aphrody-terminal-wasm/src/lib.rs`, 361 l.) |
| T-1 | `crates/aphrody-terminal-backend` — portable-pty (ConPTY/openpty) + WS | ✅ (`crates/aphrody-terminal-backend/src/lib.rs`, 287 l.) |
| T-2 | VT extension Ink/React TUI essentials (alt-screen, mouse SGR 1006, true color 24-bit, cursor save/restore, bracketed paste, focus events, DECSTBM, insert/delete line, OSC 0 title, OSC 52 clipboard) | ✅ (`crates/aphrody-terminal-vt` tests passed) |
| T-3 | `crates/aphrody-terminal-llm` — sub-agent stream multiplexer + MCP status bus + hook event surface + skill activation slot | ✅ (`crates/aphrody-terminal-llm/src/lib.rs` + modules `sub_agent.rs`, `mcp.rs`, `hook.rs`, `skill.rs`, `task.rs`, `osc.rs`) |
| T-4 | `crates/aphrody-terminal-markdown` — comrak CommonMark + syntect highlight + OSC `aphrody-md` detector | ✅ (`crates/aphrody-terminal-markdown` built and tested) |
| T-5 | `crates/aphrody-terminal-json-out` — frame stdout/stderr in JSONL envelopes + passthrough app-JSON | ✅ (`crates/aphrody-terminal-json-out` built and tested) |
| T-6 | `crates/aphrody-terminal-config` — `~/.aphrody/terminal.json` strict schema + claude.json / mcp.json / settings.json import shims | ✅ (`crates/aphrody-terminal-config` built and tested) |
| T-6b | `crates/aphrody-terminal-browser` — bridge LLM ↔ DOM (bxc in-process + agent-browser RPC + edge headless fallback) + OSC `aphrody-browser-*` extensions | ✅ (`crates/aphrody-terminal-browser/src/lib.rs` 306 l. + backends `bxc.rs`, `agent_browser.rs`, `edge.rs` + `osc.rs` + `proto.rs`) |
| T-7 | `aphrody term` CLI subcommand (serves backend + prints WASM UI URL) | ✅ (`crates/cli/src/commands.rs:1411-1445` + `Commands::Term` dispatch dans `main.rs:238`) |
| T-7 | `crates/aphrody-wasm/examples/aphrody-terminal-demo.html` — pixel-perfect M3 demo showcase | ✅ (`crates/aphrody-wasm/examples/aphrody-terminal-demo.html` exists) |
| T-8 | Demo gif : Claude Code running inside aphrody-terminal w/ live sub-agent pane (D+8-15 hero asset) | ✅ (assets générés via agg resvg) |
| T-8 | `docs/audits/2026-05-17-wterm-vs-microsoft-terminal-vs-aphrody-terminal.md` | ✅ (audit doc présent) |
| T-9 | `packages/aphrody-jsx` — Bun-native react-reconciler → `aphrody-jsx-*` OSC bridge (Ink-compatible API, M3 native, dual target vt/wasm) | ✅ (`packages/aphrody-jsx/src/reconciler.ts` 455 l. + jsx-runtime + 6 components + 6 hooks + tests + examples) |
| T-10 | `crates/aphrody-tui` — pure Rust ratatui-style DSL (canonical long-term, 60fps target, zero JS) | ✅ (`crates/aphrody-tui` built and tested) |

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
