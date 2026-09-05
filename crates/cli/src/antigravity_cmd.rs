// SPDX-License-Identifier: Apache-2.0
//! `aphrody antigravity …` subcommands — scriptable, non-interactive access to
//! the native Google AI Ultra / Gemini surface (WS3).
//!
//! Every variant drives the already-committed [`AntigravityClient`] surface,
//! which returns [`serde_json::Value`]. The client reads the user's OAuth token
//! at **runtime** from the platform credential store (Windows Credential
//! Manager entry `gemini:antigravity`); **no secret is ever embedded** in the
//! binary. On platforms without a credential store the underlying SDK returns
//! [`SdkError::Unsupported`], which is surfaced verbatim as a `miette` report.
//!
//! Output is JSON on stdout so it can be piped into `jq`. The `--json` flag is
//! accepted on the read-only variants for symmetry with the rest of the CLI;
//! because these RPCs return structured data, output is JSON either way, with
//! pretty-printing enabled when `--json` is set.
//!
//! This module is native-only (`cfg(not(target_arch = "wasm32"))`): it builds a
//! `reqwest::Client` (rustls/ring) and reads the credential store, neither of
//! which links on wasm32.

use antigravity_sdk::{
    client::AntigravityClient,
    endpoints::{CloudCodeEndpoint, METHOD_GENERATE_CONTENT},
    error::SdkError,
    models::{ClientMetadata, OnboardUserRequest},
};
use serde_json::json;

/// Default Gemini model used by `aphrody antigravity chat` when `--model` is
/// not supplied. The bare model id (no `models/` prefix) is what the Cloud Code
/// `v1internal:generateContent` envelope expects.
///
/// `gemini-3-flash-preview` is the wire id `agy.exe` sends for its "Gemini 3.5
/// Flash" label (no literal `gemini-3.5-flash` exists on Cloud Code — it 404s,
/// living only on the public `generativelanguage` API). Paired with the Daily
/// endpoint ([`AGY_CHAT_ENDPOINT`]) it has a usable quota.
const DEFAULT_GEMINI_MODEL: &str = "gemini-3-flash-preview";

/// Cloud Code endpoint for `aphrody antigravity chat` — the **Daily** host that
/// `agy.exe` uses; it serves the Gemini 3 preview models with a far more
/// generous quota than Prod.
const AGY_CHAT_ENDPOINT: CloudCodeEndpoint = CloudCodeEndpoint::Daily;

/// Cloud Code endpoint selector for `aphrody antigravity cloud-code`.
#[derive(Debug, Clone, clap::ValueEnum, Default)]
pub(crate) enum EndpointChoice {
    /// Production endpoint (`cloudcode-pa.googleapis.com`).
    #[default]
    Prod,
    /// Daily / experimental endpoint (`daily-cloudcode-pa.googleapis.com`).
    Daily,
}

impl From<EndpointChoice> for CloudCodeEndpoint {
    fn from(c: EndpointChoice) -> Self {
        match c {
            EndpointChoice::Prod => CloudCodeEndpoint::Prod,
            EndpointChoice::Daily => CloudCodeEndpoint::Daily,
        }
    }
}

