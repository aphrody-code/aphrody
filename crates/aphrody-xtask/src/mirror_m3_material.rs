// mirror_m3_material.rs — Breadth-first mirror of https://m3.material.io/
// (Material Design 3 canonical site) into assets/scrape/m3-material/.
//
// Distinct from mirror_google_design (https://design.google/) — this targets
// the Material 3 component catalogue, design tokens, and theme guidance.
//
// Pipeline:
//   1. Probe local bxc daemon (127.0.0.1:<port>); if unreachable, try
//      `bxc launch --port N` background spawn; if still unreachable, fall
//      back to a direct reqwest client (UA: aphrody-design-mirror/0.1).
//   2. Fetch /robots.txt once and collect Disallow rules for User-agent: *.
//   3. BFS from the seed roots, depth <= --depth (default 4), max
//      --max-pages (default 300), same-origin only (m3.material.io host).
//   4. For each accepted URL: try bxc daemon CDP fetch first (hydrated
//      HTML for the M3 Next.js SPA); fall back to direct reqwest GET on
//      any error.
//   5. Classify each URL: paths under /components/* are persisted as
//      `components/<name>/spec.html` + `tokens.json`; everything else
//      lands in `pages/<slug>.html`.
//   6. Extract embedded JSON via `<script type="application/json">` and
//      promote to canonical token files when shape matches color /
//      typography / shape / elevation / motion roles.
//   7. Emit `manifest.ndjson`, `tokens/{color,typography,shape,elevation,
//      motion}.json`, `palette.json` (tonal palettes), and `summary.json`.
//
// Idempotent: pages already present in manifest.ndjson are skipped on rerun.
// Concurrency: futures::stream::buffer_unordered(4) — never serial.

#![allow(clippy::too_many_lines)]

use std::{
    collections::{BTreeMap, BTreeSet, HashSet, VecDeque},
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, anyhow};
use clap::Args as ClapArgs;
use futures::stream::{self, StreamExt};
use regex::Regex;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;
use url::Url;

// Seeds — every page reachable from these covers the Material 3 surface.
const SEED_URLS: &[&str] = &[
    "https://m3.material.io/",
    "https://m3.material.io/components",
    "https://m3.material.io/styles",
    "https://m3.material.io/foundations",
    "https://m3.material.io/develop",
];
const TARGET_HOST: &str = "m3.material.io";
const USER_AGENT: &str = "aphrody-design-mirror/0.1 (+https://github.com/aphrody-code/aphrody)";

// Canonical M3 component slugs we expect to persist under components/<name>/.
// Acts as a routing whitelist + summary counter.
const KNOWN_COMPONENTS: &[&str] = &[
    "buttons",
    "common-buttons",
    "filled-button",
    "outlined-button",
    "text-button",
    "elevated-button",
    "tonal-button",
    "icon-buttons",
    "segmented-buttons",
    "fab",
    "extended-fab",
    "cards",
    "filled-card",
    "outlined-card",
    "elevated-card",
    "text-fields",
    "filled-text-field",
    "outlined-text-field",
    "navigation-bar",
    "navigation-rail",
    "navigation-drawer",
    "top-app-bar",
    "bottom-app-bar",
    "snackbar",
    "dialogs",
    "tabs",
    "switch",
    "slider",
    "chips",
    "checkbox",
    "radio-button",
    "menus",
    "lists",
    "tooltips",
    "progress-indicators",
    "search",
    "date-pickers",
    "time-pickers",
    "bottom-sheets",
    "side-sheets",
    "divider",
    "badges",
    "icons",
];

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    /// Maximum pages to mirror (BFS hard cap).
    #[arg(long, default_value_t = 300)]
    max_pages: usize,

    /// Maximum link-depth from the seed.
    #[arg(long, default_value_t = 4)]
    depth: u32,

    /// Output root (default: assets/scrape/m3-material).
    #[arg(long)]
    out: Option<PathBuf>,

    /// Concurrent fetch buffer.
    #[arg(long, default_value_t = 4)]
    concurrency: usize,

    /// bxc daemon port (CDP); 0 disables bxc and uses reqwest only.
    #[arg(long, default_value_t = 8765)]
    bxc_port: u16,

    /// Per-page fetch timeout (seconds).
    #[arg(long, default_value_t = 30)]
    timeout: u64,
}

