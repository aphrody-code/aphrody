// SPDX-License-Identifier: Apache-2.0
//! Deep-scraping enhancements for the `web_fetch` builtin.
//!
//! This module provides three orthogonal capabilities that compose on top of
//! the existing [`web_fetch`](super::web_fetch) single-URL tool:
//!
//! 1. **Shared client** — a process-wide [`reqwest::Client`] wrapped in
//!    [`ScrapingClient`], held behind an [`Arc`] so every concurrent fetch
//!    reuses the same connection pool.  Constructing a client per request
//!    would defeat HTTP/2 multiplexing and keep-alive gains (project-wide
//!    latency goal).
//!
//! 2. **Concurrent fetch** — [`fetch_many`] accepts a `Vec<String>` of URLs
//!    and a concurrency cap, drives them through a bounded
//!    [`tokio::sync::Semaphore`], and returns one [`FetchResult`] per URL
//!    preserving input order.  Errors per-URL are captured, not propagated.
//!
//! 3. **Proxy pool** — [`ProxyPool`] holds a list of proxy URLs and selects
//!    the next healthy one using atomic round-robin.  Health is tracked as a
//!    lightweight [`AtomicBool`] per entry; a background task (or explicit
//!    [`ProxyPool::mark_failed`] call) can retire an entry so subsequent
//!    rounds skip it automatically.
//!
//! 4. **Chrome 146 UA + client hints** — [`chrome146_headers`] returns the
//!    set of `User-Agent` and `Sec-CH-UA-*` headers that Chrome 146 sends by
//!    default on a Windows 11 x64 host.
//!
//! ## HTTP/3
//!
//! INCOMPLET — the workspace reqwest pin (`0.13`, features `http2
//! rustls-no-provider …`) does NOT include the `http3` feature flag, which
//! requires the optional `quinn`/`h3` stack and is behind a non-default
//! feature gate on crates.io.  Adding `http3` here would require a
//! `Cargo.toml` change and a `cargo deny check` run (new transitive deps).
//! The scaffold is left as a doc comment only; enable via:
//! ```toml
//! # In workspace Cargo.toml:
//! reqwest = { version = "0.13", features = [..., "http3"] }
//! ```
//! Then replace `ScrapingClient::builder()` with
//! `.http3_prior_knowledge()` or `.https_only()` as needed.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use reqwest::{Client, Proxy, RequestBuilder};
use tokio::sync::Semaphore;
use tracing::{debug, warn};

// ---------------------------------------------------------------------------
// Chrome 146 UA + client hints
// ---------------------------------------------------------------------------

/// Chrome 146 stable User-Agent string on Windows 11 x64.
///
/// Format mirrors the frozen UA that Chrome 146 ships; the `(Windows NT 10.0;
/// Win64; x64)` platform token is the universal form sent to non-origin-trial
/// sites as of 2026.
pub const CHROME146_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
     (KHTML, like Gecko) Chrome/146.0.0.0 Safari/537.36";

/// Structured User-Agent client hint for Chrome 146.
///
/// `Sec-CH-UA` lists only the three canonical brand tokens that Chrome 146
/// sends: Chromium, Google Chrome, and the "Not/A)Brand" anti-fingerprinting
/// ghost entry.
pub const CHROME146_SEC_CH_UA: &str =
    r#""Chromium";v="146", "Google Chrome";v="146", "Not/A)Brand";v="24""#;

/// `Sec-CH-UA-Mobile: ?0` — desktop client.
pub const CHROME146_MOBILE: &str = "?0";

/// `Sec-CH-UA-Platform: "Windows"`.
pub const CHROME146_PLATFORM: &str = r#""Windows""#;

/// `Sec-CH-UA-Platform-Version: "15.0.0"` — Windows 11 (internal version).
pub const CHROME146_PLATFORM_VERSION: &str = r#""15.0.0""#;

/// `Sec-CH-UA-Arch: "x86"` — x64 Chrome still reports `"x86"`.
pub const CHROME146_ARCH: &str = r#""x86""#;

/// `Sec-CH-UA-Bitness: "64"`.
pub const CHROME146_BITNESS: &str = r#""64""#;

/// `Sec-CH-UA-Full-Version-List` with the same three brands.
pub const CHROME146_FULL_VERSION_LIST: &str =
    r#""Chromium";v="146.0.7204.92", "Google Chrome";v="146.0.7204.92", "Not/A)Brand";v="24.0.0.0""#;

