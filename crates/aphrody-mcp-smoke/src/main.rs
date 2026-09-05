// SPDX-License-Identifier: Apache-2.0
//! End-to-end smoke test for the unified `aphrody-mcp` stdio server.
//!
//! Drives the real binary over a child process pipe pair, performs the
//! standard MCP handshake, lists every advertised tool, and calls each
//! one with the smallest valid argument set defined in [`FIXTURES`].
//!
//! Per-tool outcome is one of:
//! - `pass`    : `result.isError == false`
//! - `expected_error` : `result.isError == true` AND the tool is flagged `network_dependent` /
//!   `creds_required` / `daemon_required` in the fixture catalog (e.g. an MCP scrape call when the
//!   bxc daemon is down, or `voice_synthesize` without `ELEVENLABS_API_KEY`).
//! - `fail`    : unexpected JSON-RPC error, or `isError == true` without a matching expected-error
//!   flag.
//! - `skip`    : platform-incompatible (Windows-only tools on Linux), or `start_dashboard`
//!   (side-effect: binds a TCP port for the lifetime of the MCP process — never called by the
//!   runner).
//!
//! Report is line-delimited JSON on the `--report` path AND stdout,
//! followed by a single summary line. Exit code is 0 if no `fail`,
//! 1 otherwise.

use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::Instant,
};

use anyhow::{Context, Result, anyhow, bail};
use clap::Parser;
use serde_json::{Value, json};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, Command},
};

const PROTOCOL_VERSION: &str = "2025-06-18";

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the aphrody-mcp binary. If omitted, resolves via
    /// $APHRODY_MCP_BIN → $PATH → ~/.local/bin/aphrody-mcp[.exe] →
    /// target/release/aphrody-mcp[.exe].
    #[arg(long)]
    bin: Option<PathBuf>,

    /// Output NDJSON report path. Parent directory is created on demand.
    #[arg(long, default_value = "var/smoke/mcp-smoke.ndjson")]
    report: PathBuf,

    /// Per-call timeout in milliseconds. Defaults match the manifest
    /// `BXC_TIMEOUT_MS=30000` and accommodate `dns_recon`'s full subdomain
    /// enumeration sweep.
    #[arg(long, default_value_t = 30_000)]
    timeout_ms: u64,
}

#[derive(Debug, Clone, Copy)]
struct Fixture {
    name: &'static str,
    args_json: &'static str,
    /// True if the tool may legitimately fail without external state
    /// (network down, daemon down, missing creds, …) — counted as
    /// `expected_error` instead of `fail`.
    network_dependent: bool,
    daemon_required: bool,
    creds_required: bool,
    /// True if calling the tool has a side effect we don't want during
    /// the smoke run (e.g. binds a port). Counted as `skip`.
    side_effect: bool,
    /// True if the tool only works on Windows (uses Win32 APIs).
    windows_only: bool,
}

const F: Fixture = Fixture {
    name: "",
    args_json: "{}",
    network_dependent: false,
    daemon_required: false,
    creds_required: false,
    side_effect: false,
    windows_only: false,
};

