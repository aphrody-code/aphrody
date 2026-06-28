// SPDX-License-Identifier: Apache-2.0
//! `aphrody agent "..."` — the flagship autonomous-agent CLI surface.
//!
//! This module is the CLI front door of aphrody's Rust rewrite of Antigravity
//! (see `docs/VISION.md`, Phase 3b). It wires the agent engine
//! ([`aphrody_agent_runtime`]) and the full-screen terminal surface
//! ([`aphrody_tui`]) into a single command so that `aphrody agent "do X"`
//! launches an autonomous agent immediately.
//!
//! Two execution shapes are supported:
//!
//! - **Headless** (default): one full turn is driven to completion with
//!   [`AgentRuntime::run_once`] and the final agent message is printed on
//!   stdout. With `--verbose` the streamed text/tool events are echoed on
//!   stderr as they arrive in the collected event log.
//! - **Interactive** (`--tui`): a long-lived session actor is spawned with
//!   [`AgentRuntime::spawn`] and bridged to [`aphrody_tui::run`]. The TUI emits
//!   [`Submission`](aphrody_agent_proto::Submission)s, which a background task
//!   forwards to the session handle as [`Op`](aphrody_agent_proto::Op)s; the
//!   session's [`Event`](aphrody_agent_proto::Event) stream feeds the TUI.
//!
//! The model backend is either a live Gemini client (`GEMINI_API_KEY` /
//! `GOOGLE_API_KEY`) or, with `--stub`, a deterministic offline replay that
//! needs no network and no API key — the basis of the offline smoke test.
//!
//! Per CLAUDE.md §0.1 the default autonomy is
//! [`AutonomyMode::FullAuto`](aphrody_agent_runtime::AutonomyMode::FullAuto);
//! `--gated` switches to [`AutonomyMode::Gated`].

use std::io::Read as _;
use std::path::PathBuf;

use aphrody_agent_proto::{EventMsg, Submission};
use aphrody_agent_runtime::{
    AgentRuntime, AutonomyMode, ModelChoice, RuntimeConfig, ScriptedTurn, StubModelClient,
};
use aphrody_chat::DEFAULT_SYSTEM_PROMPT;
use miette::IntoDiagnostic as _;

/// Default model id used when `--model` is omitted. A fast, low-latency Gemini
/// flash tier, consistent with the project's latency objective.
const DEFAULT_MODEL: &str = "gemini-2.5-flash";

/// Default model id used with `--local` when `--model` is omitted. Qwen3 is the
/// best agentic tool-caller that fits 12 GB (`docs/local-llm-models.md`); pull it
/// with `ollama pull qwen3`, or pass `--model gemma4:12b` for the resident model.
const DEFAULT_LOCAL_MODEL: &str = "qwen3";

/// Deterministic reply the `--stub` backend emits for its single scripted turn.
/// Echoing the prompt keeps the offline smoke test observable end to end.
const STUB_REPLY_PREFIX: &str = "stub agent: received prompt -> ";

/// Short French-language directive (CLAUDE.md §0.2) appended to a caller-supplied
/// `--system` so the immutable French voice holds even with a custom persona.
const FRENCH_DIRECTIVE: &str =
    "Réponds toujours en français, quelle que soit la langue de la requête.";

/// Compose the per-turn system prompt. With no `--system`, use aphrody's canonical
/// persona ([`DEFAULT_SYSTEM_PROMPT`]: autonomous, omniscient, always-on, in
/// French); with one, keep the caller's persona but still enforce the French voice.
fn french_voice(user_system: Option<&str>) -> String {
    match user_system {
        Some(s) if !s.trim().is_empty() => format!("{s}\n\n{FRENCH_DIRECTIVE}"),
        _ => DEFAULT_SYSTEM_PROMPT.to_string(),
    }
}

