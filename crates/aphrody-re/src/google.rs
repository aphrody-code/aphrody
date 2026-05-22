// SPDX-License-Identifier: Apache-2.0
//! Google binary analyser — identifies Electron / WebView2 / Chromium / Go /
//! Node / V8 artefacts and extracts OAuth client IDs, API endpoints, updater
//! URLs, and code-signing hints from raw binary bytes.
//!
//! All detection is performed on the raw byte slice via [`extract_strings`]
//! (from the parent crate) combined with byte-pattern scanning using
//! `memchr::memmem` for needle searches and `regex` for structured patterns.
//! No additional crates are introduced beyond those already in the workspace.
//!
//! # Example
//!
//! ```
//! use aphrody_re::google::{analyze_google, BinaryFamily};
//!
//! // Minimal buffer that looks like an Electron binary.
//! let mut buf = b"MZ".to_vec();
//! buf.extend_from_slice(&[0u8; 62]);
//! buf.extend_from_slice(b"electron\x00app.asar\x00chrome_100_percent.pak\x00");
//! let report = analyze_google(&buf);
//! assert_eq!(report.family, BinaryFamily::Electron);
//! ```

use memchr::memmem;
use regex::bytes::Regex;
use serde::Serialize;

use crate::extract_strings;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Broad classification of the Google binary under analysis.
///
/// Detection order (highest-priority first):
/// `Electron` → `WebView2` → `Chromium` → `GoBinary` → `NodeBundle` →
/// `V8Snapshot` → `Generic`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BinaryFamily {
    /// Electron application (ships `app.asar` / `chrome_100_percent.pak` /
    /// the string `"electron"`).
    Electron,
    /// Microsoft Edge WebView2 host application — embeds the Evergreen
    /// WebView2 runtime (`msedgewebview2`, `WebView2Loader`,
    /// `CoreWebView2`, `EBWebView` user-data folder). Distinct from a raw
    /// Chromium browser: the app hosts web content via the Edge runtime
    /// rather than shipping its own Chromium. This is the family of the
    /// `google.exe` desktop launcher (user-data under
    /// `…\Google\latest\default\WebView2\EBWebView`).
    WebView2,
    /// Chromium / Chrome browser binary (`Chrome/`, `chrome.dll`, `Crashpad`).
    Chromium,
    /// Go-compiled binary (`.gopclntab` section / `go:buildid` / `Go build ID:`).
    GoBinary,
    /// Node.js bundled executable (`require(` / `node:internal`).
    NodeBundle,
    /// V8 snapshot blob (`v8_context_snapshot` / `snapshot_blob`).
    V8Snapshot,
    /// Could not match any of the above families.
    Generic,
}

/// Full analysis report for a Google binary.
///
/// Serialises to JSON — all `Vec` fields are always present (never `null`),
/// `Option` fields are `null` when not found.
#[derive(Debug, Clone, Serialize)]
pub struct GoogleReport {
    /// Detected binary family.
    pub family: BinaryFamily,
    /// Chromium version string (`"M.m.b.p"` form), if present.
    pub chromium_version: Option<String>,
    /// Google OAuth2 client IDs found in the binary
    /// (`\d+-[a-z0-9]+\.apps\.googleusercontent\.com`).
    pub oauth_client_ids: Vec<String>,
    /// Google API / CDN / service hosts found
    /// (`*.googleapis.com`, `*.google.com`, `*.gstatic.com`,
    ///  `*.googleusercontent.com`, `*.run.app`).
    pub google_endpoints: Vec<String>,
    /// URLs referencing an auto-updater service or Cloud Run.
    pub updater_urls: Vec<String>,
    /// Code-signing subject hint (`"Google LLC"` or `"Google Inc"`), if the
    /// certificate chain or any DER-embedded string is detected.
    pub code_sign_subject: Option<String>,
    /// Fully-qualified protobuf gRPC service names discovered in a Go binary
    /// (`exa.<package>_pb.<Name>Service`) plus Codeium proto package symbols
    /// (`codeium_common_go_proto`, `exa.<package>_pb`). Sorted + deduped.
    /// Always present (possibly empty).
    pub grpc_services: Vec<String>,
    /// gRPC RPC method names extracted from embedded service paths
    /// (`/exa.<package>_pb.<Name>Service/<Method>`) and from a conservative
    /// verb-noun allowlist of identifiers found in the string corpus. Sorted +
    /// deduped. Always present (possibly empty).
    pub grpc_methods: Vec<String>,
    /// Human-readable list of detection signals used to classify the binary.
    pub indicators: Vec<String>,
}

