// SPDX-License-Identifier: Apache-2.0
//
// RFC 7636 — Proof Key for Code Exchange by OAuth Public Clients.
//
// Algorithm:
//   1. Generate a cryptographically random `code_verifier` (43–128 URL-safe characters, RFC 7636
//      §4.1).
//   2. Derive `code_challenge = BASE64URL(SHA256(ASCII(code_verifier)))`.
//   3. Send `code_challenge` + `code_challenge_method=S256` in the authorize request; send the raw
//      `code_verifier` in the token exchange.

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::RngCore;
use sha2::{Digest, Sha256};

/// Number of random bytes for the verifier.  After base64url encoding:
/// 64 bytes → 86 characters (within the 43–128 range of RFC 7636 §4.1).
const VERIFIER_BYTES: usize = 64;

/// A PKCE pair: raw verifier (sent at token exchange) and its SHA-256
/// base64url-encoded challenge (sent in the authorize request).
#[derive(Debug, Clone)]
pub struct Pkce {
    /// Raw code_verifier.  Keep secret — only send to the token endpoint.
    pub code_verifier: String,
    /// SHA-256(code_verifier), base64url-encoded, no padding.
    pub code_challenge: String,
}

impl Pkce {
    /// Generate a fresh PKCE pair using the OS CSPRNG.
    ///
    /// Uses `rand::thread_rng()` which seeds from the OS (via `getrandom`).
    /// The verifier is 86 URL-safe characters (64 random bytes encoded).
    #[must_use]
    pub fn new() -> Self {
        let mut raw = [0u8; VERIFIER_BYTES];
        rand::thread_rng().fill_bytes(&mut raw);
        let code_verifier = URL_SAFE_NO_PAD.encode(raw);
        let code_challenge = derive_challenge(&code_verifier);
        Self { code_verifier, code_challenge }
    }
}

/// Derive the S256 code_challenge from an existing verifier string.
///
/// `challenge = BASE64URL-NO-PADDING(SHA-256(ASCII(verifier)))`
#[must_use]
pub fn derive_challenge(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}

/// Generate a URL-safe random state token (32 bytes → 43 characters).
#[must_use]
pub fn generate_state() -> String {
    let mut raw = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut raw);
    URL_SAFE_NO_PAD.encode(raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    // RFC 7636 Appendix B — known test vector.
    //
    // code_verifier  = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"
    // code_challenge = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
    //
    // (BASE64URL(SHA256(ASCII("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"))))
    #[test]
    fn pkce_rfc7636_test_vector() {
        const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        const EXPECTED_CHALLENGE: &str = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
        assert_eq!(derive_challenge(VERIFIER), EXPECTED_CHALLENGE);
    }

    // Pkce::new() round-trip: re-derive challenge from verifier and compare.
    #[test]
    fn pkce_new_roundtrip() {
        let pkce = Pkce::new();
        // Verifier must be non-empty and within RFC 7636 §4.1 length bounds.
        assert!(!pkce.code_verifier.is_empty());
        assert!(pkce.code_verifier.len() >= 43);
        assert!(pkce.code_verifier.len() <= 128);
        // Challenge derived from verifier must match stored challenge.
        let re_derived = derive_challenge(&pkce.code_verifier);
        assert_eq!(pkce.code_challenge, re_derived);
    }

    // Challenge must not equal the verifier (would be trivial attack).
    #[test]
    fn pkce_challenge_differs_from_verifier() {
        let pkce = Pkce::new();
        assert_ne!(pkce.code_verifier, pkce.code_challenge);
    }

    // State tokens must be distinct across invocations.
    #[test]
    fn state_uniqueness() {
        let s1 = generate_state();
        let s2 = generate_state();
        assert_ne!(s1, s2);
        // 43 chars = 32 bytes base64url-no-pad.
        assert_eq!(s1.len(), 43);
    }
}
