// SPDX-License-Identifier: Apache-2.0
//! Slash-command surface for the aphrody chat agent.
//!
//! Reproduces the in-chat slash commands an agent IDE (Antigravity / Cascade,
//! Windsurf, Claude Code) exposes, adapted to aphrody's **scriptable,
//! non-interactive** ethos: a one-shot `--prompt` beginning with `/` is parsed
//! here instead of being sent verbatim to the LLM.
//!
//! Two kinds of command:
//!
//! * [`SlashKind::Meta`] — handled locally (no LLM round-trip): introspect the
//!   agent (`/help`, `/model`, `/tools`).
//! * [`SlashKind::Template`] — the Cascade-style code commands (`/explain`,
//!   `/fix`, `/test`, …). They expand the trailing argument into a templated
//!   prompt that is then sent through the normal turn loop.
//!
//! The [`SLASH_COMMANDS`] table is the single source of truth; the CLI consumes
//! it both for dispatch and for `/help` rendering.

/// Whether a slash command is resolved locally or expands into an LLM prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlashKind {
    /// Resolved locally by the host (no model call). E.g. `/help`, `/tools`.
    Meta,
    /// Expands its argument into a templated prompt, then runs a normal turn.
    Template,
}

/// A single slash command specification.
#[derive(Debug, Clone, Copy)]
pub struct SlashSpec {
    /// Primary name without the leading slash (e.g. `"explain"`).
    pub name: &'static str,
    /// Alternate spellings (without the leading slash).
    pub aliases: &'static [&'static str],
    /// One-line human description, shown by `/help`.
    pub description: &'static str,
    /// Local vs prompt-expanding.
    pub kind: SlashKind,
    /// Prompt template for [`SlashKind::Template`] commands. `{arg}` is replaced
    /// with the trailing argument text. `None` for [`SlashKind::Meta`].
    pub template: Option<&'static str>,
}

impl SlashSpec {
    /// Expand a [`SlashKind::Template`] command with `arg`.
    ///
    /// Returns `None` for meta commands (which have no template). When the
    /// command has a template but `arg` is empty, the `{arg}` placeholder is
    /// replaced with an empty string (the model still receives a coherent
    /// instruction, e.g. acting on the current selection / latest context).
    #[must_use]
    pub fn expand(&self, arg: &str) -> Option<String> {
        self.template.map(|t| t.replace("{arg}", arg.trim()))
    }
}

/// Canonical slash-command table. Meta commands first, then the Cascade-style
/// code templates.
pub const SLASH_COMMANDS: &[SlashSpec] = &[
    SlashSpec {
        name: "help",
        aliases: &["?", "commands"],
        description: "List the available slash commands.",
        kind: SlashKind::Meta,
        template: None,
    },
    SlashSpec {
        name: "model",
        aliases: &[],
        description: "Show the active model id (and how to switch it).",
        kind: SlashKind::Meta,
        template: None,
    },
    SlashSpec {
        name: "tools",
        aliases: &[],
        description: "List the tools wired into the agent.",
        kind: SlashKind::Meta,
        template: None,
    },
    SlashSpec {
        name: "explain",
        aliases: &["exp"],
        description: "Explain code / a concept clearly.",
        kind: SlashKind::Template,
        template: Some(
            "Explain the following clearly and concisely, calling out anything \
             non-obvious:\n\n{arg}",
        ),
    },
    SlashSpec {
        name: "fix",
        aliases: &[],
        description: "Diagnose and fix a bug or error.",
        kind: SlashKind::Template,
        template: Some(
            "Diagnose the root cause and provide a corrected version. Show only \
             the minimal necessary change and explain why it fixes it:\n\n{arg}",
        ),
    },
    SlashSpec {
        name: "test",
        aliases: &["tests"],
        description: "Write tests for the given code.",
        kind: SlashKind::Template,
        template: Some(
            "Write thorough unit tests covering the happy path and edge cases \
             for the following. Use the idiomatic test framework for its \
             language:\n\n{arg}",
        ),
    },
    SlashSpec {
        name: "docs",
        aliases: &["doc", "document"],
        description: "Write documentation / doc-comments.",
        kind: SlashKind::Template,
        template: Some(
            "Write clear documentation (doc-comments + a short usage example) \
             for the following:\n\n{arg}",
        ),
    },
    SlashSpec {
        name: "refactor",
        aliases: &["rf"],
        description: "Refactor code for clarity without changing behaviour.",
        kind: SlashKind::Template,
        template: Some(
            "Refactor the following for clarity and idiomatic style WITHOUT \
             changing its observable behaviour. Explain each change:\n\n{arg}",
        ),
    },
    SlashSpec {
        name: "review",
        aliases: &["cr"],
        description: "Review code for bugs, style and security.",
        kind: SlashKind::Template,
        template: Some(
            "Review the following code for correctness, security, performance \
             and style. List concrete issues with severity and a suggested \
             fix:\n\n{arg}",
        ),
    },
    SlashSpec {
        name: "commit",
        aliases: &["cm"],
        description: "Write a Conventional Commit message for a diff.",
        kind: SlashKind::Template,
        template: Some(
            "Write a single Conventional Commits message (type(scope): subject \
             + body) for the following diff. Output only the message:\n\n{arg}",
        ),
    },
    SlashSpec {
        name: "plan",
        aliases: &[],
        description: "Produce a step-by-step implementation plan.",
        kind: SlashKind::Template,
        template: Some(
            "Produce a concrete, ordered implementation plan (numbered steps, \
             files to touch, risks) for the following task:\n\n{arg}",
        ),
    },
    SlashSpec {
        name: "optimize",
        aliases: &["optimise", "perf"],
        description: "Optimize code for performance.",
        kind: SlashKind::Template,
        template: Some(
            "Optimize the following for performance. Identify the bottleneck, \
             propose the change, and note the expected complexity / allocation \
             improvement:\n\n{arg}",
        ),
    },
];

