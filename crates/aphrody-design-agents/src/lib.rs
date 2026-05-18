// SPDX-License-Identifier: Apache-2.0
//! Native Rust detector + spawner for 3 coding-agent CLIs.
//!
//! Fuses three existing sources to ship a single self-contained registry:
//!  - The upstream open-design runtime registry (`apps/daemon/src/runtimes/`)
//!    which exposes `RuntimeAgentDef` shape (id, bin, fallback bins,
//!    `versionArgs`, `buildArgs`, `streamFormat`). Only the three relevant
//!    defs (Claude Code, Gemini, plus the Antigravity hypothesis) are
//!    distilled into native Rust constants.
//!  - The aphrody `crates/cli/src/commands.rs` pattern of `which::which`
//!    PATH resolution with `APHRODY_*_BIN` env override taking precedence.
//!  - The open-design `executables.ts` `resolveOnPath` semantics: probe the
//!    env override first, fall through to the primary binary name, then
//!    every documented fallback. Honours Windows `PATHEXT` automatically via
//!    the `which` crate.
//!
//! Three transports converge in [`Protocol`]: `Stdio` (Claude Code, Gemini),
//! `Acp` (Antigravity, which speaks the Agent Client Protocol JSON-RPC
//! dialect like Hermes/Kimi upstream), and a reserved `Sse` variant for
//! HTTP-bridged adapters.
//!
//! The matching binary stub `src/bin/main.rs` exposes `list`, `detect` and
//! `spawn` subcommands so an operator can verify the registry from the
//! command line:
//!
//! ```text
//! aphrody-design-agents list --json
//! aphrody-design-agents detect --id claude-code
//! aphrody-design-agents spawn --id gemini --cwd . --prompt "hello"
//! ```

pub mod protocol;
pub mod spawn;

use std::{collections::BTreeMap, env, path::PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub use protocol::{Protocol, ProtocolError, RpcId, RpcMessage};

/// Stable identifier for one of the 3 supported coding-agent CLIs.
///
/// The string value is what `discover()` keys its results on, what the CLI
/// surfaces in `list --json`, and what the user passes to `--id` when
/// spawning. Slugs are stable so configs port cleanly between aphrody
/// subprojects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentId {
    ClaudeCode,
    Gemini,
    Antigravity,
}

impl AgentId {
    /// Canonical kebab-case slug.
    pub const fn slug(self) -> &'static str {
        match self {
            AgentId::ClaudeCode => "claude-code",
            AgentId::Gemini => "gemini",
            AgentId::Antigravity => "antigravity",
        }
    }

    /// Resolve a slug back to its enum variant. Returns `None` for unknown
    /// slugs so callers can surface a clean error instead of guessing.
    pub fn from_slug(slug: &str) -> Option<Self> {
        for variant in Self::all() {
            if variant.slug() == slug {
                return Some(variant);
            }
        }
        None
    }

    /// Full ordered list of the 3 known agents, used by `discover()`.
    pub const fn all() -> [Self; 3] {
        [AgentId::ClaudeCode, AgentId::Gemini, AgentId::Antigravity]
    }

    /// Override env var consulted before PATH (e.g. `APHRODY_CC_BIN`).
    pub const fn env_override(self) -> &'static str {
        match self {
            AgentId::ClaudeCode => "APHRODY_CC_BIN",
            AgentId::Gemini => "APHRODY_GEMINI_BIN",
            AgentId::Antigravity => "APHRODY_ANTIGRAVITY_BIN",
        }
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.slug())
    }
}