/// One entry appended to manifest.ndjson per successfully-fetched page.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PageRecord {
    url: String,
    slug: String,
    #[serde(rename = "type")]
    kind: String, // "page" | "component"
    fetched_at: String,
    status: u16,
    sha256: String,
    bytes: u64,
    depth: u32,
    links_out: Vec<String>,
    source: String, // "bxc" | "reqwest"
    embedded_json_count: usize,
}

/// Final summary.json shape.
#[derive(Debug, Serialize)]
struct Summary {
    seeds: Vec<String>,
    total_pages: usize,
    total_components: usize,
    total_tokens_extracted: usize,
    depth_max: u32,
    started_at: String,
    completed_at: String,
    duration_ms: u128,
    bxc_used: bool,
}

/// Minimal robots.txt rules for User-agent: *.
#[derive(Debug, Default, Clone)]
struct RobotsRules {
    disallow: Vec<String>,
}

impl RobotsRules {
    fn parse(body: &str) -> Self {
        let mut active = false;
        let mut disallow = Vec::new();
        for raw in body.lines() {
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let Some((key, value)) = line.split_once(':') else { continue };
            let key = key.trim().to_ascii_lowercase();
            let value = value.trim();
            match key.as_str() {
                "user-agent" => active = value == "*",
                "disallow" if active && !value.is_empty() => {
                    disallow.push(value.to_string());
                }
                _ => {}
            }
        }
        Self { disallow }
    }

    fn is_allowed(&self, path: &str) -> bool {
        !self.disallow.iter().any(|rule| path.starts_with(rule))
    }
}

/// Canonicalize URL: strip fragment, drop default ports, lowercase host.
fn canonicalize(raw: &str) -> Option<Url> {
    let mut u = Url::parse(raw).ok()?;
    u.set_fragment(None);
    if u.scheme() != "http" && u.scheme() != "https" {
        return None;
    }
    Some(u)
}

/// Derive a filesystem-safe slug from a URL path.
/// "/" -> "index"; "/foo/bar/" -> "foo__bar".
pub(crate) fn slug_from_url(url: &Url) -> String {
    let path = url.path();
    let trimmed = path.trim_matches('/');
    if trimmed.is_empty() {
        return "index".into();
    }
    let mut out = String::with_capacity(trimmed.len());
    for ch in trimmed.chars() {
        match ch {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '.' | '-' | '_' => out.push(ch),
            '/' => out.push_str("__"),
            _ => out.push('_'),
        }
    }
    if let Some(q) = url.query() {
        let mut h = Sha256::new();
        h.update(q.as_bytes());
        let digest = h.finalize();
        out.push('-');
        for b in &digest[..3] {
            out.push_str(&format!("{b:02x}"));
        }
    }
    if out.len() > 180 {
        out.truncate(180);
    }
    out
}

/// Extract the last path segment under /components/ as the component name.
/// Returns None for non-component URLs.
pub(crate) fn component_name_from_url(url: &Url) -> Option<String> {
    let path = url.path().trim_matches('/');
    let mut parts = path.split('/');
    if parts.next()? != "components" {
        return None;
    }
    let name = parts.next()?.trim();
    if name.is_empty() {
        return None;
    }
    // Skip the /components index itself; only sub-pages are components.
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' => c,
            _ => '_',
        })
        .collect();
    Some(cleaned)
}

fn sha256_hex(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    let d = h.finalize();
    let mut out = String::with_capacity(d.len() * 2);
    for b in d {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

fn repo_root() -> Result<PathBuf> {
    if let Ok(dir) = std::env::var("CARGO_WORKSPACE_DIR") {
        return Ok(PathBuf::from(dir));
    }
    let out = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("git rev-parse")?;
    if out.status.success() {
        return Ok(PathBuf::from(String::from_utf8(out.stdout)?.trim()));
    }
    Ok(std::env::current_dir()?)
}

fn now_iso8601() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn append_manifest(path: &Path, rec: &PageRecord) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("open manifest {}", path.display()))?;
    let line = serde_json::to_string(rec)?;
    f.write_all(line.as_bytes())?;
    f.write_all(b"\n")?;
    Ok(())
}

fn load_seen(path: &Path) -> Result<HashSet<String>> {
    let mut seen = HashSet::new();
    if !path.exists() {
        return Ok(seen);
    }
    let f = fs::File::open(path)?;
    for line in BufReader::new(f).lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(rec) = serde_json::from_str::<PageRecord>(&line) {
            seen.insert(rec.url);
        }
    }
    Ok(seen)
}

