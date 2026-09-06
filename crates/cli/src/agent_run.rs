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
//! - **Headless** (default): one full turn is driven to completion with [`AgentRuntime::run_once`]
//!   and the final agent message is printed on stdout. With `--verbose` the streamed text/tool
//!   events are echoed on stderr as they arrive in the collected event log.
//! - **Interactive** (`--tui`): a long-lived session actor is spawned with [`AgentRuntime::spawn`]
//!   and bridged to [`aphrody_tui::run`]. The TUI emits
//!   [`Submission`](aphrody_agent_proto::Submission)s, which a background task forwards to the
//!   session handle as [`Op`](aphrody_agent_proto::Op)s; the session's
//!   [`Event`](aphrody_agent_proto::Event) stream feeds the TUI.
//!
//! The default model backend is a self-hosted OpenAI Responses-compatible
//! endpoint (`APHRODY_LOCAL_MODEL_URL`); `--stub` selects a deterministic
//! offline replay. Neither path requires an API key.
//!
//! Per CLAUDE.md §0.1 the default autonomy is
//! [`AutonomyMode::FullAuto`](aphrody_agent_runtime::AutonomyMode::FullAuto);
//! `--gated` switches to [`AutonomyMode::Gated`].

use std::{io::Read as _, path::PathBuf};

use aphrody_agent_home::{AgentHome, BootstrapBudget, HomeOptions};
use aphrody_agent_proto::{EventMsg, Submission};
use aphrody_agent_runtime::{
    AgentRuntime, AutonomyMode, ModelChoice, RuntimeConfig, ScriptedTurn, StubModelClient,
};
use miette::IntoDiagnostic as _;

/// Default local model id used when `--model` is omitted.
const DEFAULT_MODEL: &str = "llama3.2";

/// Compact global behavior policy shared by the CLI and TUI runtime.
const DEFAULT_SYSTEM_PROMPT: &str = concat!(
    "Act proactively. Execute local tools immediately without asking for confirmation. ",
    "For unfamiliar work, inspect before acting; use repository search and structured data \
     extraction. ",
    "Prefer local RAG/memory, OCR for images and documents, and computer-use/browser tools when \
     available. ",
    "Cross-check findings, then return concise, direct results with essential evidence and next \
     action."
);

/// Deterministic reply the `--stub` backend emits for its single scripted turn.
/// Echoing the prompt keeps the offline smoke test observable end to end.
const STUB_REPLY_PREFIX: &str = "stub agent: received prompt -> ";

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
    /// Model id (defaults to [`DEFAULT_MODEL`]).
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
}

/// Resolve the autonomy mode from the `--gated` flag.
fn autonomy_for(gated: bool) -> AutonomyMode {
    if gated { AutonomyMode::Gated } else { AutonomyMode::FullAuto }
}

/// Build the [`RuntimeConfig`] for `args`, applying model / system / autonomy /
/// cwd. Pure (no I/O), so it carries a unit test.
fn build_config(args: &AgentArgs) -> RuntimeConfig {
    let model = args.model.clone().unwrap_or_else(|| DEFAULT_MODEL.to_string());
    let home_prompt = AgentHome::open(HomeOptions::default())
        .ok()
        .map(|home| home.system_prompt(&BootstrapBudget::default()).render())
        .filter(|prompt| !prompt.trim().is_empty());
    let system = args.system.as_deref().map_or_else(
        || format!("{DEFAULT_SYSTEM_PROMPT}\n\n{}", home_prompt.as_deref().unwrap_or("")),
        |custom| {
            format!(
                "{DEFAULT_SYSTEM_PROMPT}\n\nOperator instructions:\n{custom}\n\n{}",
                home_prompt.as_deref().unwrap_or("")
            )
        },
    );
    let mut config = RuntimeConfig::new(model)
        .with_autonomy(autonomy_for(args.gated))
        .with_system_prompt(system);
    if let Some(cwd) = &args.cwd {
        config = config.with_cwd(cwd.clone());
    }
    config
}

/// Build the [`ModelChoice`] for `args`: an offline stub when `--stub`, else a
/// local OpenAI Responses-compatible client with no API key.
///
/// # Errors
fn build_model(args: &AgentArgs, model_id: &str) -> miette::Result<ModelChoice> {
    if args.stub {
        let reply = format!("{STUB_REPLY_PREFIX}{}", args.prompt.as_deref().unwrap_or(""));
        let stub = StubModelClient::new(model_id, vec![ScriptedTurn::text(reply)]);
        return Ok(ModelChoice::stub(stub));
    }

    let base_url = std::env::var("APHRODY_LOCAL_MODEL_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());
    Ok(ModelChoice::local_responses(base_url, model_id.to_string()))
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
            "agent: no prompt provided. Pass a prompt argument, pipe one on stdin, or use --tui \
             for the interactive surface."
        ));
    }
    Ok(trimmed.to_string())
}

