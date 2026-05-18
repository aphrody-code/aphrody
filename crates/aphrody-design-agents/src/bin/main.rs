// SPDX-License-Identifier: Apache-2.0
//! `aphrody-design-agents` — list, detect, and spawn the 3 supported
//! coding-agent CLIs (Claude Code, Gemini CLI, Antigravity CLI).
//!
//! ```text
//! aphrody-design-agents list                              # human table
//! aphrody-design-agents list --json                       # machine output
//! aphrody-design-agents detect --id claude-code           # single agent JSON
//! aphrody-design-agents spawn --id gemini \
//!     --cwd . --prompt "what is 2+2"                      # stream events
//! ```

use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand};
use tokio_stream::StreamExt;

use aphrody_design_agents::{
    AGENT_DEFS, AgentDescriptor, AgentId, AgentRegistry, agent_def,
    spawn::{AgentEvent, SpawnOptions, spawn_agent},
};

#[derive(Debug, Parser)]
#[command(name = "aphrody-design-agents", about = "Detect + spawn coding-agent CLIs", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Walk the static agent registry and report which binaries are
    /// available on this host.
    List {
        /// Emit JSON instead of a human-readable table.
        #[arg(long, default_value_t = true)]
        json: bool,
        /// Spawn each detected agent with `--version` (or its equivalent)
        /// to capture the installed version string.
        #[arg(long)]
        probe_versions: bool,
    },
    /// Detect a single agent by slug.
    Detect {
        /// Slug: `claude-code`, `gemini`, or `antigravity`.
        #[arg(long)]
        id: String,
        #[arg(long)]
        probe_version: bool,
    },
    /// Spawn one agent with a prompt and stream its events to stdout as
    /// JSONL.
    Spawn {
        /// Slug: `claude-code`, `gemini`, or `antigravity`.
        #[arg(long)]
        id: String,
        #[arg(long, default_value = ".")]
        cwd: PathBuf,
        #[arg(long)]
        prompt: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::List { json, probe_versions } => list(json, probe_versions).await,
        Command::Detect { id, probe_version } => detect(&id, probe_version).await,
        Command::Spawn { id, cwd, prompt } => spawn(&id, cwd, prompt).await,
    }
}

async fn list(json: bool, probe_versions: bool) -> Result<()> {
    let mut registry = AgentRegistry::discover();
    if probe_versions {
        registry.probe_versions().await;
    }
    if json {
        // Emit a single JSON document so downstream tooling can parse it
        // in one read.
        let payload: Vec<&AgentDescriptor> =
            registry.agents().map(|(_, d)| d).collect();
        let body = serde_json::json!({
            "schema": "aphrody-design-agents.list/v1",
            "available": payload,
            "known_total": AGENT_DEFS.len(),
            "detected_total": registry.len(),
        });
        println!("{}", serde_json::to_string_pretty(&body)?);
    } else {
        println!("{:<14} {:<10} {:<8} path", "agent", "protocol", "source");
        for (id, desc) in registry.agents() {
            let source = if desc.from_env_override { "env" } else { "PATH" };
            println!(
                "{:<14} {:<10} {:<8} {}",
                id,
                desc.protocol.as_str(),
                source,
                desc.binary_path.display(),
            );
        }
        println!();
        println!(
            "detected {} of {} known agents",
            registry.len(),
            AGENT_DEFS.len(),
        );
    }
    Ok(())
}

async fn detect(slug: &str, probe_version: bool) -> Result<()> {
    let id = AgentId::from_slug(slug).ok_or_else(|| {
        anyhow!(
            "unknown agent slug `{slug}`. Known: {}",
            AGENT_DEFS
                .iter()
                .map(|d| d.id.slug())
                .collect::<Vec<_>>()
                .join(", ")
        )
    })?;
    let mut registry = AgentRegistry::discover();
    let desc = registry
        .get(id)
        .cloned()
        .ok_or_else(|| anyhow!("agent `{slug}` is not installed on PATH and no override is set"))?;
    let def = agent_def(id);
    let mut out = serde_json::json!({
        "id": desc.id,
        "display_name": desc.display_name,
        "binary_path": desc.binary_path,
        "protocol": desc.protocol,
        "from_env_override": desc.from_env_override,
        "spawn_args": def.spawn_args,
        "prompt_via_stdin": def.prompt_via_stdin,
        "env_override_key": id.env_override(),
    });
    if probe_version {
        registry.probe_versions().await;
        if let Some(refreshed) = registry.get(id) {
            out["version"] = serde_json::to_value(&refreshed.version)?;
        }
    }
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

async fn spawn(slug: &str, cwd: PathBuf, prompt: String) -> Result<()> {
    let id = AgentId::from_slug(slug).ok_or_else(|| anyhow!("unknown agent slug `{slug}`"))?;
    let registry = AgentRegistry::discover();
    let desc = registry
        .get(id)
        .cloned()
        .ok_or_else(|| anyhow!("agent `{slug}` is not installed; refusing to spawn"))?;
    let opts = SpawnOptions::new(
        cwd.canonicalize().unwrap_or(cwd.clone()),
        prompt,
    );
    let mut stream = spawn_agent(&desc, opts)
        .await
        .with_context(|| format!("spawning {slug}"))?;
    while let Some(event) = stream.next().await {
        let line = match &event {
            AgentEvent::ChatChunk { text } => serde_json::json!({"type":"chat_chunk","text":text}),
            AgentEvent::ToolCall { name, args } => {
                serde_json::json!({"type":"tool_call","name":name,"args":args})
            }
            AgentEvent::Done { ok, message } => {
                serde_json::json!({"type":"done","ok":ok,"message":message})
            }
            AgentEvent::Error { message } => {
                serde_json::json!({"type":"error","message":message})
            }
        };
        println!("{}", serde_json::to_string(&line)?);
        if matches!(event, AgentEvent::Done { .. }) {
            break;
        }
    }
    Ok(())
}