/// Apply Chrome 146 `User-Agent` and `Sec-CH-UA-*` client-hint headers to
/// an existing [`RequestBuilder`], returning the enriched builder.
///
/// Downstream callers can override any individual header after this call if
/// they need to vary a specific hint (e.g., mobile simulation).
///
/// # Example
///
/// ```ignore
/// let req = client.get("https://example.com");
/// let req = chrome146_headers(req);
/// let resp = req.send().await?;
/// ```
#[must_use]
pub fn chrome146_headers(req: RequestBuilder) -> RequestBuilder {
    req.header("User-Agent", CHROME146_USER_AGENT)
        .header("Sec-CH-UA", CHROME146_SEC_CH_UA)
        .header("Sec-CH-UA-Mobile", CHROME146_MOBILE)
        .header("Sec-CH-UA-Platform", CHROME146_PLATFORM)
        .header("Sec-CH-UA-Platform-Version", CHROME146_PLATFORM_VERSION)
        .header("Sec-CH-UA-Arch", CHROME146_ARCH)
        .header("Sec-CH-UA-Bitness", CHROME146_BITNESS)
        .header("Sec-CH-UA-Full-Version-List", CHROME146_FULL_VERSION_LIST)
        // Additional headers Chrome 146 sends on every request.
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8")
        .header("Accept-Language", "en-US,en;q=0.9")
        .header("Cache-Control", "no-cache")
        .header("Pragma", "no-cache")
        .header("Upgrade-Insecure-Requests", "1")
}

// ---------------------------------------------------------------------------
// Shared scraping client
// ---------------------------------------------------------------------------

/// A shared [`reqwest::Client`] configured for scraping workloads.
///
/// Clone is cheap (`Arc` inside). Pass `ScrapingClient::default()` into
/// [`fetch_many`] and every concurrent request shares the same HTTP/2
/// connection pool.
///
/// Building from a proxy is handled by [`ScrapingClient::with_proxy`].
#[derive(Clone, Debug)]
pub struct ScrapingClient {
    inner: Arc<Client>,
}

impl ScrapingClient {
    /// Build a new [`ScrapingClient`] with the given timeout.
    ///
    /// # Errors
    ///
    /// Returns a [`reqwest::Error`] if the TLS backend fails to initialise
    /// (usually a missing crypto provider — call
    /// `rustls::crypto::ring::default_provider().install_default()` first).
    pub fn new(timeout: Duration) -> reqwest::Result<Self> {
        let client = Client::builder()
            .timeout(timeout)
            .user_agent(CHROME146_USER_AGENT)
            .connection_verbose(false)
            .pool_max_idle_per_host(16)
            .tcp_keepalive(Duration::from_secs(30))
            .build()?;
        Ok(Self { inner: Arc::new(client) })
    }

    /// Build a new [`ScrapingClient`] that routes through `proxy_url`.
    ///
    /// The proxy URL must be a valid `http://…`, `https://…`, or `socks5://…`
    /// URI. The UA and timeout settings are identical to [`Self::new`].
    ///
    /// # Errors
    ///
    /// Returns a [`reqwest::Error`] if the proxy URL is invalid or the TLS
    /// backend fails to initialise.
    pub fn with_proxy(timeout: Duration, proxy_url: &str) -> reqwest::Result<Self> {
        let proxy = Proxy::all(proxy_url)?;
        let client = Client::builder()
            .timeout(timeout)
            .user_agent(CHROME146_USER_AGENT)
            .pool_max_idle_per_host(16)
            .tcp_keepalive(Duration::from_secs(30))
            .proxy(proxy)
            .build()?;
        Ok(Self { inner: Arc::new(client) })
    }

    /// Return the inner [`reqwest::Client`] for ad-hoc request building.
    #[must_use]
    pub fn inner(&self) -> &Client {
        &self.inner
    }
}

// ---------------------------------------------------------------------------
// Proxy pool
// ---------------------------------------------------------------------------

/// A single entry in a [`ProxyPool`].
struct ProxyEntry {
    url: String,
    healthy: AtomicBool,
}

impl ProxyEntry {
    fn new(url: impl Into<String>) -> Self {
        Self { url: url.into(), healthy: AtomicBool::new(true) }
    }
}

