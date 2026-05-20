// SPDX-License-Identifier: Apache-2.0
//
// bxc-mcp — Rust MCP stdio server replacing bxc-scrapper/server.ts.
//
// Speaks the Model Context Protocol 1.0 over stdio (newline-delimited
// JSON-RPC 2.0) and exposes the seven bxc scraping tools previously served
// by the Bun TypeScript shim:
//
//   bxc_scrape, bxc_recon, bxc_detect, google_search, google_atlas_route,
//   extract_structured, vision_analyze
//
// Each tool POSTs to ${BXC_DAEMON_URL}/v1/<tool> (default
// http://127.0.0.1:8765). If the daemon is unreachable, the tool returns a
// structured BXC_UNAVAILABLE error (never invents data, never falls back
// to fake values). A subprocess fallback is intentionally absent: this
// binary IS the engine, so spawning a sibling would be a tautology.
//
// Cross-platform: pure Rust, no FFI, no JS host. Compiles on Linux #1 +
// Windows #2 (cf. CLAUDE.md §0). Discovery via --list-tools (stdout) and
// --help (stderr) for human inspection without driving a JSON-RPC session.

use std::io::IsTerminal;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

const SERVER_NAME: &str = "bxc-scrapper";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
const PROTOCOL_VERSION: &str = "2024-11-05";

// ── CLI surface ──────────────────────────────────────────────────────────────

fn print_help() {
    eprintln!(
        "{SERVER_NAME} v{SERVER_VERSION} — MCP stdio server for the bxc scraping engine.

USAGE:
  bxc-mcp                Run as an MCP stdio server (default).
  bxc-mcp --list-tools   Print the JSON tool catalog and exit.
  bxc-mcp --help         Print this help and exit.

ENV:
  BXC_DAEMON_URL          (default http://127.0.0.1:8765)
  BXC_TIMEOUT_MS          (default 30000) per-tool HTTP timeout
  BXC_VISION_MIN_BYTES    (default 1024)  refuse vision on tiny images

PROTOCOL:
  Newline-delimited JSON-RPC 2.0 over stdin/stdout (MCP {PROTOCOL_VERSION}).
  stderr is reserved for diagnostics.
"
    );
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    // Ring provider must be installed before any reqwest::Client touches TLS,
    // otherwise rustls panics on first use (cf. CLAUDE.md §7).
    let _ = rustls::crypto::ring::default_provider().install_default();

    let argv: Vec<String> = std::env::args().skip(1).collect();
    for a in &argv {
        match a.as_str() {
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            "--list-tools" => {
                let catalog = tool_catalog();
                println!("{}", serde_json::to_string_pretty(&catalog)?);
                return Ok(());
            }
            "--version" | "-V" => {
                println!("{SERVER_NAME} {SERVER_VERSION}");
                return Ok(());
            }
            other if other.starts_with("--") => {
                eprintln!("unknown flag: {other}");
                print_help();
                std::process::exit(2);
            }
            _ => {}
        }
    }

    // Refuse to dangle on a TTY — that's a user mistake (no stdio peer will
    // ever speak JSON-RPC on a keyboard). Print a hint and exit.
    if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
        eprintln!(
            "{SERVER_NAME} v{SERVER_VERSION}: stdin is a TTY — refusing to wait \
             for JSON-RPC. Use --help or --list-tools, or pipe MCP frames in."
        );
        std::process::exit(2);
    }

    run_stdio().await
}

// ── JSON-RPC frames ─────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct RpcMessage {
    #[allow(dead_code)]
    jsonrpc: String,
    #[serde(default)]
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Serialize)]
struct RpcResponse {
    jsonrpc: &'static str,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<RpcError>,
}

#[derive(Serialize)]
struct RpcError {
    code: i32,
    message: String,
}

