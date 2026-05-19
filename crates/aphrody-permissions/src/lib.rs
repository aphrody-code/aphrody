// SPDX-License-Identifier: Apache-2.0
//! aphrody-permissions — tool-level permission engine for the aphrody runtime.
//!
//! Mirrors the contract of agent-framework `settings.json` `permissions` blocks
//! (the de-facto JSON shape consumed by aphrody-hooks, aphrody-mcp and the
//! skills runtime): a per-rule allow / ask / deny list, an active permission
//! mode (`default`, `acceptEdits`, `plan`, `bypassPermissions`), and a set of
//! `additional_directories` widening the allowed filesystem roots.
//!
//! ## Rule grammar
//!
//! A *rule string* has one of two shapes:
//!
//! | Rule string                  | Tool       | Matcher                |
//! |------------------------------|------------|------------------------|
//! | `WebFetch`                   | `WebFetch` | *(any invocation)*     |
//! | `Read(/etc/**)`              | `Read`     | glob `/etc/**`         |
//! | `Bash(rm -rf /*)`            | `Bash`     | glob `rm -rf /*`       |
//! | `Edit(src/**/*.rs)`          | `Edit`     | glob `src/**/*.rs`     |
//!
//! Matchers are interpreted with the [`glob`] crate (POSIX-style, `**`
//! is recursive). For [`Bash`] tools, the matcher is matched against the
//! `command` field of the tool arguments. For path-bearing tools (`Read`,
//! `Edit`, `Write`, `NotebookEdit`) the matcher is matched against any of
//! `file_path`, `path`, `notebook_path`. For other tools the matcher is
//! matched against `args.to_string()` after compacting whitespace.
//!
//! ## Precedence
//!
//! 1. **Mode override.** [`PermissionMode::BypassPermissions`] short-circuits
//!    every tool to [`PermissionDecision::Allow`]. [`PermissionMode::Plan`]
//!    short-circuits write-class tools to [`PermissionDecision::Deny`] (so a
//!    plan-mode agent cannot mutate the workspace by accident).
//! 2. **Deny wins.** Any matched [`PermissionConfig::deny`] rule beats any
//!    `allow` / `ask` match.
//! 3. **Allow.** Any matched [`PermissionConfig::allow`] rule grants access.
//! 4. **Ask.** Any matched [`PermissionConfig::ask`] rule surfaces a prompt.
//! 5. **Fallback.** If nothing matches, the [`PermissionConfig::default_mode`]
//!    selects the resting behaviour (`Default` → `Ask`, `AcceptEdits` → `Allow`
//!    for editing tools / `Ask` otherwise, `Plan` → `Ask` for read-only,
//!    `BypassPermissions` → `Allow`).
//!
//! ## Merge order
//!
//! Following the layered-settings convention used by aphrody-hooks /
//! aphrody-mcp, three settings layers compose into one engine:
//!
//! ```text
//! local (.local.json)  >  project (.json)  >  user ($HOME/.config/aphrody)
//! ```
//!
//! [`PermissionEngine::merge`] keeps the highest-precedence value for scalar
//! fields (`mode`, `default_mode`) and concatenates the rule vectors so a
//! local override may *widen* or *narrow* the parent layer. `deny` rules are
//! concatenated unconditionally (a parent `deny` is never silently removed by
//! a child layer).

#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Recoverable failures emitted by the permission engine.
#[derive(Debug, Error)]
pub enum PermissionError {
    /// JSON (de)serialization failed while parsing a settings block.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    /// A rule string could not be split into `Tool` and optional `(matcher)`.
    #[error("invalid rule: {0}")]
    InvalidRule(String),
    /// A matcher inside a rule failed to compile as a glob pattern.
    #[error("invalid glob in rule {rule:?}: {source}")]
    InvalidGlob {
        /// Original rule string for diagnostics.
        rule: String,
        /// Underlying glob compile error.
        #[source]
        source: glob::PatternError,
    },
    /// The `defaultMode` / `mode` string did not map to a known mode.
    #[error("invalid mode: {0}")]
    InvalidMode(String),
}

// ---------------------------------------------------------------------------
// Permission mode
// ---------------------------------------------------------------------------