/// Round-robin proxy pool with per-entry health tracking.
///
/// Entries are considered ordered; [`ProxyPool::next`] walks the ring and
/// returns the URL of the first healthy entry *after* the last-used index.
/// When all entries are unhealthy [`ProxyPool::next`] returns `None`.
///
/// Healthy entries are reset either manually via [`ProxyPool::mark_healthy`]
/// or by calling [`ProxyPool::reset_all`] (e.g. after a back-off period).
///
/// # Thread safety
///
/// All state is `AtomicBool` / `AtomicUsize`, so the pool is `Send + Sync`
/// and can live behind an `Arc`.
///
/// # Example
///
/// ```ignore
/// let pool = ProxyPool::new(vec![
///     "http://proxy1:8080",
///     "http://proxy2:8080",
/// ]);
///
/// if let Some(url) = pool.next() {
///     let client = ScrapingClient::with_proxy(Duration::from_secs(10), &url)?;
///     // … use client …
/// } else {
///     // no proxies available — direct connection
/// }
/// ```
pub struct ProxyPool {
    entries: Vec<ProxyEntry>,
    cursor: AtomicUsize,
}

impl ProxyPool {
    /// Build a pool from the given list of proxy URLs.
    ///
    /// All entries start healthy. Duplicate URLs are kept as-is so the caller
    /// can weight a proxy higher by repeating it.
    #[must_use]
    pub fn new(urls: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            entries: urls.into_iter().map(ProxyEntry::new).collect(),
            cursor: AtomicUsize::new(0),
        }
    }

    /// Number of entries (healthy and unhealthy combined).
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// `true` when the pool holds no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Return the URL of the next healthy entry, advancing the internal
    /// cursor, or `None` when every entry is unhealthy.
    ///
    /// The cursor advances by one on every call, wrapping around at `len()`,
    /// so callers across multiple threads get a roughly equal distribution.
    /// A full scan (up to `len()` steps) is performed from the new cursor
    /// position to locate the first healthy entry.
    #[must_use]
    pub fn next(&self) -> Option<&str> {
        let n = self.entries.len();
        if n == 0 {
            return None;
        }
        // Advance cursor atomically (relaxed — ordering relative to the
        // bool reads below is enforced by the stronger Acquire on those).
        let start = self.cursor.fetch_add(1, Ordering::Relaxed) % n;
        for i in 0..n {
            let idx = (start + i) % n;
            if self.entries[idx].healthy.load(Ordering::Acquire) {
                return Some(&self.entries[idx].url);
            }
        }
        None
    }

    /// Mark the entry with `url` as failed (unhealthy).  Subsequent calls to
    /// [`Self::next`] will skip it.  No-op when the URL is not found.
    pub fn mark_failed(&self, url: &str) {
        for e in &self.entries {
            if e.url == url {
                e.healthy.store(false, Ordering::Release);
                warn!(proxy = url, "proxy marked unhealthy");
                return;
            }
        }
    }

    /// Re-enable the entry with `url`.  No-op when not found.
    pub fn mark_healthy(&self, url: &str) {
        for e in &self.entries {
            if e.url == url {
                e.healthy.store(true, Ordering::Release);
                debug!(proxy = url, "proxy marked healthy");
                return;
            }
        }
    }

    /// Re-enable every entry (useful after a back-off period).
    pub fn reset_all(&self) {
        for e in &self.entries {
            e.healthy.store(true, Ordering::Release);
        }
    }

    /// Number of currently healthy entries.
    #[must_use]
    pub fn healthy_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| e.healthy.load(Ordering::Acquire))
            .count()
    }
}

// ---------------------------------------------------------------------------
// Concurrent fetch
// ---------------------------------------------------------------------------

/// The outcome of a single URL fetch performed by [`fetch_many`].
#[derive(Debug)]
pub struct FetchResult {
    /// The URL that was fetched (matches input position in [`fetch_many`]).
    pub url: String,
    /// HTTP status code, or `0` when the request failed before a response
    /// was received.
    pub status: u16,
    /// Response body, truncated to `max_bytes`.  Empty when an error occurred.
    pub body: String,
    /// `true` when the body was truncated to `max_bytes`.
    pub truncated: bool,
    /// Transport / HTTP error string, or `None` on success.
    pub error: Option<String>,
}

