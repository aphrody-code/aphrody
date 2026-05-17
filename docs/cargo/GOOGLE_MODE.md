# Google Mode — Full coverage matrix (2026-05-17)

> État final de l'adoption des patterns Google Production-grade.
> Toutes les recommandations Android (AOSP), Chromium, Fuchsia, ChromeOS,
> et Mozilla supply-chain auxquelles le projet peut s'aligner.

## Matrice de couverture

| Catégorie | Pattern source | Notre implémentation | Statut |
|---|---|---|---|
| **Toolchain pinning** | Chromium `tools/rust/` | `rust-toolchain.toml` nightly + 14 targets | ✅ |
| **Workspace deps centralisées** | Android `rustlibs:` | `[workspace.dependencies]` 80 deps | ✅ |
| **Workspace inheritance** | Android Soong | `[workspace.package]` + `*.workspace = true` | ✅ |
| **`crate-type` cdylib+rlib** | Android `rust_ffi` | `bun_ffi`, `google_os`, `python_ffi` | ✅ |
| **Lints android-strict** | Android `lints: "android"` | Preset opt-in `docs/cargo/LINTS.md` | ✅ |
| **`cxx` primary FFI** | Chromium | `cxx` + `cxx-build` workspace deps prêts | ✅ |
| **`bindgen` large APIs** | Chromium `//build/rust/rust_bindgen.gni` | `bindgen` workspace dep prêt | ✅ |
| **Crubit auto-bindings** | Chromium (exp) | Non adopté (comme Chromium) | ⏸ |
| **Supply-chain audits** | Fuchsia / ChromeOS `cargo-vet` | 7 feeds (Google/Mozilla/Fuchsia/ChromeOS/BCA/Embark/Zcash) | ✅ |
| **`ub-risk-*` criteria** | Fuchsia | 4 niveaux dans `supply-chain/audits.toml` | ✅ |
| **`cargo-deny`** | Mozilla supply-chain | `deny.toml` 4 axes (CVE/licence/bans/sources) | ✅ |
| **`cargo-auditable` SBOM** | 2026 industry standard | Alias `dist-auditable`, profil `reproducible` | ✅ |
| **Lockfile-only (no vendor)** | Modern Go module proxy style | `Cargo.lock` SHA-256 + sparse registry | ✅ |
| **Cross-platform binary** | Chromium release engineering | 14 targets + `crates/cli/src/platform.rs` | ✅ |
| **Android NDK targets** | AOSP | 4 archis Android dans toolchain + doc | ✅ |
| **MUSL static** | Distroless Docker | `Dockerfile` MUSL static + 2 targets | ✅ |
| **Fuzz at attacker boundary** | Fuchsia 2026 best practice | `crates/bun_ffi/fuzz/` libfuzzer-sys | ✅ |
| **`miri` UB detector** | Rust nightly standard | Composant + alias `cargo miri-test` | ✅ |
| **`cargo-careful` extra UB** | rust-secure-code | Alias + profile `careful` | ✅ |
| **Sanitizers (ASAN/MSAN/TSAN)** | Chromium / Google sanitizer suite | Aliases + profile `asan` | ✅ |
| **Deterministic builds** | Bazel / Buck reproducibility | `--remap-path-prefix`, profile `reproducible`, `SOURCE_DATE_EPOCH` | ✅ |
| **CI matrix multi-OS** | Chromium / Fuchsia waterfalls | `.github/workflows/cross-platform.yml` 6 OS + Android NDK | ✅ |
| **Release multi-target** | Chromium release builders | `.github/workflows/release.yml` 8 archis + SHA256 | ✅ |
| **Coverage llvm-cov** | Standard 2026 | `.github/workflows/coverage.yml` + Codecov | ✅ |
| **Docs CI + GH Pages** | Standard | `.github/workflows/docs.yml` rustdoc + mdBook | ✅ |
| **Pre-commit hooks** | Standard | `.pre-commit-config.yaml` fmt/clippy/deny/machete | ✅ |
| **`.editorconfig`** | Cross-IDE consistency | `.editorconfig` Google/Chromium style | ✅ |
| **VSCode workspace** | Rust ecosystem default | `.vscode/settings.json` + `extensions.json` | ✅ |
| **Issue/PR templates** | Standard | bug + feature + security + config.yml | ✅ |
| **`rustfmt` Fuchsia style** | Fuchsia `fx rustfmt` | `rustfmt.toml` unstable_features + 100w | ✅ |
| **Mutation testing** | rust-secure-code | Alias `cargo mutants` | ⏸ tooling présent, pas en CI |
| **Bin size analysis** | Standard | Alias `cargo bloat` + `bloat-fn` | ✅ |
| **Criterion benches** | Rust standard | Alias `cargo benches` | ✅ |

