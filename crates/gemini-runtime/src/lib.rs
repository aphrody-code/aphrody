// SPDX-License-Identifier: Apache-2.0
//! Gemini CLI runtime adapter.
//!
//! Cross-platform adapter that locates the `gemini` binary on `PATH`, queries
//! its version, and spawns conversation turns using
//! `--output-format stream-json --yolo`.  Prompts are written to the child
//! process via **stdin** to stay well under the 32 KB `CreateProcess` argument
//! limit on Windows.
//!
//! # Usage
//!
//! ```no_run
//! use futures_util::StreamExt as _;
//! use gemini_runtime::{TurnOptions, detect};
//!
//! # async fn run() {
//! let runtime = detect().await.expect("gemini not found on PATH");
//! let opts = TurnOptions::default();
//! let mut stream = runtime.turn("Explain io_uring in one sentence.", &opts).await;
//! while let Some(event) = stream.next().await {
//!     println!("{event:?}");
//! }
//! # }
//! ```

// `auth.rs` tests need `unsafe { std::env::set_var }` (nightly Rust mandates
// unsafe for env-var mutation since 1.83). Confine the relaxation to test
// builds; the non-test runtime surface remains 100% safe code.
#![cfg_attr(not(test), forbid(unsafe_code))]

pub mod auth;
pub mod mcp_client;
pub mod mcp_config;
pub mod tools;
pub mod web_search;

pub use auth::{
    AuthError, GeminiCliCredentials, TokenResponse, api_key_headers, default_creds_path,
    exchange_code_for_tokens, refresh_tokens, request_token_grant, resolve_client_id_from_env,
    resolve_client_secret_from_env,
};
pub use mcp_client::{
    DEFAULT_CONNECT_TIMEOUT as MCP_DEFAULT_CONNECT_TIMEOUT,
    DEFAULT_RPC_TIMEOUT as MCP_DEFAULT_RPC_TIMEOUT, MCP_PROTOCOL_VERSION, McpClient, McpError,
    McpTool, ServerInfo as McpServerInfo,
};
pub use mcp_config::{
    ConfigError as McpConfigError, McpConfig, McpServerConfig, McpTransportConfig,
    default_config_path as mcp_default_config_path, load_default_mcp_config, load_mcp_config,
    parse_mcp_config,
};
pub use tools::{McpProxyTool, Tool, ToolDescriptor, ToolError, ToolRegistry, WebSearchTool};
pub use web_search::{
    Citation, DEFAULT_BASE_URL as WEB_SEARCH_DEFAULT_BASE_URL,
    DEFAULT_MODEL as WEB_SEARCH_DEFAULT_MODEL, SearchResults, TimeRange, WebSearchConfig,
    WebSearchError, build_endpoint_url, build_request_body, parse_response, web_search,
};

use std::{path::PathBuf, process::Stdio};

use futures_util::{
    StreamExt as _,
    stream::{self, BoxStream},
};
use serde::Deserialize;
use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt as _, AsyncWriteExt as _, BufReader},
    process::Command,
};

// ---------------------------------------------------------------------------
// Public constants
// ---------------------------------------------------------------------------

/// Fallback model list mirroring `geminiAgentDef.fallbackModels` in the
/// upstream TypeScript daemon (`apps/daemon/src/runtimes/defs/gemini.ts`).
///
/// Each tuple is `(id, label)`.  The first entry is the "default" model token
/// understood by `buildArgs` (maps to no `--model` flag).
pub const FALLBACK_MODELS: &[(&str, &str)] = &[
    ("default", "Default"),
    ("gemini-3-pro-preview", "gemini-3-pro-preview"),
    ("gemini-3-flash-preview", "gemini-3-flash-preview"),
    ("gemini-2.5-pro", "gemini-2.5-pro"),
    ("gemini-2.5-flash", "gemini-2.5-flash"),
    ("gemini-2.5-flash-lite", "gemini-2.5-flash-lite"),
];

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// All errors that can originate from this crate.
#[derive(Debug, Error)]
pub enum RuntimeError {
    /// The `gemini` binary was not found on `PATH`.
    #[error("gemini binary not found on PATH: {0}")]
    NotFound(#[from] which::Error),

    /// An I/O error occurred while spawning or communicating with the child.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The `gemini --version` command produced output that could not be parsed.
    #[error("could not parse gemini version from output: {raw:?}")]
    VersionParse { raw: String },

    /// The child process exited with a non-zero status code.
    #[error("gemini exited with status {code}: {stderr}")]
    ChildFailed { code: i32, stderr: String },
}

// ---------------------------------------------------------------------------
// Stream event types
// ---------------------------------------------------------------------------

/// A single event emitted by a `gemini --output-format stream-json` session.
///
/// The stream-json schema used by the Gemini CLI NDJSON output is modelled
/// here; unknown top-level `type` values are silently forwarded as
/// [`StreamEvent::Unknown`] so that callers remain forward-compatible.
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// A chunk of assistant text.
    Text(String),

