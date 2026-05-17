// SPDX-License-Identifier: Apache-2.0
//
// Criterion micro-benchmarks for the `base` crate.
//
// All benches here are pure-CPU: no network, no filesystem mutation, no
// subprocess spawn.  They exercise the algorithmic hot paths actually
// exported from `base/src/lib.rs`:
//
//   1. vfs_new                  — Vfs::new() construction cost (HashMap + cwd)
//   2. vfs_resolve_hit_short    — Vfs::resolve() prefix-match + PathBuf build
//   3. vfs_resolve_hit_deep     — Vfs::resolve() with a deeply nested path
//   4. vfs_resolve_miss         — Vfs::resolve() error branch (no mount match)
//   5. aes_gcm_decrypt_64b      — Crypto::decrypt_aes_gcm() on a 64 B payload
//   6. aes_gcm_decrypt_64kib    — Crypto::decrypt_aes_gcm() on a 64 KiB payload
//
// These cover both the `Vfs` and `Crypto` public surface, which are the only
// CPU-bound, cross-platform primitives the crate exposes.  `ProcessManager`
// is not benched because spawning real subprocesses is non-deterministic and
// not representative of in-process throughput.

use std::hint::black_box;

use aes_gcm::{Aes256Gcm, Key, KeyInit, Nonce, aead::Aead};
use base::{Crypto, Vfs};
use criterion::{Criterion, criterion_group, criterion_main};

// ---------------------------------------------------------------------------
// Helper: build a Chromium-format AES-GCM wire blob.
// Layout: [3 bytes version "v10"][12 bytes nonce][AES-GCM ciphertext + 16 B tag]
// `Crypto::decrypt_aes_gcm` parses exactly this layout.
// ---------------------------------------------------------------------------
fn build_aes_gcm_wire(key: &[u8; 32], nonce: &[u8; 12], plaintext: &[u8]) -> Vec<u8> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let ct = cipher
        .encrypt(Nonce::from_slice(nonce), plaintext)
        .expect("AES-GCM encrypt must succeed in bench setup");

    let mut wire = Vec::with_capacity(3 + 12 + ct.len());
    wire.extend_from_slice(b"v10");
    wire.extend_from_slice(nonce);
    wire.extend_from_slice(&ct);
    wire
}

// ---------------------------------------------------------------------------
// Bench 1: Vfs::new() — construction throughput.
// Reads `std::env::current_dir()` (one syscall on first call, then likely
// cached by the OS) and populates a 4-entry HashMap.  This is a one-shot
// startup cost for any consumer of the crate.
// ---------------------------------------------------------------------------
fn bench_vfs_new(c: &mut Criterion) {
    c.bench_function("vfs_new", |b| {
        b.iter(|| {
            let vfs = Vfs::new().expect("Vfs::new must succeed in a valid working directory");
            black_box(vfs);
        });
    });
}

// ---------------------------------------------------------------------------
// Bench 2: Vfs::resolve() — hit on `/bin` (shortest mount).
// Exercises the HashMap iteration, prefix-match, remainder trim, and
// PathBuf::push chain.  Path hits on the first or second mount tried.
// ---------------------------------------------------------------------------
fn bench_vfs_resolve_hit_short(c: &mut Criterion) {
    let vfs = Vfs::new().expect("Vfs::new must succeed in a valid working directory");

    c.bench_function("vfs_resolve_hit_short", |b| {
        b.iter(|| {
            let result = vfs.resolve(black_box("/bin/aphrody"));
            black_box(result.expect("mount /bin must resolve"));
        });
    });
}

// ---------------------------------------------------------------------------
// Bench 3: Vfs::resolve() — hit on `/var/mirror` with deep nesting.
// Exercises the same path but with a longer remainder that triggers more
// PathBuf component pushes.
// ---------------------------------------------------------------------------
fn bench_vfs_resolve_hit_deep(c: &mut Criterion) {
    let vfs = Vfs::new().expect("Vfs::new must succeed in a valid working directory");

    c.bench_function("vfs_resolve_hit_deep", |b| {
        b.iter(|| {
            let result =
                vfs.resolve(black_box("/var/mirror/fonts/google/sans/v32/regular-latin.woff2"));
            black_box(result.expect("mount /var/mirror must resolve"));
        });
    });
}

// ---------------------------------------------------------------------------
// Bench 4: Vfs::resolve() — miss branch (no mount prefix matches).
// Exercises the worst case: every mount iterated, no match, anyhow error
// constructed.  This is the fast failure path that callers rely on.
// ---------------------------------------------------------------------------
fn bench_vfs_resolve_miss(c: &mut Criterion) {
    let vfs = Vfs::new().expect("Vfs::new must succeed in a valid working directory");

    c.bench_function("vfs_resolve_miss", |b| {
        b.iter(|| {
            let result = vfs.resolve(black_box("/this/path/is/not/mounted/anywhere"));
            // The miss path returns Err; consume it without panicking.
            black_box(result.is_err());
        });
    });
}

// ---------------------------------------------------------------------------
// Bench 5: Crypto::decrypt_aes_gcm() — 64 B payload (Chromium cookie size).
// Mirrors the per-cookie decrypt cost in a credential-extraction sweep.
// Fixed-cost authenticated-decryption overhead dominates here.
// ---------------------------------------------------------------------------
fn bench_aes_gcm_decrypt_64b(c: &mut Criterion) {
    let key = [0x42u8; 32];
    let nonce = [0x11u8; 12];
    let plaintext = vec![0xabu8; 64];
    let wire = build_aes_gcm_wire(&key, &nonce, &plaintext);

    c.bench_function("aes_gcm_decrypt_64b", |b| {
        b.iter(|| {
            let result = Crypto::decrypt_aes_gcm(black_box(&wire), black_box(&key));
            black_box(result.expect("AES-GCM decrypt must succeed in bench"));
        });
    });
}

// ---------------------------------------------------------------------------
// Bench 6: Crypto::decrypt_aes_gcm() — 64 KiB payload (large blob).
// Demonstrates throughput-dominated regime where the AES-GCM block cipher
// per-byte cost outweighs the fixed setup/tag-verify overhead.
// ---------------------------------------------------------------------------
fn bench_aes_gcm_decrypt_64kib(c: &mut Criterion) {
    let key = [0x42u8; 32];
    let nonce = [0x11u8; 12];
    let plaintext = vec![0xabu8; 64 * 1024];
    let wire = build_aes_gcm_wire(&key, &nonce, &plaintext);

    c.bench_function("aes_gcm_decrypt_64kib", |b| {
        b.iter(|| {
            let result = Crypto::decrypt_aes_gcm(black_box(&wire), black_box(&key));
            black_box(result.expect("AES-GCM decrypt must succeed in bench"));
        });
    });
}

criterion_group!(
    benches,
    bench_vfs_new,
    bench_vfs_resolve_hit_short,
    bench_vfs_resolve_hit_deep,
    bench_vfs_resolve_miss,
    bench_aes_gcm_decrypt_64b,
    bench_aes_gcm_decrypt_64kib,
);
criterion_main!(benches);
