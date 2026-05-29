<!-- SPDX-License-Identifier: Apache-2.0 -->
# Systems & OS Integration on Linux

Aphrody performs deep, low-level OS integrations to maximize performance, safety, and security when executing on Linux platforms.

---

## 1. Libc and Nix Interfaces

Instead of spawning shell processes for system queries, the Rust core links directly to system APIs using the `libc` and `nix` crates:
- **Process Management**: Querying child process states, signal dispatching, and process group control.
- **Terminal Control**: Managing pseudoterminals (PTYs), capturing raw terminal attributes, and formatting outputs.
- **Resource Limits**: Adjusting open file descriptor descriptors (`RLIMIT_NOFILE`) to prevent exhaustion.

---

## 2. Tokio Asynchronous Runtime Performance

On Linux, the asynchronous runtime (Tokio) is tuned for extreme throughput:
- **Event Loop**: Uses Linux-native `epoll` for efficient, non-blocking network and timer events.
- **Asynchronous Disk I/O**: High-throughput file read/writes leverage `io_uring` on modern kernels, bypassing traditional synchronous worker-thread bottlenecks.

---

## 3. Secure Isolation & Permissions Enforcement

To protect private user data and comply with supply-chain audits, credentials must be strictly isolated on the filesystem.

### Strict Modes
Aphrody enforces target permissions at creation and during load operations:
- **Folders**: Directories containing secrets (like `var/secrets/` and `~/.config/aphrody/`) are locked to mode `0700` (`rwx------`).
- **Files**: Private configuration or credential stores (like `google-cookies.json` and `antigravity-token.json`) are locked to mode `0600` (`rw-------`).

### Implementation Pattern (Rust)
```rust
use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;

pub fn enforce_private_permissions(path: &std::path::Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        let metadata = path.metadata()?;
        if metadata.is_dir() {
            std::fs::set_permissions(path, Permissions::from_mode(0o700))?;
        } else {
            std::fs::set_permissions(path, Permissions::from_mode(0o600))?;
        }
    }
    Ok(())
}
```

---

## 4. Subprocess Execution & Stream Capturing

Aphrody regularly drives other language toolchains (e.g. executing Python scraper scripts or TypeScript compiler outputs). The system execution module runs these processes with captured streams to:
1. Prevent poll/blocking locks.
2. Read exits codes reliably (avoiding shell pipeline masking).
3. Log errors to `var/forks/` or task logs without cluttering standard outputs.