/// Static definition of an agent: the binary name on PATH, drop-in fallback
/// names, the protocol the spawner talks to it with, and the argv shape used
/// when spawning a one-shot prompt. Mirrors the upstream `RuntimeAgentDef`
/// shape but flattened to what the Rust port actually consumes (no JS
/// closures, no model-list fetchers — those are handled elsewhere when
/// needed).
#[derive(Debug, Clone, Copy)]
pub struct AgentDef {
    pub id: AgentId,
    pub display_name: &'static str,
    pub bin: &'static str,
    pub fallback_bins: &'static [&'static str],
    pub version_args: &'static [&'static str],
    pub spawn_args: &'static [&'static str],
    pub protocol: Protocol,
    pub prompt_via_stdin: bool,
    /// Conservative argv budget. Windows CreateProcess caps `lpCommandLine`
    /// at ~32_768 wide chars (or ~8_192 through a `.cmd` shim); a 30_000 byte
    /// ceiling leaves headroom on either path. Adapters that stream the
    /// prompt via stdin (all three today) bypass this and use the
    /// ENAMETOOLONG fallback only when an argv carry is forced.
    pub max_prompt_arg_bytes: usize,
}

const DEFAULT_PROMPT_ARG_BUDGET: usize = 30_000;

/// Compile-time list of all 3 agents, in stable order. This is what the
/// registry seeds itself from and what the CLI iterates over for `list`.
pub const AGENT_DEFS: [AgentDef; 3] = [
    AgentDef {
        id: AgentId::ClaudeCode,
        display_name: "Claude Code",
        bin: "claude",
        // OpenClaude is a drop-in fork that ships the same CLI surface
        // (open-design issue #235). Probe `claude-code` second for installs
        // that ship the long binary name, then the OpenClaude fork third so
        // single-binary installs are auto-detected without wrapper scripts.
        fallback_bins: &["claude-code", "openclaude"],
        version_args: &["--version"],
        spawn_args: &[
            "-p",
            "--input-format",
            "stream-json",
            "--output-format",
            "stream-json",
            "--verbose",
            "--permission-mode",
            "bypassPermissions",
        ],
        protocol: Protocol::Stdio,
        prompt_via_stdin: true,
        max_prompt_arg_bytes: DEFAULT_PROMPT_ARG_BUDGET,
    },
    AgentDef {
        id: AgentId::Gemini,
        display_name: "Gemini CLI",
        bin: "gemini",
        fallback_bins: &[],
        version_args: &["--version"],
        // `--yolo` skips interactive approval prompts in the no-TTY headless
        // mode. `--output-format stream-json` matches the upstream Gemini
        // streaming envelope (see open-design `apps/daemon/src/runtimes/defs/gemini.ts`).
        spawn_args: &["--output-format", "stream-json", "--yolo"],
        protocol: Protocol::Stdio,
        prompt_via_stdin: true,
        max_prompt_arg_bytes: DEFAULT_PROMPT_ARG_BUDGET,
    },
    AgentDef {
        id: AgentId::Antigravity,
        display_name: "Antigravity CLI",
        bin: "antigravity",
        // Some early Vertex AI Antigravity builds ship a long-named binary;
        // probe the canonical name first and the prefixed variant second.
        fallback_bins: &["antigravity-cli", "vertex-antigravity"],
        version_args: &["--version"],
        // Antigravity speaks ACP JSON-RPC over stdio (same dialect as
        // Hermes/Kimi/Devin upstream). The `acp` subcommand opens the
        // handshake; `--accept-hooks` mirrors the Hermes incantation since
        // both share the protocol implementation.
        spawn_args: &["acp", "--accept-hooks"],
        protocol: Protocol::Acp,
        prompt_via_stdin: true,
        max_prompt_arg_bytes: DEFAULT_PROMPT_ARG_BUDGET,
    },
];

/// Detected on-disk descriptor for a single agent. Produced by
/// [`AgentRegistry::discover`]. The `binary_path` is always absolute and
/// has already been verified to exist as a file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDescriptor {
    pub id: String,
    pub display_name: String,
    pub binary_path: PathBuf,
    pub protocol: Protocol,
    pub version: Option<String>,
    /// `true` if `binary_path` came from the env override (e.g.
    /// `APHRODY_GEMINI_BIN`); `false` if it was found by walking PATH.
    pub from_env_override: bool,
}

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("unknown agent slug: {0}")]
    UnknownAgent(String),
    #[error("agent {0} not found on PATH and no override is set")]
    NotInstalled(AgentId),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// In-memory registry of discovered agents. Cheap to clone; the spawner