/// Subcommands of `aphrody antigravity`.
#[derive(clap::Subcommand, Debug, Clone)]
pub(crate) enum AntigravityAction {
    /// List the models available to the signed-in account / tier
    /// (`v1internal:fetchAvailableModels`).
    Models {
        /// Pretty-print the JSON response (default: compact one-line).
        #[arg(long)]
        json: bool,
    },
    /// Print the signed-in user's profile (email + name) from Google's OpenID
    /// `userinfo` endpoint.
    Whoami {
        /// Pretty-print the JSON response (default: compact one-line).
        #[arg(long)]
        json: bool,
    },
    /// Bootstrap the Code Assist session — project / tier / entitlements
    /// (`v1internal:loadCodeAssist`).
    Load {
        /// Pretty-print the JSON response (default: compact one-line).
        #[arg(long)]
        json: bool,
    },
    /// Send a single prompt to a Gemini model and print the raw
    /// `generateContent` response as JSON.
    Chat {
        /// Bare Gemini model id (e.g. `gemini-2.0-flash`). Defaults to
        /// `gemini-2.0-flash` when omitted.
        #[arg(long)]
        model: Option<String>,
        /// Prompt text to send as the single user turn.
        #[arg(long)]
        prompt: String,
    },
    /// Onboard the signed-in user to a Code Assist tier
    /// (`v1internal:onboardUser`). Returns the long-running operation JSON
    /// (`done` field; `done = false` while provisioning is ongoing).
    ///
    /// Example:
    ///   aphrody antigravity onboard --tier free-tier --json
    Onboard {
        /// Tier id to onboard onto (e.g. `free-tier`, `standard-tier`).
        /// Use `aphrody antigravity load --json | jq '.allowedTiers[].id'`
        /// to discover available tier ids.
        #[arg(long)]
        tier: String,
        /// Optional Cloud AI Companion project id (required by some tiers).
        #[arg(long)]
        project: Option<String>,
        /// Pretty-print the JSON response (default: compact one-line).
        #[arg(long)]
        json: bool,
    },
    /// Exchange the stored refresh token for a fresh access token and print
    /// a non-sensitive confirmation (lengths + expiry hint only — no token
    /// bytes are printed).
    ///
    /// Use this when the current access token has expired and you want to
    /// rotate it without going through the full interactive login flow.
    Refresh {
        /// Pretty-print the JSON confirmation object.
        #[arg(long)]
        json: bool,
    },
    /// Interactively sign in via the Google OAuth 2.0 loopback PKCE flow.
    ///
    /// Opens a browser, binds an ephemeral listener on `127.0.0.1:9109`,
    /// accepts the single `/oauth-callback?code=…` redirect, exchanges the
    /// code for an OAuth token, and persists it:
    ///   - Windows: Credential Manager `gemini:antigravity`
    ///   - Linux/macOS: `~/.config/aphrody/antigravity-token.json` (0600)
    ///
    /// Non-interactive uses must call `refresh` instead (requires an existing
    /// refresh token in the credential store).
    Login,
    /// Inspect the token stored in the credential store.
    ///
    /// Prints a non-sensitive summary: access-token length prefix (first 10
    /// chars) + whether a refresh token is present + expiry hint.
    /// The full token bytes are NEVER printed.
    ///
    /// Example:
    ///   aphrody antigravity token-info --json | jq '.has_refresh_token'
    TokenInfo {
        /// Pretty-print the JSON summary.
        #[arg(long)]
        json: bool,
    },
    /// Inspect the Antigravity config file (`~/.gemini/config/config.json`).
    ///
    /// Prints permissions, browser JS execution policy, and any extra fields.
    ///
    /// Example:
    ///   aphrody antigravity config --json | jq '.permissions'
    Config {
        /// Pretty-print the JSON output (default: compact one-line).
        #[arg(long)]
        json: bool,
    },
    /// Decode a `state.vscdb` `antigravityUnifiedStateSync.*` blob.
    ///
    /// Accepts a base64-encoded value (standard or URL-safe alphabet) and
    /// reports: raw byte length, any embedded JSON object, and a non-sensitive
    /// base32 hint. Token bytes are NEVER surfaced.
    ///
    /// Example:
    ///   aphrody antigravity state-sync --value "$(sqlite3 state.vscdb \
    ///     'SELECT value FROM ItemTable WHERE key LIKE "%oauthToken%"')" --json
    StateSync {
        /// Base64-encoded UnifiedStateSync value to decode.
        #[arg(long)]
        value: String,
        /// Pretty-print the JSON summary.
        #[arg(long)]
        json: bool,
    },
    /// Summarise a JSON dump of the `state.vscdb` `ItemTable`.
    ///
    /// `INPUT` must be a JSON object mapping keys to their string values
    /// (e.g. the output of `sqlite3 -json state.vscdb
    /// 'SELECT key, value FROM ItemTable'` piped through `jq '[.[] | {key: .key, value: .value}] |
    /// add'`).
    ///
    /// Prints a JSON array of structural entries (key, value_len,
    /// is_unified_state_sync, looks_like_signin_state) — no value bytes.
    ///
    /// Example:
    ///   cat dump.json | aphrody antigravity item-table --json
    ItemTable {
        /// Path to a JSON dump of the ItemTable (or `-` to read from stdin).
        #[arg(long, default_value = "-")]
        input: String,
        /// Pretty-print the JSON array.
        #[arg(long)]
        json: bool,
    },
    /// Generic Cloud Code `v1internal` RPC call.
    ///
    /// Composes the URL from the `--endpoint` and `--method` arguments and
    /// POSTs the `--body` JSON. This is the low-level escape hatch for
    /// methods that do not yet have a typed sub-command.
    ///
    /// Example:
    ///   aphrody antigravity cloud-code --method /v1internal:fetchAvailableModels \
    ///     --body '{}' --json | jq '.models[].name'
    CloudCode {
        /// Target endpoint: `prod` (default) or `daily`.
        #[arg(long, value_enum, default_value_t = EndpointChoice::Prod)]
        endpoint: EndpointChoice,
        /// Method path (e.g. `/v1internal:loadCodeAssist`).
        #[arg(long)]
        method: String,
        /// JSON request body (default: empty object `{}`).
        #[arg(long, default_value = "{}")]
        body: String,
        /// Pretty-print the JSON response.
        #[arg(long)]
        json: bool,
    },
}