/// Active permission posture for a session.
///
/// Mirrors the agent-framework `permissions.defaultMode` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum PermissionMode {
    /// Ask the user for every tool not explicitly listed in `allow`.
    #[default]
    Default,
    /// Auto-allow file-editing tools (`Edit`, `Write`, `NotebookEdit`); other
    /// tools still consult the allow/ask/deny lists.
    AcceptEdits,
    /// Read-only "plan" mode: writes are denied, reads / searches are asked
    /// (or allowed if explicitly listed).
    Plan,
    /// Skip every permission check. Use with care; intended for headless CI.
    BypassPermissions,
}

impl PermissionMode {
    /// Parse from the JSON string form, accepting both camelCase and
    /// snake_case spellings.
    pub fn from_token(s: &str) -> Result<Self, PermissionError> {
        match s {
            "default" => Ok(Self::Default),
            "acceptEdits" | "accept_edits" => Ok(Self::AcceptEdits),
            "plan" => Ok(Self::Plan),
            "bypassPermissions" | "bypass_permissions" => Ok(Self::BypassPermissions),
            other => Err(PermissionError::InvalidMode(other.to_string())),
        }
    }

    /// JSON-stable string form.
    #[must_use]
    pub fn as_token(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::AcceptEdits => "acceptEdits",
            Self::Plan => "plan",
            Self::BypassPermissions => "bypassPermissions",
        }
    }
}

// ---------------------------------------------------------------------------
// Tool rule
// ---------------------------------------------------------------------------

/// Parsed `Tool` / `Tool(matcher)` rule descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolRule {
    /// Tool name. Compared case-sensitively (`Read`, `Bash`, `WebFetch`, …).
    pub tool: String,
    /// Optional glob matcher. `None` matches every invocation of `tool`.
    pub matcher: Option<String>,
}

impl ToolRule {
    /// Parse a single rule string into its `(tool, matcher)` parts.
    ///
    /// # Errors
    ///
    /// Returns [`PermissionError::InvalidRule`] if the string is empty,
    /// missing a closing `)` after an opening `(`, or starts with a `(`.
    /// Returns [`PermissionError::InvalidGlob`] if the matcher is non-empty
    /// and does not compile as a [`glob::Pattern`].
    pub fn parse(rule: &str) -> Result<Self, PermissionError> {
        let trimmed = rule.trim();
        if trimmed.is_empty() {
            return Err(PermissionError::InvalidRule(rule.to_string()));
        }
        let Some(open) = trimmed.find('(') else {
            // Bare tool name; no matcher.
            return Ok(Self { tool: trimmed.to_string(), matcher: None });
        };
        if open == 0 {
            return Err(PermissionError::InvalidRule(rule.to_string()));
        }
        if !trimmed.ends_with(')') {
            return Err(PermissionError::InvalidRule(rule.to_string()));
        }
        let tool = trimmed[..open].trim();
        let matcher = trimmed[open + 1..trimmed.len() - 1].trim();
        if tool.is_empty() {
            return Err(PermissionError::InvalidRule(rule.to_string()));
        }
        if matcher.is_empty() {
            return Ok(Self { tool: tool.to_string(), matcher: None });
        }
        // Validate the matcher compiles as a glob pattern.
        glob::Pattern::new(matcher).map_err(|source| PermissionError::InvalidGlob {
            rule: rule.to_string(),
            source,
        })?;
        Ok(Self { tool: tool.to_string(), matcher: Some(matcher.to_string()) })
    }

    /// Does this rule's `tool` field match the supplied tool name?
    #[must_use]
    fn tool_matches(&self, tool: &str) -> bool {
        // Case-sensitive (camelCase tool names). Wildcard "*" matches any tool.
        self.tool == "*" || self.tool == tool
    }

    /// Does the supplied subject string satisfy this rule's optional matcher?
    /// A rule with no matcher always succeeds at this stage.
    #[must_use]
    fn matcher_matches(&self, subject: &str) -> bool {
        let Some(pat) = self.matcher.as_deref() else {
            return true;
        };
        let Ok(compiled) = glob::Pattern::new(pat) else {
            return false;
        };
        compiled.matches(subject)
    }
}

// ---------------------------------------------------------------------------
// Decision
// ---------------------------------------------------------------------------

/// Outcome of evaluating a tool invocation against the configured rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PermissionDecision {
    /// Run the tool without prompting.
    Allow {
        /// Human-readable rationale (rule id, mode override, …).
        reason: String,
    },
    /// Ask the user before running the tool.
    Ask {
        /// Human-readable rationale.
        reason: String,
    },
    /// Refuse to run the tool.
    Deny {
        /// Human-readable rationale.
        reason: String,
    },
}

