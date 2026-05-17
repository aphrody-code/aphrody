<!-- SPDX-License-Identifier: Apache-2.0 -->
# Cross-platform Rust: one crate, three targets, zero excuses

> Aphrody dev journal, 2026-05-17.
> Author: aphrody-code &lt;noreply@aphrody-code.dev&gt;

[See post #1 on cross-Claude A2A coordination](./2026-05-ai-json.md)
[See post #2 on the parallel YOLO grind loop](./2026-05-yolo-grind-loop.md)

---

## The problem

Three targets. One codebase. No compromises.

The 17-crate aphrody workspace runs natively on Linux Ubuntu 26.04, Windows 11
Insider Canary, and compiles to WebAssembly (`wasm32-unknown-unknown` for
browsers, `wasm32-wasip1` for WASI runtimes). Each target has a completely
different I/O model, threading substrate, memory layout, and OS security API.
Linux gives you `io_uring`, splice calls, `/proc/<pid>`, and a kernel that
does what you ask. Windows gives you IOCP, ConPTY, DPAPI, and `NtQuerySystemInformation`.
WASM gives you a single-threaded event loop, no filesystem, and Web Crypto as
your only entropy source.

Most projects pick one of these targets as the real one, mark the others as
"best-effort", and quietly ship a binary that silently degrades on two thirds
of their stated matrix. Every six months someone opens an issue saying `cargo
build` fails on their platform, and the fix is a one-line `#[cfg(...)]` that
any maintainer could have written in 2 minutes if they had been running CI on
all three targets from day one.

Aphrody does not do this. Linux is the canonical build target — if it fails
there, nothing merges. Windows is the second gate. WASM is the third. All
three jobs in CI are marked fail-the-run; none has `continue-on-error: true`.
This post documents the concrete patterns that make this tenable without
turning the codebase into an unmaintainable tangle of conditional compilation.

---

## The cfg(...) discipline

The first instinct when you encounter platform-specific code is to split it
into a separate file: `mod windows_impl;`, `mod linux_impl;`, `mod wasm_impl;`,
each gated at the top of `mod.rs` with a `#[cfg]`. This feels clean. It is
not.

The problem with per-file gating is that you are now maintaining three
parallel sets of types, function signatures, doc comments, and test scaffolding.
When you refactor the shared logic you touch all three files. When a reviewer
reads the code they have to mentally switch between three files to understand
what a single function does on each platform. The diff for a trivial change
touches three files and triggers three clippy runs.

The pattern aphrody uses instead is per-function cfg gating, keeping the
platform-specific bodies adjacent to the shared interface:

```rust
pub fn process_entropy(buf: &mut [u8]) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        // getrandom(2) syscall directly — no libc wrapper
        linux_getrandom(buf)?;
    }
    #[cfg(target_os = "windows")]
    {
        // BCryptGenRandom via windows-rs
        windows_bcrypt_random(buf)?;
    }
    #[cfg(target_arch = "wasm32")]
    {
        // Web Crypto API via js-sys
        web_crypto_random(buf)?;
    }
    Ok(())
}
```

The full set of attributes in use across the workspace:

- `#[cfg(target_os = "linux")]` — Linux-only code paths (io_uring, `/proc`, netlink)
- `#[cfg(target_os = "windows")]` — Windows-only (DPAPI, ConPTY, NTDLL, IOCP)
- `#[cfg(target_arch = "wasm32")]` — any WASM target, WASI or browser
- `#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]` — browser-only
  (where `target_os = "unknown"` distinguishes the browser from WASI, which
  reports `target_os = "wasi"`)
- `#[cfg(not(target_arch = "wasm32"))]` — everything that is not WASM
  (native Linux + Windows + macOS); used heavily in `crates/cli/Cargo.toml` to
  pull in the full tokio runtime, reqwest, and backend

One rule: Windows-specific code **must never block the Linux build**. If a
module calls `NtQuerySystemInformation` or DPAPI, every call site is gated.
There is no "we'll fix the Linux build later". Commit `d222d0061` documents
what happens when this slips: five `cargo check` errors and a blocked CI run
until the `#[cfg(target_os = "windows")]` guards land on every affected call site.

---

## Per-target dependency declaration

Cargo's `[target.'cfg(...)'.dependencies]` table is how you express that a
dependency only exists on one target. The real `crates/base/Cargo.toml` has:

```toml
# getrandom is gated to wasm32 below; machete cannot see cfg-gated deps,
# so we suppress the false-positive warning here.
[package.metadata.cargo-machete]
ignored = ["getrandom"]

# Browser-WASM: getrandom must opt into the "js" backend so transitively
# pulled randomness (via aes-gcm -> aead -> rand_core -> getrandom) resolves
# to the Web Crypto API instead of erroring out at compile-time.
[target.'cfg(all(target_arch = "wasm32", target_os = "unknown"))'.dependencies]
getrandom = { version = "0.2", features = ["js"] }

[target.'cfg(windows)'.dependencies]
windows = { workspace = true, features = [
    "Win32_Foundation",
    "Win32_Security_Cryptography",
    "Win32_System_Memory",
    "Win32_System_Diagnostics_Debug",
    "Win32_System_LibraryLoader",
    "Win32_System_Threading",
] }
```

The `cargo-machete` metadata block deserves its own paragraph. `cargo machete`
detects unused dependencies by scanning `use` statements. It cannot see
dependencies that are only reachable through `#[cfg(...)]` gates because those
gates are evaluated by the compiler, not by machete's static scan. Without the
`ignored` annotation, machete would report `getrandom` as unused and a naively
automated `cargo fix` pass could delete it, breaking the WASM build silently.
The annotation is the explicit contract that says: "this dep is invisible to
machete for a known reason; do not autoremove it."

The same pattern applies across the workspace. `aphrody-wasm` has
`ignored = ["wasm-bindgen"]`, `a2a-pb` has its own list. Every cfg-gated
transitive dep that machete cannot follow gets an entry.

---

## Workspace-level dependency centralisation

With 17 crates that each need different slices of the same dependency graph,
the workspace `Cargo.toml` `[workspace.dependencies]` table is where every
version lives exactly once. Individual crate `Cargo.toml` files reference deps
with `{ workspace = true }` and add only the features they actually need.

This matters at the per-target level too. `crates/cli/Cargo.toml` splits deps
across three tables:

```toml
# Unconditional — builds on all targets
[dependencies]
clap          = { workspace = true }
clap_complete = { workspace = true }
serde         = { workspace = true }

# Native only (Linux / Windows / macOS)
[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
tokio         = { workspace = true }
reqwest       = { workspace = true }
mimalloc      = { workspace = true }
backend       = { path = "../backend" }
rustls        = { workspace = true }

# WebAssembly — minimal tokio surface
[target.'cfg(target_arch = "wasm32")'.dependencies]
tokio = { version = "1.52", default-features = false, features = ["sync", "macros", "io-util", "rt", "time"] }
```

The minimal tokio block for WASM is intentional: tokio's `net` and `process`
features pull in mio and platform I/O abstractions that do not compile on
wasm32. Selecting only `sync`, `macros`, `io-util`, `rt`, and `time` gives you
channels, `#[tokio::main]`, basic I/O utilities, a single-threaded runtime,
and timers — everything the WASM CLI stub needs without touching the
`tokio::net` or `tokio::process` subsystems that mio drives.

---

## Cross-compile with cargo-zigbuild

Cross-compiling from a Windows host to `x86_64-unknown-linux-gnu` without a
Linux VM is done via `cargo-zigbuild`:

```
cargo install cargo-zigbuild
cargo zigbuild --target x86_64-unknown-linux-gnu --release
```

`cargo-zigbuild` wraps Zig's C compiler as the linker, which understands the
Linux ELF ABI and the glibc symbol versioning requirements without needing a
sysroot installed locally. For most crates this just works.

There is one sharp edge documented in CLAUDE.md §7 and fixed in commit
`98cd26e43`: the `--icf=all` linker flag (Identical Code Folding, full mode)
is rejected by zigcc. The flag was in `.cargo/config.toml` under the
`x86_64-unknown-linux-gnu` rustflags entry for binary size reduction. zigcc
logs the error at link time and aborts the build. The fix is to drop `--icf=all`
and keep only `--gc-sections`, which zigcc handles correctly and covers the
vast majority of dead-code stripping. The size difference is roughly 1-3%
across the binary — acceptable given that the build now works.

---

## WASM-specific patterns

### tokio

`tokio = { features = ["full"] }` does not compile on wasm32. The `net`
subsystem pulls mio, which requires OS-level poll mechanisms that do not exist
in the browser sandbox. The `process` subsystem wants `fork(2)`. Use the
minimal feature set shown in the dep table above, and use `wasm-bindgen-futures`
to bridge between Rust futures and JavaScript promises.

### getrandom and entropy

Any crate in the dependency tree that eventually calls getrandom — and that
includes anything using `aes-gcm`, `ring`, `rand`, or similar — must be told
to use the `js` feature when compiling for `wasm32-unknown-unknown`. Without
it, getrandom attempts to call `getrandom(2)` or `/dev/urandom` and fails at
compile time with an error about missing symbols. The `features = ["js"]`
activation in `base/Cargo.toml` covers the entire transitive chain for browser
targets.

### reqwest in the browser

`reqwest` does not work in `wasm32-unknown-unknown` browser builds. It depends
on `hyper`, which depends on `tokio::net`, which depends on mio. For WASM use
`web-sys::fetch` directly or the higher-level `gloo-net` crate, which wraps
the browser Fetch API without pulling in native networking stacks.

### wasm32-wasip1

The CI matrix formerly used `wasm32-wasi` as the target name. This was
deprecated by rustup in favour of `wasm32-wasip1` (WASI Preview 1). Commit
`a1d5d97ca` aligned the workflow and local toolchain after rustup started
logging deprecation warnings. If your CI uses the old alias, pin and update
before the alias is removed entirely.

---

## CI matrix: all three gates, always failing

The `.github/workflows/cross-platform.yml` workflow runs the following jobs
every push and pull request, all in parallel:

| Job              | What it checks                                               | Fail-on-error |
|------------------|--------------------------------------------------------------|---------------|
| `lint`           | `cargo fmt --check`, `cargo clippy -D warnings`, all targets | yes           |
| `linux-priority` | `cargo check -p aphrody --target x86_64-unknown-linux-gnu`   | yes           |
| `linux-native`   | `cargo build --release` + `cargo nextest run` + smoke test   | yes           |
| `windows-priority` | `cargo check -p aphrody --target x86_64-pc-windows-msvc`   | yes           |
| `wasm-priority`  | `cargo check --target wasm32-unknown-unknown`                | yes           |
| `macos-native`   | same as linux-native                                         | best-effort   |
| `cross-extended` | niche targets (android, ARM, etc.)                           | yes           |
| `supply-chain`   | `cargo deny check` + `cargo vet`                             | yes           |
| `android`        | `cargo check --target aarch64-linux-android`                 | yes           |
| `docs`           | `mdbook build` + `cargo doc`                                 | yes           |

Every job pins the Rust toolchain to a commit SHA:

```yaml
- uses: dtolnay/rust-toolchain@5b842231ba77f5c045dba54ac5560fed2db780e2
```

This is more conservative than pinning to a semver tag. The SHA is the exact
commit on the `nightly` branch of the `dtolnay/rust-toolchain` action at the
time of writing. It will not silently change when a new nightly drops. If a
new nightly breaks something, the CI remains green on the pinned SHA while you
investigate; you bump the SHA deliberately when you confirm the new nightly is
clean.

One subtle issue caught during the initial CI wiring (commit `13514d98d`):
the supply-chain job failed because the workflow-level `RUSTC_WRAPPER=sccache`
environment variable was inherited by the `cargo deny` step running inside a
container where `sccache` was not installed. The fix was to unset
`RUSTC_WRAPPER` and `SCCACHE_GHA_ENABLED` in the supply-chain job's `env`
block. CI environment variables that enable build acceleration on native runners
can quietly break jobs that run in containers or without the accelerator binary.

---

## The rustls 0.23 CryptoProvider requirement

This one caused a real production panic: `aphrody --version` was panicking at
boot on Windows with a stack trace pointing to `reqwest`'s async client
internals.

The root cause: since rustls 0.23, the library requires an explicit
`CryptoProvider` to be installed before any TLS operation. Calling
`reqwest::Client::new()` without one causes a panic at the first handshake
attempt. In aphrody's case, `GoogleContext::new()` was called in the very first
line of `main()` and it constructed a `reqwest::Client`. The provider was never
installed. Fix, now at `crates/cli/src/main.rs:177`:

```rust
// Must come before the first reqwest::Client::new().
let _ = rustls::crypto::ring::default_provider().install_default();
```

The `let _ =` discards the `Err` variant that occurs if a provider was already
installed (it is idempotent after the first call; error means "already set",
which is fine). Commit `8859ca785` documents the smoke test that confirmed the
fix: `aphrody --version` on a Windows release build went from panic to clean
output.

This is not a bug in rustls. It is a deliberate design choice that forces
consumers to explicitly opt into a cryptography backend rather than silently
defaulting to one. It becomes surprising only when a dependency (`reqwest`) has
a transitive dependency on rustls that is not visible in the application's own
`Cargo.toml`. The fix is simple once you know where to look; the diagnostic
output is less obvious because the panic happens inside an internal assertion
deep in the TLS machinery.

---

## The mrx workspace_key path-separator bug

`mrx-audit` walks a monorepo and counts files per workspace (by `apps/<name>`
or `packages/<name>` directory). On Linux it worked. On Windows the
`file_count` for every workspace was zero.

The bug: the function that matched a relative file path against a workspace key
was comparing the display string of a `std::path::Path` — which uses backslash
as the separator on Windows — against a hardcoded key built with forward slashes
in the `format!("{top}/{name}")` call. On Linux, `std::path::MAIN_SEPARATOR`
is `/`, so the strings matched. On Windows, the display output was
`apps\utils` and the lookup key was `apps/utils`. The hash map lookup silently
returned `None` and the file counter was never incremented.

The fix was a one-line normalisation at the point where the display string
is produced:

```rust
let ws_key = ws_rel.display().to_string().replace('\\', "/");
```

Now the key is always forward-slash normalised before the hash map lookup,
matching the key that `workspace_key()` produces via `format!("{top}/{name}")`.
A unit test (`workspace_key_normalises_windows_paths`) pins this invariant.
The fix landed as part of commit `7addef68b` (yolo-tick11-14 mega-batch).

This class of bug — platform-specific path separator divergence that only
surfaces when a string representation is compared rather than a `Path` —
is one of the most common sources of silent failures in cross-platform Rust
code. The general rule: never compare path strings directly. Either compare
`Path` values, or normalise to a canonical form (forward slashes are the
de-facto cross-platform separator for human-readable keys) before storing or
comparing.

---

## Lessons from three targets

After running this matrix through an intensive 24-hour grind of parallel agents
and YOLO loops (see [post #2](./2026-05-yolo-grind-loop.md)), the patterns that
consistently cause friction are:

**Treat all three targets as first-class from the start.** Retrofitting
`#[cfg(...)]` guards onto a codebase that was written assuming a single target
is painful and error-prone. It is far easier to write the guard at the first
commit that introduces OS-specific behaviour than to audit every call site six
months later when you decide to add WASM support.

**Make CI loud on any target regression.** Every target job in the matrix is
set to fail the run. The moment a change breaks the WASM build, the PR cannot
merge. This sounds obvious but many projects set WASM to `continue-on-error`
and then wonder why it is always broken.

**Document every `#[cfg(...)]` gotcha in a single place.** CLAUDE.md §7 ("Known
traps / institutional memory") is where aphrody accumulates these: the
`--icf=all` linker flag, the rustls CryptoProvider requirement, the WASM tokio
feature set, the getrandom `js` feature chain, the wasm32-wasi deprecation, the
path separator normalisation. They accumulate fast. If they are not written
down, the next agent or engineer hits the same wall.

**Pin the toolchain to a commit SHA, not a channel.** Nightly breakages are
real. A pinned SHA means the breakage is an explicit decision you make when you
bump, not a surprise that fails CI on a Tuesday morning before you have context.

**Normalise path representations at the boundary.** Any code that converts a
`Path` to a string for use as a map key, a display label, or a comparison must
normalise separators explicitly. The `replace('\\', "/")` line is not elegant;
it is correct.

---

## Try it

```bash
git clone https://github.com/aphrody-code/aphrody
cd aphrody

# Check all three priority targets
cargo check -p aphrody --target x86_64-unknown-linux-gnu --locked
cargo check -p aphrody --target x86_64-pc-windows-msvc --locked
cargo check -p aphrody --target wasm32-unknown-unknown --locked

# Full supply-chain check
cargo deny check
```

If all three `cargo check` invocations pass, you are working in the green zone.
If any one of them fails after your change, that is the ratchet. The discipline
is not complex; it just requires the CI to enforce it unconditionally.
