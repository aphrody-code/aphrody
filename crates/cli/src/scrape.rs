// SPDX-License-Identifier: Apache-2.0
/// scrape.rs — BXC daemon HTTP client for `aphrody scrape` and `aphrody tokens`.
///
/// The BXC engine is a headless-browser daemon (`packages/bxc/`, Bun runtime)
/// that exposes a JSON API at `BXC_DAEMON_URL` (default: `http://localhost:8765`).
/// Routes are prefixed `/api/` (cf. `packages/bxc/src/cli/api.ts`).
///
/// Auto-start: if the daemon is unreachable on the configured port, this
/// client spawns it via `aphrody bxc daemon --port <p>` and waits for
/// `/healthz` to return 200 OK (up to 10 s) before issuing the user query.
use serde::{Deserialize, Serialize};

// ── Public API types ─────────────────────────────────────────────────────────

/// Reconnaissance result returned by `bxc recon`.
///
/// The bxc Bun API returns a rich, evolving JSON schema (URL, DNS, CDN
/// detection, frontend/backend stacks, etc.). Rather than mirroring every
/// field — which would rot at every bxc release — we hand the raw JSON back
/// to the caller. The CLI just pretty-prints it; downstream automation can
/// pick fields with `jq` or any JSON tool.
pub(crate) type ReconResult = serde_json::Value;

/// One element matched by a CSS selector via `bxc /api/scrape`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SelectorMatch {
    pub index: u32,
    pub text: String,
}

/// Result of a CSS-selector extraction pass.
///
/// `matches` mirrors the bxc Bun `/api/scrape` response (`Array<{index, text}>`),
/// re-wrapped with the original `url` + `selector` for caller-friendly output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SelectorResult {
    pub url: String,
    pub selector: String,
    pub matches: Vec<SelectorMatch>,
}

/// Deep technology / framework detection — raw bxc payload (frontend, backend,
/// CDN, CMS, hosting, ...). Schema evolves with bxc upstream; pass through.
pub(crate) type DetectResult = serde_json::Value;

/// Typed Material Design 3 design-token map (legacy — not served by bxc Bun
/// `/api/`, kept for the eventual `crates/bxc-engine` driver). Keys are CSS
/// custom-property names (`--md-sys-color-primary`, …).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct M3TokenMap {
    pub source_url: String,
    pub tokens: std::collections::HashMap<String, String>,
}

// ── Client ───────────────────────────────────────────────────────────────────

const DEFAULT_DAEMON_URL: &str = "http://localhost:8765";
/// Default API path prefix (matches `packages/bxc/src/cli/api.ts` routes).
const API_PREFIX: &str = "/api";
/// Total time to wait for the daemon to become healthy after auto-spawn.
const HEALTHZ_TIMEOUT_S: u64 = 10;
/// Per-poll timeout while waiting for `/healthz`.
const HEALTHZ_POLL_INTERVAL_MS: u64 = 200;

/// Async HTTP client that speaks to the BXC daemon JSON API.
pub(crate) struct ScrapeClient {
    pub(crate) base_url: String,
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

    /// Build a full endpoint URL combining base_url + `/api/<route>`.
    /// If `base_url` already ends with `/api`, the prefix is not doubled.
    fn endpoint(&self, route: &str) -> String {
        let trimmed = self.base_url.trim_end_matches('/');
        if trimmed.ends_with(API_PREFIX) {
            format!("{trimmed}/{route}")
        } else {
            format!("{trimmed}{API_PREFIX}/{route}")
        }
    }

    /// Extract `host:port` from the base URL so we can spawn a daemon on the
    /// matching port. Falls back to `8765` when parsing fails.
    fn parsed_port(&self) -> u16 {
        url_port_or_default(&self.base_url, 8765)
    }

    /// Returns a human-readable error message when the daemon is unreachable.
    fn daemon_error(&self) -> String {
        format!(
            "bxc daemon not reachable at {}. Start it via: aphrody bxc daemon --port {}",
            self.base_url,
            self.parsed_port()
        )
    }