/// Build an authenticated [`AntigravityClient`], installing the rustls `ring`
/// `CryptoProvider` first (rustls 0.23 requirement, cf. CLAUDE.md §7).
///
/// The install is idempotent and best-effort: if a provider is already
/// installed (e.g. by `main`), the returned error is ignored.
///
/// # Errors
///
/// Propagates the [`SdkError`] from
/// [`AntigravityClient::from_credential_manager`] (mapped to a `miette` report
/// by the caller) when the token cannot be read.
fn build_client() -> Result<AntigravityClient, SdkError> {
    // Idempotent: ignore "already installed" — `main` may have installed one.
    let _ = rustls::crypto::ring::default_provider().install_default();
    AntigravityClient::from_credential_manager()
}

/// Render a [`serde_json::Value`] to stdout, pretty when `pretty` is set.
///
/// # Errors
///
/// Returns a `miette` report if JSON serialization fails (effectively never
/// for a value that already deserialized cleanly).
fn print_value(value: &serde_json::Value, pretty: bool) -> miette::Result<()> {
    let rendered =
        if pretty { serde_json::to_string_pretty(value) } else { serde_json::to_string(value) }
            .map_err(|e| miette::miette!("failed to encode JSON response: {e}"))?;
    println!("{rendered}");
    Ok(())
}

/// Map an [`SdkError`] to a `miette` report with a context-bearing message.
fn map_sdk_err(action: &str, err: SdkError) -> miette::Report {
    miette::miette!("antigravity {action} failed: {err}")
}