/// borrows from this when it needs an `AgentDescriptor`.
#[derive(Debug, Default, Clone)]
pub struct AgentRegistry {
    detected: BTreeMap<AgentId, AgentDescriptor>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Walk the static [`AGENT_DEFS`] list, resolve each candidate via
    /// `env override → primary bin → fallback bins → PATH`, and collect the
    /// survivors. Does not run `--version` probes (use
    /// [`Self::probe_versions`] for that — it spawns child processes, so
    /// it's gated behind a tokio runtime).
    pub fn discover() -> Self {
        Self::discover_with_env(|key| env::var(key).ok())
    }

    /// Same as [`Self::discover`] but with an injectable env-reader so unit
    /// tests can simulate env overrides without polluting the real process
    /// env (which would race with parallel tests).
    pub fn discover_with_env<F>(env_reader: F) -> Self
    where
        F: Fn(&str) -> Option<String>,
    {
        let mut registry = Self::new();
        for def in AGENT_DEFS.iter() {
            if let Some(desc) = resolve_agent(def, &env_reader) {
                registry.detected.insert(def.id, desc);
            }
        }
        registry
    }

    pub fn get(&self, id: AgentId) -> Option<&AgentDescriptor> {
        self.detected.get(&id)
    }

    pub fn agents(&self) -> impl Iterator<Item = (&AgentId, &AgentDescriptor)> {
        self.detected.iter()
    }

    pub fn len(&self) -> usize {
        self.detected.len()
    }

    pub fn is_empty(&self) -> bool {
        self.detected.is_empty()
    }

    /// Spawn `bin --version` against every detected agent, mutating the
    /// stored `version` field in place. Best-effort: a probe failure leaves
    /// the entry alone with `version: None`.
    pub async fn probe_versions(&mut self) {
        let ids: Vec<AgentId> = self.detected.keys().copied().collect();
        for id in ids {
            let def = AGENT_DEFS
                .iter()
                .find(|d| d.id == id)
                .copied()
                .expect("registry only stores ids that map to AGENT_DEFS");
            let bin_path = self
                .detected
                .get(&id)
                .map(|d| d.binary_path.clone())
                .expect("just iterated keys");
            let v = probe_version(&bin_path, def.version_args).await;
            if let Some(entry) = self.detected.get_mut(&id) {
                entry.version = v;
            }
        }
    }
}

fn resolve_agent<F>(def: &AgentDef, env_reader: &F) -> Option<AgentDescriptor>
where
    F: Fn(&str) -> Option<String>,
{
    // Env override wins: a user who pinned the binary in `APHRODY_*_BIN`
    // gets that exact path back, no PATH fall-through. Mirrors
    // open-design's `configuredExecutableOverride`.
    let env_key = def.id.env_override();
    if let Some(raw) = env_reader(env_key) {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            let candidate = PathBuf::from(trimmed);
            if candidate.is_file() {
                return Some(AgentDescriptor {
                    id: def.id.slug().to_string(),
                    display_name: def.display_name.to_string(),
                    binary_path: candidate,
                    protocol: def.protocol,
                    version: None,
                    from_env_override: true,
                });
            }
        }
    }
    for bin in std::iter::once(def.bin).chain(def.fallback_bins.iter().copied()) {
        if let Ok(path) = which::which(bin) {
            return Some(AgentDescriptor {
                id: def.id.slug().to_string(),
                display_name: def.display_name.to_string(),
                binary_path: path,
                protocol: def.protocol,
                version: None,
                from_env_override: false,
            });
        }
    }
    None
}

async fn probe_version(bin: &std::path::Path, args: &[&str]) -> Option<String> {
    let out = tokio::time::timeout(
        std::time::Duration::from_millis(5_000),
        tokio::process::Command::new(bin)
            .args(args)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output(),
    )
    .await
    .ok()?
    .ok()?;
    if !out.status.success() {
        return None;
    }
    let combined = if !out.stdout.is_empty() {
        out.stdout
    } else {
        out.stderr
    };
    let s = String::from_utf8_lossy(&combined);
    let first = s.lines().next()?.trim();
    if first.is_empty() {
        None
    } else {
        Some(first.to_string())
    }
}