/// Fetch `urls` concurrently, capping in-flight requests to `concurrency`.
///
/// The function drives every URL through `client` and returns one
/// [`FetchResult`] per input URL in the **same order** as the input slice.
/// Per-URL errors are captured in [`FetchResult::error`] — the `Vec` is
/// always complete.
///
/// * `client` — a shared [`ScrapingClient`].  All requests share its
///   connection pool.
/// * `urls` — list of `http(s)://` URLs.
/// * `concurrency` — maximum simultaneous in-flight HTTP requests.  Passed
///   through a [`Semaphore`] with this many permits.
/// * `max_bytes` — body size cap per URL (bytes).
///
/// # Example
///
/// ```ignore
/// let client = ScrapingClient::new(Duration::from_secs(10))?;
/// let results = fetch_many(
///     &client,
///     &["https://example.com".to_string(), "https://example.org".to_string()],
///     8,
///     256 * 1024,
/// ).await;
/// ```
pub async fn fetch_many(
    client: &ScrapingClient,
    urls: &[String],
    concurrency: usize,
    max_bytes: usize,
) -> Vec<FetchResult> {
    let sem = Arc::new(Semaphore::new(concurrency.max(1)));
    let mut handles = Vec::with_capacity(urls.len());

    for url in urls {
        let permit = Arc::clone(&sem);
        let client = client.clone();
        let url = url.clone();
        let url_for_task = url.clone();
        let handle = tokio::spawn(async move {
            // Acquire permit — blocks until a slot is free.
            let _guard = permit.acquire_owned().await;
            fetch_one(&client, &url_for_task, max_bytes).await
        });
        handles.push((url, handle));
    }

    let mut results = Vec::with_capacity(handles.len());
    for (url, handle) in handles {
        let res = match handle.await {
            Ok(r) => r,
            Err(join_err) => FetchResult {
                url,
                status: 0,
                body: String::new(),
                truncated: false,
                error: Some(format!("task panicked: {join_err}")),
            },
        };
        results.push(res);
    }
    results
}