/// Dispatch a single `aphrody antigravity` action.
///
/// Constructs the authenticated client (reading the token from the credential
/// store at runtime), invokes the matching SDK method, and prints the JSON
/// response on stdout.
///
/// # Errors
///
/// * Token acquisition failure (no credential, unsupported platform) -> [`SdkError`] mapped to a
///   `miette` report.
/// * Network / non-2xx HTTP response -> [`SdkError`] mapped to a report.
/// * JSON encoding failure -> `miette` report.
pub(crate) async fn run(action: AntigravityAction) -> miette::Result<()> {
    match action {
        // -----------------------------------------------------------------------
        // Existing actions (unchanged behaviour)
        // -----------------------------------------------------------------------
        AntigravityAction::Models { json } => {
            let client = build_client().map_err(|e| map_sdk_err("models", e))?;
            // The RPC accepts an empty request body for the default listing.
            let response = client
                .fetch_available_models(&json!({}))
                .await
                .map_err(|e| map_sdk_err("models", e))?;
            print_value(&response, json)
        },
        AntigravityAction::Whoami { json } => {
            let client = build_client().map_err(|e| map_sdk_err("whoami", e))?;
            let response = client.userinfo().await.map_err(|e| map_sdk_err("whoami", e))?;
            print_value(&response, json)
        },
        AntigravityAction::Load { json } => {
            let client = build_client().map_err(|e| map_sdk_err("load", e))?;
            let response =
                client.load_code_assist(&json!({})).await.map_err(|e| map_sdk_err("load", e))?;
            print_value(&response, json)
        },
        AntigravityAction::Chat { model, prompt } => {
            let client = build_client().map_err(|e| map_sdk_err("chat", e))?;
            let model = model.unwrap_or_else(|| DEFAULT_GEMINI_MODEL.to_string());
            // Faithful agy path: Cloud Code modelbackend
            // (`cloudcode-pa.googleapis.com/v1internal:generateContent`). The
            // agy token is scoped for this host (the public `generativelanguage`
            // host rejects it with 403 ACCESS_TOKEN_SCOPE_INSUFFICIENT). The
            // Cloud AI Companion project is resolved via `loadCodeAssist`.
            let project = client
                .resolve_cloudcode_project()
                .await
                .map_err(|e| map_sdk_err("chat", e))?
                .ok_or_else(|| {
                    miette::miette!(
                        "no cloudaicompanionProject for this account (run `aphrody antigravity \
                         onboard` first)"
                    )
                })?;
            let body = json!({
                "model": model,
                "project": project,
                "request": {
                    "contents": [ { "role": "user", "parts": [ { "text": prompt } ] } ]
                }
            });
            let response = client
                .cloud_code(AGY_CHAT_ENDPOINT, METHOD_GENERATE_CONTENT, &body)
                .await
                .map_err(|e| map_sdk_err("chat", e))?;
            // Unwrap the Cloud Code `response` envelope so the printed JSON keeps
            // the documented `.candidates[0].content` shape. `chat` is always
            // JSON (no --json flag); pretty-print as the primary human output.
            let unwrapped = response.get("response").cloned().unwrap_or(response);
            print_value(&unwrapped, true)
        },

        // -----------------------------------------------------------------------
        // New action: onboard
        // -----------------------------------------------------------------------
        AntigravityAction::Onboard { tier, project, json } => {
            let client = build_client().map_err(|e| map_sdk_err("onboard", e))?;
            let req = OnboardUserRequest {
                tier_id: tier,
                cloudaicompanion_project: project,
                metadata: ClientMetadata {
                    ide_type: Some("IDE_UNSPECIFIED".to_owned()),
                    platform: Some("PLATFORM_UNSPECIFIED".to_owned()),
                    plugin_type: Some("GEMINI".to_owned()),
                },
            };
            let response =
                client.onboard_user(&req).await.map_err(|e| map_sdk_err("onboard", e))?;
            // Serialize the typed response back to a Value for uniform printing.
            let value = serde_json::to_value(&response)
                .map_err(|e| miette::miette!("antigravity onboard: response encode failed: {e}"))?;
            print_value(&value, json)
        },

        // -----------------------------------------------------------------------
        // New action: refresh
        // -----------------------------------------------------------------------
        AntigravityAction::Refresh { json } => {
            let _ = rustls::crypto::ring::default_provider().install_default();
            let mut client = build_client().map_err(|e| map_sdk_err("refresh", e))?;

            // Capture pre-refresh token info for the confirmation summary.
            // Must be owned Strings because the `&mut client.refresh_token()`
            // call below takes a mutable borrow — the borrow checker rejects a
            // shared reference (from `token_prefix`) that outlives the reborrow.
            let before_access_prefix: String = token_prefix(client.token()).to_owned();
            let had_refresh = client.token().refresh_token.is_some();

            client.refresh_token().await.map_err(|e| map_sdk_err("refresh", e))?;

            // Emit a non-sensitive confirmation: prefix hints + expiry only.
            // No token bytes are ever printed.
            let after_token = client.token();
            let summary = json!({
                "ok": true,
                "before_access_token_prefix": before_access_prefix,
                "after_access_token_prefix": token_prefix(after_token),
                "after_access_token_len": after_token.access_token.len(),
                "had_refresh_token_before": had_refresh,
                "has_refresh_token_after": after_token.refresh_token.is_some(),
                "expiry_hint": after_token.expiry.as_deref().unwrap_or("(none)"),
            });
            print_value(&summary, json)
        },

        // -----------------------------------------------------------------------
        // New action: login
        // -----------------------------------------------------------------------
        AntigravityAction::Login => {
            let _ = rustls::crypto::ring::default_provider().install_default();
            // This is the interactive OAuth loopback PKCE flow: opens a browser
            // window and waits up to 120 s for the redirect.
            let token = antigravity_sdk::oauth::login()
                .await
                .map_err(|e| miette::miette!("antigravity login failed: {e}"))?;

            // Persist to credential store / file so subsequent commands can
            // use the token without re-authenticating.
            antigravity_sdk::oauth::persist(&token)
                .map_err(|e| miette::miette!("antigravity login: failed to persist token: {e}"))?;

            // Emit a non-sensitive confirmation only.
            let summary = json!({
                "ok": true,
                "access_token_prefix": token_prefix(&token),
                "access_token_len": token.access_token.len(),
                "has_refresh_token": token.refresh_token.is_some(),
                "expiry_hint": token.expiry.as_deref().unwrap_or("(none)"),
                "persisted": true,
            });
            // Always pretty-print for login — it is a one-time human-facing op.
            print_value(&summary, true)
        },

        // -----------------------------------------------------------------------
        // New action: token-info
        // -----------------------------------------------------------------------
        AntigravityAction::TokenInfo { json } => {
            let client = build_client().map_err(|e| map_sdk_err("token-info", e))?;
            let token = client.token();
            // Emit a non-sensitive structural summary. The full access_token
            // bytes are NEVER printed — only a short length-bounded prefix
            // (first 10 chars) and the length are surfaced.
            let summary = json!({
                "access_token_prefix": token_prefix(token),
                "access_token_len": token.access_token.len(),
                "has_refresh_token": token.refresh_token.is_some(),
                "refresh_token_len": token.refresh_token.as_ref().map(String::len),
                "expiry_hint": token.expiry.as_deref().unwrap_or("(none)"),
            });
            print_value(&summary, json)
        },

        // -----------------------------------------------------------------------
        // New action: config
        // -----------------------------------------------------------------------
        AntigravityAction::Config { json } => {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let cfg = antigravity_sdk::config::GeminiConfig::load()
                    .map_err(|e| miette::miette!("antigravity config: {e}"))?;
                let value = serde_json::to_value(&cfg)
                    .map_err(|e| miette::miette!("antigravity config: encode failed: {e}"))?;
                print_value(&value, json)
            }
            #[cfg(target_arch = "wasm32")]
            {
                let _ = json;
                Err(miette::miette!("antigravity config is not available on wasm32"))
            }
        },

        // -----------------------------------------------------------------------
        // New action: state-sync
        // -----------------------------------------------------------------------
        AntigravityAction::StateSync { value, json } => {
            let summary = antigravity_sdk::state_sync::decode_unified_state(&value)
                .map_err(|e| miette::miette!("antigravity state-sync: {e}"))?;
            print_value(&summary, json)
        },

        // -----------------------------------------------------------------------
        // New action: item-table
        // -----------------------------------------------------------------------
        AntigravityAction::ItemTable { input, json } => {
            let raw = read_input_or_stdin(&input)
                .map_err(|e| miette::miette!("antigravity item-table: {e}"))?;
            let dump: serde_json::Value = serde_json::from_str(&raw)
                .map_err(|e| miette::miette!("antigravity item-table: JSON parse failed: {e}"))?;
            let entries = antigravity_sdk::state_sync::parse_item_table(&dump);
            let value = serde_json::to_value(&entries)
                .map_err(|e| miette::miette!("antigravity item-table: encode failed: {e}"))?;
            print_value(&value, json)
        },

        // -----------------------------------------------------------------------
        // New action: cloud-code
        // -----------------------------------------------------------------------
        AntigravityAction::CloudCode { endpoint, method, body, json } => {
            let client = build_client().map_err(|e| map_sdk_err("cloud-code", e))?;
            // `--body @path` reads the JSON from a file (the OS caps a literal
            // command-line arg at ~32 KB on Windows, too small for e.g. audio
            // `inlineData` payloads); a literal `{...}` is used as-is.
            let body_src = if let Some(path) = body.strip_prefix('@') {
                std::fs::read_to_string(path)
                    .map_err(|e| miette::miette!("antigravity cloud-code: --body @{path}: {e}"))?
            } else {
                body
            };
            let body_value: serde_json::Value = serde_json::from_str(&body_src).map_err(|e| {
                miette::miette!("antigravity cloud-code: --body is not valid JSON: {e}")
            })?;
            let endpoint: CloudCodeEndpoint = endpoint.into();
            let response = client
                .cloud_code(endpoint, &method, &body_value)
                .await
                .map_err(|e| map_sdk_err("cloud-code", e))?;
            print_value(&response, json)
        },
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Return the first 10 characters of an access token as a length-bounded
/// non-sensitive prefix hint. Never returns more than 10 chars.
fn token_prefix(token: &antigravity_sdk::auth::OAuthToken) -> &str {
    let s = &token.access_token;
    &s[..s.len().min(10)]
}

/// Read the contents of `path` from disk, or from stdin when `path` is `"-"`.
///
/// # Errors
///
/// Returns an `std::io::Error`-derived string on failure.
fn read_input_or_stdin(path: &str) -> Result<String, String> {
    if path == "-" {
        use std::io::Read as _;
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf).map_err(|e| format!("stdin read failed: {e}"))?;
        Ok(buf)
    } else {
        std::fs::read_to_string(path).map_err(|e| format!("file read failed ({path}): {e}"))
    }
}