/// Parsed `aphrody agent` invocation, decoupled from clap so it can be
/// unit-tested directly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentArgs {
    /// The positional prompt. When `None` in headless mode the prompt is read
    /// from stdin.
    pub prompt: Option<String>,
    /// Launch the full-screen interactive TUI instead of a headless turn.
    pub tui: bool,
    /// Use the offline deterministic stub backend (no network, no API key).
    pub stub: bool,
    /// Drive a local OpenAI-compatible backend (Ollama / llama.cpp / vLLM /
    /// `aphrody serve`) instead of cloud Gemini — no API key required.
    pub local: bool,
    /// Base URL for `--local` (includes `/v1`). Defaults to `OPENAI_BASE_URL`
    /// then Ollama (`http://127.0.0.1:11434/v1`).
    pub base_url: Option<String>,
    /// Model id (defaults to [`DEFAULT_MODEL`], or [`DEFAULT_LOCAL_MODEL`] with
    /// `--local`).
    pub model: Option<String>,
    /// Optional system prompt prepended to every model request.
    pub system: Option<String>,
    /// Require approval for each tool call ([`AutonomyMode::Gated`]); otherwise
    /// [`AutonomyMode::FullAuto`].
    pub gated: bool,
    /// Working directory the tools resolve relative paths against.
    pub cwd: Option<PathBuf>,
    /// Echo streamed text / tool events on stderr in headless mode.
    pub verbose: bool,
    /// Disable recall-before-think (skip injecting aphrody's self-knowledge +
    /// past lessons into the turn). Memory recall is on by default.
    pub no_memory: bool,
}

/// Resolve the autonomy mode from the `--gated` flag.
fn autonomy_for(gated: bool) -> AutonomyMode {
    if gated {
        AutonomyMode::Gated
    } else {
        AutonomyMode::FullAuto
    }
}

/// Build the [`RuntimeConfig`] for `args`, applying model / system / autonomy /
/// cwd. Pure (no I/O), so it carries a unit test.
fn build_config(args: &AgentArgs, recalled: Option<&str>) -> RuntimeConfig {
    let model = args.model.clone().unwrap_or_else(|| {
        if args.local {
            DEFAULT_LOCAL_MODEL
        } else {
            DEFAULT_MODEL
        }
        .to_string()
    });
    let mut system = french_voice(args.system.as_deref());
    if let Some(ctx) = recalled {
        if !ctx.trim().is_empty() {
            system.push_str("\n\n");
            system.push_str(ctx);
        }
    }
    let mut config = RuntimeConfig::new(model)
        .with_autonomy(autonomy_for(args.gated))
        .with_system_prompt(system);
    if let Some(cwd) = &args.cwd {
        config = config.with_cwd(cwd.clone());
    }
    config
}

/// Resolve the Gemini API key from the environment.
///
/// `GEMINI_API_KEY` is preferred; `GOOGLE_API_KEY` is the documented fallback.
fn gemini_api_key() -> Option<String> {
    std::env::var("GEMINI_API_KEY")
        .ok()
        .filter(|k| !k.trim().is_empty())
        .or_else(|| {
            std::env::var("GOOGLE_API_KEY")
                .ok()
                .filter(|k| !k.trim().is_empty())
        })
}

/// Resolve the local backend base URL for `--local`.
///
/// `--base-url` wins, then `OPENAI_BASE_URL`, then the Ollama default. The base
/// URL includes the `/v1` prefix.
fn local_base_url(args: &AgentArgs) -> String {
    args.base_url
        .clone()
        .filter(|u| !u.trim().is_empty())
        .or_else(|| {
            std::env::var("OPENAI_BASE_URL")
                .ok()
                .filter(|u| !u.trim().is_empty())
        })
        .unwrap_or_else(|| aphrody_agent_runtime::DEFAULT_LOCAL_BASE_URL.to_string())
}

