<!-- SPDX-License-Identifier: Apache-2.0 -->
# Cross-platform build

> Réf. : `.cargo/config.toml`, `rust-toolchain.toml`, `crates/cli/src/platform.rs`.
> Axe principal du projet (2026-05-17) : **un binaire Rust unique cross-platform** Windows / Linux / macOS / wasm.

## Cibles supportées

Déclarées dans `rust-toolchain.toml` :

| Target triple | Host typique | Statut binaire `cli` |
|---|---|---|
| `x86_64-pc-windows-msvc` | Windows x64 (dev par défaut) | ✅ Validé `cargo check --locked` |
| `aarch64-pc-windows-msvc` | Windows ARM | ✅ Code-level ; build pas validé |
| `x86_64-unknown-linux-gnu` | Linux x64 (CI / serveurs) | ✅ Code-level ; build requiert zigbuild |
| `aarch64-unknown-linux-gnu` | Linux ARM (Raspberry, AWS Graviton) | ✅ Code-level |
| `x86_64-apple-darwin` | macOS Intel | ✅ Code-level |
| `aarch64-apple-darwin` | macOS Apple Silicon | ✅ Code-level |
| `wasm32-unknown-unknown` | Browser sandbox | ⚠️ Partiel (pas de tokio multi-thread) |
| `wasm32-wasip1` | WASI runtimes | ✅ Code-level |

## Architecture cross-platform du `cli`

```
crates/cli/
├── src/
│   ├── main.rs         ← entrypoint, clap dispatch (PURE Rust, no cfg)
│   ├── commands.rs     ← logique métier, utilise platform::
│   ├── context.rs      ← état global (Arc<Vfs>, Arc<Md3Mirror>)
│   └── platform.rs     ← ABSTRACTION OS (local_app_data, home_dir, chrome_user_data...)
└── Cargo.toml          ← pas de [target.*.dependencies] = portable
```

**Règle d'or :** aucun `std::env::var("LOCALAPPDATA")` dans `commands.rs` — toujours passer par `platform::`.

## Module `cli/src/platform.rs` — abstractions disponibles

```rust
pub(crate) fn os_short_name() -> &'static str
    // → "windows" | "linux" | "macos" | "freebsd" | "wasm" | "unknown"

pub(crate) fn local_app_data() -> Result<PathBuf>
    // Win  : %LOCALAPPDATA%
    // mac  : $HOME/Library/Application Support
    // linux: $XDG_DATA_HOME ?? $HOME/.local/share

pub(crate) fn config_dir() -> Result<PathBuf>
    // Win  : %APPDATA%
    // mac  : $HOME/Library/Preferences
    // linux: $XDG_CONFIG_HOME ?? $HOME/.config

pub(crate) fn home_dir() -> Result<PathBuf>
    // Win  : %USERPROFILE% (fallback %HOME%)
    // *nix : $HOME

pub(crate) fn chrome_user_data() -> Option<PathBuf>
    // Win  : %LOCALAPPDATA%\Google\Chrome\User Data
    // mac  : $HOME/Library/Application Support/Google/Chrome
    // linux: $XDG_CONFIG_HOME ?? $HOME/.config / google-chrome

pub(crate) fn chrome_canary_user_data() -> Option<PathBuf>
    // Win  : %LOCALAPPDATA%\Google\Chrome SxS\User Data
    // mac  : $HOME/Library/Application Support/Google/Chrome Canary
    // linux: .../google-chrome-unstable
```

## Crates Windows-only

| Crate | Pourquoi Windows-only | Mécanisme |
|---|---|---|
| `google_os` | Pont POSIX↔NT, IOCP, NTDLL, DPAPI | `#![cfg(windows)]` au lib.rs → crate vide ailleurs |
| `bun_ffi` | `[target.'cfg(windows)'.dependencies]` sur `windows` | Stub portable possible (TODO) |
| `base` | DPAPI Windows-only | `[target.'cfg(windows)'.dependencies]` sur `windows` |
| `gui` | wry+tao tirent GTK3 sur Linux | Doit être désactivé pour CLI-only Linux build |

Les consumers cross-platform de ces crates doivent gater :

```toml
# Cargo.toml d'un consumer cross-platform
[target.'cfg(windows)'.dependencies]
google_os = { path = "../google_os" }
```

```rust
// src/lib.rs d'un consumer cross-platform
#[cfg(windows)]
use google_os::kernel::Process;
```

