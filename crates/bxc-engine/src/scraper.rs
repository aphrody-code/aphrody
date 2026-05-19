// SPDX-License-Identifier: Apache-2.0
//! Pure-HTTP reconnaissance, detection, and CSS-selector extraction.
//!
//! This module ports the structured-data extraction lanes of
//! `packages/bxc/src/cli/{recon,detect,scrape}.ts` to Rust without
//! requiring a headless browser. It uses `reqwest` for HTTP, `scraper`
//! for CSS selectors, and a small built-in fingerprint table for the
//! CDN / framework heuristics that the TS reference applies at the
//! response-header level.
//!
//! For full SSR / hydration support (`/api/next`), a CDP-backed driver
//! is required — that lives in [`crate::cdp`] and the daemon's
//! `/api/next` route activates only when a CDP target is reachable.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

// ─────────────────────────────────────────────────────────────────────────────
// Public types — mirror `packages/bxc/src/cli/recon.ts` `ReconResult`.
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconHeaders {
    pub server: Option<String>,
    #[serde(rename = "xPoweredBy")]
    pub x_powered_by: Option<String>,
    #[serde(rename = "cdnRayId")]
    pub cdn_ray_id: Option<String>,
    #[serde(rename = "cdnVendor")]
    pub cdn_vendor: String,
    #[serde(rename = "cspHosts")]
    pub csp_hosts: Vec<String>,
    #[serde(rename = "cacheControl")]
    pub cache_control: Option<String>,
    #[serde(rename = "contentSecurityPolicy")]
    pub content_security_policy: Option<String>,
    #[serde(rename = "contentType")]
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconAsset {
    /// `stylesheet` | `script` | `image` | `font` | `iframe`.
    #[serde(rename = "type")]
    pub kind: String,
    pub url: String,
    pub host: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconFramework {
    pub name: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub categories: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconResult {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub url: String,
    #[serde(rename = "finalUrl")]
    pub final_url: String,
    #[serde(rename = "httpStatus")]
    pub http_status: u16,
    pub bytes: usize,
    #[serde(rename = "gotoMs")]
    pub goto_ms: u128,
    pub profile: String,
    pub headers: ReconHeaders,
    pub frameworks: Vec<ReconFramework>,
    pub assets: Vec<ReconAsset>,
    #[serde(rename = "cssSelectors")]
    pub css_selectors: Vec<String>,
    /// SHA-256 of the response body, hex-lowercase. Rust extension over the
    /// bxc-recon-v1 schema (Bun version omits this); kept stable so the
    /// daemon caller can use it as an ETag.
    #[serde(rename = "bodyHash")]
    pub body_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionEvidence {
    pub name: String,
    pub evidence: String,
    pub source: String, // header|dns|ip|body|csp|wappalyzer|cert
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub categories: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DetectResult {
    pub url: String,
    #[serde(rename = "finalUrl")]
    pub final_url: String,
    #[serde(rename = "httpStatus")]
    pub http_status: u16,
    pub hostname: String,
    pub frontend: Vec<DetectionEvidence>,
    pub backend: Vec<DetectionEvidence>,
    pub cdn: Vec<DetectionEvidence>,
    pub server: Vec<DetectionEvidence>,
    pub cms: Vec<DetectionEvidence>,
    pub analytics: Vec<DetectionEvidence>,
    pub framework: Vec<DetectionEvidence>,
    pub library: Vec<DetectionEvidence>,
    pub other: Vec<DetectionEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapeMatch {
    pub index: u32,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct M3TokenMap {
    #[serde(rename = "sourceUrl")]
    pub source_url: String,
    pub tokens: HashMap<String, String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Client
// ─────────────────────────────────────────────────────────────────────────────

const UA: &str = "bxc-engine/0.1 (+https://github.com/aphrody-code/aphrody)";
const DEFAULT_TIMEOUT_S: u64 = 25;

/// Build a reqwest client with timeouts + a Chrome-ish UA so we don't get
/// fingerprinted out of the common edge caches.
pub fn http_client() -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(DEFAULT_TIMEOUT_S))
        .user_agent(UA)
        .build()
}

// ─────────────────────────────────────────────────────────────────────────────
// CDN / framework fingerprints — ported from
// packages/bxc/src/cli/recon.ts `fingerprintCdn` + packages/bxc/src/detect.ts.
// ─────────────────────────────────────────────────────────────────────────────

pub fn fingerprint_cdn(headers: &HashMap<String, String>) -> String {
    let get = |k: &str| headers.get(&k.to_ascii_lowercase()).cloned().unwrap_or_default();
    let server = get("server").to_ascii_lowercase();
    if !get("cf-ray").is_empty() || server.contains("cloudflare") {
        return "Cloudflare".into();
    }
    if !get("x-amz-cf-id").is_empty() || !get("x-amz-cf-pop").is_empty() {
        return "AWS CloudFront".into();
    }
    if !get("x-served-by").is_empty() && !get("x-cache").is_empty() {
        return "Fastly".into();
    }
    if server.contains("akamai") || !get("x-akamai-edge").is_empty() {
        return "Akamai".into();
    }
    if server == "google frontend"
        || server.contains("gws")
        || server.contains("esf")
        || !get("x-cloud-trace-context").is_empty()
    {
        return "Google Frontend".into();
    }
    if !get("x-vercel-id").is_empty() || server.contains("vercel") {
        return "Vercel".into();
    }
    if server.contains("nginx") {
        return "nginx".into();
    }
    "unknown".into()
}

/// Tiny body-based framework sniffer (HTML markers). Mirrors enough of
/// wappalyzer to make `aphrody bxc detect` useful without pulling a 50 MB
/// signature DB. We score by counting marker occurrences — every match
/// produces a `DetectionEvidence` with confidence in `[0, 1]`.
pub fn fingerprint_frameworks(html: &str) -> Vec<DetectionEvidence> {
    const MARKERS: &[(&str, &str, &str, &str)] = &[
        ("Next.js", "__NEXT_DATA__", "body", "frontend"),
        ("Next.js (App)", "__next_f.push", "body", "frontend"),
        ("Nuxt", "__NUXT__", "body", "frontend"),
        ("Astro", "astro-island", "body", "frontend"),
        ("Remix", "window.__remixContext", "body", "frontend"),
        ("SvelteKit", "__sveltekit", "body", "frontend"),
        ("React", "data-reactroot", "body", "library"),
        ("React (modern)", "id=\"__react", "body", "library"),
        ("Vue", "data-v-", "body", "library"),
        ("Tailwind", "tailwindcss", "body", "library"),
        ("Material Web", "md-sys-color-", "body", "library"),
        ("WordPress", "/wp-content/", "body", "cms"),
        ("Wagtail", "/admin/wagtail", "body", "cms"),
        ("Shopify", "Shopify.theme", "body", "cms"),
        ("Hubspot", "_hsq.push", "body", "analytics"),
        ("Google Analytics", "google-analytics.com/ga.js", "body", "analytics"),
        ("Plausible", "plausible.io/js/", "body", "analytics"),
        ("Sentry", "sentry-trace", "body", "library"),
        ("Cloudflare Insights", "static.cloudflareinsights.com", "body", "analytics"),
        ("htmx", "hx-get", "body", "library"),
        ("Alpine.js", "x-data=", "body", "library"),
    ];
    let mut out = Vec::new();
    for (name, marker, source, category) in MARKERS {
        if html.contains(marker) {
            out.push(DetectionEvidence {
                name: (*name).to_string(),
                evidence: (*marker).to_string(),
                source: (*source).to_string(),
                confidence: Some(0.85),
                version: None,
                categories: vec![(*category).to_string()],
            });
        }
    }
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// Recon
// ─────────────────────────────────────────────────────────────────────────────

fn lower_headers(resp: &reqwest::Response) -> HashMap<String, String> {
    resp.headers()
        .iter()
        .map(|(k, v)| (k.as_str().to_ascii_lowercase(), v.to_str().unwrap_or("").to_string()))
        .collect()
}

fn extract_csp_hosts(csp: &str) -> Vec<String> {
    let mut hosts = Vec::new();
    for part in csp.split([';', ' ']) {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("https://") {
            if let Some(host) = rest.split('/').next() {
                hosts.push(host.to_string());
            }
        } else if let Some(rest) = trimmed.strip_prefix("http://") {
            if let Some(host) = rest.split('/').next() {
                hosts.push(host.to_string());
            }
        } else if trimmed.contains('.') && !trimmed.contains('\'') {
            hosts.push(trimmed.trim_end_matches(';').to_string());
        }
    }
    hosts.sort();
    hosts.dedup();
    hosts
}

fn collect_assets(html: &Html, base: &url::Url) -> Vec<ReconAsset> {
    let mut out = Vec::new();
    let pairs: &[(&str, &str, &str)] = &[
        ("link[rel=stylesheet][href]", "href", "stylesheet"),
        ("script[src]", "src", "script"),
        ("img[src]", "src", "image"),
        ("link[rel*=icon][href]", "href", "image"),
        ("link[as=font][href]", "href", "font"),
        ("iframe[src]", "src", "iframe"),
    ];
    for (sel_str, attr, kind) in pairs {
        let Ok(sel) = Selector::parse(sel_str) else { continue };
        for el in html.select(&sel) {
            let Some(raw) = el.value().attr(attr) else { continue };
            let Ok(absolute) = base.join(raw) else { continue };
            let host = absolute.host_str().unwrap_or("").to_string();
            out.push(ReconAsset {
                kind: (*kind).to_string(),
                url: absolute.to_string(),
                host,
            });
        }
    }
    out
}

fn collect_css_selectors(html: &Html) -> Vec<String> {
    let mut out = Vec::new();
    let Ok(sel) = Selector::parse("style") else { return out };
    for style_el in html.select(&sel) {
        let body: String = style_el.text().collect();
        for line in body.split(['{', '}']) {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("@") || trimmed.contains(':') {
                continue;
            }
            // Each comma-separated selector is one entry.
            for part in trimmed.split(',') {
                let p = part.trim();
                if !p.is_empty() && p.len() < 240 {
                    out.push(p.to_string());
                }
            }
        }
    }
    out.sort();
    out.dedup();
    out.truncate(500);
    out
}

/// Run a recon pass against `url_str`. Pure HTTP — no JS, no headless
/// browser. The `profile` field of the result is hard-coded to `"http"`
/// so the consumer knows hydration analysis was NOT performed.
pub async fn recon(client: &reqwest::Client, url_str: &str) -> anyhow::Result<ReconResult> {
    let start = Instant::now();
    let resp = client
        .get(url_str)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("HTTP fetch failed for {url_str}: {e}"))?;

    let final_url = resp.url().clone();
    let status = resp.status().as_u16();
    let raw_headers = lower_headers(&resp);
    let body = resp.bytes().await.map_err(|e| anyhow::anyhow!("Body read failed: {e}"))?;
    let bytes_len = body.len();

    let mut hasher = Sha256::new();
    hasher.update(&body);
    let body_hash = format!("{:x}", hasher.finalize());

    let body_str = String::from_utf8_lossy(&body);
    let dom = Html::parse_document(&body_str);

    let cdn_vendor = fingerprint_cdn(&raw_headers);
    let csp = raw_headers.get("content-security-policy").cloned();
    let csp_hosts = csp.as_deref().map(extract_csp_hosts).unwrap_or_default();
    let headers = ReconHeaders {
        server: raw_headers.get("server").cloned(),
        x_powered_by: raw_headers.get("x-powered-by").cloned(),
        cdn_ray_id: raw_headers
            .get("cf-ray")
            .or_else(|| raw_headers.get("x-amz-cf-id"))
            .cloned(),
        cdn_vendor,
        csp_hosts,
        cache_control: raw_headers.get("cache-control").cloned(),
        content_security_policy: csp,
        content_type: raw_headers.get("content-type").cloned(),
    };

    let frameworks: Vec<ReconFramework> = fingerprint_frameworks(&body_str)
        .into_iter()
        .map(|ev| ReconFramework { name: ev.name, categories: ev.categories, version: ev.version })
        .collect();

    let assets = collect_assets(&dom, &final_url);
    let css_selectors = collect_css_selectors(&dom);

    Ok(ReconResult {
        schema: "bxc-recon-v1".into(),
        url: url_str.to_string(),
        final_url: final_url.to_string(),
        http_status: status,
        bytes: bytes_len,
        goto_ms: start.elapsed().as_millis(),
        profile: "http".into(),
        headers,
        frameworks,
        assets,
        css_selectors,
        body_hash,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Detect
// ─────────────────────────────────────────────────────────────────────────────

/// Deep detection: combines header CDN/server fingerprints + body
/// framework markers + CSP host extraction. Pure HTTP.
pub async fn detect(client: &reqwest::Client, url_str: &str) -> anyhow::Result<DetectResult> {
    let resp = client
        .get(url_str)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("HTTP fetch failed for {url_str}: {e}"))?;
    let final_url = resp.url().clone();
    let status = resp.status().as_u16();
    let raw_headers = lower_headers(&resp);
    let hostname = final_url.host_str().unwrap_or("").to_string();
    let body = resp.bytes().await.map_err(|e| anyhow::anyhow!("Body read failed: {e}"))?;
    let body_str = String::from_utf8_lossy(&body);

    let mut result = DetectResult {
        url: url_str.to_string(),
        final_url: final_url.to_string(),
        http_status: status,
        hostname,
        ..Default::default()
    };

    // CDN bucket from headers.
    let cdn = fingerprint_cdn(&raw_headers);
    if cdn != "unknown" {
        result.cdn.push(DetectionEvidence {
            name: cdn,
            evidence: "response headers".into(),
            source: "header".into(),
            confidence: Some(0.95),
            version: None,
            categories: vec!["cdn".into()],
        });
    }
    if let Some(server) = raw_headers.get("server") {
        result.server.push(DetectionEvidence {
            name: server.clone(),
            evidence: format!("Server: {server}"),
            source: "header".into(),
            confidence: Some(1.0),
            version: None,
            categories: vec!["server".into()],
        });
    }
    if let Some(xpb) = raw_headers.get("x-powered-by") {
        result.backend.push(DetectionEvidence {
            name: xpb.clone(),
            evidence: format!("X-Powered-By: {xpb}"),
            source: "header".into(),
            confidence: Some(0.9),
            version: None,
            categories: vec!["backend".into()],
        });
    }

    // Body markers → distribute by category.
    for ev in fingerprint_frameworks(&body_str) {
        let bucket = ev.categories.first().cloned().unwrap_or_else(|| "other".into());
        match bucket.as_str() {
            "frontend" => result.frontend.push(ev),
            "backend" => result.backend.push(ev),
            "cms" => result.cms.push(ev),
            "analytics" => result.analytics.push(ev),
            "library" => result.library.push(ev),
            "framework" => result.framework.push(ev),
            _ => result.other.push(ev),
        }
    }

    Ok(result)
}

// ─────────────────────────────────────────────────────────────────────────────
// Scrape (CSS selector → textContent)
// ─────────────────────────────────────────────────────────────────────────────

pub async fn scrape(
    client: &reqwest::Client,
    url_str: &str,
    selector: &str,
    max: usize,
) -> anyhow::Result<Vec<ScrapeMatch>> {
    let resp = client
        .get(url_str)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("HTTP fetch failed for {url_str}: {e}"))?;
    let body = resp.text().await.map_err(|e| anyhow::anyhow!("Body read failed: {e}"))?;
    let dom = Html::parse_document(&body);
    let sel = Selector::parse(selector)
        .map_err(|e| anyhow::anyhow!("Invalid CSS selector `{selector}`: {e:?}"))?;
    let mut out = Vec::new();
    for (i, el) in dom.select(&sel).enumerate() {
        if i >= max {
            break;
        }
        let text: String = el.text().collect::<Vec<_>>().join(" ");
        let trimmed = text.split_whitespace().collect::<Vec<_>>().join(" ");
        let clipped: String = trimmed.chars().take(500).collect();
        out.push(ScrapeMatch { index: i as u32, text: clipped });
    }
    Ok(out)
}

// ─────────────────────────────────────────────────────────────────────────────
// M3 design tokens harvester
// ─────────────────────────────────────────────────────────────────────────────

/// Collect every absolute external stylesheet URL referenced by the
/// document.  Skips `data:` / `chrome-extension:` / `blob:` schemes and
/// any `href` that fails to resolve against `base`. Protocol-relative
/// (`//cdn…/…`) and absolute paths (`/static/app.css`) are resolved via
/// [`url::Url::join`].
fn extract_stylesheet_urls(html: &Html, base: &url::Url) -> Vec<url::Url> {
    let Ok(sel) = Selector::parse("link[rel~=\"stylesheet\"][href]") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for el in html.select(&sel) {
        let Some(raw) = el.value().attr("href") else { continue };
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(absolute) = base.join(trimmed) else { continue };
        match absolute.scheme() {
            "http" | "https" => out.push(absolute),
            _ => continue,
        }
    }
    out
}

/// In-place harvest of every `--md-sys-…` CSS custom property from `css`.
///
/// Keys are preserved with the full `--md-sys-` prefix to stay
/// drop-in-compatible with the existing daemon smoke test and the
/// CLI consumer in `crates/cli/src/scrape.rs`. The algorithm mirrors
/// `crate::google::style::extract_m3_tokens` (single-pass, depth-aware
/// for paren-balanced values) but without prefix stripping.
fn harvest_md_sys_tokens(css: &str, out: &mut HashMap<String, String>) {
    const PREFIX: &str = "--md-sys-";
    let bytes = css.as_bytes();
    let mut i = 0_usize;
    while i + PREFIX.len() < bytes.len() {
        if &bytes[i..i + PREFIX.len()] != PREFIX.as_bytes() {
            i += 1;
            continue;
        }
        // Walk the identifier: letters, digits, `-`, `_`, `.`.
        let name_start = i;
        let mut name_end = i + PREFIX.len();
        while name_end < bytes.len() {
            let b = bytes[name_end];
            if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' {
                name_end += 1;
            } else {
                break;
            }
        }
        if name_end == name_start + PREFIX.len() {
            i = name_end;
            continue;
        }
        // Skip horizontal whitespace, expect ':'.
        let mut j = name_end;
        while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t') {
            j += 1;
        }
        if j >= bytes.len() || bytes[j] != b':' {
            i = name_end;
            continue;
        }
        j += 1;
        while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t') {
            j += 1;
        }
        let value_start = j;
        let mut depth = 0_i32;
        let mut value_end = j;
        while value_end < bytes.len() {
            let b = bytes[value_end];
            if depth == 0 && (b == b';' || b == b'}') {
                break;
            }
            if b == b'(' {
                depth += 1;
            } else if b == b')' {
                depth = depth.saturating_sub(1);
            }
            value_end += 1;
        }
        let mut trimmed_end = value_end;
        while trimmed_end > value_start
            && matches!(bytes[trimmed_end - 1], b' ' | b'\t' | b'\n' | b'\r')
        {
            trimmed_end -= 1;
        }
        // We only ever slice at boundaries we located via ASCII-byte
        // checks (alphanumeric, `:`, `;`, `}`, whitespace, parens) so
        // UTF-8 multibyte sequences inside the value are preserved
        // intact — they can only appear strictly between the markers.
        let name = std::str::from_utf8(&bytes[name_start..name_end]).unwrap_or("");
        let value = std::str::from_utf8(&bytes[value_start..trimmed_end]).unwrap_or("");
        if !name.is_empty() {
            out.insert(name.to_string(), value.to_string());
        }
        i = value_end;
    }
}

/// Harvest Material Design 3 system tokens (`--md-sys-…`) from a page,
/// including stylesheets loaded via `<link rel="stylesheet">`.
///
/// Pipeline:
///
/// 1. Fetch the page HTML at `url_str`.
/// 2. Parse the DOM, concatenate every inline `<style>` body.
/// 3. Collect every external `<link rel="stylesheet" href="…">`, resolve
///    against the final URL, fetch each stylesheet via the shared client
///    (sequential — token pages reference <10 sheets), and append.
///    Failed fetches are logged via `tracing::warn!` and skipped — the
///    call never fails because one CDN sheet is unreachable.
/// 4. Run [`harvest_md_sys_tokens`] over the combined CSS buffer.
///
/// Keys keep their full `--md-sys-…` prefix (drop-in compatible with the
/// daemon smoke test + CLI consumer).
pub async fn tokens_m3(client: &reqwest::Client, url_str: &str) -> anyhow::Result<M3TokenMap> {
    let resp = client
        .get(url_str)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("HTTP fetch failed for {url_str}: {e}"))?;
    let final_url = resp.url().clone();
    let html_text = resp.text().await.map_err(|e| anyhow::anyhow!("Body read failed: {e}"))?;

    // Parse the DOM, extract every CSS-bearing artefact, then drop the
    // DOM. `scraper::Html` holds `Rc`s and is therefore `!Send`, so it
    // must not be alive across any subsequent `.await` (the stylesheet
    // fetch loop below) — otherwise the handler future stops being
    // `Send` and axum's `Handler` trait bound breaks.
    let (mut buf, stylesheet_urls) = {
        let dom = Html::parse_document(&html_text);
        let mut buf = String::new();
        if let Ok(sel) = Selector::parse("style") {
            for style_el in dom.select(&sel) {
                let body: String = style_el.text().collect();
                buf.push_str(&body);
                buf.push('\n');
            }
        }
        let urls = extract_stylesheet_urls(&dom, &final_url);
        (buf, urls)
    };

    // External <link rel="stylesheet" href="…"> resources.
    for sheet_url in stylesheet_urls {
        match client.get(sheet_url.clone()).send().await {
            Ok(sheet_resp) => {
                if !sheet_resp.status().is_success() {
                    tracing::warn!(
                        target: "bxc::tokens_m3",
                        status = sheet_resp.status().as_u16(),
                        url = %sheet_url,
                        "stylesheet returned non-2xx, skipping"
                    );
                    continue;
                }
                match sheet_resp.text().await {
                    Ok(css) => {
                        buf.push_str(&css);
                        buf.push('\n');
                    }
                    Err(e) => tracing::warn!(
                        target: "bxc::tokens_m3",
                        error = %e,
                        url = %sheet_url,
                        "stylesheet body read failed, skipping"
                    ),
                }
            }
            Err(e) => tracing::warn!(
                target: "bxc::tokens_m3",
                error = %e,
                url = %sheet_url,
                "stylesheet fetch failed, skipping"
            ),
        }
    }

    let mut tokens = HashMap::new();
    harvest_md_sys_tokens(&buf, &mut tokens);
    Ok(M3TokenMap { source_url: url_str.to_string(), tokens })
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cdn_cloudflare_via_cf_ray() {
        let mut h = HashMap::new();
        h.insert("cf-ray".into(), "abc-CDG".into());
        assert_eq!(fingerprint_cdn(&h), "Cloudflare");
    }

    #[test]
    fn cdn_vercel_via_x_vercel_id() {
        let mut h = HashMap::new();
        h.insert("x-vercel-id".into(), "lhr1::xyz".into());
        assert_eq!(fingerprint_cdn(&h), "Vercel");
    }

    #[test]
    fn cdn_unknown_when_no_marker() {
        let h = HashMap::new();
        assert_eq!(fingerprint_cdn(&h), "unknown");
    }

    #[test]
    fn framework_detects_next_data_marker() {
        let html = r#"<html><script id="__NEXT_DATA__">{"props":{}}</script></html>"#;
        let evs = fingerprint_frameworks(html);
        assert!(evs.iter().any(|e| e.name == "Next.js"));
    }

    #[test]
    fn framework_detects_material_web_via_token_marker() {
        let html = "<style>:root{--md-sys-color-primary:#000;}</style>";
        let evs = fingerprint_frameworks(html);
        assert!(evs.iter().any(|e| e.name == "Material Web"));
    }

    #[test]
    fn csp_extracts_https_hosts() {
        let csp = "default-src 'self' https://cdn.example.com; \
                   script-src 'self' https://js.stripe.com";
        let hosts = extract_csp_hosts(csp);
        assert!(hosts.contains(&"cdn.example.com".to_string()));
        assert!(hosts.contains(&"js.stripe.com".to_string()));
    }

    #[test]
    fn tokens_m3_extracts_external_link_hrefs() {
        let html = r#"<!doctype html><html><head>
            <link rel="stylesheet" href="/static/app.css">
            <link rel="stylesheet" href="https://cdn.example.com/m3.css">
            <link rel="stylesheet" href="//cdn.example.com/proto-relative.css">
            <link rel="stylesheet" href="data:text/css,body{}">
            <link rel="icon" href="/favicon.ico">
            <link rel="stylesheet">
            <style>:root{--md-sys-color-primary:#000;}</style>
        </head><body></body></html>"#;
        let dom = Html::parse_document(html);
        let base = url::Url::parse("https://m3.material.io/foundations/design-tokens").unwrap();
        let urls = extract_stylesheet_urls(&dom, &base);
        let strs: Vec<String> = urls.iter().map(ToString::to_string).collect();
        assert!(
            strs.contains(&"https://m3.material.io/static/app.css".to_string()),
            "relative href must resolve against base: {strs:?}"
        );
        assert!(
            strs.contains(&"https://cdn.example.com/m3.css".to_string()),
            "absolute https href must be kept: {strs:?}"
        );
        assert!(
            strs.contains(&"https://cdn.example.com/proto-relative.css".to_string()),
            "protocol-relative href must inherit scheme: {strs:?}"
        );
        assert!(
            !strs.iter().any(|s| s.starts_with("data:")),
            "data: scheme must be filtered out: {strs:?}"
        );
        assert!(
            !strs.iter().any(|s| s.ends_with("/favicon.ico")),
            "non-stylesheet rels must be ignored: {strs:?}"
        );
    }

    #[test]
    fn tokens_m3_harvest_preserves_full_md_sys_prefix() {
        let css = r#":root {
            --md-sys-color-primary: #6750A4;
            --md-sys-color-on-primary: #ffffff;
            --md-sys-typescale-body-large-size: 16px;
            --md-sys-color-surface-tint: rgba(103, 80, 164, 0.05);
            --other-token: ignored;
        }"#;
        let mut tokens = HashMap::new();
        harvest_md_sys_tokens(css, &mut tokens);
        assert_eq!(
            tokens.get("--md-sys-color-primary").map(String::as_str),
            Some("#6750A4")
        );
        assert_eq!(
            tokens.get("--md-sys-color-on-primary").map(String::as_str),
            Some("#ffffff")
        );
        assert_eq!(
            tokens.get("--md-sys-typescale-body-large-size").map(String::as_str),
            Some("16px")
        );
        // Paren-balanced values must survive without truncation at the
        // commas inside `rgba(...)`.
        assert_eq!(
            tokens.get("--md-sys-color-surface-tint").map(String::as_str),
            Some("rgba(103, 80, 164, 0.05)")
        );
        assert!(!tokens.contains_key("--other-token"));
        assert!(!tokens.contains_key("--md-sys-"));
    }

    #[test]
    fn tokens_m3_harvest_handles_concatenated_inline_and_external_buffers() {
        // Simulate the concatenated buffer produced by tokens_m3 when
        // inline <style> and external CSS are appended back-to-back.
        let combined = "\
            :root{--md-sys-color-primary:#6750A4;}\n\
            /* external sheet starts here */\n\
            .surface{--md-sys-color-surface:#fff;color:red;}\n";
        let mut tokens = HashMap::new();
        harvest_md_sys_tokens(combined, &mut tokens);
        assert_eq!(tokens.len(), 2, "expected exactly 2 tokens, got {tokens:?}");
        assert!(tokens.contains_key("--md-sys-color-primary"));
        assert!(tokens.contains_key("--md-sys-color-surface"));
    }
}
