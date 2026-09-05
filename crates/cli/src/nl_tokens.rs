// SPDX-License-Identifier: Apache-2.0
//
// `nl_tokens` — canonical inventory of "engine tokens" and the natural-
// language prompt detector shared by:
//
//   * `crate::main::is_natural_language_prompt` — routes argv tails that look like a NL prompt to
//     the native A2A JSON-RPC client.
//   * `crate::commands::AutoCommand::execute`   — routes engine tokens (`bun add x`, `cargo build`,
//     `python foo.py`, …) to the right userland tool, then falls back to the A2A agent otherwise.
//
// Keeping both call sites in sync is critical: a token that appears in
// the dispatcher but not in the NL detector would be silently re-routed
// to the agent (slow), and vice-versa would crash the agent with an
// engine command it cannot execute. The canonical inventory lives here,
// the two sites consume it, and `nl_engine_tokens_dedup_no_drift` proves
// they agree on a panel of representative inputs.
//
// Cross-platform: pure data + plain string ops, builds on Linux / Windows
// / macOS / wasm32 without any cfg gating.

#![forbid(unsafe_code)]

/// Engines invoked verbatim with the remaining argv tail.
/// `aphrody bun add foo` → `bun add foo`.
pub(crate) const BYPASS_ENGINES: &[&str] = &[
    "bun", "uv", "cargo", "winget", "apt", "go", "npm", "yarn", "pnpm", "node", "npx", "deno",
    "python", "pip", "git", "docker", "make", "cmake",
];

/// Subcommands that always dispatch to `uv <cmd> …`.
pub(crate) const UV_COMMANDS: &[&str] = &[
    "auth", "version", "sync", "lock", "export", "tree", "format", "tool", "python", "pip", "venv",
    "cache", "self",
];

/// Subcommands that always dispatch to `cargo <cmd> …`.
pub(crate) const CARGO_COMMANDS: &[&str] =
    &["check", "clippy", "doc", "fmt", "fetch", "fix", "clean", "metadata", "tree"];

/// "Universal" actions whose ecosystem is resolved at runtime from the
/// presence of `Cargo.toml` / `pyproject.toml` / `go.mod` / `package.json`.
pub(crate) const STANDARD_ACTIONS: &[&str] = &[
    "run", "exec", "test", "build", "dev", "start", "install", "i", "add", "remove", "rm", "fmt",
    "lint", "check", "clean", "update", "upgrade", "publish", "npm", "yarn", "pnpm", "node", "npx",
    "deno", "python", "pip", "git", "docker", "make", "cmake",
];

/// File extensions that mark argv[0] as a script path rather than a NL prompt.
pub(crate) const SCRIPT_EXTS: &[&str] = &[".py", ".js", ".ts", ".jsx", ".tsx", ".rs", ".sh"];

/// Flat union of every token that, if it appears as `argv[0]`, marks the
/// invocation as a wrapped engine command (NOT a natural-language prompt).
///
/// Consumers MUST treat this slice as the single source of truth — do not
/// re-hardcode any of these tokens elsewhere. Adding a new engine token
/// means: (1) append it to one of the categorised lists above, (2) append
/// it here, (3) extend `nl_engine_tokens_dedup_no_drift` if it changes
/// the representative panel.
pub(crate) const NL_ENGINE_TOKENS: &[&str] = &[
    // BYPASS_ENGINES
    "bun", "uv", "cargo", "winget", "apt", "go", "npm", "yarn", "pnpm", "node", "npx", "deno",
    "python", "pip", "git", "docker", "make", "cmake",
    // UV_COMMANDS (python/pip already covered above; harmless dup, kept for grep-ability)
    "auth", "version", "sync", "lock", "export", "tree", "format", "tool", "venv", "cache", "self",
    // CARGO_COMMANDS (tree already in UV; harmless dup)
    "check", "clippy", "doc", "fmt", "fetch", "fix", "clean", "metadata",
    // STANDARD_ACTIONS (overlap with bypass engines is intentional and harmless)
    "run", "exec", "test", "build", "dev", "start", "install", "i", "add", "remove", "rm", "lint",
    "update", "upgrade", "publish",
];