// ---------------------------------------------------------------------------
// Main public entry point
// ---------------------------------------------------------------------------

/// Default string-corpus extraction limit for [`analyze_google`].
///
/// At 8 192 entries the corpus is large enough to find all OAuth IDs /
/// endpoints in typical small binaries while remaining fast on host builds
/// with regular-sized binaries (< 20 MB). For large Go sidecars (> 100 MB,
/// e.g. `language_server_*.exe` at ~133 MB) the corpus traversal dominates
/// latency — use [`analyze_google_bounded`] with a smaller limit.
pub const GOOGLE_STRINGS_LIMIT_DEFAULT: usize = 8_192;

/// Bounded string-corpus limit used by `aphrody ide re` for the Go language
/// server sidecar (~133 MB). 512 entries is sufficient to capture all
/// `exa.*` service paths, OAuth IDs, and Cloud endpoints embedded near the
/// binary's string-rich regions while completing in < 200 ms on both Windows
/// and Linux.
pub const GOOGLE_STRINGS_LIMIT_SIDECAR: usize = 512;

/// Analyse a raw binary blob for Google-specific artefacts.
///
/// Uses [`GOOGLE_STRINGS_LIMIT_DEFAULT`] for the string corpus extraction.
/// For large binaries (> 50 MB) prefer [`analyze_google_bounded`] with a
/// smaller limit to keep latency bounded.
///
/// Never panics on arbitrary input; all regex / memchr calls are applied to
/// controlled byte ranges only.
///
/// # Example
///
/// ```
/// use aphrody_re::google::{analyze_google, BinaryFamily};
///
/// // Buffer containing a Go build-id marker.
/// let buf = b"some bytes go:buildid more bytes";
/// let r = analyze_google(buf);
/// assert_eq!(r.family, BinaryFamily::GoBinary);
/// assert!(r.indicators.iter().any(|s| s.contains("go:buildid")));
/// ```
#[must_use]
pub fn analyze_google(bytes: &[u8]) -> GoogleReport {
    analyze_google_bounded(bytes, GOOGLE_STRINGS_LIMIT_DEFAULT)
}

/// Analyse a raw binary blob for Google-specific artefacts with an explicit
/// string-corpus extraction limit.
///
/// `strings_limit` caps the number of ASCII / UTF-16LE strings extracted from
/// `bytes` for the regex-based passes (OAuth IDs, endpoints, updater URLs, gRPC
/// verb-noun allowlist). The byte-level needle passes (family detection, gRPC
/// routing paths, Chromium version, code-sign subject) always scan the **full**
/// byte slice regardless of this limit, so the critical structured data is never
/// missed.
///
/// Use [`GOOGLE_STRINGS_LIMIT_SIDECAR`] (512) for the Antigravity / Codeium
/// `language_server_*.exe` (~133 MB) to keep end-to-end latency under 200 ms.
///
/// # Example
///
/// ```
/// use aphrody_re::google::{analyze_google_bounded, BinaryFamily, GOOGLE_STRINGS_LIMIT_SIDECAR};
///
/// let buf = b"runtime.goexit\x00go:buildid\x00";
/// let r = analyze_google_bounded(buf, GOOGLE_STRINGS_LIMIT_SIDECAR);
/// assert_eq!(r.family, BinaryFamily::GoBinary);
/// ```
#[must_use]
pub fn analyze_google_bounded(bytes: &[u8], strings_limit: usize) -> GoogleReport {
    let mut indicators: Vec<String> = Vec::new();

    // --- 1. Extract the string corpus (ASCII + UTF-16LE) once; everything
    //         below operates on this corpus + raw byte needles.
    //         Byte-level needle passes (family, Chromium version, code-sign,
    //         gRPC routing paths) always scan the full slice — not bounded.
    let strings = extract_strings(bytes, 4, strings_limit);

    // --- 2. Detect BinaryFamily via byte-level needle search -----------------
    let family = detect_family(bytes, &mut indicators);

    // --- 3. Chromium version — `Chrome/M.m.b.p` in the raw bytes ------------
    let chromium_version = extract_chromium_version(bytes, &mut indicators);

    // --- 4. OAuth client IDs — run regex over the string corpus -------------
    let oauth_client_ids = extract_oauth_ids(&strings, &mut indicators);

    // --- 5. Google endpoints — regex over string corpus ---------------------
    let google_endpoints = extract_google_endpoints(&strings, &mut indicators);

    // --- 6. Updater URLs — subset of strings containing updater keywords ----
    let updater_urls = extract_updater_urls(&strings, &mut indicators);

    // --- 7. Code-signing subject — best-effort byte scan --------------------
    let code_sign_subject = detect_code_sign_subject(bytes, &mut indicators);

    // --- 8. gRPC `exa.*` services + RPC methods (Go binaries: Codeium LS) ----
    let (grpc_services, grpc_methods) =
        extract_grpc_surface(bytes, &strings, &mut indicators);

    GoogleReport {
        family,
        chromium_version,
        oauth_client_ids,
        google_endpoints,
        updater_urls,
        code_sign_subject,
        grpc_services,
        grpc_methods,
        indicators,
    }
}