    /// A tool/function call emitted by the model.
    ToolCall {
        /// Name of the function to invoke.
        name: String,
        /// Raw JSON arguments object.
        arguments: serde_json::Value,
        /// Optional call identifier supplied by the model.
        call_id: Option<String>,
    },

    /// The turn has ended successfully.  The `model` field reflects which
    /// model was used when `--model` was not supplied (i.e. the server-side
    /// default).
    Done { model: Option<String> },

    /// A structured error reported inside the stream (not an I/O failure).
    Error { message: String, code: Option<i32> },

    /// A raw JSON envelope whose `type` field is not yet handled by this
    /// crate.  The full parsed value is forwarded unchanged.
    Unknown(serde_json::Value),
}

// ---------------------------------------------------------------------------
// Intermediate deserialization helpers (crate-private)
// ---------------------------------------------------------------------------

/// Raw envelope as emitted by `gemini --output-format stream-json`.
///
/// The Gemini CLI writes one JSON object per line (NDJSON).  The `type` field
/// discriminates between the different payload shapes.
#[derive(Deserialize)]
struct RawEnvelope {
    #[serde(rename = "type")]
    kind: String,
    #[serde(flatten)]
    rest: serde_json::Value,
}

/// Attempts to convert a raw envelope into a typed [`StreamEvent`].
fn parse_envelope(raw: RawEnvelope) -> StreamEvent {
    match raw.kind.as_str() {
        "content" | "text" => {
            // Gemini CLI emits `{"type":"content","content":"..."}` or
            // `{"type":"text","text":"..."}` depending on the build.
            let text = raw
                .rest
                .get("content")
                .or_else(|| raw.rest.get("text"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned();
            StreamEvent::Text(text)
        },
        "tool_call" | "function_call" => {
            let name = raw.rest.get("name").and_then(|v| v.as_str()).unwrap_or("").to_owned();
            let arguments = raw
                .rest
                .get("arguments")
                .or_else(|| raw.rest.get("args"))
                .cloned()
                .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
            let call_id = raw
                .rest
                .get("id")
                .or_else(|| raw.rest.get("call_id"))
                .and_then(|v| v.as_str())
                .map(str::to_owned);
            StreamEvent::ToolCall { name, arguments, call_id }
        },
        "done" | "finish" | "end" => {
            let model = raw.rest.get("model").and_then(|v| v.as_str()).map(str::to_owned);
            StreamEvent::Done { model }
        },
        "error" => {
            let message = raw
                .rest
                .get("message")
                .or_else(|| raw.rest.get("error"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error")
                .to_owned();
            let code = raw.rest.get("code").and_then(|v| v.as_i64()).map(|c| c as i32);
            StreamEvent::Error { message, code }
        },
        "result" if raw.rest.get("status").and_then(|v| v.as_str()) == Some("error") => {
            let message = raw.rest.get("error")
                .and_then(|e| e.get("message"))
                .and_then(|v| v.as_str())
                .unwrap_or("API result error")
                .to_owned();
            StreamEvent::Error { message, code: None }
        },
        _ => {
            // Reconstruct the original value so callers can inspect it.
            let mut obj = match raw.rest {
                serde_json::Value::Object(m) => m,
                other => {
                    // Defensive: wrap a non-object rest in a new map.
                    let mut m = serde_json::Map::new();
                    m.insert("_rest".to_owned(), other);
                    m
                },
            };
            obj.insert("type".to_owned(), serde_json::Value::String(raw.kind));
            StreamEvent::Unknown(serde_json::Value::Object(obj))
        },
    }
}

// ---------------------------------------------------------------------------
// TurnOptions
// ---------------------------------------------------------------------------

/// Options that control how a single `gemini` turn is spawned.
#[derive(Debug, Clone, Default)]
pub struct TurnOptions {
    /// Override the model used for this turn.  When `None` or when the value
    /// is the string `"default"`, no `--model` flag is appended, letting the
    /// CLI use its server-side default.
    pub model: Option<String>,

    /// Absolute paths to images to attach.  Each path is appended as
    /// `--image <path>` after the base flags.
    pub image_paths: Vec<PathBuf>,

    /// Arbitrary extra flags forwarded verbatim to the gemini CLI invocation,
    /// appended after all other arguments.
    pub extra_args: Vec<String>,
}

// ---------------------------------------------------------------------------
// GeminiRuntime
// ---------------------------------------------------------------------------

/// A handle to a detected and version-verified Gemini CLI installation.
///
/// Obtain one via [`detect()`].
#[derive(Debug, Clone)]
pub struct GeminiRuntime {
    /// Absolute path to the `gemini` binary.
    pub bin: PathBuf,
    /// Version string returned by `gemini --version`.
    pub version: String,
}

impl GeminiRuntime {
    /// Spawn a single turn and return a stream of [`StreamEvent`] items.
    ///
    /// The prompt is written to the child's **stdin** and stdin is closed
    /// before the first byte of stdout is read.  This avoids the Windows
    /// `CreateProcess` 32 KB command-line limit for large prompts.
    ///
    /// The environment variable `GEMINI_CLI_TRUST_WORKSPACE=true` is injected
    /// to suppress interactive workspace-trust prompts without relying on
    /// `--skip-trust`, which is not accepted by all Gemini CLI builds.
    ///
    /// # Errors
    ///
    /// Individual stream items never return `Err`; instead, I/O failures and
    /// parse errors are surfaced as [`StreamEvent::Error`] items so that
    /// callers can continue consuming the stream after a transient error.
    pub async fn turn<'a>(
        &'a self,
        prompt: &'a str,
        opts: &'a TurnOptions,
    ) -> BoxStream<'a, StreamEvent> {
        let result = self.spawn_turn(prompt, opts).await;

        match result {
            Err(e) => {
                let event = StreamEvent::Error { message: e.to_string(), code: None };
                stream::once(async move { event }).boxed()
            },
            Ok(child_stream) => child_stream,
        }
    }

    /// Internal: build the `Command`, write stdin, and return the stdout
    /// NDJSON stream.
    async fn spawn_turn<'a>(
        &'a self,
        prompt: &'a str,
        opts: &'a TurnOptions,
    ) -> Result<BoxStream<'a, StreamEvent>, RuntimeError> {
        let mut args: Vec<String> =
            vec!["--output-format".into(), "stream-json".into(), "--yolo".into()];

        // Append --model only when a non-default model is requested.
        if let Some(ref model) = opts.model {
            if model != "default" && !model.is_empty() {
                args.push("--model".into());
                args.push(model.clone());
            }
        }

        // Attach image paths if any.
        for path in &opts.image_paths {
            args.push("--image".into());
            args.push(path.display().to_string());
        }

        // Extra caller-supplied flags.
        args.extend(opts.extra_args.iter().cloned());

        let mut cmd = Command::new(&self.bin);
        cmd.args(&args)
            .env("GEMINI_CLI_TRUST_WORKSPACE", "true")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            // Capture stderr so we can include it in error messages.
            .stderr(Stdio::piped())
            // Do not inherit the parent's terminal handle — required for
            // `--yolo` / non-interactive mode to work correctly.
            .kill_on_drop(true);

        let mut child = cmd.spawn()?;

        // Write the prompt to stdin, then close the pipe so the CLI does not
        // block waiting for more input.
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(prompt.as_bytes()).await?;
            // Explicit drop closes the pipe; no need for shutdown() here.
        }

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| std::io::Error::other("failed to open child stdout"))?;

        let reader = BufReader::new(stdout);
        let lines = reader.lines();

        // Convert the async lines iterator into a stream of StreamEvent.
        let event_stream = stream::unfold(lines, |mut lines| async move {
            match lines.next_line().await {
                Err(e) => {
                    let event = StreamEvent::Error {
                        message: format!("stdout read error: {e}"),
                        code: None,
                    };
                    Some((event, lines))
                },
                Ok(None) => None,
                Ok(Some(line)) => {
                    let line = line.trim().to_owned();
                    if line.is_empty() {
                        // Skip blank separator lines.
                        // We recurse by yielding a sentinel and immediately
                        // returning to the unfold loop — easier than async
                        // recursion: produce nothing, keep going.
                        // Since unfold expects `Option<(item, state)>`, we
                        // emit an Unknown with an empty object as a no-op
                        // marker; callers should filter `Unknown({})` if they
                        // care. Alternatively we could use a custom loop.
                        // This path is rare enough in practice.
                        Some((
                            StreamEvent::Unknown(serde_json::Value::Object(serde_json::Map::new())),
                            lines,
                        ))
                    } else {
                        let event = match serde_json::from_str::<RawEnvelope>(&line) {
                            Ok(envelope) => parse_envelope(envelope),
                            Err(e) => StreamEvent::Error {
                                message: format!("JSON parse error: {e}; line={line}"),
                                code: None,
                            },
                        };
                        Some((event, lines))
                    }
                },
            }
        });

        Ok(event_stream.boxed())
    }
}