/// Build the [`ModelChoice`] for `args`: an offline stub when `--stub`, a local
/// OpenAI-compatible backend when `--local`, else a live Gemini client
/// authenticated from the environment.
///
/// # Errors
/// Returns a miette error when the cloud Gemini backend is requested but no API
/// key is available in `GEMINI_API_KEY` / `GOOGLE_API_KEY`. The `--stub` and
/// `--local` backends never consult cloud credentials.
fn build_model(args: &AgentArgs, model_id: &str) -> miette::Result<ModelChoice> {
    if args.stub {
        let reply = format!(
            "{STUB_REPLY_PREFIX}{}",
            args.prompt.as_deref().unwrap_or("")
        );
        let stub = StubModelClient::new(model_id, vec![ScriptedTurn::text(reply)]);
        return Ok(ModelChoice::stub(stub));
    }

    if args.local {
        let base = local_base_url(args);
        // Local backends ignore the bearer token; Ollama's convention is the
        // literal `ollama`. `OPENAI_API_KEY` overrides for guarded gateways.
        let key = std::env::var("OPENAI_API_KEY")
            .ok()
            .filter(|k| !k.trim().is_empty())
            .unwrap_or_else(|| "ollama".to_string());
        return Ok(ModelChoice::local(base, model_id.to_string(), key));
    }

    let key = gemini_api_key().ok_or_else(|| {
        miette::miette!(
            "agent: no Gemini API key found. Set GEMINI_API_KEY (or GOOGLE_API_KEY), \
             or pass --stub for an offline deterministic run."
        )
    })?;
    Ok(ModelChoice::gemini(key, model_id.to_string()))
}

/// Read a prompt from stdin when none was supplied positionally.
///
/// # Errors
/// Returns a miette error if stdin cannot be read or contains only whitespace.
fn read_prompt_from_stdin() -> miette::Result<String> {
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .into_diagnostic()
        .map_err(|e| miette::miette!("agent: failed to read prompt from stdin: {e}"))?;
    let trimmed = buf.trim();
    if trimmed.is_empty() {
        return Err(miette::miette!(
            "agent: no prompt provided. Pass a prompt argument, pipe one on stdin, \
             or use --tui for the interactive surface."
        ));
    }
    Ok(trimmed.to_string())
}

/// Echo a single event's visible text to stderr in `--verbose` headless mode.
fn echo_event(msg: &EventMsg) {
    match msg {
        EventMsg::AgentMessageDelta { delta } | EventMsg::AgentReasoningDelta { delta } => {
            eprint!("{delta}");
        }
        EventMsg::ExecCommandBegin { command, .. } => {
            eprintln!("[exec] {}", command.join(" "));
        }
        EventMsg::ToolCallBegin { name, .. } => {
            eprintln!("[tool] {name}");
        }
        EventMsg::Error { message } => {
            eprintln!("[error] {message}");
        }
        _ => {}
    }
}

/// Entry point for `aphrody agent`.
///
/// # Errors
/// Propagates configuration, backend-selection, engine, and I/O failures as
/// miette diagnostics.
pub(crate) async fn run(args: AgentArgs) -> miette::Result<()> {
    // TUI : surface interactive multi-tours, pas de rappel mono-prompt ici.
    if args.tui {
        let config = build_config(&args, None);
        let model_id = config.model.clone();
        let model = build_model(&args, &model_id)?;
        let runtime = AgentRuntime::builder()
            .config(config)
            .model(model)
            .build()
            .map_err(|e| miette::miette!("agent: {e}"))?;
        return run_tui(runtime).await;
    }

    // Headless : on résout le prompt, on rappelle la mémoire d'aphrody
    // (recall-before-think, best-effort), on l'injecte dans la persona, puis on
    // exécute le tour.
    let prompt = match &args.prompt {
        Some(p) => p.clone(),
        None => read_prompt_from_stdin()?,
    };
    let recalled = recall_context(&args, &prompt).await;
    let config = build_config(&args, recalled.as_deref());
    let model_id = config.model.clone();
    let model = build_model(&args, &model_id)?;
    let runtime = AgentRuntime::builder()
        .config(config)
        .model(model)
        .build()
        .map_err(|e| miette::miette!("agent: {e}"))?;

    run_headless(runtime, prompt, args.verbose).await
}