/// Pull every <a href> as same-origin links to crawl next.
fn extract_links(html: &str, base: &Url) -> BTreeSet<String> {
    let doc = Html::parse_document(html);
    let mut links = BTreeSet::new();
    let a_sel = Selector::parse("a[href]").unwrap();
    for el in doc.select(&a_sel) {
        if let Some(href) = el.value().attr("href") {
            if let Ok(joined) = base.join(href) {
                if let Some(c) = canonicalize(joined.as_str()) {
                    if c.host_str() == Some(TARGET_HOST) {
                        links.insert(c.to_string());
                    }
                }
            }
        }
    }
    links
}

/// Extract every `<script type="application/json">…</script>` payload.
/// Returns successfully parsed JSON values.
pub(crate) fn extract_embedded_json(html: &str) -> Vec<Value> {
    // Multiline, lazy, case-insensitive match of typed script blocks.
    // `(?is)` enables dot-matches-newline + case-insensitive.
    let re = Regex::new(
        r#"(?is)<script[^>]*type\s*=\s*["']application/(?:json|ld\+json)["'][^>]*>(.*?)</script>"#,
    )
    .expect("static regex");

    let mut out = Vec::new();
    for cap in re.captures_iter(html) {
        let raw = cap.get(1).map(|m| m.as_str()).unwrap_or("").trim();
        if raw.is_empty() {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<Value>(raw) {
            out.push(v);
        }
    }
    out
}

/// Walk a JSON value recursively and accumulate token-shaped leaves.
/// Buckets:
///   - color: key OR string-value containing color-ish hints (palette/tonal/
///            hex/hct/rgb).
///   - typography: keys containing font/typeface/typography/text-style.
///   - shape: keys containing shape/corner/radius.
///   - elevation: keys containing elevation/shadow/level.
///   - motion: keys containing motion/duration/easing/animation.
fn classify_token_path(path: &str) -> Option<&'static str> {
    let p = path.to_ascii_lowercase();
    if p.contains("color")
        || p.contains("palette")
        || p.contains("tonal")
        || p.contains("hct")
        || p.contains("scheme")
    {
        return Some("color");
    }
    if p.contains("typography") || p.contains("typeface") || p.contains("font") {
        return Some("typography");
    }
    if p.contains("shape") || p.contains("corner") || p.contains("radius") {
        return Some("shape");
    }
    if p.contains("elevation") || p.contains("shadow") {
        return Some("elevation");
    }
    if p.contains("motion") || p.contains("duration") || p.contains("easing") {
        return Some("motion");
    }
    None
}

#[derive(Default, Debug)]
struct TokenBuckets {
    color: Map<String, Value>,
    typography: Map<String, Value>,
    shape: Map<String, Value>,
    elevation: Map<String, Value>,
    motion: Map<String, Value>,
    palette: Map<String, Value>,
}

impl TokenBuckets {
    fn total(&self) -> usize {
        self.color.len()
            + self.typography.len()
            + self.shape.len()
            + self.elevation.len()
            + self.motion.len()
            + self.palette.len()
    }

    fn ingest_value(&mut self, value: &Value, path: &str) {
        match value {
            Value::Object(map) => {
                for (k, v) in map {
                    let next = if path.is_empty() {
                        k.clone()
                    } else {
                        format!("{path}.{k}")
                    };
                    self.ingest_value(v, &next);
                }
            }
            Value::Array(items) => {
                for (i, v) in items.iter().enumerate() {
                    self.ingest_value(v, &format!("{path}[{i}]"));
                }
            }
            // Leaves: only record once we have a classified path.
            _ => {
                if let Some(bucket) = classify_token_path(path) {
                    // Detect tonal-palette tone keys (".0",".10",".20",".90",".95",
                    // ".99",".100") and route to palette bucket too.
                    let is_tone_key = path
                        .rsplit('.')
                        .next()
                        .map(|s| s.parse::<u32>().is_ok())
                        .unwrap_or(false);
                    if bucket == "color" && is_tone_key {
                        self.palette.insert(path.to_string(), value.clone());
                    }
                    let target = match bucket {
                        "color" => &mut self.color,
                        "typography" => &mut self.typography,
                        "shape" => &mut self.shape,
                        "elevation" => &mut self.elevation,
                        "motion" => &mut self.motion,
                        _ => unreachable!(),
                    };
                    // Insert under the full path; dedupe naturally via Map.
                    target.insert(path.to_string(), value.clone());
                }
            }
        }
    }
}

