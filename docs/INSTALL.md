<!-- SPDX-License-Identifier: Apache-2.0 -->

# Installing aphrody

`aphrody` is a single static binary. Pick the channel that matches your OS
and package manager. Every command on this page is cross-checked against the
real manifests under [`packaging/`](../packaging/).

Supported targets (in strict priority order from `CLAUDE.md` §0):

1. **Linux** — Ubuntu 26.04 LTS amd64/arm64 (cible #1).
2. **Windows 11** — Insider Canary, x64/arm64 (cible #2).
3. **WebAssembly** — `wasm32-unknown-unknown` for browsers, `wasm32-wasip1`
   for serverless (cible #3).
4. **macOS** — Apple Silicon + Intel, best-effort.

---

## 1. Linux

### Ubuntu / Debian (cible #1)

```bash
# Option A — Debian package (recommended once GitHub Releases are live)
curl -L https://github.com/aphrody-code/aphrody/releases/latest/download/aphrody.deb -o aphrody.deb
sudo dpkg -i aphrody.deb
sudo apt -f install                  # resolves libc6 / libssl3 if missing

# Option B — Snap (classic confinement, auto-updates)
sudo snap install aphrody --classic

# Option C — POSIX one-liner (downloads tarball, verifies SHA-256, installs into ~/.local/bin)
curl -sSf https://raw.githubusercontent.com/aphrody-code/aphrody/main/packaging/install.sh | sh
```

### Arch / Manjaro / EndeavourOS

```bash
# AUR (paru / yay / pikaur — pick your helper)
yay -S aphrody

# Or build the PKGBUILD locally
git clone https://aur.archlinux.org/aphrody.git && cd aphrody && makepkg -si
```

### Fedora / openSUSE / Debian — Flatpak (universal)

```bash
flatpak install flathub com.aphrody.aphrody
flatpak run com.aphrody.aphrody --version
```

### NixOS / any system with Nix

```bash
# Run without installing
nix run github:aphrody-code/aphrody?dir=packaging/nix

# Or drop into a dev shell with the full Rust nightly + Bun + cargo-nextest
nix develop github:aphrody-code/aphrody?dir=packaging/nix
```

---

## 2. Windows (cible #2 — Windows 11 Insider Canary)

### Scoop (recommended for developers)

```powershell
scoop bucket add aphrody https://github.com/aphrody-code/scoop-bucket
scoop install aphrody
```

### Winget (recommended for first-time users)

```powershell
winget install aphrody-code.aphrody
```

The winget manifest is a `portable` installer scoped to the current user, so
no admin elevation is required.

### PowerShell one-liner (no package manager)

```powershell
irm https://raw.githubusercontent.com/aphrody-code/aphrody/main/packaging/install.ps1 | iex
```

### Direct download

Grab `aphrody-v<VERSION>-x86_64-pc-windows-msvc.zip` (or the `aarch64`
variant) from
[GitHub Releases](https://github.com/aphrody-code/aphrody/releases),
extract, and drop `aphrody.exe` somewhere on your `PATH`. Every archive
ships an adjacent `.sha256` you should verify with `Get-FileHash`.

---

## 3. macOS (best-effort, jamais bloquant pour merge)

### Homebrew (recommended)

```bash
brew install aphrody-code/tap/aphrody
```

The tap repository is published at `aphrody-code/homebrew-tap` and ships
bottles for both `aarch64-apple-darwin` and `x86_64-apple-darwin`.

### Build from source

```bash
brew install rustup-init pkg-config openssl
rustup-init --default-toolchain nightly -y
git clone https://github.com/aphrody-code/aphrody && cd aphrody
cargo build --release -p aphrody --locked
cp target/release/aphrody /usr/local/bin/
```

---

## 4. WASM (cible #3 — browser-runnable)

### Browser playground (no install)

Build the wasm bindings once and open the bundled HTML:

```bash
# From the repo root
wasm-pack build --target web --release crates/aphrody-wasm
open crates/aphrody-wasm/examples/browser-playground.html   # or xdg-open / start
```

### npm — `@aphrody-code/aphrody-wasm` (when published)

```bash
bun add @aphrody-code/aphrody-wasm
# npm install @aphrody-code/aphrody-wasm   # node interdit (cf. CLAUDE.md §2), but the package itself is npm-publishable
```

See [`packaging/README.md`](../packaging/README.md) for the canonical
`wasm-pack publish` recipe used by the release pipeline.

---

## 5. From source (every platform)

The source build is the universal fallback and the only path that always
works against the very latest commit on `main`.

```bash
# Prerequisites — Rust nightly + Bun (NOT node) + nextest + zigbuild
rustup install nightly
curl -fsSL https://bun.sh/install | bash
cargo install cargo-nextest cargo-zigbuild

# Clone + build the CLI
git clone https://github.com/aphrody-code/aphrody
cd aphrody
cargo build --release -p aphrody --locked
./target/release/aphrody --version
```

Linux-only build deps (Ubuntu/Debian):

```bash
sudo apt install -y build-essential pkg-config libssl-dev curl
```

Windows-only build deps: Visual Studio 2026 Build Tools with the
"Desktop development with C++" workload (MSVC, Windows 11 SDK 26100, CMake,
Ninja). Scoop or winget can install these too.

> **Note:** `cargo install --locked aphrody` will work once the crate is
> published to crates.io (the workspace currently keeps `publish = false`
> until `base`, `backend`, and the `a2a-*` family land upstream — track
> [`docs/PLAN.md`](PLAN.md)).

---

## 6. Verify the installation

```bash
aphrody --version
# aphrody 1.0.0-canary

aphrody doctor
# [runtime] rust nightly, ok
# [peer A2A coord] ai.json discoverable, inbox writable
# Verdict: HEALTHY
```

If `aphrody doctor` reports anything other than `HEALTHY`, jump to §8.

---

## 7. Uninstall

| Channel | Command |
|---|---|
| `.deb` (Ubuntu / Debian) | `sudo apt remove aphrody` |
| Snap | `sudo snap remove aphrody` |
| AUR / `makepkg` | `sudo pacman -R aphrody` |
| Flatpak | `flatpak uninstall com.aphrody.aphrody` |
| Nix flake | `nix profile remove aphrody` (only if installed via `nix profile install`) |
| Scoop | `scoop uninstall aphrody` |
| Winget | `winget uninstall aphrody-code.aphrody` |
| Homebrew | `brew uninstall aphrody` |
| `install.sh` | `rm "$HOME/.local/bin/aphrody"` |
| `install.ps1` | Delete `aphrody.exe` from the directory printed by the installer |
| From source | `rm -rf <repo>/target` and remove the copied binary |

---

## 8. Troubleshooting

Common installation issues (PATH not picked up, SHA-256 mismatch, OpenSSL
missing on Linux, nightly toolchain pin, GTK CVE warnings on `cargo deny`)
are catalogued in [`docs/TROUBLESHOOTING.md`](TROUBLESHOOTING.md) _when
published_. Until then, the fastest path is:

1. Re-run `aphrody doctor` and copy the failing section.
2. Open an issue at
   [github.com/aphrody-code/aphrody/issues](https://github.com/aphrody-code/aphrody/issues)
   with the `installation` label, paste the `doctor` output, and include
   `uname -a` (Linux/macOS) or `winver` (Windows).
3. For supply-chain or signature questions, see the SBOM embedded in every
   release binary via `cargo-auditable` (`cargo audit bin <path>`).

For the full distribution-channel matrix and release-pipeline details,
see [`packaging/README.md`](../packaging/README.md).