const FIXTURES: &[Fixture] = &[
    Fixture { name: "native_hooks", ..F },
    Fixture {
        name: "coding_style_guide",
        args_json: r#"{"topic":"rust"}"#,
        network_dependent: true,
        ..F
    },
    Fixture {
        name: "dns_recon",
        args_json: r#"{"domain":"example.com"}"#,
        network_dependent: true,
        ..F
    },
    Fixture {
        name: "advanced_recon",
        args_json: r#"{"target":"127.0.0.1","ports":[80,443]}"#,
        network_dependent: true,
        ..F
    },
    Fixture {
        name: "universal_web_fetch",
        args_json: r#"{"url":"https://example.com/"}"#,
        network_dependent: true,
        ..F
    },
    Fixture { name: "auth_extract", args_json: "{}", windows_only: true, ..F },
    Fixture {
        name: "chrome_autopsy",
        args_json: r#"{"pid":4,"size":64}"#,
        windows_only: true,
        ..F
    },
    Fixture { name: "start_dashboard", args_json: r#"{"port":0}"#, side_effect: true, ..F },
    Fixture {
        name: "bxc_scrape",
        args_json: r#"{"url":"https://example.com/","selector":"h1"}"#,
        daemon_required: true,
        ..F
    },
    Fixture {
        name: "bxc_recon",
        args_json: r#"{"url":"https://example.com/"}"#,
        daemon_required: true,
        ..F
    },
    Fixture {
        name: "bxc_detect",
        args_json: r#"{"url":"https://example.com/"}"#,
        daemon_required: true,
        ..F
    },
    Fixture {
        name: "google_search",
        args_json: r#"{"query":"rust mcp","hl":"en"}"#,
        daemon_required: true,
        ..F
    },
    Fixture {
        name: "google_atlas_route",
        args_json: r#"{"url":"https://www.google.com/search?q=rust"}"#,
        daemon_required: true,
        ..F
    },
    Fixture {
        name: "extract_structured",
        args_json: r#"{"html":"<html><body><h1>Title</h1></body></html>","zod_schema_json":"{\"type\":\"object\",\"properties\":{\"title\":{\"type\":\"string\"}}}"}"#,
        network_dependent: true,
        ..F
    },
    Fixture {
        name: "vision_analyze",
        args_json: r#"{"screenshot_path":"var/smoke/nonexistent.png"}"#,
        network_dependent: true,
        ..F
    },
    Fixture {
        name: "voice_synthesize",
        args_json: r#"{"text":"smoke test"}"#,
        creds_required: true,
        ..F
    },
    Fixture {
        name: "voice_transcribe",
        args_json: r#"{"audio_base64":"","mime_type":"audio/mpeg"}"#,
        creds_required: true,
        ..F
    },
    Fixture {
        name: "context7_resolve_library_id",
        args_json: r#"{"query":"How to spawn an async task","library_name":"tokio"}"#,
        network_dependent: true,
        ..F
    },
    Fixture {
        name: "context7_query_docs",
        args_json: r#"{"library_id":"/tokio-rs/tokio","query":"spawn"}"#,
        network_dependent: true,
        ..F
    },
    Fixture {
        name: "re_triage",
        args_json: r#"{"path":"target/x86_64-pc-windows-msvc/release/aphrody-mcp.exe"}"#,
        network_dependent: true,
        ..F
    },
    Fixture {
        name: "microsoft_docs_search",
        args_json: r#"{"query":"Azure Functions Python timeout"}"#,
        network_dependent: true,
        ..F
    },
    Fixture {
        name: "microsoft_docs_fetch",
        args_json: r#"{"url":"https://learn.microsoft.com/en-us/azure/azure-functions/functions-reference"}"#,
        network_dependent: true,
        ..F
    },
    Fixture {
        name: "microsoft_code_sample_search",
        args_json: r#"{"query":"upload blob storage","language":"csharp"}"#,
        network_dependent: true,
        ..F
    },
    Fixture {
        name: "docs_auto_search",
        args_json: r#"{"query":"tokio spawn task","library_name":"tokio","language":"rust"}"#,
        network_dependent: true,
        ..F
    },
];

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let args = Args::parse();
    let bin = resolve_binary(args.bin.as_deref())?;

    let mut child = Command::new(&bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("failed to spawn {}", bin.display()))?;

    let stdin = child.stdin.take().ok_or_else(|| anyhow!("no stdin"))?;
    let stdout = child.stdout.take().ok_or_else(|| anyhow!("no stdout"))?;
    let mut writer = stdin;
    let mut reader = BufReader::new(stdout).lines();

    let timeout = std::time::Duration::from_millis(args.timeout_ms);

    // ── 1. initialize ────────────────────────────────────────────────
    send(
        &mut writer,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": { "name": "aphrody-mcp-smoke", "version": env!("CARGO_PKG_VERSION") }
            }
        }),
    )
    .await?;
    let init = recv(&mut reader, timeout).await.context("initialize")?;
    let server_info = init.pointer("/result/serverInfo").cloned().unwrap_or(Value::Null);
    let proto = init
        .pointer("/result/protocolVersion")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();

    // ── 2. initialized notification ──────────────────────────────────
    send(&mut writer, &json!({ "jsonrpc": "2.0", "method": "notifications/initialized" })).await?;

    // ── 3. tools/list ────────────────────────────────────────────────
    send(&mut writer, &json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" })).await?;
    let listed = recv(&mut reader, timeout).await.context("tools/list")?;
    let tools = listed
        .pointer("/result/tools")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("tools/list did not return a tools array: {listed}"))?
        .clone();
    let advertised: std::collections::BTreeSet<String> = tools
        .iter()
        .filter_map(|t| t.get("name").and_then(Value::as_str).map(str::to_owned))
        .collect();

    // ── 4. tools/call sweep ──────────────────────────────────────────
    let mut report_lines: Vec<Value> = Vec::with_capacity(FIXTURES.len() + 4);
    let mut latencies_ms: Vec<u128> = Vec::with_capacity(FIXTURES.len());
    let mut pass = 0_usize;
    let mut expected_err = 0_usize;
    let mut fail = 0_usize;
    let mut skip = 0_usize;

    report_lines.push(json!({
        "type": "header",
        "binary": bin.display().to_string(),
        "protocol": proto,
        "server_info": server_info,
        "tools_advertised": advertised.len(),
        "tools_fixtures": FIXTURES.len(),
    }));

    let mut next_id: i64 = 3;
    for fix in FIXTURES {
        let mut entry = json!({
            "type": "tool",
            "tool": fix.name,
            "advertised": advertised.contains(fix.name),
        });

        if !advertised.contains(fix.name) {
            entry["status"] = json!("fail");
            entry["reason"] = json!("tool not advertised by server");
            fail += 1;
            report_lines.push(entry);
            continue;
        }

        if fix.windows_only && !cfg!(target_os = "windows") {
            entry["status"] = json!("skip");
            entry["reason"] = json!("windows-only tool, not on this platform");
            skip += 1;
            report_lines.push(entry);
            continue;
        }
        if fix.side_effect {
            entry["status"] = json!("skip");
            entry["reason"] = json!("side-effect tool (binds resources)");
            skip += 1;
            report_lines.push(entry);
            continue;
        }

        let arguments: Value = serde_json::from_str(fix.args_json)
            .with_context(|| format!("fixture {} args_json", fix.name))?;
        let id = next_id;
        next_id += 1;

        let started = Instant::now();
        send(
            &mut writer,
            &json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "tools/call",
                "params": { "name": fix.name, "arguments": arguments }
            }),
        )
        .await?;
        let response = match recv(&mut reader, timeout).await {
            Ok(v) => v,
            Err(err) => {
                let elapsed = started.elapsed().as_millis();
                latencies_ms.push(elapsed);
                entry["status"] = json!("fail");
                entry["latency_ms"] = json!(elapsed);
                entry["reason"] = json!(format!("transport error: {err}"));
                fail += 1;
                report_lines.push(entry);
                continue;
            },
        };
        let elapsed = started.elapsed().as_millis();
        latencies_ms.push(elapsed);
        entry["latency_ms"] = json!(elapsed);

        let is_error =
            response.pointer("/result/isError").and_then(Value::as_bool).unwrap_or(false);
        let jsonrpc_error = response.get("error").cloned();

        if let Some(err) = jsonrpc_error {
            entry["status"] = json!("fail");
            entry["reason"] = json!(format!("jsonrpc error: {err}"));
            fail += 1;
        } else if is_error {
            let snippet = response
                .pointer("/result/content/0/text")
                .and_then(Value::as_str)
                .unwrap_or("")
                .chars()
                .take(160)
                .collect::<String>();
            entry["error_snippet"] = json!(snippet);
            if fix.network_dependent || fix.daemon_required || fix.creds_required {
                entry["status"] = json!("expected_error");
                entry["reason"] = json!(classify_expected(fix));
                expected_err += 1;
            } else {
                entry["status"] = json!("fail");
                entry["reason"] = json!("tool reported isError=true with no expected-error flag");
                fail += 1;
            }
        } else {
            entry["status"] = json!("pass");
            pass += 1;
        }
        report_lines.push(entry);
    }

    // ── 5. summary ───────────────────────────────────────────────────
    let (p50, p95) = percentiles(&mut latencies_ms);
    let summary = json!({
        "type": "summary",
        "pass": pass,
        "expected_error": expected_err,
        "skip": skip,
        "fail": fail,
        "calls": pass + expected_err + fail,
        "latency_ms_p50": p50,
        "latency_ms_p95": p95,
    });
    report_lines.push(summary.clone());

    // ── 6. drop stdin and wait for child exit (best-effort) ─────────
    drop(writer);
    let _ = wait_with_timeout(&mut child, std::time::Duration::from_secs(3)).await;

    // ── 7. write NDJSON report + stdout ──────────────────────────────
    write_report(&args.report, &report_lines).await?;
    for line in &report_lines {
        println!("{line}");
    }

    if fail > 0 {
        bail!(
            "smoke FAIL: {fail} unexpected failure(s) out of {} call(s); see {}",
            pass + expected_err + fail,
            args.report.display()
        );
    }
    Ok(())
}

