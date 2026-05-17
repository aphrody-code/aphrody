<!-- SPDX-License-Identifier: Apache-2.0 -->

# base

Foundational primitives for Aphrody crates, `no_std`-compatible where the target permits.
Every other `aphrody-code/*` crate depends on `base` directly or transitively. It provides
virtual filesystem abstraction, child-process lifecycle management, AES-256-GCM
cryptography (Chromium v80+ wire format), and Windows-only helpers — DPAPI decryption
and DLL injection via Win32 — all behind strict `#[cfg(target_os = "windows")]` guards so
non-Windows targets are never affected.

## Install

```toml
[dependencies]
base = "1.0.0-canary"
```

Or pin to the monorepo:

```toml
[dependencies]
base = { git = "https://github.com/aphrody-code/aphrody", package = "base" }
```

## Features

No user-facing `[features]` table. Platform behaviour is selected automatically:

| Target | Activated |
|--------|-----------|
| `wasm32-unknown-unknown` | `getrandom/js` — routes entropy through the Web Crypto API |
| `cfg(windows)` | `windows-rs` with Foundation, Cryptography, Memory, Threading, LibraryLoader, Diagnostics |

`anyhow`, `tracing`, and `aes-gcm` are unconditional on all targets.

## Cross-platform

- **Linux** — native, no OS-specific code; compiles cleanly against `std`.
- **Windows** — full `windows-rs` integration; DPAPI and injection APIs are Windows-only at
  the type level, never present in other target compilations.
- **WASM (`wasm32-unknown-unknown` + `wasm32-wasip1`)** — AES-GCM works without changes;
  `getrandom` resolves to the browser Web Crypto API so there is no runtime panic.

## Public API

### `Vfs::resolve`

```rust
pub fn resolve(&self, unix_path: &str) -> Result<PathBuf>
```

Translates an abstract Unix-style mount point (`/var/mirror`, `/tmp`, `/etc/google`, `/bin`)
to a concrete `PathBuf` rooted at the process working directory.

### `Vfs::initialize_physical_layout`

```rust
pub fn initialize_physical_layout(&self) -> Result<()>
```

Materialises every configured mount as a real directory, creating missing ones with
`fs::create_dir_all`.

### `ProcessManager::spawn`

```rust
pub fn spawn(&mut self, name: &str, command: &str, args: &[&str]) -> Result<()>
```

Launches a named child process and tracks it by name for later inspection or restart.

### `Crypto::decrypt_aes_gcm`

```rust
pub fn decrypt_aes_gcm(ciphertext: &[u8], key: &[u8]) -> Result<Vec<u8>>
```

Decrypts data using AES-256-GCM. Expected wire format: `[3-byte version][12-byte
nonce][ciphertext]` — the format written by Chromium v80+.

### `Crypto::decrypt_dpapi` / `injector::inject_dll` (Windows only)

```rust
#[cfg(target_os = "windows")]
pub fn decrypt_dpapi(data: &[u8]) -> Result<Vec<u8>>

#[cfg(target_os = "windows")]
pub fn inject_dll(pid: u32, dll_path: &str) -> Result<()>
```

`decrypt_dpapi` unwraps a DPAPI-encrypted blob via `CryptUnprotectData` and frees the
Win32-allocated buffer with `LocalFree`. `inject_dll` writes a DLL path into the target
process via `VirtualAllocEx` / `WriteProcessMemory` / `CreateRemoteThread`.

## Examples

```rust
use base::Vfs;

let vfs = Vfs::new()?;
vfs.initialize_physical_layout()?;
let path = vfs.resolve("/tmp/scratch.log")?;
println!("{}", path.display());
```

```rust
use base::Crypto;

// raw_key is the AES master key from DPAPI (Windows) or the system keychain.
let plaintext = Crypto::decrypt_aes_gcm(&ciphertext, &raw_key)?;
```

## License

Apache-2.0. See [LICENSE](../../LICENSE) at the repository root.

## Related crates

Once published, the rest of the `aphrody-code` family will be at:

- [`aphrody`](https://crates.io/crates/aphrody) — main CLI binary
- [`backend`](https://crates.io/crates/backend) — forensics and network layer
- [`a2a-pb`](https://crates.io/crates/a2a-pb) — A2A protobuf types
- [`a2a-client`](https://crates.io/crates/a2a-client) — A2A gRPC client
- [`a2a-server`](https://crates.io/crates/a2a-server) — A2A gRPC server
- [`a2a-grpc`](https://crates.io/crates/a2a-grpc) — A2A gRPC transport layer