/// Lookup the static [`AgentDef`] for an [`AgentId`]. Cheap (linear scan
/// over 3 entries, all in cache).
pub fn agent_def(id: AgentId) -> AgentDef {
    AGENT_DEFS
        .iter()
        .find(|d| d.id == id)
        .copied()
        .expect("AGENT_DEFS covers every AgentId variant")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn agent_defs_cover_every_variant() {
        // If someone adds an AgentId variant but forgets to extend
        // AGENT_DEFS, this catches it before the registry silently drops
        // the new agent.
        for id in AgentId::all() {
            let def = AGENT_DEFS.iter().find(|d| d.id == id);
            assert!(def.is_some(), "missing AGENT_DEFS entry for {id:?}");
        }
        assert_eq!(AGENT_DEFS.len(), 3);
    }

    #[test]
    fn agent_ids_round_trip_through_slug() {
        for id in AgentId::all() {
            let slug = id.slug();
            let parsed = AgentId::from_slug(slug).expect("known slug round-trips");
            assert_eq!(parsed, id, "slug {slug} parsed back to wrong variant");
        }
        assert_eq!(AgentId::from_slug("does-not-exist"), None);
        // Legacy slugs from earlier 16-CLI shape must NOT resolve, otherwise
        // callers would silently get a stale agent dispatched.
        assert_eq!(AgentId::from_slug("codex"), None);
        assert_eq!(AgentId::from_slug("copilot"), None);
        assert_eq!(AgentId::from_slug("deepseek"), None);
    }

    #[test]
    fn agent_id_from_str_invalid_returns_none() {
        for bad in ["", " ", "claude", "Claude-Code", "CLAUDE-CODE", "antigrav", "gemini "] {
            // Slugs are exact kebab-case matches; case + whitespace must
            // fail closed so we never dispatch the wrong adapter.
            assert_eq!(AgentId::from_slug(bad), None, "bad slug `{bad}` parsed");
        }
    }

    #[test]
    fn env_overrides_are_unique_and_well_named() {
        // A duplicate APHRODY_*_BIN would silently make two agents share a
        // PATH override which would be impossible to debug.
        let mut seen: HashMap<&'static str, AgentId> = HashMap::new();
        for id in AgentId::all() {
            let key = id.env_override();
            assert!(
                key.starts_with("APHRODY_") && key.ends_with("_BIN"),
                "{id:?} override key {key} must look like APHRODY_*_BIN"
            );
            if let Some(prev) = seen.insert(key, id) {
                panic!("env override {key} collides between {prev:?} and {id:?}");
            }
        }
        // Lock in the exact three keys callers depend on in scripts.
        assert_eq!(AgentId::ClaudeCode.env_override(), "APHRODY_CC_BIN");
        assert_eq!(AgentId::Gemini.env_override(), "APHRODY_GEMINI_BIN");
        assert_eq!(AgentId::Antigravity.env_override(), "APHRODY_ANTIGRAVITY_BIN");
    }

    #[test]
    fn env_override_takes_priority_over_path() {
        // Fake an env override pointing at this very test binary; resolve
        // should pick that up regardless of whether `claude` is on PATH.
        let fake = std::env::current_exe().unwrap();
        let env = |k: &str| -> Option<String> {
            if k == AgentId::ClaudeCode.env_override() {
                Some(fake.display().to_string())
            } else {
                None
            }
        };
        let def = agent_def(AgentId::ClaudeCode);
        let resolved = resolve_agent(&def, &env).expect("env override resolves");
        assert!(resolved.from_env_override);
        assert_eq!(resolved.binary_path, fake);
        assert_eq!(resolved.id, "claude-code");
    }

    #[test]
    fn env_override_pointing_at_nonexistent_path_is_skipped() {
        // A stale override (binary uninstalled, path renamed) must not
        // shadow a perfectly valid PATH hit; otherwise the user can't tell
        // why detection silently broke.
        let env = |k: &str| -> Option<String> {
            if k == AgentId::Gemini.env_override() {
                Some("/this/path/definitely/does/not/exist/gemini".to_string())
            } else {
                None
            }
        };
        let def = agent_def(AgentId::Gemini);
        // resolve_agent will fall through to which::which("gemini").
        // We can't assert a specific outcome (depends on the host) but the
        // override branch must not return a stale descriptor.
        let resolved = resolve_agent(&def, &env);
        if let Some(d) = resolved {
            assert!(
                !d.from_env_override,
                "stale env override should not be reported as the source"
            );
        }
    }

    #[test]
    fn discover_with_empty_env_does_not_panic() {
        let registry = AgentRegistry::discover_with_env(|_| None);
        // Whatever is on PATH is fine — what matters is the API doesn't
        // panic on a host with zero agents installed.
        assert!(registry.len() <= 3);
    }

    #[test]
    fn registry_get_returns_inserted_entry_with_correct_protocol() {
        let fake = std::env::current_exe().unwrap();
        let env = |k: &str| -> Option<String> {
            if k == AgentId::Antigravity.env_override() {
                Some(fake.display().to_string())
            } else {
                None
            }
        };
        let registry = AgentRegistry::discover_with_env(env);
        let antigrav = registry
            .get(AgentId::Antigravity)
            .expect("antigravity injected via env override");
        assert_eq!(antigrav.id, "antigravity");
        // Antigravity is the only ACP agent in the 3-CLI set; this is the
        // single test that locks the protocol classification.
        assert_eq!(antigrav.protocol, Protocol::Acp);
    }

    #[test]
    fn protocols_match_expected_classification() {
        // Cross-check the protocol assignment for every entry. Drift here
        // means we'd spawn the wrong transport.
        let expected: &[(AgentId, Protocol)] = &[
            (AgentId::ClaudeCode, Protocol::Stdio),
            (AgentId::Gemini, Protocol::Stdio),
            (AgentId::Antigravity, Protocol::Acp),
        ];
        for (id, want) in expected {
            let got = agent_def(*id).protocol;
            assert_eq!(got, *want, "{id:?} protocol changed");
        }
    }

    #[test]
    fn agent_def_lookup_works_for_every_variant() {
        for id in AgentId::all() {
            let def = agent_def(id);
            assert_eq!(def.id, id);
            assert!(!def.display_name.is_empty());
            assert!(!def.bin.is_empty());
            assert!(!def.version_args.is_empty());
        }
    }

    #[test]
    fn fallback_bins_documented_for_claude_only() {
        // Only Claude Code and Antigravity carry fallback binary names in
        // the 3-CLI matrix; Gemini ships under a single canonical name.
        let claude = agent_def(AgentId::ClaudeCode);
        assert!(claude.fallback_bins.contains(&"openclaude"));
        assert!(claude.fallback_bins.contains(&"claude-code"));

        let gemini = agent_def(AgentId::Gemini);
        assert!(gemini.fallback_bins.is_empty());

        let antigrav = agent_def(AgentId::Antigravity);
        assert!(antigrav.fallback_bins.contains(&"antigravity-cli"));
    }

    #[test]
    fn all_agents_pipe_prompt_through_stdin() {
        // Every adapter in the 3-CLI set is supposed to stream the prompt
        // via stdin to avoid the Windows ~32 KB CreateProcess cap. If we
        // ever flip an agent to argv-prompt, the ENAMETOOLONG fallback in
        // spawn::spawn_agent must be in place — this guard makes the intent
        // explicit.
        for def in AGENT_DEFS.iter() {
            assert!(
                def.prompt_via_stdin,
                "{:?} must keep prompt_via_stdin=true (use spawn::write_prompt_tempfile if argv carry is forced)",
                def.id
            );
        }
    }
}
