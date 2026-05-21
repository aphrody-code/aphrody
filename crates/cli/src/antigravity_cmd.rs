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

use antigravity_sdk::client::AntigravityClient;
use antigravity_sdk::endpoints::GEMINI_API_HOST;
use antigravity_sdk::error::SdkError;
use serde_json::json;

/// Default Gemini model used by `aphrody antigravity chat` when `--model` is
/// not supplied. The bare model id (no `models/` prefix) is what the
/// `generativelanguage` `v1beta` `generateContent` path expects.
const DEFAULT_GEMINI_MODEL: &str = "gemini-2.0-flash";

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
    let rendered = if pretty {
        serde_json::to_string_pretty(value)
    } else {
        serde_json::to_string(value)
    }
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
/// * Token acquisition failure (no credential, unsupported platform) →
///   [`SdkError`] mapped to a `miette` report.
/// * Network / non-2xx HTTP response → [`SdkError`] mapped to a report.
/// * JSON encoding failure → `miette` report.
pub(crate) async fn run(action: AntigravityAction) -> miette::Result<()> {
    match action {
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
            let response = client
                .userinfo()
                .await
                .map_err(|e| map_sdk_err("whoami", e))?;
            print_value(&response, json)
        },
        AntigravityAction::Load { json } => {
            let client = build_client().map_err(|e| map_sdk_err("load", e))?;
            let response = client
                .load_code_assist(&json!({}))
                .await
                .map_err(|e| map_sdk_err("load", e))?;
            print_value(&response, json)
        },
        AntigravityAction::Chat { model, prompt } => {
            let client = build_client().map_err(|e| map_sdk_err("chat", e))?;
            let model = model.unwrap_or_else(|| DEFAULT_GEMINI_MODEL.to_string());
            // Gemini generative-language v1beta generateContent surface.
            // URL is composed inline against the GEMINI host constant.
            let url = format!("{GEMINI_API_HOST}/v1beta/models/{model}:generateContent");
            let body = json!({
                "contents": [
                    {
                        "role": "user",
                        "parts": [ { "text": prompt } ]
                    }
                ]
            });
            let response = client
                .post_json(&url, &body)
                .await
                .map_err(|e| map_sdk_err("chat", e))?;
            // `chat` is always JSON (no --json flag); pretty-print for
            // readability since it is the primary human-facing output.
            print_value(&response, true)
        },
    }
}
