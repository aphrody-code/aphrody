<!-- SPDX-License-Identifier: Apache-2.0 -->
# GOOGLE.md â€” Alignement Google ecosystem

> **Statut :** 2026-05-17 â€” aligned production-grade cross-platform.
> Ce document remplace l'ancienne version Â« God Mode Â» par une description factuelle
> des points d'alignement avec les Ã©cosystÃ¨mes Android, Chromium et Fuchsia.

---

## 1. Alignement Canary

Le projet suit officiellement les branches **Canary** des Ã©cosystÃ¨mes Google :
- **Chrome Canary** (`Google.Chrome.Canary` via WinGet) â€” rÃ©fÃ©rence pour le rendu web.
- **Android Studio Canary** (`Google.AndroidStudio`) â€” IDE de rÃ©fÃ©rence.
- **Cloud SDK** (`gcloud`, `gsutil`, `bq`) â€” dÃ©ploiement.
- **Rust nightly** (mirror du canal Chromium `tools/rust/`) â€” toolchain unique.

Catalogue Google complet : voir [`google.json`](../google.json).

## 2. Trinity Architecture (HISTORIQUE — abandonnée)

> **OBSOLÈTE.** La « Trinity Architecture » (God Mode / Google OS / kernel
> hybride NT + fork C++ de Windows Terminal) a été **abandonnée au pivot
> 2026-05-17** (cf. [`SOURCE_OF_TRUTH.md`](SOURCE_OF_TRUTH.md) §7). Les crates
> `bun_ffi`, `gui` et `google_os` ont été supprimés/archivés. État réel : un
> cœur 100 % Rust cross-platform (`crates/cli`, `base`, `backend`, …) + une
> surface UI Material Design 3 en Bun/TS (`packages/*`, `apps/web`). Table
> conservée pour mémoire historique uniquement.

| Pilier (historique) | Tech | Rôle (abandonné) |
|---|---|---|
| **I. Rust Core** | `crates/cli`, `base`, `backend` (`bun_ffi`/`gui`/`google_os` supprimés) | Binaire principal cross-platform, FFI zero-copy |
| **II. Native Terminal** | C++ `Microsoft.WindowsTerminalCanary` (Windows uniquement) | ❌ fork abandonné — terminal LLM-first en Rust pur |
| **III. Bun + MD3** | `packages/*` (`@aphrody-code/*`), `apps/web` | ✅ devenu la vraie surface UI (monorepo M3 Bun + Turborepo) |

**Axe principal cross-platform actuel** : le cœur Rust (`crates/cli`) — voir [`docs/cargo/CROSS_PLATFORM.md`](./cargo/CROSS_PLATFORM.md).

## 3. Patterns adoptÃ©s depuis Google

| Source | Pattern | Notre implÃ©mentation |
|---|---|---|
| **Android Soong** | `rust_library` + `rustlibs:` | `[workspace.dependencies]` + workspace inheritance |
| **Android Soong** | `rust_ffi` (cdylib+rlib) | `crate-type = ["cdylib", "rlib"]` |
| **Android Soong** | `lints: "android"` | Preset `android-strict` opt-in per-crate |
| **Android NDK** | Cross-compile pour Android | `cargo ndk` + 4 targets Android dans `rust-toolchain.toml` |
| **Chromium** | Memory-safe untrusted-data handling | Tout nouveau code en Rust (cf. CLAUDE.md) |
| **Chromium** | `cxx` primary FFI | `cxx` + `bindgen` dans `[workspace.dependencies]` |
| **Chromium** | Pinned toolchain (`tools/rust/`) | `rust-toolchain.toml` nightly pinnÃ© |
| **Fuchsia** | `cargo-vet` + critÃ¨res `ub-risk-*` | `supply-chain/audits.toml` avec ub-risk-0/1/2/3 |
| **Fuchsia** | `safe-to-deploy` default | `cargo deny check` requis avant merge |
| **ChromeOS** | Imports d'audits cross-project | `supply-chain/config.toml` import Google/Mozilla/Fuchsia/ChromeOS |

DÃ©tails : [`docs/cargo/CHROMIUM_ANDROID_PATTERNS.md`](./cargo/CHROMIUM_ANDROID_PATTERNS.md).

## 4. Hardware acceleration

- **Rust UI surfaces** (Pilier I) : backed by native Direct3D 11/12 sur Windows via `windows-rs Win32_Graphics_*`. Sur Linux/macOS, fallback CPU pour `cli` (pas de rendu graphique).
- **Web tier** (Pilier III) : Chromium GPU Process via `Chrome SxS\chrome.exe (Canary)`, WebGPU + WebGL forced active.

## 5. PrivilÃ¨ges systÃ¨me (Windows-only, `google_os`)

Le crate `google_os` (Windows-only, `#![cfg(windows)]`) expose les capacitÃ©s suivantes via `windows-rs` :
- **Firewall bypass** : `HNetCfg.FwPolicy2` COM injection.
- **Token privileges** : `SeDebugPrivilege`, `SeTakeOwnershipPrivilege`, `SeBackupPrivilege`, `SeRestorePrivilege`.
- **DPAPI decryption** : extraction des cookies/passwords Chromium (cf. `crates/backend/src/chromium.rs`).
- **NTDLL direct** : `NtSetInformationFile` (FileDispositionInformation) pour suppression atomique (cf. `docs/google-os-plan/ntdll_bypass.md`).

**Politique :** ces capacitÃ©s sont disponibles UNIQUEMENT sur Windows et UNIQUEMENT depuis du code Rust audittÃ©. Pas d'exposition au tier Bun/JS.

## 6. Workflow contributeurs

Ordre de lecture pour tout nouveau contributeur :

1. [`README.md`](../README.md) â€” entrÃ©e du projet.
2. [`CLAUDE.md`](../CLAUDE.md) â€” directives techniques Rust/C/FFI.
3. [`GEMINI.md`](../GEMINI.md) â€” directives stratÃ©giques.
5. [`docs/cargo/CHROMIUM_ANDROID_PATTERNS.md`](./cargo/CHROMIUM_ANDROID_PATTERNS.md) â€” alignement Google.
6. [`docs/cargo/CROSS_PLATFORM.md`](./cargo/CROSS_PLATFORM.md) â€” axe principal.

## 7. Typography (UI Pilier III)

- **`Google Sans Flex`** (variable, opsz, wght) â€” UI principale.
- **`Google Sans Mono`** â€” code et terminal.
- **`Google Sans Text`** â€” long-form reading.

Source : Material Design 3 + Google Workspace Sans guidelines.

## 8. Outils Google externes utilisÃ©s

Liste exhaustive dans `google.json`. SynthÃ¨se :

| CLI | Provider | RÃ´le dans le projet |
|---|---|---|
| `gcloud` | Google Cloud SDK | Auth GCP, dÃ©ploiement |
| `dart` | Dart SDK | Compilateur AOT (rÃ©fÃ©rence) |
| `firebase` | Firebase CLI | Backend services |
| `protoc` | Protocol Buffers | SÃ©rialisation IPC (cf. `a2a-pb`) |
| `flatc` | FlatBuffers | SÃ©rialisation zero-copy |
| `adb`, `fastboot` | Android Platform-Tools | Android debugging |
| `osv-scanner` | Google OSV | Audit vulnÃ©rabilitÃ©s SBOM (post-`cargo-auditable`) |
| `perfetto` | Google Perfetto | Tracing systÃ¨me |
| `magika` | Google Magika | File type detection (build pipeline) |

---

*Pour la trajectoire complÃ¨te et les phases livrÃ©es, voir [`docs/PLAN.md`](./PLAN.md).*