**Couverture totale** : **31/34 ✅ + 3 ⏸** (Crubit, mutation testing CI, et a2a-slimrpc upstream — tous trois bloqués sur dépendances externes).

## Topologie finale du workspace

```
aphrody/
├── .cargo/config.toml         ← target tuning + 30+ aliases (ci, dist, audit-*, build-*, fuzz, cov, …)
├── .clippy.toml               ← thresholds relâchés FFI/kernel
├── .editorconfig              ← cross-IDE
├── .github/
│   ├── PULL_REQUEST_TEMPLATE.md
│   ├── ISSUE_TEMPLATE/        ← bug + feature + security + config.yml
│   └── workflows/
│       ├── build.yml          ← Windows-only legacy
│       ├── cross-platform.yml ← matrix 6 OS + Android NDK
│       ├── coverage.yml       ← llvm-cov → Codecov
│       ├── release.yml        ← 8 archis sur tag
│       └── docs.yml           ← rustdoc + mdBook → GH Pages
├── .pre-commit-config.yaml    ← fmt + clippy + deny + machete + summary drift
├── .vscode/                   ← settings.json + extensions.json
├── Cargo.toml                 ← workspace.deps × 80 + lints + 6 profils
├── Cargo.lock                 ← SHA-256 pins
├── deny.toml                  ← CVE + licence + bans + sources Google-grade
├── rust-toolchain.toml        ← nightly + 14 targets + composants miri/cranelift/llvm-tools
├── rustfmt.toml               ← Fuchsia-style + unstable
├── supply-chain/
│   ├── config.toml            ← 7 imports d'audits (Google/Mozilla/Fuchsia/...)
│   ├── audits.toml            ← criteria ub-risk-0/1/2/3 + crypto-safe + audits locaux
│   └── imports.lock           ← pins des feeds upstream
├── Dockerfile                 ← distroless MUSL hermetic --locked
├── crates/cli/src/platform.rs ← abstractions OS cross-platform
├── crates/bun_ffi/fuzz/       ← libfuzzer attacker boundary
└── docs/cargo/                ← 12 pages (README, WORKSPACE, CRATES, PROFILES, LINTS,
                                   DEPENDENCIES, SUPPLY_CHAIN, FFI_POLICY, MIGRATION,
                                   CROSS_PLATFORM, CHROMIUM_ANDROID_PATTERNS,
                                   ANDROID_TARGET, CHEATSHEET, GOOGLE_MODE)
```

## Validation gate (Google-grade golden path)

Avant tout merge, ces 6 commandes doivent être vertes :

```bash
cargo ci-offline              # clippy --locked --offline -D warnings
cargo xt-offline              # nextest --locked --offline
cargo deny check              # advisories + bans + licenses + sources
cargo vet                     # signed audits ok
cargo audit-machete           # zero unused deps
bun run docs:summary:check    # SUMMARY drift
```

Et idéalement (CI matrix) :
```bash
cargo check -p aphrody --target <each-of-14-targets> --locked
```

## Outils à installer (Google Mode setup)

```bash
# Supply-chain
cargo install --locked cargo-vet cargo-deny cargo-machete

# Coverage / fuzz / mutation
cargo install --locked cargo-llvm-cov cargo-fuzz cargo-mutants cargo-careful

# Cross-compile
cargo install --locked cargo-zigbuild cargo-ndk
winget install -e --id Ziglang.Zig    # zig pour zigbuild

# Reproducible / SBOM
cargo install --locked cargo-auditable

# Performance
cargo install --locked cargo-bloat cargo-criterion

# Release
cargo install --locked cargo-dist cargo-release

# Tests
cargo install --locked cargo-nextest cargo-hack cargo-udeps

# Pre-commit
pipx install pre-commit && pre-commit install
```

## Roadmap restante (hors-scope automatisable)

1. **Audits cargo-vet réels** — convertir les exemptions auto en `safe-to-deploy` après review crate par crate (humain, ~70 deps).
2. **Premier `cxx::bridge` réel** — quand un binding C++ non trivial est introduit dans le projet.
3. **Mutation testing en CI** — coût compute élevé, attendre que la base de tests soit étoffée.
4. **a2a-slimrpc ré-intégré** — bloqué upstream `agntcy-slim-mls` (nightly lifetime issue).
5. **path-bases stable** — attendre Cargo 1.98+.
6. **Crubit Chromium** — suivre `crbug/470466915`, adopter quand officiel.