impl RpcResponse {
    fn ok(id: Value, result: Value) -> Self {
        Self { jsonrpc: "2.0", id, result: Some(result), error: None }
    }
    fn err(id: Value, code: i32, message: impl Into<String>) -> Self {
        Self { jsonrpc: "2.0", id, result: None, error: Some(RpcError { code, message: message.into() }) }
    }
}

// ── Server loop ─────────────────────────────────────────────────────────────

async fn run_stdio() -> anyhow::Result<()> {
    let client = build_client()?;

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin);
    let mut writer = stdout;

    let daemon = daemon_url();
    eprintln!(
        "[{SERVER_NAME}] v{SERVER_VERSION} on stdio | daemon={daemon} timeout={}ms",
        timeout_ms()
    );

    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            return Ok(());
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let msg: RpcMessage = match serde_json::from_str(trimmed) {
            Ok(m) => m,
            Err(e) => {
                let resp = RpcResponse::err(Value::Null, -32700, format!("parse error: {e}"));
                write_resp(&mut writer, &resp).await?;
                continue;
            }
        };

        // Notifications (no id) need no response.
        if msg.id.is_none() {
            continue;
        }
        let id = msg.id.clone().unwrap_or(Value::Null);

        let resp = dispatch(&msg.method, id, &msg.params, &client).await;
        write_resp(&mut writer, &resp).await?;
    }
}

async fn write_resp<W>(w: &mut W, resp: &RpcResponse) -> std::io::Result<()>
where
    W: AsyncWriteExt + Unpin,
{
    let mut body = serde_json::to_string(resp).expect("RpcResponse serializes");
    body.push('\n');
    w.write_all(body.as_bytes()).await?;
    w.flush().await
}

async fn dispatch(method: &str, id: Value, params: &Value, client: &reqwest::Client) -> RpcResponse {
    match method {
        "initialize" => handle_initialize(id, params),
        "initialized" | "notifications/initialized" => RpcResponse::ok(id, json!({})),
        "ping" => RpcResponse::ok(id, json!({})),
        "tools/list" => RpcResponse::ok(id, json!({ "tools": tool_catalog() })),
        "tools/call" => handle_tool_call(id, params, client).await,
        "resources/list" => RpcResponse::ok(id, json!({ "resources": [] })),
        "prompts/list" => RpcResponse::ok(id, json!({ "prompts": [] })),
        other => RpcResponse::err(id, -32601, format!("Unknown method: {other}")),
    }
}

fn handle_initialize(id: Value, _params: &Value) -> RpcResponse {
    RpcResponse::ok(
        id,
        json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": { "tools": {} },
            "serverInfo": { "name": SERVER_NAME, "version": SERVER_VERSION },
            "instructions":
                "bxc-scrapper exposes the aphrody bxc engine over MCP stdio. \
                 Prefer `bxc_recon` for first-touch reconnaissance, `bxc_scrape` \
                 for targeted text extraction, `bxc_detect` for framework \
                 fingerprinting, the `google_*` tools for SERP work, and \
                 `vision_analyze` / `extract_structured` for structured data \
                 extraction. All tools return JSON; on failure they surface a \
                 stable error code (BXC_UNAVAILABLE / BXC_INVALID_RESPONSE / \
                 BXC_BAD_REQUEST / BXC_TIMEOUT) — never invent values."
        }),
    )
}

// ── Tool catalog ────────────────────────────────────────────────────────────