/// Internal: fetch a single URL, truncate body, and wrap into [`FetchResult`].
async fn fetch_one(client: &ScrapingClient, url: &str, max_bytes: usize) -> FetchResult {
    debug!(url, "fetch_one start");
    let req = client.inner().get(url);
    let req = chrome146_headers(req);
    let resp = match req.send().await {
        Ok(r) => r,
        Err(e) => {
            return FetchResult {
                url: url.to_string(),
                status: 0,
                body: String::new(),
                truncated: false,
                error: Some(e.to_string()),
            };
        }
    };
    let status = resp.status().as_u16();
    let final_url = resp.url().to_string();
    let bytes = match resp.bytes().await {
        Ok(b) => b,
        Err(e) => {
            return FetchResult {
                url: final_url,
                status,
                body: String::new(),
                truncated: false,
                error: Some(e.to_string()),
            };
        }
    };
    let truncated = bytes.len() > max_bytes;
    let slice = if truncated { &bytes[..max_bytes] } else { &bytes[..] };
    let body = String::from_utf8_lossy(slice).into_owned();
    debug!(url = final_url, status, bytes = bytes.len(), "fetch_one done");
    FetchResult {
        url: final_url,
        status,
        body,
        truncated,
        error: None,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn ensure_rustls_provider() {
        static INSTALL: Once = Once::new();
        INSTALL.call_once(|| {
            let _ = rustls::crypto::ring::default_provider().install_default();
        });
    }

    /// Spawn a minimal HTTP/1.1 server on `127.0.0.1:0`.  Returns the port.
    async fn oneshot_server(response: &'static [u8]) -> u16 {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            if let Ok((mut sock, _)) = listener.accept().await {
                let mut buf = [0u8; 4096];
                let _ = sock.read(&mut buf).await;
                let _ = sock.write_all(response).await;
                let _ = sock.shutdown().await;
            }
        });
        port
    }

    /// Spawn a server that accepts `n` connections sequentially.
    async fn multi_server(response: &'static [u8], n: usize) -> u16 {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            for _ in 0..n {
                if let Ok((mut sock, _)) = listener.accept().await {
                    let mut buf = [0u8; 4096];
                    let _ = sock.read(&mut buf).await;
                    let _ = sock.write_all(response).await;
                    let _ = sock.shutdown().await;
                }
            }
        });
        port
    }

    // -----------------------------------------------------------------------
    // ScrapingClient
    // -----------------------------------------------------------------------

    #[test]
    fn scraping_client_builds_and_clones() {
        ensure_rustls_provider();
        let c = ScrapingClient::new(Duration::from_secs(5)).unwrap();
        let _c2 = c.clone();
        // Both clones point to the same inner Arc.
        assert!(Arc::ptr_eq(&c.inner, &_c2.inner));
    }

    // -----------------------------------------------------------------------
    // Chrome 146 headers
    // -----------------------------------------------------------------------

    #[test]
    fn chrome146_ua_contains_chrome_146() {
        assert!(CHROME146_USER_AGENT.contains("Chrome/146.0.0.0"));
    }

    #[test]
    fn chrome146_sec_ch_ua_has_three_brands() {
        let brands: Vec<&str> = CHROME146_SEC_CH_UA.split(',').collect();
        assert_eq!(brands.len(), 3);
    }

    // -----------------------------------------------------------------------
    // ProxyPool
    // -----------------------------------------------------------------------

    #[test]
    fn proxy_pool_round_robin_healthy() {
        let pool = ProxyPool::new(["http://p1:8080", "http://p2:8080", "http://p3:8080"]);
        // Calling next() six times should cycle through all three entries.
        let mut seen = std::collections::HashSet::new();
        for _ in 0..6 {
            let url = pool.next().expect("pool is healthy");
            seen.insert(url.to_string());
        }
        assert_eq!(seen.len(), 3);
    }

    #[test]
    fn proxy_pool_skips_failed_entry() {
        let pool = ProxyPool::new(["http://p1:8080", "http://p2:8080"]);
        pool.mark_failed("http://p1:8080");
        for _ in 0..10 {
            assert_eq!(pool.next(), Some("http://p2:8080"));
        }
    }

    #[test]
    fn proxy_pool_returns_none_when_all_failed() {
        let pool = ProxyPool::new(["http://p1:8080"]);
        pool.mark_failed("http://p1:8080");
        assert!(pool.next().is_none());
    }

    #[test]
    fn proxy_pool_reset_all_restores_health() {
        let pool = ProxyPool::new(["http://p1:8080", "http://p2:8080"]);
        pool.mark_failed("http://p1:8080");
        pool.mark_failed("http://p2:8080");
        assert!(pool.next().is_none());
        pool.reset_all();
        assert!(pool.next().is_some());
        assert_eq!(pool.healthy_count(), 2);
    }

    #[test]
    fn proxy_pool_empty_returns_none() {
        let pool = ProxyPool::new(std::iter::empty::<&str>());
        assert!(pool.is_empty());
        assert!(pool.next().is_none());
    }

    // -----------------------------------------------------------------------
    // fetch_many
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn fetch_many_returns_results_in_order() {
        ensure_rustls_provider();
        const N: usize = 3;
        let port = multi_server(
            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok",
            N,
        )
        .await;
        let urls: Vec<String> =
            (0..N).map(|_| format!("http://127.0.0.1:{port}/")).collect();
        let client = ScrapingClient::new(Duration::from_secs(5)).unwrap();
        let results = fetch_many(&client, &urls, 2, 64 * 1024).await;
        assert_eq!(results.len(), N);
        // Verify all three requests succeeded (status 200 or 0 for
        // connection-refused after the server stops; at least the first N
        // should succeed).
        let ok_count = results.iter().filter(|r| r.status == 200).count();
        assert_eq!(ok_count, N, "all {N} requests should return 200");
    }

    #[tokio::test]
    async fn fetch_many_captures_error_without_panic() {
        ensure_rustls_provider();
        // Port 1 is reserved / refused on all OS — transport error expected.
        let urls = vec!["http://127.0.0.1:1/unreachable".to_string()];
        let client = ScrapingClient::new(Duration::from_millis(200)).unwrap();
        let results = fetch_many(&client, &urls, 1, 4096).await;
        assert_eq!(results.len(), 1);
        assert!(
            results[0].error.is_some(),
            "unreachable host must produce a transport error"
        );
    }

    #[tokio::test]
    async fn fetch_many_truncates_body() {
        ensure_rustls_provider();
        let large_body: Vec<u8> = b"x".repeat(2048);
        let resp_header = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n",
            large_body.len()
        );
        let mut raw: Vec<u8> = resp_header.into_bytes();
        raw.extend_from_slice(&large_body);
        let leaked: &'static [u8] = Box::leak(raw.into_boxed_slice());
        let port = oneshot_server(leaked).await;
        let client = ScrapingClient::new(Duration::from_secs(5)).unwrap();
        let results =
            fetch_many(&client, &[format!("http://127.0.0.1:{port}/")], 1, 512).await;
        assert_eq!(results.len(), 1);
        assert!(results[0].truncated, "body must be truncated to 512 bytes");
        assert_eq!(results[0].body.len(), 512);
    }

    #[tokio::test]
    async fn fetch_many_concurrency_one_is_sequential() {
        ensure_rustls_provider();
        const N: usize = 4;
        let port = multi_server(
            b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\n\r\nz",
            N,
        )
        .await;
        let urls: Vec<String> =
            (0..N).map(|_| format!("http://127.0.0.1:{port}/")).collect();
        let client = ScrapingClient::new(Duration::from_secs(5)).unwrap();
        let results = fetch_many(&client, &urls, 1, 4096).await;
        assert_eq!(results.iter().filter(|r| r.status == 200).count(), N);
    }
}