// ---------------------------------------------------------------------------
// Family detection
// ---------------------------------------------------------------------------

fn detect_family(bytes: &[u8], indicators: &mut Vec<String>) -> BinaryFamily {
    // Needles are ordered by specificity; first match wins.

    // Electron: ships `app.asar` packager artefact or the `electron` string,
    // and always embeds a Chromium PAK resource.
    let electron_needles: &[&[u8]] = &[
        b"app.asar",
        b"electron",
        b"chrome_100_percent.pak",
        b"ELECTRON_RUN_AS_NODE",
    ];
    for needle in electron_needles {
        if memmem::find(bytes, needle).is_some() {
            indicators.push(format!(
                "Electron marker: {}",
                std::str::from_utf8(needle).unwrap_or("<binary>")
            ));
        }
    }
    if indicators.iter().any(|s| s.starts_with("Electron")) {
        return BinaryFamily::Electron;
    }

    // WebView2: Microsoft Edge Evergreen runtime host. Checked before plain
    // Chromium because a WebView2 host also carries `Chrome/` user-agent and
    // `Crashpad` strings — these markers are more specific and win the tie.
    let webview2_needles: &[&[u8]] = &[
        b"msedgewebview2",
        b"WebView2Loader",
        b"CoreWebView2",
        b"EmbeddedBrowserWebView",
        b"EBWebView",
        b"Microsoft.Web.WebView2",
        b"WEBVIEW2_USER_DATA_FOLDER",
        b"WEBVIEW2_BROWSER_EXECUTABLE_FOLDER",
    ];
    for needle in webview2_needles {
        if memmem::find(bytes, needle).is_some() {
            indicators.push(format!(
                "WebView2 marker: {}",
                std::str::from_utf8(needle).unwrap_or("<binary>")
            ));
        }
    }
    if indicators.iter().any(|s| s.starts_with("WebView2")) {
        return BinaryFamily::WebView2;
    }

    // Chromium: core browser or headless shell.
    let chromium_needles: &[&[u8]] = &[
        b"Chrome/",
        b"chrome.dll",
        b"Crashpad",
        b"ChromeDriver",
        b"HeadlessChrome",
    ];
    for needle in chromium_needles {
        if memmem::find(bytes, needle).is_some() {
            indicators.push(format!(
                "Chromium marker: {}",
                std::str::from_utf8(needle).unwrap_or("<binary>")
            ));
        }
    }
    if indicators.iter().any(|s| s.starts_with("Chromium")) {
        return BinaryFamily::Chromium;
    }

    // Go binary: build-id tag injected by the Go linker, or the PC-line table
    // section name embedded in the binary.
    let go_needles: &[&[u8]] = &[b"go:buildid", b"Go build ID:", b".gopclntab", b"runtime.goexit"];
    for needle in go_needles {
        if memmem::find(bytes, needle).is_some() {
            indicators.push(format!(
                "Go marker: {}",
                std::str::from_utf8(needle).unwrap_or("<binary>")
            ));
        }
    }
    if indicators.iter().any(|s| s.starts_with("Go")) {
        return BinaryFamily::GoBinary;
    }

    // Node.js bundled binary.
    let node_needles: &[&[u8]] = &[b"node:internal", b"require(", b"NODE_PATH", b"node_modules"];
    for needle in node_needles {
        if memmem::find(bytes, needle).is_some() {
            indicators.push(format!(
                "Node marker: {}",
                std::str::from_utf8(needle).unwrap_or("<binary>")
            ));
        }
    }
    if indicators.iter().any(|s| s.starts_with("Node")) {
        return BinaryFamily::NodeBundle;
    }

    // V8 snapshot blob.
    let v8_needles: &[&[u8]] = &[b"v8_context_snapshot", b"snapshot_blob", b"V8_SNAPSHOT"];
    for needle in v8_needles {
        if memmem::find(bytes, needle).is_some() {
            indicators.push(format!(
                "V8Snapshot marker: {}",
                std::str::from_utf8(needle).unwrap_or("<binary>")
            ));
        }
    }
    if indicators.iter().any(|s| s.starts_with("V8Snapshot")) {
        return BinaryFamily::V8Snapshot;
    }

    BinaryFamily::Generic
}