fn classify_expected(fix: &Fixture) -> &'static str {
    if fix.daemon_required {
        "bxc daemon dependent (BXC_DAEMON_URL unreachable or returned non-2xx)"
    } else if fix.creds_required {
        "credentials required (e.g. ELEVENLABS_API_KEY unset)"
    } else {
        "network dependent (upstream unreachable or returned non-2xx)"
    }
}

fn percentiles(samples: &mut [u128]) -> (Option<u128>, Option<u128>) {
    if samples.is_empty() {
        return (None, None);
    }
    samples.sort_unstable();
    let n = samples.len();
    let pick = |q: f64| {
        let idx = ((q * (n as f64 - 1.0)).round() as usize).min(n - 1);
        samples[idx]
    };
    (Some(pick(0.50)), Some(pick(0.95)))
}

async fn send<W: AsyncWriteExt + Unpin>(w: &mut W, msg: &Value) -> Result<()> {
    let mut s = serde_json::to_vec(msg)?;
    s.push(b'\n');
    w.write_all(&s).await?;
    w.flush().await?;
    Ok(())
}

async fn recv<R: AsyncBufReadExt + Unpin>(
    r: &mut tokio::io::Lines<R>,
    timeout: std::time::Duration,
) -> Result<Value> {
    loop {
        let line = tokio::time::timeout(timeout, r.next_line())
            .await
            .map_err(|_| anyhow!("timeout waiting for server response after {:?}", timeout))?
            .map_err(anyhow::Error::from)?
            .ok_or_else(|| anyhow!("server closed stdout unexpectedly"))?;
        if line.trim().is_empty() {
            continue;
        }
        let v: Value = serde_json::from_str(&line)
            .with_context(|| format!("non-JSON line on server stdout: {line}"))?;
        // Skip notifications / log frames (no `id` field) — wait for the response.
        if v.get("id").is_some() || v.get("error").is_some() {
            return Ok(v);
        }
    }
}

