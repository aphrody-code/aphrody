[//]: # (SPDX-License-Identifier: Apache-2.0)

# aphrody packaging

This directory contains distribution manifests and install scripts for each
platform target.

## Channels

| Subdirectory | Channel |
|---|---|
| `scoop/` | Windows — Scoop bucket manifest |
| `winget/` | Windows — WinGet manifest tree |
| `homebrew/` | macOS — Homebrew formula |
| `deb/` | Linux — `.deb` package metadata |
| `snap/` | Linux — Ubuntu Snap (Snap Store) manifest |
| `arch/` | Linux — Arch User Repository (AUR) `PKGBUILD` |
| `windows-terminal/` | Windows Terminal profile fragment |
| `install.sh` | One-liner install (Linux / macOS) |
| `install.ps1` | One-liner install (Windows PowerShell) |

## npm — `@aphrody-code/aphrody-wasm`

The WASM npm package is built and published separately from the native binaries.

### Build

Run from the workspace root:

```sh
D:\cargo\bin\wasm-pack.exe build \
  --target web \
  --release \
  crates/aphrody-wasm/ \
  --out-dir target/aphrody-wasm-pkg \
  --scope aphrody-code
```

Output lands in `crates/aphrody-wasm/target/aphrody-wasm-pkg/`.

### Authenticate

```sh
npm login --scope=@aphrody-code --registry=https://registry.npmjs.org
```

Enter the `aphrody-code` npm organisation credentials (or use an automation
token set in `NPM_TOKEN`).

### Publish

```sh
wasm-pack publish --access public crates/aphrody-wasm/target/aphrody-wasm-pkg
```

Or equivalently:

```sh
npm publish crates/aphrody-wasm/target/aphrody-wasm-pkg --access public
```

The `--access public` flag is required for scoped packages on the free npm tier.
