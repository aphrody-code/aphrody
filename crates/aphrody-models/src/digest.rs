// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors
//
// SHA-256 helpers shared by the cache-path derivation, the downloader and the
// `verify` path. Kept in its own module so `id.rs` (wasm-safe, pure) can hash
// a URL without pulling in any of the filesystem code.

use sha2::{Digest, Sha256};

/// Lower-case hex SHA-256 of an in-memory byte slice.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Streaming SHA-256 accumulator.
///
/// The downloader feeds it each chunk as it lands so a multi-gigabyte weight
/// file is hashed in one pass, without ever holding the artefact in memory.
#[derive(Default)]
pub struct Hasher(Sha256);

impl Hasher {
    /// Start a fresh digest.
    #[must_use]
    pub fn new() -> Self {
        Self(Sha256::new())
    }

    /// Absorb the next chunk.
    pub fn update(&mut self, chunk: &[u8]) {
        self.0.update(chunk);
    }

    /// Finish and render the digest as lower-case hex.
    #[must_use]
    pub fn finish_hex(self) -> String {
        hex::encode(self.0.finalize())
    }
}

/// Normalise a user-supplied digest for comparison.
///
/// Accepts both the bare hex form and the `sha256:`-prefixed form, and is
/// case-insensitive, so a digest copied from a Hugging Face page, a catalog
/// entry or an OCI descriptor all compare equal.
#[must_use]
pub fn normalize_digest(raw: &str) -> String {
    raw.trim().trim_start_matches("sha256:").trim_start_matches("SHA256:").to_ascii_lowercase()
}

/// SHA-256 of a file on disk, streamed in 1 MiB chunks.
///
/// # Errors
///
/// Returns [`crate::ModelError::Io`] if the file cannot be opened or read.
#[cfg(not(target_arch = "wasm32"))]
pub fn sha256_file(path: &std::path::Path) -> crate::Result<String> {
    use std::io::Read as _;

    let mut file =
        std::fs::File::open(path).map_err(|e| crate::ModelError::io(path.to_path_buf(), e))?;
    let mut hasher = Hasher::new();
    let mut buf = vec![0_u8; 1 << 20];
    loop {
        let n = file.read(&mut buf).map_err(|e| crate::ModelError::io(path.to_path_buf(), e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finish_hex())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_vector() {
        // NIST/FIPS-180 test vector for "abc".
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn streaming_matches_one_shot() {
        let data = b"the quick brown fox jumps over the lazy dog";
        let mut h = Hasher::new();
        for chunk in data.chunks(7) {
            h.update(chunk);
        }
        assert_eq!(h.finish_hex(), sha256_hex(data));
    }

    #[test]
    fn digest_prefix_and_case_are_normalised() {
        let bare = "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD";
        assert_eq!(normalize_digest(&format!("sha256:{bare}")), normalize_digest(bare));
        assert_eq!(normalize_digest(bare), sha256_hex(b"abc"));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn file_digest_matches_memory_digest() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("blob.bin");
        // Larger than the 1 MiB read buffer, to exercise the loop.
        let payload = vec![0xA5_u8; (1 << 20) + 1234];
        std::fs::write(&path, &payload).unwrap();
        assert_eq!(sha256_file(&path).unwrap(), sha256_hex(&payload));
    }
}