fn tool_catalog() -> Value {
    json!([
        {
            "name": "bxc_scrape",
            "description": "Extracts text content from <url> matching <selector> (default `body`). Uses the bxc static profile when JS is not needed. Returns { extractions: string[] }.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "url": { "type": "string", "format": "uri", "description": "Fully-qualified http(s) URL to scrape." },
                    "selector": { "type": "string", "default": "body", "description": "CSS selector. Defaults to `body`." }
                },
                "required": ["url"]
            }
        },
        {
            "name": "bxc_recon",
            "description": "First-touch reconnaissance: HTTP headers, CDN, framework fingerprints, asset inventory (js/css/img/font), embedded CSS, and a screenshot saved by the daemon. Returns { headers, cdn, frameworks, assets, css, screenshot_path }.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "url": { "type": "string", "format": "uri", "description": "Fully-qualified http(s) URL to recon." }
                },
                "required": ["url"]
            }
        },
        {
            "name": "bxc_detect",
            "description": "Framework / CMS / library fingerprinting via wappalyzergo, IP-range matching, header heuristics, and CSP analysis. Returns { tech: DetectedTech[] }.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "url": { "type": "string", "format": "uri", "description": "Fully-qualified http(s) URL to fingerprint." }
                },
                "required": ["url"]
            }
        },
        {
            "name": "google_search",
            "description": "Google web search via bxc's ghost profile (Lightpanda + stealth). Returns organic SERP results. Honors mandate-guard (Google-only routing). Returns { results: OrganicResult[] }.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "minLength": 1, "description": "Search query string." },
                    "hl": { "type": "string", "default": "en", "description": "Interface language (ISO 639-1 code)." }
                },
                "required": ["query"]
            }
        },
        {
            "name": "google_atlas_route",
            "description": "For a Google-domain URL, returns the bxc Atlas recommendation: which profile to use (static / fast / stealth-wiz / stealth-spa / stealth-lit / max), stealth hints, and the detected framework. Returns { profile, stealth_hints, framework }.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "url": { "type": "string", "format": "uri", "description": "Fully-qualified Google-domain URL." }
                },
                "required": ["url"]
            }
        },
        {
            "name": "extract_structured",
            "description": "Typed structured extraction: takes raw HTML and a JSON-Schema. First call uses the local Gemma 4 (llama.cpp) to learn CSS selectors; subsequent calls reuse the cache for <1ms/page. Returns the validated typed JSON.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "html": { "type": "string", "minLength": 1, "description": "Raw HTML to extract from." },
                    "zod_schema_json": { "type": "string", "minLength": 2, "description": "JSON-Schema-shaped extraction target. Server validates the LLM output against this schema." }
                },
                "required": ["html", "zod_schema_json"]
            }
        },
        {
            "name": "vision_analyze",
            "description": "Vision pipeline over a screenshot saved by bxc_recon (or any local PNG/WebP). Extracts dominant colors, font candidates, visible text, element layout, and a coarse hierarchy. Returns { elements, text, colors, fonts, hierarchy }.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "screenshot_path": { "type": "string", "minLength": 1, "description": "Absolute or workspace-relative path to a PNG / WebP screenshot." }
                },
                "required": ["screenshot_path"]
            }
        }
    ])
}

// ── Tool dispatch ───────────────────────────────────────────────────────────

async fn handle_tool_call(id: Value, params: &Value, client: &reqwest::Client) -> RpcResponse {
    let name = match params.get("name").and_then(Value::as_str) {
        Some(n) => n,
        None => return RpcResponse::err(id, -32602, "Missing tool name"),
    };
    let args = params.get("arguments").cloned().unwrap_or(Value::Object(Default::default()));

    // Local pre-checks (mirror the TS shim exactly so error semantics are stable).
    let outcome: BxcResult = match name {
        "bxc_scrape" => {
            let url = required_str(&args, "url");
            let selector = args.get("selector").and_then(Value::as_str).unwrap_or("body");
            match url {
                Ok(url) => call_daemon(client, "scrape", &json!({ "url": url, "selector": selector })).await,
                Err(e) => bad_request(e),
            }
        }
        "bxc_recon" => call_url_tool(client, "recon", &args).await,
        "bxc_detect" => call_url_tool(client, "detect", &args).await,
        "google_atlas_route" => call_url_tool(client, "google_atlas_route", &args).await,
        "google_search" => {
            let query = required_str(&args, "query");
            let hl = args.get("hl").and_then(Value::as_str).unwrap_or("en");
            match query {
                Ok("") => bad_request("query must be a non-empty string"),
                Ok(q) => call_daemon(client, "google_search", &json!({ "query": q, "hl": hl })).await,
                Err(e) => bad_request(e),
            }
        }
        "extract_structured" => {
            let html = required_str(&args, "html");
            let schema = required_str(&args, "zod_schema_json");
            match (html, schema) {
                (Ok(html), Ok(schema)) => {
                    if html.is_empty() {
                        bad_request("html must be non-empty")
                    } else if let Err(e) = serde_json::from_str::<Value>(schema) {
                        bad_request(format!("zod_schema_json is not valid JSON: {e}"))
                    } else {
                        call_daemon(
                            client,
                            "extract_structured",
                            &json!({ "html": html, "schema": schema }),
                        )
                        .await
                    }
                }
                (Err(e), _) | (_, Err(e)) => bad_request(e),
            }
        }
        "vision_analyze" => {
            let path = required_str(&args, "screenshot_path");
            match path {
                Ok(p) => match validate_screenshot(p) {
                    Ok(()) => {
                        call_daemon(client, "vision_analyze", &json!({ "screenshot_path": p })).await
                    }
                    Err(reason) => bad_request(reason),
                },
                Err(e) => bad_request(e),
            }
        }
        other => bad_request(format!("Unknown tool: {other}")),
    };

    as_mcp_result(id, outcome)
}