/// Echo a single event's visible text to stderr in `--verbose` headless mode.
fn echo_event(msg: &EventMsg) {
    match msg {
        EventMsg::AgentMessageDelta { delta } | EventMsg::AgentReasoningDelta { delta } => {
            eprint!("{delta}");
        },
        EventMsg::ExecCommandBegin { command, .. } => {
            eprintln!("[exec] {}", command.join(" "));
        },
        EventMsg::ToolCallBegin { name, .. } => {
            eprintln!("[tool] {name}");
        },
        EventMsg::Error { message } => {
            eprintln!("[error] {message}");
        },
        _ => {},
    }
}

/// Entry point for `aphrody agent`.
///
/// # Errors
/// Propagates configuration, backend-selection, engine, and I/O failures as
/// miette diagnostics.
pub(crate) async fn run(args: AgentArgs) -> miette::Result<()> {
    let config = build_config(&args);
    let model_id = config.model.clone();
    let model = build_model(&args, &model_id)?;

    let runtime = AgentRuntime::builder()
        .config(config)
        .model(model)
        .build()
        .map_err(|e| miette::miette!("agent: {e}"))?;

    if args.tui {
        return run_tui(runtime).await;
    }

    run_headless(runtime, args).await
}

/// Headless mode: drive one full turn and print the final agent message.
async fn run_headless(runtime: AgentRuntime, args: AgentArgs) -> miette::Result<()> {
    let prompt = match args.prompt {
        Some(prompt) => prompt,
        None => read_prompt_from_stdin()?,
    };

    let result = runtime.run_once(prompt).await.map_err(|e| miette::miette!("agent: {e}"))?;

    if args.verbose {
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
    let mut handle = runtime.spawn().await.map_err(|e| miette::miette!("agent: {e}"))?;

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

    tui_result.into_diagnostic().map_err(|e| miette::miette!("agent: terminal error: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_args() -> AgentArgs {
        AgentArgs {
            prompt: Some("ping".to_string()),
            tui: false,
            stub: true,
            model: None,
            system: None,
            gated: false,
            cwd: None,
            verbose: false,
        }
    }

    #[test]
    fn autonomy_defaults_to_full_auto() {
        assert_eq!(autonomy_for(false), AutonomyMode::FullAuto);
        assert_eq!(autonomy_for(true), AutonomyMode::Gated);
    }

    #[test]
    fn config_uses_default_model_when_unset() {
        let config = build_config(&base_args());
        assert_eq!(config.model, DEFAULT_MODEL);
        assert_eq!(config.autonomy, AutonomyMode::FullAuto);
        let prompt = config.system_prompt.as_deref().expect("default prompt");
        assert!(prompt.contains(DEFAULT_SYSTEM_PROMPT));
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
        let config = build_config(&args);
        assert_eq!(config.model, "gemini-3-pro");
        assert_eq!(config.autonomy, AutonomyMode::Gated);
        let prompt = config.system_prompt.as_deref().expect("configured prompt");
        assert!(prompt.contains(DEFAULT_SYSTEM_PROMPT));
        assert!(prompt.contains("Operator instructions:\nbe terse"));
        assert_eq!(config.cwd.as_deref(), Some(std::path::Path::new("/tmp/work")));
    }

    #[test]
    fn stub_model_builds_without_api_key() {
        // The stub backend must never consult the environment.
        let choice = build_model(&base_args(), DEFAULT_MODEL);
        assert!(choice.is_ok(), "stub model should build offline");
    }

    #[tokio::test]
    async fn stub_run_once_echoes_prompt() {
        // End-to-end offline: assemble the runtime with the stub backend and
        // drive one headless turn, asserting the scripted reply comes back.
        let args = base_args();
        let config = build_config(&args);
        let model_id = config.model.clone();
        let model = build_model(&args, &model_id).expect("stub model");
        let runtime =
            AgentRuntime::builder().config(config).model(model).build().expect("runtime builds");
        let result = runtime.run_once("ping").await.expect("run_once");
        let message = result.last_agent_message.expect("a visible message");
        assert!(message.contains("stub agent: received prompt"));
        assert!(message.contains("ping"));
    }
}