impl PermissionDecision {
    /// True if this decision allows the tool to run.
    #[must_use]
    pub fn is_allow(&self) -> bool {
        matches!(self, Self::Allow { .. })
    }
    /// True if this decision blocks the tool.
    #[must_use]
    pub fn is_deny(&self) -> bool {
        matches!(self, Self::Deny { .. })
    }
    /// True if this decision wants user prompting.
    #[must_use]
    pub fn is_ask(&self) -> bool {
        matches!(self, Self::Ask { .. })
    }
}

// ---------------------------------------------------------------------------
// Permission config (raw settings.json mirror)
// ---------------------------------------------------------------------------

/// Raw configuration block as parsed from a settings file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionConfig {
    /// Currently active mode (session-scoped).
    #[serde(default)]
    pub mode: PermissionMode,
    /// Fallback mode used when no rule matches.
    #[serde(default)]
    pub default_mode: PermissionMode,
    /// Auto-allow rules.
    #[serde(default)]
    pub allow: Vec<String>,
    /// Prompt-before-run rules.
    #[serde(default)]
    pub ask: Vec<String>,
    /// Block rules. Win over `allow`.
    #[serde(default)]
    pub deny: Vec<String>,
    /// Extra filesystem roots widening the allowed path set for `Read` /
    /// `Edit` / `Write` operations (otherwise rooted at CWD).
    #[serde(default)]
    pub additional_directories: Vec<PathBuf>,
}

// ---------------------------------------------------------------------------
// Engine
// ---------------------------------------------------------------------------

/// Compiled permission engine. Holds pre-parsed [`ToolRule`] lists for fast
/// repeated `evaluate` calls.
#[derive(Debug, Clone)]
pub struct PermissionEngine {
    mode: PermissionMode,
    default_mode: PermissionMode,
    allow: Vec<ToolRule>,
    ask: Vec<ToolRule>,
    deny: Vec<ToolRule>,
    extra_dirs: Vec<PathBuf>,
}

/// Tool names whose intent is "modify state on disk". Used by `Plan` mode and
/// `AcceptEdits` mode to short-circuit.
const WRITE_TOOLS: &[&str] = &["Edit", "Write", "NotebookEdit", "MultiEdit"];

/// Tool names that consume a path. Used by [`PermissionEngine::evaluate`] to
/// derive the subject string from common argument shapes.
const PATH_TOOLS: &[&str] = &["Read", "Edit", "Write", "NotebookEdit", "MultiEdit", "Glob"];

impl PermissionEngine {
    /// Build an engine from a freshly-parsed [`PermissionConfig`].
    ///
    /// # Errors
    ///
    /// Returns any [`PermissionError::InvalidRule`] / [`PermissionError::InvalidGlob`]
    /// encountered while pre-compiling the rule strings.
    pub fn from_config(cfg: PermissionConfig) -> Result<Self, PermissionError> {
        Ok(Self {
            mode: cfg.mode,
            default_mode: cfg.default_mode,
            allow: parse_rules(&cfg.allow)?,
            ask: parse_rules(&cfg.ask)?,
            deny: parse_rules(&cfg.deny)?,
            extra_dirs: cfg.additional_directories,
        })
    }

    /// Parse a JSON value laid out as a settings.json `permissions` block:
    ///
    /// ```json
    /// {
    ///   "permissions": {
    ///     "defaultMode": "default",
    ///     "allow": ["Read", "Bash(git status)"],
    ///     "ask":   ["WebFetch"],
    ///     "deny":  ["Bash(rm -rf /*)"]
    ///   }
    /// }
    /// ```
    ///
    /// Either the wrapper object or the inner `permissions` value is accepted.
    ///
    /// # Errors
    ///
    /// Bubbles up [`PermissionError::Json`] from `serde_json`, plus any rule
    /// parse error.
    pub fn from_settings(value: &serde_json::Value) -> Result<Self, PermissionError> {
        let block = value.get("permissions").unwrap_or(value);
        let cfg: PermissionConfig = serde_json::from_value(block.clone())?;
        Self::from_config(cfg)
    }