// ---------------------------------------------------------------------------
// detect() — resolution chain
// ---------------------------------------------------------------------------

/// Environment override : when set to a non-empty path, takes precedence over
/// every other resolution step. Mirrors `aphrody-gateway::gemini_cli::ENV_GEMINI_BIN`.
pub const ENV_GEMINI_BIN: &str = "APHRODY_GEMINI_BIN";

/// Resolve the Gemini binary path **without** invoking it.
///
/// Resolution order (first match wins) :
/// 1. `$APHRODY_GEMINI_BIN` env (must point to an existing file).
/// 2. Sibling of `std::env::current_exe()` named `gemini[.exe]` — picks up the
///    binary that `aphrody self bootstrap` installs next to `aphrody.exe`.
/// 3. Walk up from `$CWD` looking for the in-tree fork bundle at
///    `packages/gemini-cli/bundle/gemini[.exe|.js]` — the canonical aphrody-owned
///    Gemini source (cf. `project_aphrody_owned_tools` + CLAUDE.md §0.45).
/// 4. `which("gemini")` — PATH lookup (upstream global install).
///
/// Returns the resolved path. The caller is responsible for spawning it.
///
/// # Errors
///
/// [`RuntimeError::NotFound`] when no step yields a path.
pub fn resolve_bin() -> Result<PathBuf, RuntimeError> {
    // Step 1 : explicit env override.
    if let Ok(explicit) = std::env::var(ENV_GEMINI_BIN) {
        let trimmed = explicit.trim();
        if !trimmed.is_empty() {
            let p = PathBuf::from(trimmed);
            if p.exists() {
                return Ok(p);
            }
        }
    }

    // Step 2 : sibling of the running aphrody binary.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for cand in ["gemini.exe", "gemini"] {
                let p = dir.join(cand);
                if p.exists() {
                    return Ok(p);
                }
            }
        }
    }

    // Step 3 : walk up from CWD looking for the in-tree fork bundle.
    if let Ok(cwd) = std::env::current_dir() {
        let mut here: Option<&std::path::Path> = Some(cwd.as_path());
        while let Some(dir) = here {
            for cand in [
                "packages/gemini-cli/bundle/gemini.exe",
                "packages/gemini-cli/bundle/gemini",
                "packages/gemini-cli/bundle/gemini.js",
            ] {
                let p = dir.join(cand);
                if p.exists() {
                    return Ok(p);
                }
            }
            here = dir.parent();
        }
    }

    // Step 4 : PATH lookup.
    Ok(which::which("gemini")?)
}