/// A successfully parsed slash command: the matched spec plus its trailing
/// argument (everything after the command token, trimmed).
#[derive(Debug, Clone)]
pub struct ParsedSlash {
    /// The matched command specification.
    pub spec: &'static SlashSpec,
    /// The trailing argument text (may be empty).
    pub arg: String,
}

impl ParsedSlash {
    /// For [`SlashKind::Template`] commands, the expanded prompt to send to the
    /// model. `None` for meta commands.
    #[must_use]
    pub fn expanded_prompt(&self) -> Option<String> {
        self.spec.expand(&self.arg)
    }
}

/// Look up a command spec by name or alias (without the leading slash).
#[must_use]
pub fn lookup(name: &str) -> Option<&'static SlashSpec> {
    let n = name.trim_start_matches('/');
    SLASH_COMMANDS
        .iter()
        .find(|s| s.name == n || s.aliases.contains(&n))
}

/// Parse `input` as a slash command.
///
/// Returns `None` when `input` does not start with `/` (after trimming leading
/// whitespace) — the caller should then treat it as an ordinary prompt. A `/`
/// prefix that names no known command also returns `None` so the host can emit
/// an "unknown command" hint without misrouting.
#[must_use]
pub fn parse(input: &str) -> Option<ParsedSlash> {
    let trimmed = input.trim_start();
    let rest = trimmed.strip_prefix('/')?;
    // Split the command token from its argument.
    let (token, arg) = match rest.split_once(char::is_whitespace) {
        Some((t, a)) => (t, a.trim()),
        None => (rest, ""),
    };
    let spec = lookup(token)?;
    Some(ParsedSlash { spec, arg: arg.to_owned() })
}

/// Render the `/help` listing from [`SLASH_COMMANDS`].
#[must_use]
pub fn help_text() -> String {
    let mut out = String::from("Available slash commands:\n");
    for s in SLASH_COMMANDS {
        let kind = match s.kind {
            SlashKind::Meta => "meta",
            SlashKind::Template => "prompt",
        };
        out.push_str(&format!("  /{:<10} [{kind}]  {}\n", s.name, s.description));
    }
    out.push_str(
        "\nTemplate commands take a trailing argument, e.g.\n  \
         aphrody chat --prompt \"/explain how tokio::select works\"\n",
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_slash_input_is_not_a_command() {
        assert!(parse("hello world").is_none());
        assert!(parse("  plain text /not-at-start").is_none());
    }

    #[test]
    fn unknown_command_returns_none() {
        assert!(parse("/totallybogus arg").is_none());
    }

    #[test]
    fn parses_meta_command_without_arg() {
        let p = parse("/help").expect("help parses");
        assert_eq!(p.spec.name, "help");
        assert_eq!(p.spec.kind, SlashKind::Meta);
        assert!(p.arg.is_empty());
        assert!(p.expanded_prompt().is_none());
    }

    #[test]
    fn parses_template_command_and_expands() {
        let p = parse("/explain how does tokio::select work").expect("explain parses");
        assert_eq!(p.spec.name, "explain");
        assert_eq!(p.spec.kind, SlashKind::Template);
        let prompt = p.expanded_prompt().expect("template expands");
        assert!(prompt.contains("how does tokio::select work"));
        assert!(prompt.starts_with("Explain"));
        assert!(!prompt.contains("{arg}"));
    }

    #[test]
    fn aliases_resolve_to_canonical_spec() {
        assert_eq!(parse("/exp x").unwrap().spec.name, "explain");
        assert_eq!(parse("/doc x").unwrap().spec.name, "docs");
        assert_eq!(parse("/optimise x").unwrap().spec.name, "optimize");
        assert_eq!(parse("/?").unwrap().spec.name, "help");
    }

    #[test]
    fn leading_whitespace_is_tolerated() {
        assert_eq!(parse("   /fix segfault").unwrap().spec.name, "fix");
    }

    #[test]
    fn help_text_lists_every_command() {
        let h = help_text();
        for s in SLASH_COMMANDS {
            assert!(h.contains(&format!("/{}", s.name)), "missing {}", s.name);
        }
    }
}