    /// Merge three layered engines into one: `local > project > user`.
    ///
    /// Scalar fields (`mode`, `default_mode`) take the highest-precedence
    /// non-default value. Rule vectors are concatenated, with duplicates
    /// removed (preserving first occurrence order).
    #[must_use]
    pub fn merge(local: Self, project: Self, user: Self) -> Self {
        let mode = pick_mode(local.mode, project.mode, user.mode);
        let default_mode =
            pick_mode(local.default_mode, project.default_mode, user.default_mode);
        Self {
            mode,
            default_mode,
            allow: dedup_rules(
                [local.allow, project.allow, user.allow].into_iter().flatten().collect(),
            ),
            ask: dedup_rules(
                [local.ask, project.ask, user.ask].into_iter().flatten().collect(),
            ),
            deny: dedup_rules(
                [local.deny, project.deny, user.deny].into_iter().flatten().collect(),
            ),
            extra_dirs: {
                let mut seen: BTreeSet<PathBuf> = BTreeSet::new();
                let mut out = Vec::new();
                for d in local
                    .extra_dirs
                    .into_iter()
                    .chain(project.extra_dirs)
                    .chain(user.extra_dirs)
                {
                    if seen.insert(d.clone()) {
                        out.push(d);
                    }
                }
                out
            },
        }
    }

    /// Current active mode.
    #[must_use]
    pub fn mode(&self) -> PermissionMode {
        self.mode
    }

    /// Default fallback mode.
    #[must_use]
    pub fn default_mode(&self) -> PermissionMode {
        self.default_mode
    }

    /// Extra filesystem roots (read-only borrow).
    #[must_use]
    pub fn additional_directories(&self) -> &[PathBuf] {
        &self.extra_dirs
    }

    /// Evaluate a tool invocation against the configured rules.
    ///
    /// `args` is the tool's argument JSON; the engine extracts a subject
    /// string from `command` (Bash), `file_path` / `path` / `notebook_path`
    /// (path-bearing tools), or the compacted JSON for everything else.
    #[must_use]
    pub fn evaluate(&self, tool: &str, args: &serde_json::Value) -> PermissionDecision {
        // 1. Mode short-circuits.
        if self.mode == PermissionMode::BypassPermissions {
            return PermissionDecision::Allow {
                reason: "mode=bypassPermissions".to_string(),
            };
        }
        if self.mode == PermissionMode::Plan && WRITE_TOOLS.contains(&tool) {
            return PermissionDecision::Deny {
                reason: format!("mode=plan blocks write tool {tool}"),
            };
        }

        let subject = derive_subject(tool, args);

        // 2. Deny wins.
        if let Some(rule) = first_match(&self.deny, tool, &subject) {
            return PermissionDecision::Deny {
                reason: format!("deny rule matched: {}", render_rule(rule)),
            };
        }
        // 3. Allow.
        if let Some(rule) = first_match(&self.allow, tool, &subject) {
            return PermissionDecision::Allow {
                reason: format!("allow rule matched: {}", render_rule(rule)),
            };
        }
        // 4. Ask.
        if let Some(rule) = first_match(&self.ask, tool, &subject) {
            return PermissionDecision::Ask {
                reason: format!("ask rule matched: {}", render_rule(rule)),
            };
        }
        // 5. Fallback.
        self.fallback(tool)
    }

    /// Specialised path-only evaluation entry point. Equivalent to
    /// `evaluate(tool, json!({"file_path": path}))` but avoids the
    /// allocation when the caller already holds a `Path`.
    #[must_use]
    pub fn evaluate_path(&self, tool: &str, path: &Path) -> PermissionDecision {
        let subject = path.to_string_lossy().into_owned();
        if self.mode == PermissionMode::BypassPermissions {
            return PermissionDecision::Allow {
                reason: "mode=bypassPermissions".to_string(),
            };
        }
        if self.mode == PermissionMode::Plan && WRITE_TOOLS.contains(&tool) {
            return PermissionDecision::Deny {
                reason: format!("mode=plan blocks write tool {tool}"),
            };
        }
        if let Some(rule) = first_match(&self.deny, tool, &subject) {
            return PermissionDecision::Deny {
                reason: format!("deny rule matched: {}", render_rule(rule)),
            };
        }
        if let Some(rule) = first_match(&self.allow, tool, &subject) {
            return PermissionDecision::Allow {
                reason: format!("allow rule matched: {}", render_rule(rule)),
            };
        }
        if let Some(rule) = first_match(&self.ask, tool, &subject) {
            return PermissionDecision::Ask {
                reason: format!("ask rule matched: {}", render_rule(rule)),
            };
        }
        self.fallback(tool)
    }