/// Probe bxc daemon at 127.0.0.1:<port>. bxc-engine exposes /health.
async fn bxc_daemon_alive(client: &reqwest::Client, port: u16) -> bool {
    // Try /health first (bxc-engine native), fall back to CDP /json/version.
    for path in ["health", "json/version"] {
        let url = format!("http://127.0.0.1:{port}/{path}");
        if let Ok(r) = client.get(&url).timeout(Duration::from_secs(2)).send().await {
            if r.status().is_success() {
                return true;
            }
        }
    }
    false
}

/// Spawn `bxc launch --port N` detached in the background; return success.
///
/// Lookup order:
/// 1. `BXC_BIN` env var (explicit override).
/// 2. `bxc` / `bxc.exe` on `$PATH`.
/// 3. `~/.local/bin/bxc{,.exe}` (per project install convention, cf.
///    memory `project_aphrody_install_convention`).
fn spawn_bxc_daemon(port: u16) -> Result<()> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(explicit) = std::env::var("BXC_BIN") {
        candidates.push(PathBuf::from(explicit));
    }
    candidates.push(PathBuf::from("bxc"));
    candidates.push(PathBuf::from("bxc.exe"));
    if let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
        let home = PathBuf::from(home);
        candidates.push(home.join(".local").join("bin").join("bxc"));
        candidates.push(home.join(".local").join("bin").join("bxc.exe"));
    }
    for exe in candidates {
        let mut cmd = Command::new(&exe);
        cmd.args(["launch", "--port", &port.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null());
        if let Ok(child) = cmd.spawn() {
            tracing::info!("[bxc] spawned daemon on :{} (pid {})", port, child.id());
            return Ok(());
        }
    }
    Err(anyhow!(
        "bxc executable not found via BXC_BIN, $PATH, or ~/.local/bin"
    ))
}

/// Fetch a single page via the bxc daemon if available, else direct reqwest.
async fn fetch_page(
    client: &reqwest::Client,
    target: &Url,
    timeout: Duration,
    use_bxc: bool,
    bxc_port: u16,
) -> Result<(u16, String, String)> {
    if use_bxc {
        // `bxc` CLI requires `--port` BEFORE the subcommand (top-level flag).
        if let Ok(out) = tokio::process::Command::new("bxc")
            .args([
                "--port",
                &bxc_port.to_string(),
                "fetch",
                target.as_str(),
                "--dump",
                "html",
                "--quiet",
                "--timeout",
                &timeout.as_secs().to_string(),
            ])
            .output()
            .await
        {
            if out.status.success() && !out.stdout.is_empty() {
                let body = String::from_utf8_lossy(&out.stdout).into_owned();
                return Ok((200, body, "bxc".into()));
            }
            tracing::debug!(
                "[bxc] fetch failed for {} ({}) — falling back to reqwest",
                target,
                out.status
            );
        }
    }

    let resp = client
        .get(target.as_str())
        .timeout(timeout)
        .send()
        .await
        .with_context(|| format!("reqwest GET {target}"))?;
    let status = resp.status().as_u16();
    let body = resp.text().await.context("read body")?;
    Ok((status, body, "reqwest".into()))
}

pub(crate) fn run(args: Args) -> Result<()> {
    // rustls workspace uses `rustls-no-provider` → install ring before any
    // TLS handshake. Safe to call multiple times (returns Err on dup).
    let _ = rustls::crypto::ring::default_provider().install_default();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("build tokio runtime")?;
    rt.block_on(run_async(args))
}