async fn wait_with_timeout(child: &mut Child, dur: std::time::Duration) -> Result<()> {
    match tokio::time::timeout(dur, child.wait()).await {
        Ok(Ok(_status)) => Ok(()),
        Ok(Err(e)) => Err(e.into()),
        Err(_) => {
            let _ = child.kill().await;
            Ok(())
        },
    }
}

fn resolve_binary(explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(p) = explicit {
        if p.exists() {
            return Ok(p.to_path_buf());
        }
        bail!("--bin {} does not exist", p.display());
    }
    if let Ok(p) = std::env::var("APHRODY_MCP_BIN") {
        let p = PathBuf::from(p);
        if p.exists() {
            return Ok(p);
        }
    }
    let exe_name = if cfg!(windows) { "aphrody-mcp.exe" } else { "aphrody-mcp" };
    if let Ok(path) = which(exe_name) {
        return Ok(path);
    }
    if let Some(home) = home_dir() {
        let p = home.join(".local").join("bin").join(exe_name);
        if p.exists() {
            return Ok(p);
        }
    }
    for triple in [
        "x86_64-pc-windows-msvc",
        "x86_64-unknown-linux-gnu",
        "aarch64-unknown-linux-gnu",
        "aarch64-apple-darwin",
    ] {
        let p = PathBuf::from("target").join(triple).join("release").join(exe_name);
        if p.exists() {
            return Ok(p);
        }
    }
    let p = PathBuf::from("target").join("release").join(exe_name);
    if p.exists() {
        return Ok(p);
    }
    bail!(
        "aphrody-mcp binary not found. Set --bin <path>, $APHRODY_MCP_BIN, or build it with \
         `cargo build --release -p google_mcp --bin aphrody-mcp`."
    )
}

fn which(name: &str) -> Result<PathBuf> {
    let path_var = std::env::var_os("PATH").ok_or_else(|| anyhow!("PATH not set"))?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    bail!("{name} not found in PATH")
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(PathBuf::from)
}

async fn write_report(path: &Path, lines: &[Value]) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            tokio::fs::create_dir_all(parent).await.ok();
        }
    }
    let mut buf = String::with_capacity(lines.len() * 128);
    for v in lines {
        buf.push_str(&v.to_string());
        buf.push('\n');
    }
    tokio::fs::write(path, buf).await?;
    Ok(())
}