    fn fallback(&self, tool: &str) -> PermissionDecision {
        match self.default_mode {
            PermissionMode::BypassPermissions => PermissionDecision::Allow {
                reason: "default_mode=bypassPermissions".to_string(),
            },
            PermissionMode::AcceptEdits if WRITE_TOOLS.contains(&tool) => {
                PermissionDecision::Allow {
                    reason: format!("default_mode=acceptEdits auto-grants {tool}"),
                }
            }
            PermissionMode::AcceptEdits => PermissionDecision::Ask {
                reason: format!("default_mode=acceptEdits asks for non-edit tool {tool}"),
            },
            PermissionMode::Plan if WRITE_TOOLS.contains(&tool) => PermissionDecision::Deny {
                reason: format!("default_mode=plan blocks write tool {tool}"),
            },
            PermissionMode::Plan => PermissionDecision::Ask {
                reason: format!("default_mode=plan defers read/search tool {tool}"),
            },
            PermissionMode::Default => PermissionDecision::Ask {
                reason: format!("default_mode=default: no rule matched for {tool}"),
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn parse_rules(raw: &[String]) -> Result<Vec<ToolRule>, PermissionError> {
    raw.iter().map(|s| ToolRule::parse(s)).collect()
}

fn dedup_rules(rules: Vec<ToolRule>) -> Vec<ToolRule> {
    let mut seen: BTreeSet<(String, Option<String>)> = BTreeSet::new();
    let mut out = Vec::with_capacity(rules.len());
    for r in rules {
        let key = (r.tool.clone(), r.matcher.clone());
        if seen.insert(key) {
            out.push(r);
        }
    }
    out
}

fn pick_mode(local: PermissionMode, project: PermissionMode, user: PermissionMode) -> PermissionMode {
    // Highest-precedence non-default wins; if all are Default, return Default.
    if local != PermissionMode::Default {
        return local;
    }
    if project != PermissionMode::Default {
        return project;
    }
    user
}

fn first_match<'a>(rules: &'a [ToolRule], tool: &str, subject: &str) -> Option<&'a ToolRule> {
    rules.iter().find(|r| r.tool_matches(tool) && r.matcher_matches(subject))
}

fn render_rule(rule: &ToolRule) -> String {
    match rule.matcher.as_deref() {
        Some(m) => format!("{}({m})", rule.tool),
        None => rule.tool.clone(),
    }
}

/// Extract the canonical "subject" string for a tool call: the value that the
/// matcher glob is tested against.
fn derive_subject(tool: &str, args: &serde_json::Value) -> String {
    if tool == "Bash" {
        if let Some(cmd) = args.get("command").and_then(|v| v.as_str()) {
            return cmd.to_string();
        }
    }
    if PATH_TOOLS.contains(&tool) {
        for key in ["file_path", "path", "notebook_path", "pattern"] {
            if let Some(p) = args.get(key).and_then(|v| v.as_str()) {
                return p.to_string();
            }
        }
    }
    // Fallback: stringify args, collapse whitespace so glob matchers behave.
    let raw = args.to_string();
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

// ---------------------------------------------------------------------------
// Inline unit tests for the small private helpers.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod inline {
    use super::*;

    #[test]
    fn rule_parse_bare_tool() {
        let r = ToolRule::parse("WebFetch").unwrap();
        assert_eq!(r.tool, "WebFetch");
        assert!(r.matcher.is_none());
    }

    #[test]
    fn rule_parse_with_matcher() {
        let r = ToolRule::parse("Read(/etc/**)").unwrap();
        assert_eq!(r.tool, "Read");
        assert_eq!(r.matcher.as_deref(), Some("/etc/**"));
    }

    #[test]
    fn rule_parse_missing_close_paren_errors() {
        assert!(matches!(
            ToolRule::parse("Bash(rm -rf /").unwrap_err(),
            PermissionError::InvalidRule(_)
        ));
    }

    #[test]
    fn derive_subject_bash_uses_command() {
        let v = serde_json::json!({"command": "git status"});
        assert_eq!(derive_subject("Bash", &v), "git status");
    }

    #[test]
    fn derive_subject_path_tool_uses_file_path() {
        let v = serde_json::json!({"file_path": "/etc/passwd"});
        assert_eq!(derive_subject("Read", &v), "/etc/passwd");
    }

    #[test]
    fn mode_round_trip() {
        for m in [
            PermissionMode::Default,
            PermissionMode::AcceptEdits,
            PermissionMode::Plan,
            PermissionMode::BypassPermissions,
        ] {
            assert_eq!(PermissionMode::from_token(m.as_token()).unwrap(), m);
        }
    }
}