async fn call_url_tool(
    client: &reqwest::Client,
    tool: &str,
    args: &Value,
) -> BxcResult {
    match required_str(args, "url") {
        Ok(url) => call_daemon(client, tool, &json!({ "url": url })).await,
        Err(e) => bad_request(e),
    }
}

fn required_str<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("Missing required parameter: {key}"))
}

fn validate_screenshot(path: &str) -> Result<(), String> {
    let meta = std::fs::metadata(path)
        .map_err(|e| format!("unable to stat screenshot_path '{path}': {e}"))?;
    let size = meta.len();
    if size == 0 {
        return Err(format!("screenshot_path '{path}' does not exist or is empty"));
    }
    let min = vision_min_bytes();
    if size < min {
        return Err(format!(
            "screenshot_path '{path}' is {size}B (< BXC_VISION_MIN_BYTES={min}); refusing to vision-analyze a likely corrupt image"
        ));
    }
    Ok(())
}

// ── Daemon transport ────────────────────────────────────────────────────────

enum BxcResult {
    Ok(Value),
    Err(Value),
}

fn bad_request(reason: impl Into<String>) -> BxcResult {
    BxcResult::Err(json!({ "error": "BXC_BAD_REQUEST", "reason": reason.into() }))
}

async fn call_daemon(client: &reqwest::Client, tool: &str, args: &Value) -> BxcResult {
    let base = daemon_url();
    let url = format!("{base}/v1/{tool}");

    let req = client
        .post(&url)
        .header("User-Agent", format!("{SERVER_NAME}/{SERVER_VERSION}"))
        .json(args);

    let resp = match req.send().await {
        Ok(r) => r,
        Err(e) => {
            let kind = if e.is_timeout() { "BXC_TIMEOUT" } else { "BXC_UNAVAILABLE" };
            return BxcResult::Err(json!({
                "error": kind,
                "reason": format!("daemon {url}: {e}"),
                "daemon_attempt": url,
            }));
        }
    };

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        let snippet: String = body.chars().take(512).collect();
        return BxcResult::Err(json!({
            "error": "BXC_BAD_REQUEST",
            "reason": format!("daemon HTTP {status}: {snippet}"),
            "daemon_attempt": url,
        }));
    }

    let body = match resp.text().await {
        Ok(b) => b,
        Err(e) => {
            return BxcResult::Err(json!({
                "error": "BXC_INVALID_RESPONSE",
                "reason": format!("read body: {e}"),
                "daemon_attempt": url,
            }));
        }
    };

    match serde_json::from_str::<Value>(&body) {
        Ok(v) => {
            if v.get("error").and_then(Value::as_str).is_some() {
                BxcResult::Err(v)
            } else {
                BxcResult::Ok(v)
            }
        }
        Err(e) => BxcResult::Err(json!({
            "error": "BXC_INVALID_RESPONSE",
            "reason": format!("daemon body not JSON: {e}"),
            "daemon_attempt": url,
        })),
    }
}