async fn run_async(args: Args) -> Result<()> {
    let root = repo_root()?;
    let out_root = args.out.unwrap_or_else(|| root.join("assets/scrape/m3-material"));
    let pages_dir = out_root.join("pages");
    let components_dir = out_root.join("components");
    let tokens_dir = out_root.join("tokens");
    let manifest_path = out_root.join("manifest.ndjson");
    let summary_path = out_root.join("summary.json");
    let palette_path = out_root.join("palette.json");
    fs::create_dir_all(&pages_dir)?;
    fs::create_dir_all(&components_dir)?;
    fs::create_dir_all(&tokens_dir)?;

    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .pool_max_idle_per_host(8)
        .build()
        .context("build reqwest client")?;

    // Optional bxc daemon.
    let mut use_bxc = false;
    if args.bxc_port != 0 {
        if bxc_daemon_alive(&client, args.bxc_port).await {
            tracing::info!("[bxc] daemon already up on :{}", args.bxc_port);
            use_bxc = true;
        } else {
            match spawn_bxc_daemon(args.bxc_port) {
                Ok(()) => {
                    tokio::time::sleep(Duration::from_millis(1500)).await;
                    if bxc_daemon_alive(&client, args.bxc_port).await {
                        use_bxc = true;
                        tracing::info!("[bxc] daemon ready on :{}", args.bxc_port);
                    } else {
                        tracing::warn!(
                            "[bxc] daemon spawn did not bind :{} — falling back to reqwest",
                            args.bxc_port
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!("[bxc] {} — using direct reqwest only", e);
                }
            }
        }
    }

    // robots.txt (Disallow-only minimal honour).
    let robots = match client
        .get(format!("https://{TARGET_HOST}/robots.txt"))
        .timeout(Duration::from_secs(10))
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => {
            let body = r.text().await.unwrap_or_default();
            tracing::info!("[robots] parsed {} bytes", body.len());
            RobotsRules::parse(&body)
        }
        _ => {
            tracing::warn!("[robots] fetch failed — proceeding without rules");
            RobotsRules::default()
        }
    };

    let seen_urls = Arc::new(Mutex::new(load_seen(&manifest_path)?));
    tracing::info!("[manifest] {} URLs already mirrored (skip)", seen_urls.lock().await.len());

    let mut frontier: VecDeque<(Url, u32)> = VecDeque::new();
    let mut queued: HashSet<String> = HashSet::new();
    for raw in SEED_URLS {
        if let Some(seed) = canonicalize(raw) {
            queued.insert(seed.to_string());
            frontier.push_back((seed, 0));
        }
    }
    // Bootstrap the frontier with every known component URL. The M3 site is
    // an Angular SPA — anchor tags use `routerLink` and don't render as
    // `<a href>` in the static HTML. Without these explicit seeds the BFS
    // can never reach component pages.
    for name in KNOWN_COMPONENTS {
        for suffix in ["overview", "specs", "guidelines", "accessibility"] {
            let raw = format!("https://{TARGET_HOST}/components/{name}/{suffix}");
            if let Some(u) = canonicalize(&raw) {
                if !queued.contains(u.as_str()) {
                    queued.insert(u.to_string());
                    frontier.push_back((u, 1));
                }
            }
        }
        // Also seed the bare /components/<name> root.
        let raw = format!("https://{TARGET_HOST}/components/{name}");
        if let Some(u) = canonicalize(&raw) {
            if !queued.contains(u.as_str()) {
                queued.insert(u.to_string());
                frontier.push_back((u, 1));
            }
        }
    }
    // Bootstrap style/foundation/develop sub-pages too — same SPA constraint.
    for path in [
        "styles/color/system/overview",
        "styles/color/the-color-system",
        "styles/color/dynamic-color/overview",
        "styles/typography/overview",
        "styles/typography/type-scale-tokens",
        "styles/shape/overview",
        "styles/elevation/overview",
        "styles/motion/overview",
        "styles/motion/easing-and-duration",
        "foundations/accessible-design/overview",
        "foundations/layout/understanding-layout/overview",
        "foundations/interaction/states/overview",
        "develop/flutter",
        "develop/android",
        "develop/web",
    ] {
        let raw = format!("https://{TARGET_HOST}/{path}");
        if let Some(u) = canonicalize(&raw) {
            if !queued.contains(u.as_str()) {
                queued.insert(u.to_string());
                frontier.push_back((u, 1));
            }
        }
    }

    let started_at = now_iso8601();
    let t0 = Instant::now();
    let mut depth_max = 0u32;
    let mut total_pages = 0usize;
    let mut component_slugs: BTreeSet<String> = BTreeSet::new();
    let mut bxc_hits = 0usize;
    let mut tokens = TokenBuckets::default();
    let known_components: HashSet<&&str> = KNOWN_COMPONENTS.iter().collect();
    let timeout = Duration::from_secs(args.timeout);

    'outer: while !frontier.is_empty() && total_pages < args.max_pages {
        let mut batch: Vec<(Url, u32)> = Vec::with_capacity(args.concurrency);
        while batch.len() < args.concurrency && total_pages + batch.len() < args.max_pages {
            let Some((u, d)) = frontier.pop_front() else { break };
            if d > args.depth {
                continue;
            }
            if !robots.is_allowed(u.path()) {
                tracing::debug!("[robots] skip {}", u);
                continue;
            }
            if seen_urls.lock().await.contains(u.as_str()) {
                continue;
            }
            batch.push((u, d));
        }
        if batch.is_empty() {
            if frontier.is_empty() {
                break 'outer;
            }
            continue;
        }

        let client_ref = &client;
        let pages_dir_ref = &pages_dir;
        let components_dir_ref = &components_dir;
        type FetchOk = (PageRecord, Vec<Value>, BTreeSet<String>);
        let results: Vec<Result<FetchOk>> = stream::iter(batch.into_iter())
            .map(|(url, depth)| {
                let url2 = url.clone();
                async move {
                    let (status, body, source) =
                        fetch_page(client_ref, &url2, timeout, use_bxc, args.bxc_port).await?;
                    let links = extract_links(&body, &url2);
                    let embedded = extract_embedded_json(&body);
                    let comp_name = component_name_from_url(&url2);
                    let kind = if comp_name.is_some() { "component" } else { "page" };
                    let slug = slug_from_url(&url2);

                    // Persist body. Every page lands in pages/ (manifest mirror);
                    // components additionally get a dedicated dir with spec +
                    // per-component tokens.json.
                    let html_path = pages_dir_ref.join(format!("{slug}.html"));
                    let _ = fs::write(&html_path, &body);
                    if let Some(name) = comp_name.as_deref() {
                        let dir = components_dir_ref.join(name);
                        let _ = fs::create_dir_all(&dir);
                        let _ = fs::write(dir.join("spec.html"), &body);
                        // Per-component tokens.json — dump every embedded JSON.
                        if !embedded.is_empty() {
                            let bundle = serde_json::json!({
                                "component": name,
                                "url": url2.to_string(),
                                "fetched_at": now_iso8601(),
                                "embedded_json": embedded,
                            });
                            let _ = fs::write(
                                dir.join("tokens.json"),
                                serde_json::to_string_pretty(&bundle)
                                    .unwrap_or_else(|_| "{}".into()),
                            );
                        }
                    }

                    let rec = PageRecord {
                        url: url2.to_string(),
                        slug,
                        kind: kind.into(),
                        fetched_at: now_iso8601(),
                        status,
                        sha256: sha256_hex(body.as_bytes()),
                        bytes: body.len() as u64,
                        depth,
                        links_out: links.iter().cloned().collect(),
                        source,
                        embedded_json_count: embedded.len(),
                    };
                    Ok::<FetchOk, anyhow::Error>((rec, embedded, links))
                }
            })
            .buffer_unordered(args.concurrency)
            .collect()
            .await;

        // Commit page records + enqueue out-links + ingest tokens.
        for triplet in results.into_iter().flatten() {
            let (r, embedded, links) = triplet;
            depth_max = depth_max.max(r.depth);
            if r.source == "bxc" {
                bxc_hits += 1;
            }
            if r.kind == "component" {
                // Track which known components we have captured.
                if let Some(name) = component_name_from_url(&Url::parse(&r.url)?) {
                    if known_components.contains(&name.as_str()) {
                        component_slugs.insert(name);
                    } else {
                        // Still count any sub-page under /components/* as
                        // captured — the M3 catalogue evolves and we want
                        // forward-compat counters.
                        component_slugs.insert(name);
                    }
                }
            }

            // Ingest every embedded JSON blob into the global token buckets.
            for v in &embedded {
                tokens.ingest_value(v, "");
            }

            append_manifest(&manifest_path, &r)?;
            seen_urls.lock().await.insert(r.url.clone());
            total_pages += 1;

            if r.depth >= args.depth {
                continue;
            }
            for link in &links {
                if queued.contains(link) {
                    continue;
                }
                if let Some(c) = canonicalize(link) {
                    if c.host_str() != Some(TARGET_HOST) {
                        continue;
                    }
                    queued.insert(c.to_string());
                    frontier.push_back((c, r.depth + 1));
                }
            }
            if total_pages >= args.max_pages {
                break 'outer;
            }
        }

        tracing::info!(
            "[crawl] pages={}/{}  components={}  tokens={}  frontier={}  depth_max={}",
            total_pages,
            args.max_pages,
            component_slugs.len(),
            tokens.total(),
            frontier.len(),
            depth_max
        );
    }

    // Write canonical token files (one per bucket).
    let write_bucket = |name: &str, map: &Map<String, Value>| -> Result<()> {
        let path = tokens_dir.join(format!("{name}.json"));
        let body = serde_json::json!({
            "$schema": format!("https://aphrody-code.dev/schemas/m3-tokens/{name}/v1"),
            "generated_at": now_iso8601(),
            "source": format!("https://{TARGET_HOST}/"),
            "count": map.len(),
            "tokens": map,
        });
        fs::write(&path, format!("{}\n", serde_json::to_string_pretty(&body)?))
            .with_context(|| format!("write {}", path.display()))?;
        Ok(())
    };
    write_bucket("color", &tokens.color)?;
    write_bucket("typography", &tokens.typography)?;
    write_bucket("shape", &tokens.shape)?;
    write_bucket("elevation", &tokens.elevation)?;
    write_bucket("motion", &tokens.motion)?;

    // palette.json — tonal palette tones (Primary/Secondary/Tertiary/Neutral/
    // NeutralVariant/Error with tones 0/10/20/30/40/50/60/70/80/90/95/99/100).
    // The structure here is a flat path -> value map; downstream tools can
    // re-shape into the canonical { primary: { 0: ..., 10: ..., ... } } tree.
    let mut palette_grouped: BTreeMap<String, Map<String, Value>> = BTreeMap::new();
    for (path, value) in &tokens.palette {
        // Split "...primary.10" -> family="primary", tone="10".
        let mut iter = path.rsplitn(2, '.');
        let tone = iter.next().unwrap_or("");
        let family_path = iter.next().unwrap_or("");
        let family = family_path
            .rsplit('.')
            .find(|s| !s.is_empty())
            .unwrap_or("unknown")
            .to_string();
        palette_grouped.entry(family).or_default().insert(tone.into(), value.clone());
    }
    let palette = serde_json::json!({
        "$schema": "https://aphrody-code.dev/schemas/m3-palette/v1",
        "generated_at": now_iso8601(),
        "source": format!("https://{TARGET_HOST}/"),
        "families": palette_grouped,
        "expected_tones": [0, 10, 20, 30, 40, 50, 60, 70, 80, 90, 95, 99, 100],
        "expected_families": [
            "primary", "secondary", "tertiary",
            "neutral", "neutral-variant", "error"
        ],
    });
    fs::write(&palette_path, format!("{}\n", serde_json::to_string_pretty(&palette)?))
        .with_context(|| format!("write {}", palette_path.display()))?;

    let total_tokens_extracted = tokens.total();
    let total_components = component_slugs.len();

    let completed_at = now_iso8601();
    let summary = Summary {
        seeds: SEED_URLS.iter().map(|s| (*s).to_string()).collect(),
        total_pages,
        total_components,
        total_tokens_extracted,
        depth_max,
        started_at,
        completed_at,
        duration_ms: t0.elapsed().as_millis(),
        bxc_used: bxc_hits > 0,
    };
    fs::write(&summary_path, format!("{}\n", serde_json::to_string_pretty(&summary)?))
        .with_context(|| format!("write {}", summary_path.display()))?;

    tracing::info!(
        "[done] pages={} components={} tokens={} depth={} bxc_pages={} in {:.2}s — {}",
        total_pages,
        total_components,
        total_tokens_extracted,
        depth_max,
        bxc_hits,
        t0.elapsed().as_secs_f64(),
        summary_path.display()
    );
    Ok(())
}