/// Recall-before-think (best-effort). Returns `None` with `--no-memory`,
/// otherwise queries aphrody's memory; any failure (e.g. the local embeddings
/// endpoint is down) is reported on stderr and skipped — it never breaks the turn.
async fn recall_context(args: &AgentArgs, prompt: &str) -> Option<String> {
    if args.no_memory {
        return None;
    }
    match crate::self_know::recall_for_agent(prompt, 4).await {
        Ok(Some(block)) => {
            eprintln!("mémoire : contexte rappelé injecté dans le tour");
            Some(block)
        }
        Ok(None) => None,
        Err(e) => {
            eprintln!("mémoire : rappel ignoré ({e})");
            None
        }
    }
}

/// Headless mode: drive one full turn and print the final agent message.
async fn run_headless(
    runtime: AgentRuntime,
    prompt: String,
    verbose: bool,
) -> miette::Result<()> {
    let result = runtime
        .run_once(prompt)
        .await
        .map_err(|e| miette::miette!("agent: {e}"))?;

    if verbose {
        for event in &result.events {
            echo_event(&event.msg);
        }
        eprintln!();
    }

    match result.last_agent_message {
        Some(message) => println!("{message}"),
        None => eprintln!("agent: turn produced no visible message"),
    }
    Ok(())
}