    /// `GET /healthz` — quick liveness probe (200ms timeout).
    async fn healthz(&self) -> bool {
        let healthz_url = format!("{}/healthz", self.base_url.trim_end_matches('/'));
        let probe = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(500))
            .build();
        let Ok(probe) = probe else { return false };
        matches!(probe.get(&healthz_url).send().await, Ok(r) if r.status().is_success())
    }

    /// Ensure the daemon is responding. If not, auto-spawn it (best-effort)
    /// via `aphrody bxc daemon --port <p>` and poll `/healthz` until ready
    /// or `HEALTHZ_TIMEOUT_S` elapses.
    ///
    /// Returns `Ok(())` when the daemon is healthy, `Err(_)` if the spawn
    /// failed AND the daemon is still unreachable after the timeout.
    pub(crate) async fn ensure_running(&self) -> anyhow::Result<()> {
        if self.healthz().await {
            return Ok(());
        }
        // Spawn the daemon (don't block on its lifetime — it detaches itself).
        let port = self.parsed_port();
        let _ = spawn_daemon_detached(port);

        let deadline =
            std::time::Instant::now() + std::time::Duration::from_secs(HEALTHZ_TIMEOUT_S);
        while std::time::Instant::now() < deadline {
            if self.healthz().await {
                return Ok(());
            }
            tokio::time::sleep(std::time::Duration::from_millis(HEALTHZ_POLL_INTERVAL_MS)).await;
        }
        anyhow::bail!(
            "bxc daemon failed to start within {HEALTHZ_TIMEOUT_S}s at {}. {}",
            self.base_url,
            self.daemon_error()
        );
    }

    /// `GET /api/recon?url=…` — full-page reconnaissance.
    pub(crate) async fn recon(&self, url: &str) -> anyhow::Result<ReconResult> {
        self.ensure_running().await?;
        let endpoint = self.endpoint("recon");

        let resp = self
            .http
            .get(&endpoint)
            .query(&[("url", url)])
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("{}: {e}", self.daemon_error()))?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("bxc /api/recon returned HTTP {status}: {text}");
        }

        resp.json::<ReconResult>()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse /api/recon response: {e}"))
    }

    /// `GET /api/scrape?url=…&selector=…` — extract `textContent` of matched elements.
    pub(crate) async fn scrape(&self, url: &str, selector: &str) -> anyhow::Result<SelectorResult> {
        self.ensure_running().await?;
        let endpoint = self.endpoint("scrape");

        let resp = self
            .http
            .get(&endpoint)
            .query(&[("url", url), ("selector", selector)])
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("{}: {e}", self.daemon_error()))?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("bxc /api/scrape returned HTTP {status}: {text}");
        }

        let matches: Vec<SelectorMatch> = resp
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse /api/scrape response: {e}"))?;

        Ok(SelectorResult {
            url: url.to_owned(),
            selector: selector.to_owned(),
            matches,
        })
    }

    /// `GET /api/detect?url=…` — deep tech detection (frontend, backend, CDN, ...).
    #[allow(dead_code)] // (unused on wasm builds where `bxc_detect` is cfg-stripped)
    pub(crate) async fn detect(&self, url: &str) -> anyhow::Result<DetectResult> {
        self.ensure_running().await?;
        let endpoint = self.endpoint("detect");

        let resp = self
            .http
            .get(&endpoint)
            .query(&[("url", url)])
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("{}: {e}", self.daemon_error()))?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("bxc /api/detect returned HTTP {status}: {text}");
        }

        resp.json::<DetectResult>()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse /api/detect response: {e}"))
    }

    /// M3 design-token extraction via CSS-selector scrape.
    ///
    /// bxc Bun does not ship a dedicated `/tokens/m3` route; we approximate by
    /// scraping inline `<style>` blocks for `--md-*` custom properties via
    /// `/api/scrape`. For comprehensive token extraction the user should run
    /// the dedicated `crates/bxc-engine` driver or `cargo run -p aphrody-design-agents`.
    pub(crate) async fn extract_m3_tokens(&self, url: &str) -> anyhow::Result<M3TokenMap> {
        self.ensure_running().await?;
        let selector = ":root";
        let result = self.scrape(url, selector).await?;
        // Best-effort regex parse for `--md-*: <value>;` pairs across the
        // concatenated textContent of `:root` elements.
        let mut tokens = std::collections::HashMap::new();
        for m in &result.matches {
            for line in m.text.split(';') {
                let line = line.trim();
                if let Some((k, v)) = line.split_once(':') {
                    let k = k.trim();
                    if k.starts_with("--md-") {
                        tokens.insert(k.to_string(), v.trim().to_string());
                    }
                }
            }
        }
        Ok(M3TokenMap { source_url: url.to_string(), tokens })
    }
}

// ── Daemon spawn helper ──────────────────────────────────────────────────────

/// Fire-and-forget spawn of `aphrody bxc daemon --port <port>`. The child
/// detaches from the current process; this function returns as soon as the
/// fork/CreateProcess returns, regardless of whether the daemon ends up
/// healthy. Health is verified separately via [`ScrapeClient::healthz`].
fn spawn_daemon_detached(port: u16) -> std::io::Result<()> {
    let aphrody = std::env::current_exe()?;
    let mut cmd = std::process::Command::new(&aphrody);
    cmd.args(["bxc", "daemon", "--port", &port.to_string()]);
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::null());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        cmd.creation_flags(DETACHED_PROCESS);
    }
    cmd.spawn()?;
    Ok(())
}

/// Extract the port component of a URL like `http://host:port/...` without
/// pulling in the `url` crate. Returns `default` for parse failures.
fn url_port_or_default(base_url: &str, default: u16) -> u16 {
    let after_scheme = base_url.split_once("://").map(|(_, r)| r).unwrap_or(base_url);
    let host_port = after_scheme.split('/').next().unwrap_or(after_scheme);
    host_port
        .rsplit_once(':')
        .and_then(|(_, p)| p.split('/').next())
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(default)
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

    #[test]
    fn endpoint_adds_api_prefix_when_missing() {
        install_provider();
        unsafe {
            std::env::set_var("BXC_DAEMON_URL", "http://127.0.0.1:8765");
        }
        let client = ScrapeClient::new().unwrap();
        assert_eq!(client.endpoint("recon"), "http://127.0.0.1:8765/api/recon");
        unsafe {
            std::env::remove_var("BXC_DAEMON_URL");
        }
    }

    #[test]
    fn endpoint_does_not_double_api_prefix() {
        install_provider();
        unsafe {
            std::env::set_var("BXC_DAEMON_URL", "http://127.0.0.1:8765/api");
        }
        let client = ScrapeClient::new().unwrap();
        assert_eq!(client.endpoint("scrape"), "http://127.0.0.1:8765/api/scrape");
        unsafe {
            std::env::remove_var("BXC_DAEMON_URL");
        }
    }

    #[test]
    fn url_port_or_default_parses_standard_url() {
        assert_eq!(url_port_or_default("http://localhost:8765", 0), 8765);
        assert_eq!(url_port_or_default("http://127.0.0.1:9999/api", 0), 9999);
        assert_eq!(url_port_or_default("https://example.com:443/x", 80), 443);
    }

    #[test]
    fn url_port_or_default_falls_back_when_missing() {
        assert_eq!(url_port_or_default("http://example.com", 8765), 8765);
        assert_eq!(url_port_or_default("not-a-url", 8765), 8765);
    }
}