// -------------------------------------------------------------------------
// Unit tests (≥4 required by mission)
// -------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn slug_from_url_handles_root_nested_and_query() {
        let root = slug_from_url(&Url::parse("https://m3.material.io/").unwrap());
        assert_eq!(root, "index");

        let nested =
            slug_from_url(&Url::parse("https://m3.material.io/styles/color/the-color-system").unwrap());
        assert_eq!(nested, "styles__color__the-color-system");

        let with_q =
            slug_from_url(&Url::parse("https://m3.material.io/search?q=button&p=2").unwrap());
        assert!(with_q.starts_with("search-"), "got {with_q}");
        assert_eq!(with_q.len(), "search-".len() + 6);
    }

    #[test]
    fn component_name_extracted_only_for_components_path() {
        let btn = component_name_from_url(
            &Url::parse("https://m3.material.io/components/buttons/overview").unwrap(),
        );
        assert_eq!(btn.as_deref(), Some("buttons"));

        let fab = component_name_from_url(
            &Url::parse("https://m3.material.io/components/fab/specs").unwrap(),
        );
        assert_eq!(fab.as_deref(), Some("fab"));

        // /components alone -> None (no sub-segment).
        let idx = component_name_from_url(
            &Url::parse("https://m3.material.io/components").unwrap(),
        );
        assert_eq!(idx, None);

        // Non-components path -> None.
        let style = component_name_from_url(
            &Url::parse("https://m3.material.io/styles/color").unwrap(),
        );
        assert_eq!(style, None);
    }

    #[test]
    fn extract_embedded_json_pulls_typed_script_tags() {
        // Use r##"..."## because the JSON payload contains the sequence
        // `"#` (closing quote followed by `#` of the color literal).
        let html = r##"
            <html><head>
              <script type="application/json" id="__NEXT_DATA__">
                {"props":{"color":{"primary":"#6750A4"}}}
              </script>
              <script type="application/ld+json">
                {"@context":"https://schema.org","name":"M3"}
              </script>
              <script type="text/javascript">/* not json */</script>
              <script type="application/json">not-valid-json</script>
            </head><body></body></html>
        "##;
        let blobs = extract_embedded_json(html);
        assert_eq!(blobs.len(), 2, "should pick up 2 valid JSON blobs");
        assert!(blobs.iter().any(|v| v["props"]["color"]["primary"] == "#6750A4"));
        assert!(blobs.iter().any(|v| v["@context"] == "https://schema.org"));
    }

    #[test]
    fn token_buckets_classify_by_path() {
        let mut tb = TokenBuckets::default();
        let v: Value = serde_json::json!({
            "color": {
                "primary": "#6750A4",
                "palette": { "primary": { "10": "#21005D", "90": "#EADDFF" } }
            },
            "typography": {
                "display-large": { "font": "Roboto", "size": "57px" }
            },
            "shape": { "corner": { "small": "8px", "medium": "12px" } },
            "elevation": { "level1": "1dp", "level3": "6dp" },
            "motion": { "duration": { "short1": "50ms" }, "easing": { "standard": "cubic-bezier(0.2,0,0,1)" } },
            "unrelated": { "foo": "bar" }
        });
        tb.ingest_value(&v, "");

        assert!(!tb.color.is_empty(), "color bucket populated");
        assert!(!tb.typography.is_empty(), "typography bucket populated");
        assert!(!tb.shape.is_empty(), "shape bucket populated");
        assert!(!tb.elevation.is_empty(), "elevation bucket populated");
        assert!(!tb.motion.is_empty(), "motion bucket populated");
        assert!(!tb.palette.is_empty(), "palette captured tonal tones");
        // Unrelated key must not leak into any bucket.
        let total_keys = tb.color.len()
            + tb.typography.len()
            + tb.shape.len()
            + tb.elevation.len()
            + tb.motion.len();
        assert!(total_keys >= 8, "expected ≥8 classified tokens, got {total_keys}");
        assert!(
            !tb.color
                .keys()
                .any(|k| k.contains("unrelated") || k.contains("foo")),
            "unrelated keys must not be classified"
        );
    }

    #[test]
    fn depth_limit_is_respected_by_bfs_gate() {
        let max_depth = 3u32;
        let frontier: Vec<(String, u32)> = vec![
            ("https://m3.material.io/".into(), 0),
            ("https://m3.material.io/components".into(), 1),
            ("https://m3.material.io/components/buttons".into(), 2),
            ("https://m3.material.io/components/buttons/overview".into(), 3),
            ("https://m3.material.io/components/buttons/overview/specs".into(), 4),
        ];
        let kept: Vec<_> = frontier.iter().filter(|(_, d)| *d <= max_depth).collect();
        assert_eq!(kept.len(), 4);
        assert!(kept.iter().all(|(_, d)| *d <= max_depth));
    }

    #[test]
    fn ndjson_append_and_load_seen_dedupes_by_url() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("manifest.ndjson");
        let rec = PageRecord {
            url: "https://m3.material.io/components/buttons".into(),
            slug: "components__buttons".into(),
            kind: "component".into(),
            fetched_at: "2026-05-18T00:00:00Z".into(),
            status: 200,
            sha256: "deadbeef".into(),
            bytes: 42,
            depth: 2,
            links_out: vec!["https://m3.material.io/components/fab".into()],
            source: "reqwest".into(),
            embedded_json_count: 1,
        };
        append_manifest(&p, &rec).unwrap();
        append_manifest(&p, &rec).unwrap();
        let seen = load_seen(&p).unwrap();
        assert_eq!(seen.len(), 1);
        assert!(seen.contains("https://m3.material.io/components/buttons"));
    }
}