fn as_mcp_result(id: Value, outcome: BxcResult) -> RpcResponse {
    let (payload, is_error) = match outcome {
        BxcResult::Ok(v) => (v, false),
        BxcResult::Err(v) => (v, true),
    };
    let text = serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string());
    let mut result = json!({
        "content": [{ "type": "text", "text": text }],
        "structuredContent": payload,
    });
    if is_error {
        result["isError"] = Value::Bool(true);
    }
    RpcResponse::ok(id, result)
}

// ── Env knobs ───────────────────────────────────────────────────────────────

fn daemon_url() -> String {
    parse_daemon_url(std::env::var("BXC_DAEMON_URL").ok().as_deref())
}

/// Pure parsing of the BXC_DAEMON_URL env value. Extracted from
/// [`daemon_url`] so unit tests don't race on the global env (which can
/// not be safely mutated from concurrent tests under cargo test default
/// scheduling).
fn parse_daemon_url(raw: Option<&str>) -> String {
    let s = raw.unwrap_or("http://127.0.0.1:8765");
    let trimmed = s.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        "http://127.0.0.1:8765".to_string()
    } else {
        trimmed.to_string()
    }
}

fn timeout_ms() -> u64 {
    std::env::var("BXC_TIMEOUT_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|&n| n > 0)
        .unwrap_or(30_000)
}

fn vision_min_bytes() -> u64 {
    std::env::var("BXC_VISION_MIN_BYTES")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(1024)
}

fn build_client() -> anyhow::Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(Duration::from_millis(timeout_ms()))
        .user_agent(format!("{SERVER_NAME}/{SERVER_VERSION}"))
        .build()
        .map_err(|e| anyhow::anyhow!("failed to build HTTP client: {e}"))
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_lists_seven_tools() {
        let cat = tool_catalog();
        let arr = cat.as_array().expect("array");
        assert_eq!(arr.len(), 7, "tool catalog must expose 7 tools to match server.ts");
        let names: Vec<&str> = arr
            .iter()
            .map(|t| t.get("name").and_then(Value::as_str).unwrap_or(""))
            .collect();
        for expected in [
            "bxc_scrape",
            "bxc_recon",
            "bxc_detect",
            "google_search",
            "google_atlas_route",
            "extract_structured",
            "vision_analyze",
        ] {
            assert!(names.contains(&expected), "missing tool: {expected}");
        }
    }

    #[test]
    fn required_str_extracts_value() {
        let v = json!({ "url": "https://example.com" });
        assert_eq!(required_str(&v, "url").unwrap(), "https://example.com");
        assert!(required_str(&v, "missing").is_err());
    }

    #[test]
    fn validate_screenshot_rejects_missing_path() {
        let err = validate_screenshot("/does/not/exist/zzz.png").unwrap_err();
        assert!(err.contains("unable to stat"), "got: {err}");
    }

    #[test]
    fn daemon_url_default_when_unset() {
        assert_eq!(parse_daemon_url(None), "http://127.0.0.1:8765");
    }

    #[test]
    fn daemon_url_strips_trailing_slash() {
        assert_eq!(
            parse_daemon_url(Some("http://localhost:9999/")),
            "http://localhost:9999"
        );
    }

    #[test]
    fn daemon_url_trims_whitespace_and_falls_back_when_empty() {
        assert_eq!(
            parse_daemon_url(Some("   http://example.com   ")),
            "http://example.com"
        );
        assert_eq!(parse_daemon_url(Some("")), "http://127.0.0.1:8765");
        assert_eq!(parse_daemon_url(Some("   ")), "http://127.0.0.1:8765");
    }

    #[test]
    fn rpc_response_ok_serializes_without_error_field() {
        let r = RpcResponse::ok(json!(1), json!({"ok": true}));
        let s = serde_json::to_string(&r).unwrap();
        assert!(s.contains("\"jsonrpc\":\"2.0\""));
        assert!(s.contains("\"id\":1"));
        assert!(s.contains("\"result\""));
        assert!(!s.contains("\"error\""));
    }

    #[test]
    fn rpc_response_err_serializes_without_result_field() {
        let r = RpcResponse::err(json!("abc"), -32601, "Unknown method: x");
        let s = serde_json::to_string(&r).unwrap();
        assert!(s.contains("\"error\""));
        assert!(s.contains("Unknown method: x"));
        assert!(!s.contains("\"result\""));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn dispatch_unknown_method_returns_minus_32601() {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let client = build_client().unwrap();
        let resp = dispatch("totally/unknown", json!(7), &json!({}), &client).await;
        let s = serde_json::to_string(&resp).unwrap();
        assert!(s.contains("-32601"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn dispatch_initialize_announces_protocol_version() {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let client = build_client().unwrap();
        let resp = dispatch("initialize", json!(1), &json!({}), &client).await;
        let s = serde_json::to_string(&resp).unwrap();
        assert!(s.contains(PROTOCOL_VERSION));
        assert!(s.contains("bxc-scrapper"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn dispatch_tools_list_returns_seven_tools() {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let client = build_client().unwrap();
        let resp = dispatch("tools/list", json!(2), &json!({}), &client).await;
        let v = serde_json::to_value(&resp).unwrap();
        let tools = v["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 7);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn dispatch_tool_call_missing_arg_returns_bad_request() {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let client = build_client().unwrap();
        // bxc_scrape requires url
        let resp = dispatch(
            "tools/call",
            json!(3),
            &json!({ "name": "bxc_scrape", "arguments": {} }),
            &client,
        )
        .await;
        let v = serde_json::to_value(&resp).unwrap();
        assert_eq!(v["result"]["isError"], json!(true));
        let body = v["result"]["structuredContent"]["error"].as_str().unwrap();
        assert_eq!(body, "BXC_BAD_REQUEST");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn dispatch_tool_call_daemon_unavailable_surfaces_structured_error() {
        let _ = rustls::crypto::ring::default_provider().install_default();
        // Point at a closed port → daemon unreachable → BXC_UNAVAILABLE.
        unsafe {
            std::env::set_var("BXC_DAEMON_URL", "http://127.0.0.1:1");
            std::env::set_var("BXC_TIMEOUT_MS", "500");
        }
        let client = build_client().unwrap();
        let resp = dispatch(
            "tools/call",
            json!(4),
            &json!({ "name": "bxc_recon", "arguments": { "url": "https://example.com" } }),
            &client,
        )
        .await;
        let v = serde_json::to_value(&resp).unwrap();
        assert_eq!(v["result"]["isError"], json!(true));
        let err = v["result"]["structuredContent"]["error"].as_str().unwrap();
        assert!(
            err == "BXC_UNAVAILABLE" || err == "BXC_TIMEOUT",
            "expected BXC_UNAVAILABLE/BXC_TIMEOUT, got {err}"
        );
        unsafe {
            std::env::remove_var("BXC_DAEMON_URL");
            std::env::remove_var("BXC_TIMEOUT_MS");
        }
    }

    #[test]
    fn extract_structured_rejects_non_json_schema_locally() {
        // Smoke: required_str returns Ok for both, but invalid JSON in
        // zod_schema_json must short-circuit with BXC_BAD_REQUEST before any
        // HTTP call. We replay that branch manually here.
        let args = json!({ "html": "<p>x</p>", "zod_schema_json": "not json!" });
        let html = required_str(&args, "html").unwrap();
        let schema = required_str(&args, "zod_schema_json").unwrap();
        assert!(!html.is_empty());
        assert!(serde_json::from_str::<Value>(schema).is_err());
    }
}
