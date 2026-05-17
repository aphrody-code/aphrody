// SPDX-License-Identifier: Apache-2.0
//
// In-memory pending-authorization-state cache.
//
// The OAuth dance is split across two HTTP requests:
//   1. POST /oauth/start   — daemon mints PKCE verifier + state token, redirects the user's browser
//      to the authorization endpoint.
//   2. GET  /oauth/callback — browser returns with `code` + `state`; daemon looks up the matching
//      PendingAuthState and calls `exchange_code`.
//
// State must survive between (1) and (2) within the same daemon process.
// We use an in-memory `HashMap` with a TTL sweeper.  Persistence is
// intentionally omitted: the user must complete auth in the same process
// session, and state tokens are single-use (consumed on first successful
// callback).
//
// Ported from the reference TypeScript `PendingAuthCache` class in
// `open-design/apps/daemon/src/mcp-oauth.ts`.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

/// Everything the callback endpoint needs to complete the authorization code
/// exchange for one in-flight OAuth request.
#[derive(Debug, Clone)]
pub struct PendingAuthState {
    /// Identifier of the MCP server being authenticated.
    pub server_id: String,
    /// Issuer URI of the authorization server.
    pub auth_server_issuer: String,
    /// Token endpoint to call at exchange time.
    pub token_endpoint: String,
    /// Registered `client_id`.
    pub client_id: String,
    /// Client secret (confidential clients only).
    pub client_secret: Option<String>,
    /// Redirect URI used in the authorize request (must match at exchange).
    pub redirect_uri: String,
    /// Raw PKCE `code_verifier`.
    pub code_verifier: String,
    /// Space-delimited scopes requested.
    pub scope: Option<String>,
    /// RFC 8707 resource indicator.
    pub resource_url: Option<String>,
    /// Monotonic clock instant at which this state was stored.
    pub created_at: Instant,
}

#[derive(Debug)]
struct Inner {
    store: HashMap<String, PendingAuthState>,
    ttl: Duration,
}

impl Inner {
    fn sweep(&mut self) {
        let now = Instant::now();
        self.store.retain(|_, v| now.duration_since(v.created_at) < self.ttl);
    }
}

/// Thread-safe cache of in-flight authorization states, keyed by the opaque
/// `state` parameter sent to the authorization server.
///
/// Clone is `O(1)` — the cache is reference-counted.
#[derive(Clone, Debug)]
pub struct PendingAuthCache {
    inner: Arc<Mutex<Inner>>,
}

impl PendingAuthCache {
    /// Create a new cache with the given TTL.
    ///
    /// Entries older than `ttl` are rejected on `consume` and lazily
    /// swept from `put`.
    #[must_use]
    pub fn new(ttl: Duration) -> Self {
        Self { inner: Arc::new(Mutex::new(Inner { store: HashMap::new(), ttl })) }
    }

    /// Create a cache with the default 10-minute TTL.
    #[must_use]
    pub fn with_default_ttl() -> Self {
        Self::new(Duration::from_secs(10 * 60))
    }

    /// Store `state` → `value`.
    ///
    /// Triggers a lazy sweep of expired entries to keep memory bounded.
    pub fn put(&self, state: String, value: PendingAuthState) {
        // Recover from mutex poison: the poisoning thread already panicked; we
        // can safely proceed with the lock data that was left behind.
        let mut lock = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        lock.sweep();
        lock.store.insert(state, value);
    }

    /// Consume the state token — returns the stored `PendingAuthState` if it
    /// exists and has not expired, then removes it (single-use semantics).
    ///
    /// Returns `None` if the state was not found, already consumed, or
    /// expired.
    pub fn consume(&self, state: &str) -> Option<PendingAuthState> {
        let mut lock = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        let v = lock.store.remove(state)?;
        if Instant::now().duration_since(v.created_at) > lock.ttl {
            return None;
        }
        Some(v)
    }

    /// Number of entries currently in the cache (including potentially
    /// expired entries not yet swept).
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.lock().unwrap_or_else(|p| p.into_inner()).store.len()
    }

    /// Returns `true` if no entries are present.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for PendingAuthCache {
    fn default() -> Self {
        Self::with_default_ttl()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn make_state(server_id: &str) -> PendingAuthState {
        PendingAuthState {
            server_id: server_id.to_owned(),
            auth_server_issuer: "https://auth.example.com".to_owned(),
            token_endpoint: "https://auth.example.com/token".to_owned(),
            client_id: "cid".to_owned(),
            client_secret: None,
            redirect_uri: "https://app.example.com/callback".to_owned(),
            code_verifier: "verifier_abc".to_owned(),
            scope: Some("read".to_owned()),
            resource_url: None,
            created_at: Instant::now(),
        }
    }

    #[test]
    fn put_and_consume_once() {
        let cache = PendingAuthCache::with_default_ttl();
        let state_key = "abc123".to_owned();
        cache.put(state_key.clone(), make_state("srv1"));
        assert_eq!(cache.len(), 1);

        let retrieved = cache.consume(&state_key).unwrap();
        assert_eq!(retrieved.server_id, "srv1");

        // Second consume returns None (single-use).
        assert!(cache.consume(&state_key).is_none());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn consume_unknown_key_returns_none() {
        let cache = PendingAuthCache::with_default_ttl();
        assert!(cache.consume("no_such_key").is_none());
    }

    #[test]
    fn expired_entry_rejected_on_consume() {
        // Zero TTL — everything is immediately expired.
        let cache = PendingAuthCache::new(Duration::ZERO);
        cache.put("key".to_owned(), make_state("srv2"));
        // Even a Duration::ZERO entry is expired the instant we call consume.
        assert!(cache.consume("key").is_none());
    }

    #[test]
    fn sweep_removes_expired_entries() {
        let cache = PendingAuthCache::new(Duration::ZERO);
        cache.put("a".to_owned(), make_state("s1"));
        cache.put("b".to_owned(), make_state("s2"));
        // Next `put` triggers a sweep — both existing entries are expired.
        cache.put("c".to_owned(), make_state("s3"));
        // After sweep, only "c" should remain (and "a"/"b" were swept).
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn clone_shares_state() {
        let cache = PendingAuthCache::with_default_ttl();
        cache.put("k".to_owned(), make_state("srv3"));

        let clone = cache.clone();
        assert_eq!(clone.len(), 1);

        let _ = clone.consume("k");
        assert_eq!(cache.len(), 0); // both point to the same Arc
    }
}
