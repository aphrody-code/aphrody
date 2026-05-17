/// scrape.rs — BXC daemon HTTP client for `aphrody scrape` and `aphrody tokens`.
///
/// The BXC engine is a headless-browser daemon (Lightpanda / Bun-based) that
/// exposes a JSON API at `BXC_DAEMON_URL` (default: `http://localhost:8765`).
/// This module never falls back to a mock: if the daemon is unreachable the
/// caller receives a descriptive error with the exact start command.
use serde::{Deserialize, Serialize};

// ── Public API types ─────────────────────────────────────────────────────────

/// Full-page reconnaissance result returned by `bxc recon`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ReconResult {
    pub url: String,
    pub status_code: u16,
    pub headers: std::collections::HashMap<String, String>,
    pub cdn: Option<String>,
    pub frameworks: Vec<String>,
    pub assets: Vec<AssetEntry>,
    pub screenshot_path: Option<String>,
}

/// A single discovered asset (script, stylesheet, image, …).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AssetEntry {
    pub kind: String,
    pub url: String,
    pub size_bytes: Option<u64>,
}

/// Result of a CSS-selector extraction pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SelectorResult {
    pub url: String,
    pub selector: String,
    pub matches: Vec<String>,
}

/// Typed Material Design 3 design-token map.
/// Keys are CSS custom-property names (`--md-sys-color-primary`, …).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct M3TokenMap {
    pub source_url: String,
    pub tokens: std::collections::HashMap<String, String>,
}

// ── Client ───────────────────────────────────────────────────────────────────

const DEFAULT_DAEMON_URL: &str = "http://localhost:8765";

/// Async HTTP client that speaks to the BXC daemon JSON API.
pub(crate) struct ScrapeClient {
    base_url: String,
    http: reqwest::Client,
}

impl ScrapeClient {
    /// Creates a new client.  Reads `BXC_DAEMON_URL` from the environment;
    /// falls back to `http://localhost:8765` if the variable is absent or empty.
    ///
    /// Returns an error only if the reqwest `Client` cannot be built (i.e. if
    /// no rustls crypto provider has been installed in the process).
    pub(crate) fn new() -> anyhow::Result<Self> {
        let base_url = std::env::var("BXC_DAEMON_URL")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| DEFAULT_DAEMON_URL.to_owned());

        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .user_agent("aphrody-cli/1.0")
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build HTTP client: {e}"))?;

        Ok(Self { base_url, http })
    }

    /// Returns a human-readable error message when the daemon is unreachable.
    fn daemon_error(&self) -> String {
        format!(
            "bxc daemon not reachable at {}. Start it via: bxc-engine serve --port 8765",
            self.base_url
        )
    }

    /// `POST /recon` — full-page reconnaissance (headers, CDN, frameworks, assets, screenshot).
    pub(crate) async fn recon(&self, url: &str) -> anyhow::Result<ReconResult> {
        let endpoint = format!("{}/recon", self.base_url);
        let body = serde_json::json!({ "url": url });

        let resp = self
            .http
            .post(&endpoint)
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("{}: {e}", self.daemon_error()))?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("bxc /recon returned HTTP {status}: {text}");
        }

        resp.json::<ReconResult>()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse /recon response: {e}"))
    }

    /// `POST /scrape` — extract `textContent` of elements matching a CSS selector.
    pub(crate) async fn scrape(&self, url: &str, selector: &str) -> anyhow::Result<SelectorResult> {
        let endpoint = format!("{}/scrape", self.base_url);
        let body = serde_json::json!({ "url": url, "selector": selector });

        let resp = self
            .http
            .post(&endpoint)
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("{}: {e}", self.daemon_error()))?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("bxc /scrape returned HTTP {status}: {text}");
        }

        resp.json::<SelectorResult>()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse /scrape response: {e}"))
    }

    /// `POST /detect` — framework + technology detection only (lightweight recon subset).
    #[allow(dead_code)]
    pub(crate) async fn detect(&self, url: &str) -> anyhow::Result<Vec<String>> {
        let endpoint = format!("{}/detect", self.base_url);
        let body = serde_json::json!({ "url": url });

        let resp = self
            .http
            .post(&endpoint)
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("{}: {e}", self.daemon_error()))?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("bxc /detect returned HTTP {status}: {text}");
        }

        resp.json::<Vec<String>>()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse /detect response: {e}"))
    }

    /// `POST /tokens/m3` — evaluate `getCSSVars()` on the page and return typed M3 token map.
    ///
    /// The BXC daemon runs a `page.evaluate` that walks `document.styleSheets` and collects
    /// every `--md-*` CSS custom property with its computed value, then returns the map as JSON.
    pub(crate) async fn extract_m3_tokens(&self, url: &str) -> anyhow::Result<M3TokenMap> {
        let endpoint = format!("{}/tokens/m3", self.base_url);
        let body = serde_json::json!({ "url": url });

        let resp = self
            .http
            .post(&endpoint)
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("{}: {e}", self.daemon_error()))?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("bxc /tokens/m3 returned HTTP {status}: {text}");
        }

        resp.json::<M3TokenMap>()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse /tokens/m3 response: {e}"))
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Install the ring provider once per process.  Idempotent: subsequent calls
    /// to `install_default_provider` are a no-op when the provider is already set.
    fn install_provider() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }

    #[test]
    fn client_new_does_not_panic() {
        install_provider();
        // Verifies construction without daemon presence: no network call, no panic.
        let client = ScrapeClient::new().expect("ScrapeClient::new() must succeed");
        assert!(!client.base_url.is_empty());
        assert!(
            client.base_url.starts_with("http://") || client.base_url.starts_with("https://"),
            "base_url must be an HTTP(S) URL, got: {}",
            client.base_url
        );
    }

    #[test]
    fn client_respects_env_override() {
        install_provider();
        // Safety: test-only; single-threaded test runner on this env var.
        unsafe {
            std::env::set_var("BXC_DAEMON_URL", "http://127.0.0.1:9999");
        }
        let client = ScrapeClient::new().expect("ScrapeClient::new() must succeed");
        assert_eq!(client.base_url, "http://127.0.0.1:9999");
        unsafe {
            std::env::remove_var("BXC_DAEMON_URL");
        }
    }
}