## Cross-compilation depuis Windows

### Pré-requis

Pour cross-compiler vers Linux / macOS depuis Windows MSVC, le linker GNU n'est pas dispo nativement. Solution Google-style :

```powershell
# Installer cargo-zigbuild (utilise zig comme cross-linker universel)
cargo install --locked cargo-zigbuild
# zig lui-même
winget install -e --id Ziglang.Zig
# Rustup target add (déjà couvert par rust-toolchain.toml)
```

### Aliases multi-target (`.cargo/config.toml`)

```bash
cargo build-win-x64        # natif MSVC
cargo build-win-arm64
cargo build-linux-x64      # via zigbuild
cargo build-linux-arm64    # via zigbuild
cargo build-darwin-x64     # via zigbuild
cargo build-darwin-arm64   # via zigbuild
cargo build-wasm           # wasm32-wasip1
```

Tous les aliases utilisent le profil `dist` (LTO fat + strip + panic=abort).

### Limites connues

- **`ring` / `aws-lc-sys`** : nécessitent un compilateur C cible (gcc pour Linux, cl/clang pour Windows). `zigbuild` fournit `zig cc` qui couvre tous les cas.
- **`gtk3-sys`** (tiré par `tao`/`wry` sur Linux/macOS) : pkg-config sysroot requis. Pour un CLI-only build, exclure `gui` :
  ```bash
  cargo build -p aphrody --target x86_64-unknown-linux-gnu  # skip workspace gui
  ```

## CI cross-platform (.github/workflows recommandé)

```yaml
strategy:
  matrix:
    target:
      - x86_64-pc-windows-msvc
      - x86_64-unknown-linux-gnu
      - aarch64-apple-darwin
runs-on:
  - ${{ contains(matrix.target, 'windows') && 'windows-latest'
       || contains(matrix.target, 'darwin')  && 'macos-latest'
       || 'ubuntu-latest' }}
steps:
  - uses: actions/checkout@v4
  - uses: dtolnay/rust-toolchain@nightly
  - uses: mozilla-actions/sccache-action@v0.0.4
  - run: cargo check -p aphrody --target ${{ matrix.target }} --locked
  - run: cargo nextest run -p aphrody --target ${{ matrix.target }} --locked
```

Le `rust-toolchain.toml` garantit que **chaque runner installe exactement la même nightly + composants**.

## Patterns Chromium / Android adoptés

### Chromium
- ✅ **Toolchain pinnée** via `rust-toolchain.toml` (équivalent `tools/rust/`).
- ✅ **Lints android-strict** disponibles en preset opt-in per-crate (cf. `docs/cargo/LINTS.md`).
- ✅ **Supply-chain audits** : `cargo-vet` avec feeds Google/Mozilla/Fuchsia/ChromeOS.
- ✅ **`cxx` + `bindgen`** ajoutés à `workspace.dependencies` (pas encore utilisés — quand surface FFI grandit).
- ⏸ **`crubit`** (expérimental, non adopté — comme Chromium).

### Android
- ✅ **`rust_library` ≡ workspace.dependencies + path deps** (équivalent `rustlibs:`).
- ✅ **`rust_ffi` ≡ `[lib] crate-type = ["cdylib", "rlib"]`** (cf. `google_os`, `bun_ffi`, `python_ffi`).
- ✅ **`rust_test` ≡ `cargo nextest run`** + `[dev-dependencies]` rstest/proptest/criterion.
- ✅ **`lints: "android"` ≡ preset `android-strict`** (opt-in per crate).
- ⏸ **`rust_bindgen` modules** ≡ build.rs avec `bindgen` (à intégrer quand bindings C nécessaires).
- ⏸ **`rust_fuzz`** ≡ `cargo fuzz` (à intégrer pour `google_os` / `bun_ffi`).

## Référence pour les contributors

- Toute nouvelle commande / API du `cli` doit fonctionner sur **les 3 OS principaux** (Windows/Linux/macOS).
- Pas de `std::env::var("LOCALAPPDATA")` ou équivalent direct — utiliser `platform::`.
- Si une fonctionnalité ne peut PAS être cross-platform (ex : DPAPI), elle vit dans une crate gated (`google_os`, `base`) et la lib appelante doit la gater.
- Les CI checks (`cargo ci-offline`) doivent passer sur les 3 plateformes avant merge.
