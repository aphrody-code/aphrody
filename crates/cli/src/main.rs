// SPDX-License-Identifier: Apache-2.0
//! `aphrody` binary entry point.
//!
//! A thin shell over the `aphrody` library crate (`src/lib.rs`): every command
//! lives in the library so the exact same surface is reachable in-process via
//! the `aphrody-ffi` cdylib (Bun `bun:ffi` / any C-ABI host), not only as a
//! spawned process. The global allocator is installed here (in the binary) so
//! the library stays allocator-agnostic and the `aphrody-ffi` cdylib can
//! install its own without a duplicate-`#[global_allocator]` error.

// mimalloc as the global allocator on native targets — fast multithreaded
// allocation for the tokio runtime + reqwest hot paths.
#[cfg(not(target_arch = "wasm32"))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> std::process::ExitCode {
    aphrody::run_native()
}

#[cfg(target_arch = "wasm32")]
fn main() {
    aphrody::run_wasm();
}