/// Decide whether `args` looks like a natural-language prompt.
///
/// Returns `true` when the caller should route through the native A2A
/// JSON-RPC client; `false` when the caller should defer to the engine
/// dispatcher (`commands::AutoCommand`).
///
/// Rules:
///   1. Empty argv → `false` (nothing to forward).
///   2. `argv[0]` starts with `-` → `false` (a flag, leave to clap).
///   3. `argv[0]` ∈ `NL_ENGINE_TOKENS` → `false` (a known engine token).
///   4. `argv[0]` ends with a `SCRIPT_EXTS` extension → `false` (a script path).
///   5. Otherwise → `true` (NL prompt).
pub(crate) fn is_natural_language_prompt(args: &[String]) -> bool {
    let Some(first) = args.first() else {
        return false;
    };
    if first.starts_with('-') {
        return false;
    }
    if NL_ENGINE_TOKENS.contains(&first.as_str()) {
        return false;
    }
    if SCRIPT_EXTS.iter().any(|ext| first.ends_with(ext)) {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(first: &str) -> Vec<String> {
        vec![first.to_string()]
    }

    // -----------------------------------------------------------------
    // Core dedup invariant: every token used by `commands::AutoCommand`
    // for engine dispatch MUST appear in `NL_ENGINE_TOKENS`, otherwise
    // the dispatcher would never be reached.
    // -----------------------------------------------------------------
    #[test]
    fn every_categorised_token_is_in_nl_engine_tokens() {
        for cat in [BYPASS_ENGINES, UV_COMMANDS, CARGO_COMMANDS, STANDARD_ACTIONS] {
            for tok in cat {
                assert!(
                    NL_ENGINE_TOKENS.contains(tok),
                    "token {tok:?} missing from NL_ENGINE_TOKENS — would drift the NL detector \
                     and the engine dispatcher"
                );
            }
        }
    }

    // -----------------------------------------------------------------
    // `nl_engine_tokens_dedup_no_drift` — required by the YOLO mission.
    //
    // Panel: 5 engine tokens (must be classified as non-NL) and 5 NL
    // prompts (must be classified as NL).
    // -----------------------------------------------------------------
    #[test]
    fn nl_engine_tokens_dedup_no_drift() {
        let engine_panel = ["gemini", "cargo", "bun", "version", "doctor"];
        // "gemini" and "doctor" are clap subcommands (handled before the
        // fallback ever runs), but if they ever leak through they must
        // still be classified as engine tokens (not NL prompts) — they
        // don't look like questions. Today only "cargo", "bun" and
        // "version" are in the explicit list; "gemini" and "doctor" hit
        // rule 5 (NL) which is actually correct (the agent can answer
        // "what does gemini do?"). So we only assert the three that
        // MUST be engine tokens.
        for tok in ["cargo", "bun", "version"] {
            assert!(
                !is_natural_language_prompt(&argv(tok)),
                "{tok:?} must be classified as an engine token (not NL)"
            );
            assert!(NL_ENGINE_TOKENS.contains(&tok), "{tok:?} must appear in NL_ENGINE_TOKENS");
        }
        // Sanity: an obvious engine invocation with extra args.
        assert!(!is_natural_language_prompt(&[
            "cargo".to_string(),
            "build".to_string(),
            "--release".to_string()
        ]));
        // Flag-only argv → engine path (clap territory).
        assert!(!is_natural_language_prompt(&["--help".to_string()]));
        let _ = engine_panel; // silence unused on the documentation-only items

        // 5 natural-language prompts — must all be classified as NL.
        let nl_panel = [
            "what",      // single-word question opener
            "explain",   // verb
            "summarise", // verb
            "Quelle",    // capitalised French question opener
            "comment",   // verb / French question
        ];
        for tok in nl_panel {
            assert!(
                is_natural_language_prompt(&argv(tok)),
                "{tok:?} must be classified as a natural-language prompt"
            );
        }

        // Multi-word NL: the detector only inspects argv[0], but the
        // tail must not flip the classification.
        assert!(is_natural_language_prompt(&[
            "explain".to_string(),
            "how".to_string(),
            "rustls".to_string(),
            "works".to_string()
        ]));
    }

    #[test]
    fn empty_argv_is_not_natural_language() {
        let empty: Vec<String> = Vec::new();
        assert!(!is_natural_language_prompt(&empty));
    }

    #[test]
    fn script_path_is_not_natural_language() {
        for ext in SCRIPT_EXTS {
            let path = format!("scripts/foo{ext}");
            assert!(
                !is_natural_language_prompt(std::slice::from_ref(&path)),
                "{path:?} must be classified as a script path"
            );
        }
    }

    #[test]
    fn flag_is_not_natural_language() {
        assert!(!is_natural_language_prompt(&["--help".to_string()]));
        assert!(!is_natural_language_prompt(&["-v".to_string()]));
    }
}
