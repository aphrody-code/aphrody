// SPDX-License-Identifier: Apache-2.0
//
// Criterion micro-benchmarks for the `backend` crate.
//
// All benches here are pure-CPU: no network, no filesystem I/O.  They exercise
// the algorithmic primitives that are hot in production paths:
//
//   1. vfs_resolve          — VFS mount-table lookup (HashMap + string slice)
//   2. dns_dedup_sort       — post-OSINT domain deduplication (sort + dedup)
//   3. aes_gcm_decrypt_1kb  — AES-256-GCM decryption of a 1 KiB payload
//   4. sha256_hash_1mb      — SHA-256 digest of a 1 MiB buffer (throughput)
//   5. serde_json_parse     — JSON parsing of a realistic crt.sh-shaped payload

use base::{Crypto, Vfs};
use criterion::{BatchSize, Criterion, black_box, criterion_group, criterion_main};
use sha2::Digest as _;

// ---------------------------------------------------------------------------
// Bench 1: VFS path resolution
// Pure HashMap lookup + prefix match + PathBuf construction.  Completely
// synchronous, zero syscalls (no filesystem access).
// ---------------------------------------------------------------------------
fn bench_vfs_resolve(c: &mut Criterion) {
    let vfs = Vfs::new().expect("Vfs::new must succeed in a valid working directory");

    c.bench_function("vfs_resolve", |b| {
        b.iter(|| {
            // A path that hits the "/var/mirror" mount — exercises the full
            // prefix-match + remainder-trim + PathBuf::push chain.
            let _ = black_box(vfs.resolve(black_box("/var/mirror/fonts/google_sans.css")));
        });
    });
}

// ---------------------------------------------------------------------------
// Bench 2: Domain list deduplication
// Mirrors the CPU-only tail of `DnsRecon::run_osint`: lowercase, filter,
// sort, dedup.  Input is freshly cloned each iteration via BatchSize::SmallInput.
// ---------------------------------------------------------------------------
fn bench_dns_dedup_sort(c: &mut Criterion) {
    // Synthetic domain list representative of a mid-size crt.sh response.
    let raw: Vec<String> = {
        let base_domains = [
            "api.example.com",
            "www.example.com",
            "mail.example.com",
            "API.Example.com",    // duplicate after lowercase
            "WWW.EXAMPLE.COM",    // duplicate after lowercase
            "cdn.example.com",
            "static.example.com",
            "auth.example.com",
            "login.example.com",
            "dev.example.com",
            "staging.example.com",
            "beta.example.com",
            "v2.example.com",
            "v3.example.com",
            "legacy.example.com",
            "old.example.com",
            "new.example.com",
            "app.example.com",
            "dashboard.example.com",
            "admin.example.com",
        ];
        base_domains.iter().map(|s| s.to_string()).collect()
    };
    let domain = "example.com";

    c.bench_function("dns_dedup_sort", |b| {
        b.iter_batched(
            || raw.clone(),
            |all| {
                let mut unique: Vec<String> = all
                    .into_iter()
                    .map(|s| s.trim().to_lowercase())
                    .filter(|s| s.ends_with(black_box(domain)))
                    .collect();
                unique.sort_unstable();
                unique.dedup();
                black_box(unique)
            },
            BatchSize::SmallInput,
        );
    });
}

// ---------------------------------------------------------------------------
// Bench 3: AES-256-GCM decryption of a 1 KiB payload
// `Crypto::decrypt_aes_gcm` takes a Chromium-format ciphertext:
//   [3 bytes version] [12 bytes nonce] [ciphertext+tag]
// We pre-encrypt a 1 KiB plaintext once, then decrypt it in every iteration.
// Exercises the AES-GCM cipher path in `aes-gcm` (RustCrypto, pure Rust,
// no AESNI intrinsics at this stage — CPU-only).
// ---------------------------------------------------------------------------
fn bench_aes_gcm_decrypt_1kb(c: &mut Criterion) {
    use aes_gcm::{Aes256Gcm, Key, KeyInit, Nonce, aead::Aead};

    // Deterministic 32-byte key and 12-byte nonce (not cryptographically safe;
    // this is a benchmark, not production keying material).
    let raw_key = [0x42u8; 32];
    let raw_nonce = [0x11u8; 12];
    let plaintext = vec![0xabu8; 1024]; // 1 KiB

    // Encrypt once to obtain the ciphertext we will repeatedly decrypt.
    let key = Key::<Aes256Gcm>::from_slice(&raw_key);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(&raw_nonce);
    let ct = cipher.encrypt(nonce, plaintext.as_ref()).expect("AES-GCM encrypt must succeed");

    // Build the Chromium-format wire blob: [3 prefix bytes][12 nonce][ct+tag]
    let mut wire = Vec::with_capacity(3 + 12 + ct.len());
    wire.extend_from_slice(b"v10");           // 3-byte version prefix
    wire.extend_from_slice(&raw_nonce);       // 12-byte nonce
    wire.extend_from_slice(&ct);

    c.bench_function("aes_gcm_decrypt_1kb", |b| {
        b.iter(|| {
            let result = Crypto::decrypt_aes_gcm(black_box(&wire), black_box(&raw_key));
            black_box(result.expect("AES-GCM decrypt must succeed in bench"));
        });
    });
}

// ---------------------------------------------------------------------------
// Bench 4: SHA-256 digest of a 1 MiB buffer
// Exercises the `sha2` crate (workspace dep, pure-Rust RustCrypto).
// Gives a bytes-per-second throughput figure.
// ---------------------------------------------------------------------------
fn bench_sha256_hash_1mb(c: &mut Criterion) {
    let data = vec![0x5cu8; 1024 * 1024]; // 1 MiB

    c.bench_function("sha256_hash_1mb", |b| {
        b.iter(|| {
            let mut hasher = sha2::Sha256::new();
            hasher.update(black_box(&data));
            black_box(hasher.finalize())
        });
    });
}

// ---------------------------------------------------------------------------
// Bench 5: serde_json parsing of a crt.sh-shaped response
// Mirrors `DnsRecon::fetch_crtsh`'s `resp.json::<Vec<Value>>()` decode step.
// This is the CPU-only parse phase; no HTTP involved.
// ---------------------------------------------------------------------------
fn bench_serde_json_parse_crtsh(c: &mut Criterion) {
    // Representative minimal crt.sh JSON with 20 entries.
    let json_str: String = {
        let entries: Vec<String> = (0..20)
            .map(|i| {
                format!(
                    r#"{{"issuer_ca_id":{},"name_value":"sub{}.example.com\nwww.sub{}.example.com"}}"#,
                    1000 + i,
                    i,
                    i
                )
            })
            .collect();
        format!("[{}]", entries.join(","))
    };
    let json_bytes = json_str.as_bytes();

    c.bench_function("serde_json_parse_crtsh", |b| {
        b.iter(|| {
            let v: serde_json::Value =
                serde_json::from_slice(black_box(json_bytes)).expect("valid JSON");
            black_box(v)
        });
    });
}

criterion_group!(
    benches,
    bench_vfs_resolve,
    bench_dns_dedup_sort,
    bench_aes_gcm_decrypt_1kb,
    bench_sha256_hash_1mb,
    bench_serde_json_parse_crtsh,
);
criterion_main!(benches);