/// Locate the `gemini` binary via [`resolve_bin`] and query its version.
///
/// Returns a [`GeminiRuntime`] on success or a [`RuntimeError`] when the
/// binary is absent or when `gemini --version` fails.
///
/// # Errors
///
/// - [`RuntimeError::NotFound`] — no resolution step yielded a binary.
/// - [`RuntimeError::Io`] — spawning or reading from the version subprocess failed.
/// - [`RuntimeError::VersionParse`] — the subprocess produced no usable version string.
/// - [`RuntimeError::ChildFailed`] — the subprocess exited non-zero.
pub async fn detect() -> Result<GeminiRuntime, RuntimeError> {
    let bin = resolve_bin()?;

    let output = Command::new(&bin)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;

    if !output.status.success() {
        let code = output.status.code().unwrap_or(-1);
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        return Err(RuntimeError::ChildFailed { code, stderr });
    }

    // Accept the first non-empty line from either stdout or stderr; some
    // Gemini CLI builds print the version to stderr.
    let raw = String::from_utf8_lossy(&output.stdout).into_owned();
    let version = raw
        .lines()
        .chain(String::from_utf8_lossy(&output.stderr).lines())
        .map(str::trim)
        .find(|l| !l.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| RuntimeError::VersionParse { raw: raw.clone() })?;

    Ok(GeminiRuntime { bin, version })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: build a raw JSON line and parse it into a StreamEvent.
    fn parse_line(json: &str) -> StreamEvent {
        let envelope: RawEnvelope = serde_json::from_str(json).expect("valid JSON");
        parse_envelope(envelope)
    }

    #[test]
    fn deserialise_text_event() {
        let event = parse_line(r#"{"type":"content","content":"Hello, world!"}"#);
        match event {
            StreamEvent::Text(t) => assert_eq!(t, "Hello, world!"),
            other => panic!("expected Text, got {other:?}"),
        }
    }

    #[test]
    fn deserialise_text_event_alt_key() {
        // Some Gemini CLI builds use `"type":"text"` with key `"text"`.
        let event = parse_line(r#"{"type":"text","text":"alt key variant"}"#);
        match event {
            StreamEvent::Text(t) => assert_eq!(t, "alt key variant"),
            other => panic!("expected Text, got {other:?}"),
        }
    }

    #[test]
    fn deserialise_tool_call_event() {
        let event = parse_line(
            r#"{"type":"tool_call","name":"read_file","arguments":{"path":"/tmp/x"},"id":"tc-1"}"#,
        );
        match event {
            StreamEvent::ToolCall { name, arguments, call_id } => {
                assert_eq!(name, "read_file");
                assert_eq!(arguments["path"], "/tmp/x");
                assert_eq!(call_id.as_deref(), Some("tc-1"));
            },
            other => panic!("expected ToolCall, got {other:?}"),
        }
    }

    #[test]
    fn deserialise_done_event() {
        let event = parse_line(r#"{"type":"done","model":"gemini-2.5-pro"}"#);
        match event {
            StreamEvent::Done { model } => {
                assert_eq!(model.as_deref(), Some("gemini-2.5-pro"));
            },
            other => panic!("expected Done, got {other:?}"),
        }
    }

    #[test]
    fn deserialise_done_no_model() {
        let event = parse_line(r#"{"type":"finish"}"#);
        match event {
            StreamEvent::Done { model } => assert!(model.is_none()),
            other => panic!("expected Done, got {other:?}"),
        }
    }

    #[test]
    fn deserialise_error_event() {
        let event = parse_line(r#"{"type":"error","message":"quota exceeded","code":429}"#);
        match event {
            StreamEvent::Error { message, code } => {
                assert_eq!(message, "quota exceeded");
                assert_eq!(code, Some(429));
            },
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn deserialise_unknown_event() {
        let event = parse_line(r#"{"type":"future_type","payload":42}"#);
        match event {
            StreamEvent::Unknown(v) => {
                assert_eq!(v["type"], "future_type");
                assert_eq!(v["payload"], 42);
            },
            other => panic!("expected Unknown, got {other:?}"),
        }
    }

    #[test]
    fn fallback_models_count_and_first() {
        assert_eq!(FALLBACK_MODELS.len(), 6);
        assert_eq!(FALLBACK_MODELS[0].0, "default");
        // Gemini 3 entries must be present.
        let ids: Vec<&str> = FALLBACK_MODELS.iter().map(|m| m.0).collect();
        assert!(ids.contains(&"gemini-3-pro-preview"));
        assert!(ids.contains(&"gemini-3-flash-preview"));
        assert!(ids.contains(&"gemini-2.5-flash-lite"));
    }

    /// When no `gemini` binary is resolvable, the detector must return
    /// `RuntimeError::NotFound`, not panic. Hermetic: every `resolve_bin`
    /// input is neutralised so the binary is genuinely absent regardless of
    /// the host's installs (a half-installed global `gemini-cli` would
    /// otherwise resolve via the in-tree bundle walk-up and fail differently).
    #[tokio::test]
    async fn detect_absent_binary_returns_not_found() {
        let tmp = std::env::temp_dir().join(format!(
            "aphrody-gemini-absent-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&tmp).expect("create temp dir");
        // SAFETY: nextest runs each test in its own process, so there is no
        // other thread concurrently reading/writing the environment here.
        unsafe {
            std::env::remove_var(ENV_GEMINI_BIN); // step 1: no override
            std::env::set_var("PATH", &tmp); // step 4: empty dir → which fails
            // step 3: a temp-dir CWD has no `packages/gemini-cli/bundle` ancestor
            std::env::set_current_dir(&tmp).expect("cd to temp dir");
        }
        match detect().await {
            Err(RuntimeError::NotFound(_)) => {} // expected
            other => panic!("expected NotFound for an absent binary, got {other:?}"),
        }
    }
}