/// Interactive mode: spawn the session actor and bridge it to the TUI.
///
/// The TUI emits [`Submission`]s; a background task forwards each submission's
/// [`Op`] to the session handle. The session's [`Event`] stream is handed to
/// the TUI as its input channel.
async fn run_tui(runtime: AgentRuntime) -> miette::Result<()> {
    let mut handle = runtime
        .spawn()
        .await
        .map_err(|e| miette::miette!("agent: {e}"))?;

    let events = handle.events().ok_or_else(|| {
        miette::miette!("agent: session event stream was already taken (internal error)")
    })?;

    // The TUI writes Submissions here; we drain them and forward each op to the
    // engine. An unbounded channel matches the TUI's `run` contract and never
    // blocks the terminal loop.
    let (sub_tx, mut sub_rx) = tokio::sync::mpsc::unbounded_channel::<Submission>();

    let forwarder = tokio::spawn(async move {
        while let Some(submission) = sub_rx.recv().await {
            // If the actor has already stopped, submit returns the rejected op;
            // there is nothing left to drive, so we stop forwarding.
            if handle.submit(submission.op).is_err() {
                break;
            }
        }
        handle
    });

    // The TUI owns the real terminal until the user quits or the engine closes
    // the event channel. Restoring the terminal is handled inside `run`.
    let tui_result = aphrody_tui::run(events, sub_tx).await;

    // Dropping the TUI's `sub_tx` (consumed by `run`) closes the forwarder's
    // receiver, so this join completes promptly.
    let _ = forwarder.await;

    tui_result
        .into_diagnostic()
        .map_err(|e| miette::miette!("agent: terminal error: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_args() -> AgentArgs {
        AgentArgs {
            prompt: Some("ping".to_string()),
            tui: false,
            stub: true,
            local: false,
            base_url: None,
            model: None,
            system: None,
            gated: false,
            cwd: None,
            verbose: false,
            no_memory: true,
        }
    }

    #[test]
    fn autonomy_defaults_to_full_auto() {
        assert_eq!(autonomy_for(false), AutonomyMode::FullAuto);
        assert_eq!(autonomy_for(true), AutonomyMode::Gated);
    }

    #[test]
    fn config_uses_default_model_when_unset() {
        let config = build_config(&base_args(), None);
        assert_eq!(config.model, DEFAULT_MODEL);
        assert_eq!(config.autonomy, AutonomyMode::FullAuto);
        // Even with no `--system`, the immutable French voice is injected.
        assert!(
            config
                .system_prompt
                .as_deref()
                .unwrap_or_default()
                .contains("français"),
            "default agent voice must instruct French"
        );
        assert!(config.cwd.is_none());
    }

    #[test]
    fn config_honors_overrides() {
        let args = AgentArgs {
            model: Some("gemini-3-pro".to_string()),
            system: Some("be terse".to_string()),
            gated: true,
            cwd: Some(PathBuf::from("/tmp/work")),
            ..base_args()
        };
        let config = build_config(&args, None);
        assert_eq!(config.model, "gemini-3-pro");
        assert_eq!(config.autonomy, AutonomyMode::Gated);
        // The custom `--system` is preserved AND the French voice is still enforced.
        let system = config.system_prompt.as_deref().unwrap_or_default();
        assert!(system.contains("be terse"), "custom system prompt preserved");
        assert!(system.contains("français"), "French voice still enforced");
        assert_eq!(config.cwd.as_deref(), Some(std::path::Path::new("/tmp/work")));
    }

    #[test]
    fn recalled_context_is_injected_into_system_prompt() {
        // recall-before-think : le contexte rappelé s'ajoute à la persona, sans
        // l'écraser (le français reste imposé).
        let config = build_config(&base_args(), Some("RAPPEL: voici du code pertinent"));
        let system = config.system_prompt.as_deref().unwrap_or_default();
        assert!(system.contains("RAPPEL: voici du code pertinent"));
        assert!(system.contains("français"));
    }

    #[test]
    fn stub_model_builds_without_api_key() {
        // The stub backend must never consult the environment.
        let choice = build_model(&base_args(), DEFAULT_MODEL);
        assert!(choice.is_ok(), "stub model should build offline");
    }

    #[test]
    fn local_model_builds_without_cloud_key() {
        // The local backend must build with no Gemini key in the environment.
        let args = AgentArgs {
            stub: false,
            local: true,
            ..base_args()
        };
        let choice = build_model(&args, DEFAULT_LOCAL_MODEL);
        assert!(choice.is_ok(), "local model should build without a cloud key");
        assert!(matches!(choice.unwrap(), ModelChoice::Local(_)));
    }

    #[test]
    fn local_default_model_used_when_unset() {
        let args = AgentArgs {
            stub: false,
            local: true,
            model: None,
            ..base_args()
        };
        assert_eq!(build_config(&args, None).model, DEFAULT_LOCAL_MODEL);
    }

    #[test]
    fn local_base_url_prefers_flag_over_default() {
        let args = AgentArgs {
            local: true,
            base_url: Some("http://127.0.0.1:8080/v1".to_string()),
            ..base_args()
        };
        assert_eq!(local_base_url(&args), "http://127.0.0.1:8080/v1");
    }

    #[test]
    fn live_model_without_key_errors_clearly() {
        let args = AgentArgs {
            stub: false,
            ..base_args()
        };
        // Only meaningful when no key is set in this process's environment.
        if gemini_api_key().is_none() {
            let err = build_model(&args, DEFAULT_MODEL)
                .expect_err("live backend should fail without a key");
            assert!(err.to_string().contains("GEMINI_API_KEY"));
        }
    }

    #[tokio::test]
    async fn stub_run_once_echoes_prompt() {
        // End-to-end offline: assemble the runtime with the stub backend and
        // drive one headless turn, asserting the scripted reply comes back.
        let args = base_args();
        let config = build_config(&args, None);
        let model_id = config.model.clone();
        let model = build_model(&args, &model_id).expect("stub model");
        let runtime = AgentRuntime::builder()
            .config(config)
            .model(model)
            .build()
            .expect("runtime builds");
        let result = runtime.run_once("ping").await.expect("run_once");
        let message = result.last_agent_message.expect("a visible message");
        assert!(message.contains("stub agent: received prompt"));
        assert!(message.contains("ping"));
    }
}