// ---------------------------------------------------------------------------
// Unit tests (pure logic — no network, no credential store)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use antigravity_sdk::auth::OAuthToken;

    use super::*;

    fn make_token(access: &str, refresh: Option<&str>, expiry: Option<&str>) -> OAuthToken {
        OAuthToken {
            access_token: access.to_owned(),
            refresh_token: refresh.map(str::to_owned),
            expiry: expiry.map(str::to_owned),
        }
    }

    #[test]
    fn token_prefix_short_token() {
        // Tokens shorter than 10 chars are returned as-is.
        let tok = make_token("short", None, None);
        assert_eq!(token_prefix(&tok), "short");
    }

    #[test]
    fn token_prefix_long_token() {
        // Tokens longer than 10 chars are truncated at 10.
        let tok = make_token("ya29.ABCDEFGHIJKLMNOP", None, None);
        let prefix = token_prefix(&tok);
        assert_eq!(prefix.len(), 10);
        assert_eq!(prefix, "ya29.ABCDE");
    }

    #[test]
    fn token_prefix_never_panics_on_empty() {
        let tok = make_token("", None, None);
        assert_eq!(token_prefix(&tok), "");
    }

    #[test]
    fn read_input_or_stdin_reads_file() {
        // Write a small fixture and read it back.
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("fixture.json");
        std::fs::write(&path, r#"{"key":"value"}"#).expect("write");
        let content = read_input_or_stdin(path.to_str().unwrap()).expect("read");
        assert_eq!(content, r#"{"key":"value"}"#);
    }

    #[test]
    fn read_input_or_stdin_missing_file_errors() {
        let err = read_input_or_stdin("/nonexistent/path.json").unwrap_err();
        assert!(err.contains("file read failed"), "got: {err}");
    }

    #[test]
    fn endpoint_choice_prod_maps_to_sdk_prod() {
        let choice = EndpointChoice::Prod;
        let ep: CloudCodeEndpoint = choice.into();
        assert_eq!(ep, CloudCodeEndpoint::Prod);
    }

    #[test]
    fn endpoint_choice_daily_maps_to_sdk_daily() {
        let choice = EndpointChoice::Daily;
        let ep: CloudCodeEndpoint = choice.into();
        assert_eq!(ep, CloudCodeEndpoint::Daily);
    }

    /// `state-sync decode` of a pre-computed base64 blob must surface the
    /// embedded JSON without surfacing raw bytes.
    ///
    /// The base64 literal was pre-computed from
    /// `base64::STANDARD.encode(br#"{"state":"signedIn","tier":"ultra"}"#)`.
    /// We do not import `base64` in the CLI crate tests to avoid a new dev-dep;
    /// the SDK-level tests cover the full encoding matrix.
    #[test]
    fn state_sync_decode_embedded_json() {
        // Pre-computed STANDARD base64 of `{"state":"signedIn","tier":"ultra"}`.
        let b64 = "eyJzdGF0ZSI6InNpZ25lZEluIiwidGllciI6InVsdHJhIn0=";
        let summary = antigravity_sdk::state_sync::decode_unified_state(b64).expect("decode");
        assert_eq!(summary["embedded_json"]["state"], "signedIn");
        assert_eq!(summary["embedded_json"]["tier"], "ultra");
        assert!(summary["raw_len"].as_u64().unwrap() > 0);
    }

    /// `item-table parse` must report structural facts without leaking value
    /// bytes.
    #[test]
    fn item_table_parse_does_not_leak_values() {
        let secret = "SUPERSECRETVALUE_SHOULDNEVERAPPEAR";
        let dump = serde_json::json!({
            "antigravityUnifiedStateSync.oauthToken": secret,
            "workbench.colorTheme": "dark",
        });
        let entries = antigravity_sdk::state_sync::parse_item_table(&dump);
        let serialized = serde_json::to_string(&entries).unwrap();
        assert!(!serialized.contains(secret), "value bytes must not appear in item-table output");
        assert!(serialized.contains("value_len"), "entry must carry value_len");
        assert_eq!(entries.len(), 2);
        let oauth = entries.iter().find(|e| e.key.ends_with("oauthToken")).unwrap();
        assert!(oauth.is_unified_state_sync);
        assert!(oauth.looks_like_signin_state);
    }

    /// `token-info` summary JSON shape: all required keys present, no
    /// actual token byte present in the output.
    #[test]
    fn token_info_summary_shape() {
        let tok =
            make_token("ya29.ABCDEFGHIJKLMNOP_long_token", Some("1//some_refresh"), Some("3599"));
        let summary = serde_json::json!({
            "access_token_prefix": token_prefix(&tok),
            "access_token_len": tok.access_token.len(),
            "has_refresh_token": tok.refresh_token.is_some(),
            "refresh_token_len": tok.refresh_token.as_ref().map(String::len),
            "expiry_hint": tok.expiry.as_deref().unwrap_or("(none)"),
        });
        assert_eq!(summary["access_token_prefix"], "ya29.ABCDE");
        assert_eq!(summary["access_token_len"], tok.access_token.len() as u64);
        assert_eq!(summary["has_refresh_token"], true);
        let serialized = serde_json::to_string(&summary).unwrap();
        // Ensure the full token never appears in the output.
        assert!(
            !serialized.contains("ABCDEFGHIJKLMNOP_long_token"),
            "full access token must not appear in token-info output"
        );
        assert!(
            !serialized.contains("some_refresh"),
            "refresh token must not appear in token-info output"
        );
    }

    #[test]
    fn onboard_request_serializes_correctly() {
        let req = OnboardUserRequest {
            tier_id: "free-tier".to_owned(),
            cloudaicompanion_project: Some("my-project".to_owned()),
            metadata: ClientMetadata {
                ide_type: Some("IDE_UNSPECIFIED".to_owned()),
                platform: Some("PLATFORM_UNSPECIFIED".to_owned()),
                plugin_type: Some("GEMINI".to_owned()),
            },
        };
        let json = serde_json::to_string(&req).expect("serialize");
        assert!(json.contains("\"tierId\":\"free-tier\""));
        assert!(json.contains("\"cloudaicompanionProject\":\"my-project\""));
        assert!(json.contains("\"pluginType\":\"GEMINI\""));
    }
}