// ---------------------------------------------------------------------------
// Chromium version
// ---------------------------------------------------------------------------

fn extract_chromium_version(bytes: &[u8], indicators: &mut Vec<String>) -> Option<String> {
    // Pattern: `Chrome/M.m.b.p` (user-agent style).
    // We run the regex directly on raw bytes for efficiency.
    let re = Regex::new(r"Chrome/(\d+\.\d+\.\d+\.\d+)").expect("static regex");
    if let Some(cap) = re.captures(bytes) {
        if let Some(ver_bytes) = cap.get(1) {
            if let Ok(ver) = std::str::from_utf8(ver_bytes.as_bytes()) {
                let ver = ver.to_owned();
                indicators.push(format!("Chromium version: {ver}"));
                return Some(ver);
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// OAuth client IDs
// ---------------------------------------------------------------------------

fn extract_oauth_ids(strings: &[String], indicators: &mut Vec<String>) -> Vec<String> {
    // Pattern: `\d+-[a-z0-9]+\.apps\.googleusercontent\.com`
    let re =
        regex::Regex::new(r"\d+-[a-z0-9]+\.apps\.googleusercontent\.com").expect("static regex");
    let mut found: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for s in strings {
        for m in re.find_iter(s) {
            let id = m.as_str().to_owned();
            if seen.insert(id.clone()) {
                indicators.push(format!("OAuth client_id: {id}"));
                found.push(id);
            }
        }
    }
    found
}

// ---------------------------------------------------------------------------
// Google endpoints
// ---------------------------------------------------------------------------

fn extract_google_endpoints(strings: &[String], indicators: &mut Vec<String>) -> Vec<String> {
    // Match hosts for: googleapis.com, google.com, gstatic.com,
    // googleusercontent.com, run.app, googlevideo.com, googletagmanager.com
    let re = regex::Regex::new(
        r#"(?:https?://)?([a-z0-9](?:[a-z0-9\-]{0,61}[a-z0-9])?(?:\.[a-z0-9](?:[a-z0-9\-]{0,61}[a-z0-9])?)*)\.(?:googleapis|google|gstatic|googleusercontent|googlevideo|googletagmanager)\.com(?:/[^\s"'<>]*)?"#
    ).expect("static regex");
    let run_re =
        regex::Regex::new(r#"https?://[a-z0-9\-]+\.run\.app(?:/[^\s"'<>]*)?"#).expect("static regex");

    let mut found: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for s in strings {
        for m in re.find_iter(s) {
            let ep = m.as_str().to_owned();
            if seen.insert(ep.clone()) {
                indicators.push(format!("Google endpoint: {ep}"));
                found.push(ep);
            }
        }
        for m in run_re.find_iter(s) {
            let ep = m.as_str().to_owned();
            if seen.insert(ep.clone()) {
                indicators.push(format!("Cloud Run endpoint: {ep}"));
                found.push(ep);
            }
        }
    }
    found
}

// ---------------------------------------------------------------------------
// Updater URLs
// ---------------------------------------------------------------------------

fn extract_updater_urls(strings: &[String], indicators: &mut Vec<String>) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for s in strings {
        let lower = s.to_ascii_lowercase();
        if lower.contains("auto-updater")
            || lower.contains("autoupdate")
            || lower.contains("update.googleapis")
            || lower.contains("tools.google.com/service/update")
            || lower.contains("/update2/")
            || lower.contains("omaha")
            || (lower.contains(".run.app") && (lower.starts_with("http://") || lower.starts_with("https://")))
        {
            if seen.insert(s.clone()) {
                indicators.push(format!("Updater URL: {s}"));
                found.push(s.clone());
            }
        }
    }
    found
}

// ---------------------------------------------------------------------------
// Code-signing subject
// ---------------------------------------------------------------------------

fn detect_code_sign_subject(bytes: &[u8], indicators: &mut Vec<String>) -> Option<String> {
    // Best-effort: scan for "Google LLC" or "Google Inc" embedded as ASCII
    // (common in Authenticode certificates, Go version strings, etc.).
    // Full Authenticode parsing (PKCS#7 DER) is left for a future phase.
    let subjects: &[&[u8]] = &[b"Google LLC", b"Google Inc", b"Google Inc."];
    for needle in subjects {
        if memmem::find(bytes, needle).is_some() {
            let subject = std::str::from_utf8(needle)
                .unwrap_or("Google")
                .trim_end_matches('.')
                .to_owned();
            indicators.push(format!("Code-sign subject: {subject}"));
            return Some(subject);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// gRPC `exa.*` services + RPC methods
// ---------------------------------------------------------------------------

/// Extract protobuf gRPC service descriptors and RPC method names from a Go
/// binary such as the Codeium `language_server.exe`.
///
/// Two complementary passes are run:
///
/// 1. **Raw-byte regex** over the whole blob — Go gRPC binaries embed the
///    fully-qualified routing path verbatim
///    (`/exa.<package>_pb.<Name>Service/<Method>`), the package symbol
///    (`exa.<package>_pb`), and the Codeium proto package marker
///    (`codeium_common_go_proto`). Running on raw bytes catches paths even
///    when they straddle the `extract_strings` minimum-length window.
/// 2. **String-corpus allowlist** — conservatively promotes verb-noun
///    CamelCase identifiers (`Get*`, `Fetch*`, `Start*`, `Record*`,
///    `Accept*`, `Auth*`) that appear in the corpus to method candidates.
///    This is gated behind the presence of at least one `exa.*` service so an
///    arbitrary binary full of `GetFoo` symbols does not produce noise.
///
/// Returns `(services, methods)`, both sorted and deduplicated.
fn extract_grpc_surface(
    bytes: &[u8],
    strings: &[String],
    indicators: &mut Vec<String>,
) -> (Vec<String>, Vec<String>) {
    use std::collections::BTreeSet;

    // Fully-qualified service: `exa.<package>_pb.<Name>Service`.
    let service_re = Regex::new(r"\bexa\.[a-z0-9_]+_pb\.[A-Za-z][A-Za-z0-9]*Service\b")
        .expect("static regex");
    // Bare proto package symbol: `exa.<package>_pb` (no trailing `.Service`).
    let package_re = Regex::new(r"\bexa\.[a-z0-9_]+_pb\b").expect("static regex");
    // gRPC routing path: `/exa.<package>_pb.<Name>Service/<Method>`.
    let path_re =
        Regex::new(r"/(exa\.[a-z0-9_]+_pb\.[A-Za-z][A-Za-z0-9]*Service)/([A-Za-z][A-Za-z0-9]*)")
            .expect("static regex");
    // Codeium proto package marker.
    let codeium_marker = b"codeium_common_go_proto";

    let mut services: BTreeSet<String> = BTreeSet::new();
    let mut methods: BTreeSet<String> = BTreeSet::new();

    // --- Pass 1a: routing paths (most authoritative) on raw bytes -----------
    for cap in path_re.captures_iter(bytes) {
        if let (Some(svc), Some(meth)) = (cap.get(1), cap.get(2)) {
            if let (Ok(svc), Ok(meth)) =
                (std::str::from_utf8(svc.as_bytes()), std::str::from_utf8(meth.as_bytes()))
            {
                services.insert(svc.to_owned());
                methods.insert(meth.to_owned());
            }
        }
    }

    // --- Pass 1b: fully-qualified service names on raw bytes ----------------
    for m in service_re.find_iter(bytes) {
        if let Ok(svc) = std::str::from_utf8(m.as_bytes()) {
            services.insert(svc.to_owned());
        }
    }

    // --- Pass 1c: bare `exa.*_pb` package symbols on raw bytes --------------
    for m in package_re.find_iter(bytes) {
        if let Ok(pkg) = std::str::from_utf8(m.as_bytes()) {
            // Skip if this match is actually the prefix of a full service name
            // already recorded — keep only standalone package symbols.
            let pkg = pkg.to_owned();
            if !services.iter().any(|s| s.starts_with(&pkg) && s.len() > pkg.len()) {
                services.insert(pkg);
            }
        }
    }

    // --- Pass 1d: Codeium proto package marker ------------------------------
    if memmem::find(bytes, codeium_marker).is_some() {
        services.insert("codeium_common_go_proto".to_owned());
        indicators.push("gRPC proto package: codeium_common_go_proto".to_owned());
    }

    // --- Pass 2: conservative verb-noun allowlist over the string corpus ----
    // Only mine the corpus for extra methods once we know this binary actually
    // carries an `exa.*` gRPC surface, to keep false positives near zero.
    let has_exa_service = services.iter().any(|s| s.contains("Service"));
    if has_exa_service {
        // `<Verb><Noun>` with at least one capitalised noun segment after the
        // verb (so bare verbs like `Get` alone are rejected).
        let verb_re = Regex::new(
            r"\b(?:Get|Fetch|Start|Record|Accept|Auth)[A-Z][A-Za-z0-9]+\b",
        )
        .expect("static regex");
        for s in strings {
            for m in verb_re.find_iter(s.as_bytes()) {
                if let Ok(name) = std::str::from_utf8(m.as_bytes()) {
                    methods.insert(name.to_owned());
                }
            }
        }
    }

    let services: Vec<String> = services.into_iter().collect();
    let methods: Vec<String> = methods.into_iter().collect();

    for svc in &services {
        indicators.push(format!("gRPC service: {svc}"));
    }
    for meth in &methods {
        indicators.push(format!("gRPC method: {meth}"));
    }

    (services, methods)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Helpers -----------------------------------------------------------------

    fn make_electron_buf() -> Vec<u8> {
        let mut buf = b"FAKE_HEADER_BYTES".to_vec();
        buf.extend_from_slice(b"app.asar\x00electron\x00chrome_100_percent.pak\x00");
        buf.extend_from_slice(b"some_padding_to_make_it_longer\x00");
        buf
    }

    fn make_go_buf() -> Vec<u8> {
        let mut buf = b"FAKE_HEADER_BYTES_GO".to_vec();
        buf.extend_from_slice(b"go:buildid\x00runtime.goexit\x00");
        buf
    }

    // Family tests ------------------------------------------------------------

    #[test]
    fn google_family_electron_detected() {
        let buf = make_electron_buf();
        let r = analyze_google(&buf);
        assert_eq!(r.family, BinaryFamily::Electron, "should detect Electron, got {:?}", r.family);
        assert!(
            r.indicators.iter().any(|s| s.contains("app.asar")),
            "should record app.asar indicator"
        );
    }

    #[test]
    fn google_family_go_binary_detected() {
        let buf = make_go_buf();
        let r = analyze_google(&buf);
        assert_eq!(r.family, BinaryFamily::GoBinary, "should detect GoBinary, got {:?}", r.family);
        assert!(r.indicators.iter().any(|s| s.contains("go:buildid")));
    }

    #[test]
    fn google_family_chromium_detected() {
        let buf = b"Chrome/125.0.6422.141 HeadlessChrome\x00Crashpad\x00".to_vec();
        let r = analyze_google(&buf);
        assert_eq!(r.family, BinaryFamily::Chromium);
        assert!(r.chromium_version.is_some());
        assert_eq!(r.chromium_version.as_deref(), Some("125.0.6422.141"));
    }

    #[test]
    fn google_family_webview2_detected() {
        // A WebView2 host carries both Chromium UA markers AND WebView2-specific
        // strings; the more-specific WebView2 family must win.
        let buf =
            b"Chrome/137.0.7151.69\x00msedgewebview2.exe\x00WebView2Loader.dll\x00EBWebView\x00Crashpad\x00"
                .to_vec();
        let r = analyze_google(&buf);
        assert_eq!(
            r.family,
            BinaryFamily::WebView2,
            "WebView2 must win over Chromium, got {:?}",
            r.family
        );
        assert!(r.indicators.iter().any(|s| s.contains("msedgewebview2")));
        // The Chromium version is still extracted independently of family.
        assert_eq!(r.chromium_version.as_deref(), Some("137.0.7151.69"));
    }

    #[test]
    fn google_family_webview2_serializes_snake_case() {
        let buf = b"CoreWebView2\x00Microsoft.Web.WebView2\x00".to_vec();
        let r = analyze_google(&buf);
        let json = serde_json::to_value(&r).expect("serialize");
        assert_eq!(json["family"], "web_view2");
    }

    #[test]
    fn google_family_v8_snapshot_detected() {
        let buf = b"v8_context_snapshot\x00snapshot_blob\x00".to_vec();
        let r = analyze_google(&buf);
        assert_eq!(r.family, BinaryFamily::V8Snapshot);
    }

    #[test]
    fn google_family_generic_on_empty() {
        let r = analyze_google(b"");
        assert_eq!(r.family, BinaryFamily::Generic);
        assert!(r.oauth_client_ids.is_empty());
        assert!(r.google_endpoints.is_empty());
        assert!(r.chromium_version.is_none());
        assert!(r.code_sign_subject.is_none());
    }

    // OAuth client ID tests ---------------------------------------------------

    #[test]
    fn google_oauth_id_extracted() {
        // Synthetic buffer containing a real-shaped OAuth client ID.
        let buf = b"config: 123456789012-abcdef0123456789abcdef0123456789.apps.googleusercontent.com extra\x00".to_vec();
        let r = analyze_google(&buf);
        assert_eq!(r.oauth_client_ids.len(), 1, "expected 1 client_id, got {:?}", r.oauth_client_ids);
        assert!(r.oauth_client_ids[0].ends_with(".apps.googleusercontent.com"));
    }

    #[test]
    fn google_oauth_multiple_ids_deduplicated() {
        // Same ID twice — must appear once.
        let id = b"987654321000-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.apps.googleusercontent.com";
        let mut buf = id.to_vec();
        buf.push(b' ');
        buf.extend_from_slice(id);
        let r = analyze_google(&buf);
        assert_eq!(r.oauth_client_ids.len(), 1, "duplicate client_id must be deduplicated");
    }

    // Google endpoint tests ---------------------------------------------------

    #[test]
    fn google_endpoint_googleapis_extracted() {
        let buf = b"GET https://oauth2.googleapis.com/token HTTP/1.1\x00".to_vec();
        let r = analyze_google(&buf);
        assert!(
            r.google_endpoints.iter().any(|e| e.contains("googleapis.com")),
            "should extract googleapis.com endpoint, got {:?}",
            r.google_endpoints
        );
    }

    #[test]
    fn google_endpoint_run_app_extracted() {
        let buf = b"endpoint=https://my-service-abc123.run.app/api/v1\x00".to_vec();
        let r = analyze_google(&buf);
        assert!(
            r.google_endpoints.iter().any(|e| e.contains(".run.app")),
            "should extract .run.app endpoint, got {:?}",
            r.google_endpoints
        );
    }

    // Code-signing tests ------------------------------------------------------

    #[test]
    fn google_code_sign_llc_detected() {
        let buf = b"\x30\x82Google LLC\x00issuer\x00".to_vec();
        let r = analyze_google(&buf);
        assert_eq!(r.code_sign_subject.as_deref(), Some("Google LLC"));
    }

    #[test]
    fn google_code_sign_inc_detected() {
        let buf = b"subject=Google Inc\x00".to_vec();
        let r = analyze_google(&buf);
        assert!(r.code_sign_subject.is_some());
        assert!(r.code_sign_subject.as_deref().unwrap().starts_with("Google Inc"));
    }

    // JSON serialization -------------------------------------------------------

    #[test]
    fn google_report_serializes_stable_json_shape() {
        let buf = make_electron_buf();
        let r = analyze_google(&buf);
        let json = serde_json::to_value(&r).expect("serialize");
        assert!(json["family"].is_string());
        assert_eq!(json["family"], "electron");
        assert!(json["oauth_client_ids"].is_array());
        assert!(json["google_endpoints"].is_array());
        assert!(json["updater_urls"].is_array());
        assert!(json["indicators"].is_array());
    }

    // gRPC `exa.*` surface tests ----------------------------------------------

    #[test]
    fn google_grpc_path_extracts_service_and_method() {
        // Synthetic Go binary fragment: build-id marker + a real-shaped gRPC
        // routing path + the Codeium proto package symbol.
        let mut buf = b"Go build ID: \"abc\"\x00".to_vec();
        buf.extend_from_slice(
            b"/exa.language_server_pb.LanguageServerService/FetchUserInfo\x00",
        );
        buf.extend_from_slice(b"codeium_common_go_proto\x00");
        let r = analyze_google(&buf);

        // Family unchanged.
        assert_eq!(r.family, BinaryFamily::GoBinary, "family must stay GoBinary, got {:?}", r.family);

        // Service from the path is recorded (full FQN).
        assert!(
            r.grpc_services
                .iter()
                .any(|s| s == "exa.language_server_pb.LanguageServerService"),
            "expected service FQN, got {:?}",
            r.grpc_services
        );
        // Codeium proto package marker recorded.
        assert!(
            r.grpc_services.iter().any(|s| s == "codeium_common_go_proto"),
            "expected codeium_common_go_proto, got {:?}",
            r.grpc_services
        );
        // Method from the path is recorded.
        assert!(
            r.grpc_methods.iter().any(|m| m == "FetchUserInfo"),
            "expected FetchUserInfo method, got {:?}",
            r.grpc_methods
        );
        // Human indicators present.
        assert!(r.indicators.iter().any(|s| s.starts_with("gRPC service:")));
        assert!(r.indicators.iter().any(|s| s.starts_with("gRPC method:")));
    }

    #[test]
    fn google_grpc_methods_allowlist_and_sorted_dedup() {
        let mut buf = b".gopclntab\x00".to_vec();
        // Two distinct service paths.
        buf.extend_from_slice(b"/exa.auth_pb.AuthService/GetAuthStatus\x00");
        buf.extend_from_slice(b"/exa.models_pb.ModelService/GetAvailableModels\x00");
        // Allowlisted verb-noun identifiers loose in the corpus (mined only
        // because an exa.* service is present). Duplicate to test dedup.
        buf.extend_from_slice(b"FetchUserInfo GetModelResponse FetchUserInfo\x00");
        // A non-allowlisted CamelCase identifier that must NOT be captured.
        buf.extend_from_slice(b"RenderTemplate ComputeHash\x00");
        let r = analyze_google(&buf);

        assert_eq!(r.family, BinaryFamily::GoBinary);

        // Services sorted ascending.
        let mut sorted_svc = r.grpc_services.clone();
        sorted_svc.sort();
        assert_eq!(r.grpc_services, sorted_svc, "services must be sorted");

        // Methods sorted + deduped.
        let mut sorted_meth = r.grpc_methods.clone();
        sorted_meth.sort();
        sorted_meth.dedup();
        assert_eq!(r.grpc_methods, sorted_meth, "methods must be sorted + deduped");

        // Allowlisted verbs captured.
        for expected in ["GetAuthStatus", "GetAvailableModels", "FetchUserInfo", "GetModelResponse"] {
            assert!(
                r.grpc_methods.iter().any(|m| m == expected),
                "expected method {expected}, got {:?}",
                r.grpc_methods
            );
        }
        // FetchUserInfo only once despite appearing twice.
        assert_eq!(
            r.grpc_methods.iter().filter(|m| *m == "FetchUserInfo").count(),
            1,
            "FetchUserInfo must be deduplicated"
        );
        // Non-allowlisted identifiers excluded.
        assert!(
            !r.grpc_methods.iter().any(|m| m == "RenderTemplate" || m == "ComputeHash"),
            "non-allowlisted identifiers must not be captured, got {:?}",
            r.grpc_methods
        );
    }

    #[test]
    fn google_grpc_no_mining_without_exa_service() {
        // Verb-noun identifiers present, but NO exa.* service — the corpus
        // allowlist must stay dormant so unrelated binaries are not polluted.
        let buf = b"go:buildid\x00GetUserInfo FetchData StartServer\x00".to_vec();
        let r = analyze_google(&buf);
        assert_eq!(r.family, BinaryFamily::GoBinary);
        assert!(r.grpc_services.is_empty(), "no services expected, got {:?}", r.grpc_services);
        assert!(r.grpc_methods.is_empty(), "no methods expected, got {:?}", r.grpc_methods);
    }

    #[test]
    fn google_grpc_bare_package_symbol_extracted() {
        // A bare `exa.*_pb` package symbol with no full service routing path.
        let buf = b"runtime.goexit\x00exa.codeium_common_pb\x00".to_vec();
        let r = analyze_google(&buf);
        assert_eq!(r.family, BinaryFamily::GoBinary);
        assert!(
            r.grpc_services.iter().any(|s| s == "exa.codeium_common_pb"),
            "expected bare package symbol, got {:?}",
            r.grpc_services
        );
    }

    #[test]
    fn google_grpc_empty_on_non_go() {
        // Electron buffer without any exa.* surface — grpc vecs stay empty.
        let buf = make_electron_buf();
        let r = analyze_google(&buf);
        assert!(r.grpc_services.is_empty());
        assert!(r.grpc_methods.is_empty());
    }

    #[test]
    fn google_report_serializes_grpc_arrays() {
        let r = analyze_google(b"");
        let json = serde_json::to_value(&r).expect("serialize");
        assert!(json["grpc_services"].is_array(), "grpc_services must serialize as array");
        assert!(json["grpc_methods"].is_array(), "grpc_methods must serialize as array");
    }

    // Combined test -----------------------------------------------------------

    #[test]
    fn google_combined_electron_with_oauth_and_endpoint() {
        let mut buf = make_electron_buf();
        buf.extend_from_slice(
            b"client_id=111111111111-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx.apps.googleusercontent.com\x00"
        );
        buf.extend_from_slice(b"https://accounts.google.com/o/oauth2/token\x00");
        let r = analyze_google(&buf);
        assert_eq!(r.family, BinaryFamily::Electron);
        assert_eq!(r.oauth_client_ids.len(), 1);
        assert!(r.google_endpoints.iter().any(|e| e.contains("google.com")));
    }
}
