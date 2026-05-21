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
    /// Human-readable list of detection signals used to classify the binary.
    pub indicators: Vec<String>,
}

// ---------------------------------------------------------------------------
// Main public entry point
// ---------------------------------------------------------------------------

/// Analyse a raw binary blob for Google-specific artefacts.
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
    let mut indicators: Vec<String> = Vec::new();

    // --- 1. Extract the string corpus (ASCII + UTF-16LE) once; everything
    //         below operates on this corpus + raw byte needles.
    let strings = extract_strings(bytes, 4, 8192);

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

    GoogleReport {
        family,
        chromium_version,
        oauth_client_ids,
        google_endpoints,
        updater_urls,
        code_sign_subject,
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
