// SPDX-License-Identifier: Apache-2.0
//! Cross-platform shell command-safety classifier.
//!
//! Given a command as an already-tokenised `argv` (`&[String]`), decide whether
//! an autonomous agent may run it without approval:
//!
//! * [`Decision::Allow`] — provably read-only / side-effect-free (an allow-list
//!   of well-known tools, restricted to their safe option sets).
//! * [`Decision::Forbidden`] — matches a known-destructive pattern (`rm -rf`,
//!   `dd`, `mkfs`, `git push --force`, a fork bomb, …). Never auto-run.
//! * [`Decision::Prompt`] — everything else: unknown, so escalate to a human (or,
//!   in fully-autonomous mode, run under the sandbox / require an explicit opt-in).
//!
//! The classifier is **conservative**: when in doubt it returns `Prompt`, never
//! `Allow`. It understands `bash -lc "<script>"` (and `sh`/`zsh`) when the script
//! is a pipeline/sequence of plain commands joined by `&&`, `||`, `;`, `|` — any
//! redirection, subshell, command substitution, or backgrounding makes the whole
//! script un-provable and downgrades it to `Prompt` (or `Forbidden` if a
//! destructive command appears anywhere in it).
//!
//! Logic distilled from OpenAI Codex's `shell-command` crate (Apache-2.0),
//! re-implemented with a dependency-free shell tokenizer.

/// Outcome of classifying a command. Ordered so a multi-command script can take
/// the most restrictive verdict via `max`: `Allow < Prompt < Forbidden`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Decision {
    /// Provably safe to auto-run.
    Allow,
    /// Unknown — require approval / sandboxing.
    Prompt,
    /// Known-destructive — never auto-run.
    Forbidden,
}

/// Classify a tokenised command into a [`Decision`].
///
/// This is a pure, always-on primitive — it reports what it sees regardless of
/// any environment setting. Use [`classify_command_enforced`] for the
/// guardrail-respecting variant that callers should gate auto-approval on.
#[must_use]
pub fn classify_command(command: &[String]) -> Decision {
    if is_dangerous_command(command) {
        return Decision::Forbidden;
    }
    if is_known_safe_command(command) {
        return Decision::Allow;
    }
    Decision::Prompt
}

/// Guardrail-respecting classification.
///
/// When guardrails are **disabled** (the default — see [`crate::GUARD_ENV`]),
/// every command is reported as [`Decision::Allow`], so an autonomous agent
/// runs unimpeded. When `APHRODY_GUARD` is opted in, this delegates to
/// [`classify_command`] and real `Prompt` / `Forbidden` verdicts apply.
#[must_use]
pub fn classify_command_enforced(command: &[String]) -> Decision {
    if crate::guardrails_enabled() {
        classify_command(command)
    } else {
        Decision::Allow
    }
}

// ============================================================================
//  Known-safe (read-only) classification
// ============================================================================

/// Returns `true` if `command` is provably read-only and safe to auto-run.
#[must_use]
pub fn is_known_safe_command(command: &[String]) -> bool {
    // Treat `zsh`/`sh` like `bash` for the `-lc` script form.
    if is_safe_to_call_with_exec(command) {
        return true;
    }

    if let Some(segments) = parse_shell_lc_plain_commands(command) {
        return !segments.is_empty() && segments.iter().all(|c| is_safe_to_call_with_exec(c));
    }

    if is_safe_powershell_words(command) {
        return true;
    }

    false
}

fn is_safe_to_call_with_exec(command: &[String]) -> bool {
    let Some(cmd0) = command.first().map(String::as_str) else {
        return false;
    };

    match executable_name_lookup_key(cmd0).as_deref() {
        // GNU-only read-only tools.
        Some(cmd) if cfg!(target_os = "linux") && matches!(cmd, "numfmt" | "tac") => true,

        #[rustfmt::skip]
        Some(
            "cat" | "cd" | "cut" | "date" | "df" | "dirname" | "du" | "echo" | "env" | "expr"
            | "false" | "file" | "basename" | "grep" | "head" | "hexdump" | "hostname" | "id"
            | "ls" | "nl" | "od" | "paste" | "printenv" | "ps" | "pwd" | "readlink" | "realpath"
            | "rev" | "seq" | "sort" | "stat" | "tail" | "tr" | "true" | "uname" | "uniq"
            | "uptime" | "wc" | "which" | "whoami",
        ) => true,

        Some("base64") => {
            const UNSAFE: &[&str] = &["-o", "--output"];
            !command.iter().skip(1).any(|arg| {
                UNSAFE.contains(&arg.as_str())
                    || arg.starts_with("--output=")
                    || (arg.starts_with("-o") && arg != "-o")
            })
        }

        Some("find") => {
            // `find` can execute, delete, or write files via these options.
            #[rustfmt::skip]
            const UNSAFE: &[&str] = &[
                "-exec", "-execdir", "-ok", "-okdir",
                "-delete",
                "-fls", "-fprint", "-fprint0", "-fprintf",
            ];
            !command.iter().any(|arg| UNSAFE.contains(&arg.as_str()))
        }

        Some("rg") => {
            const UNSAFE_WITH_ARG: &[&str] = &["--pre", "--hostname-bin"];
            const UNSAFE_BARE: &[&str] = &["--search-zip", "-z"];
            !command.iter().any(|arg| {
                UNSAFE_BARE.contains(&arg.as_str())
                    || UNSAFE_WITH_ARG
                        .iter()
                        .any(|&opt| arg == opt || arg.starts_with(&format!("{opt}=")))
            })
        }

        Some("git") => is_safe_git_command(command),

        // `sed -n {N|M,N}p file` — print-only.
        Some("sed")
            if command.len() <= 4
                && command.get(1).map(String::as_str) == Some("-n")
                && is_valid_sed_n_arg(command.get(2).map(String::as_str)) =>
        {
            true
        }

        _ => false,
    }
}

fn is_safe_git_command(command: &[String]) -> bool {
    let Some((idx, sub)) =
        find_git_subcommand(command, &["status", "log", "diff", "show", "branch"])
    else {
        return false;
    };

    if git_has_unsafe_global_option(&command[1..idx]) {
        return false;
    }
    let args = &command[idx + 1..];

    match sub.as_str() {
        "status" | "log" | "diff" | "show" => git_subcommand_args_are_read_only(args),
        "branch" => git_subcommand_args_are_read_only(args) && git_branch_is_read_only(args),
        _ => false,
    }
}

/// Find the first non-option token of a `git` invocation and return it if it is
/// one of `allowed`, skipping global options that precede the subcommand.
fn find_git_subcommand(command: &[String], allowed: &[&str]) -> Option<(usize, String)> {
    let mut i = 1;
    while i < command.len() {
        let tok = command[i].as_str();
        if tok.starts_with('-') {
            // Global options that consume the following token as their value.
            if matches!(tok, "-C" | "-c" | "--git-dir" | "--work-tree" | "--namespace"
                | "--super-prefix" | "--exec-path" | "--config-env")
            {
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }
        let key = executable_name_lookup_key(tok).unwrap_or_else(|| tok.to_string());
        return allowed.contains(&key.as_str()).then_some((i, key));
    }
    None
}

fn git_branch_is_read_only(branch_args: &[String]) -> bool {
    if branch_args.is_empty() {
        return true;
    }
    let mut saw_read_only = false;
    for arg in branch_args.iter().map(String::as_str) {
        match arg {
            "--list" | "-l" | "--show-current" | "-a" | "--all" | "-r" | "--remotes" | "-v"
            | "-vv" | "--verbose" => saw_read_only = true,
            _ if arg.starts_with("--format=") => saw_read_only = true,
            _ => return false,
        }
    }
    saw_read_only
}

#[derive(Clone, Copy)]
enum GitOptionPattern {
    Exact(&'static str),
    ShortWithInlineValue(&'static str),
    Prefix(&'static str),
}

impl GitOptionPattern {
    fn matches(self, arg: &str) -> bool {
        match self {
            GitOptionPattern::Exact(o) => arg == o,
            GitOptionPattern::ShortWithInlineValue(o) => arg.starts_with(o) && arg.len() > o.len(),
            GitOptionPattern::Prefix(p) => arg.starts_with(p),
        }
    }
}

const UNSAFE_GIT_GLOBAL_OPTIONS: &[GitOptionPattern] = &[
    GitOptionPattern::Exact("-C"),
    GitOptionPattern::ShortWithInlineValue("-C"),
    GitOptionPattern::Exact("-c"),
    GitOptionPattern::ShortWithInlineValue("-c"),
    GitOptionPattern::Exact("-p"),
    GitOptionPattern::Exact("--config-env"),
    GitOptionPattern::Prefix("--config-env="),
    GitOptionPattern::Exact("--exec-path"),
    GitOptionPattern::Prefix("--exec-path="),
    GitOptionPattern::Exact("--git-dir"),
    GitOptionPattern::Prefix("--git-dir="),
    GitOptionPattern::Exact("--namespace"),
    GitOptionPattern::Prefix("--namespace="),
    GitOptionPattern::Exact("--paginate"),
    GitOptionPattern::Exact("--super-prefix"),
    GitOptionPattern::Prefix("--super-prefix="),
    GitOptionPattern::Exact("--work-tree"),
    GitOptionPattern::Prefix("--work-tree="),
];

const UNSAFE_GIT_SUBCOMMAND_OPTIONS: &[GitOptionPattern] = &[
    GitOptionPattern::Exact("--output"),
    GitOptionPattern::Prefix("--output="),
    GitOptionPattern::Exact("--ext-diff"),
    GitOptionPattern::Exact("--textconv"),
    GitOptionPattern::Exact("--exec"),
    GitOptionPattern::Prefix("--exec="),
];

fn git_has_unsafe_global_option(args: &[String]) -> bool {
    args.iter()
        .any(|a| UNSAFE_GIT_GLOBAL_OPTIONS.iter().any(|p| p.matches(a)))
}

fn git_subcommand_args_are_read_only(args: &[String]) -> bool {
    !args
        .iter()
        .any(|a| UNSAFE_GIT_SUBCOMMAND_OPTIONS.iter().any(|p| p.matches(a)))
}

/// `^(\d+,)?\d+p$`
fn is_valid_sed_n_arg(arg: Option<&str>) -> bool {
    let Some(core) = arg.and_then(|s| s.strip_suffix('p')) else {
        return false;
    };
    let parts: Vec<&str> = core.split(',').collect();
    let numeric = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
    match parts.as_slice() {
        [n] => numeric(n),
        [a, b] => numeric(a) && numeric(b),
        _ => false,
    }
}

// ============================================================================
//  PowerShell read-only safelist (used directly + for full-path invocations)
// ============================================================================

/// A small read-only PowerShell cmdlet safelist. Only `<cmdlet> [args]` forms
/// are recognised; anything with a pipe to a mutating cmdlet or a script block
/// falls through to `Prompt`.
fn is_safe_powershell_words(command: &[String]) -> bool {
    let Some(first) = command.first() else {
        return false;
    };
    // Either `pwsh -Command <cmdlet> ...` or a bare cmdlet.
    let key = executable_name_lookup_key(first).unwrap_or_default();
    let rest: &[String] = if matches!(key.as_str(), "pwsh" | "powershell") {
        match command.get(1).map(String::as_str) {
            Some("-Command" | "-c") => &command[2..],
            _ => return false,
        }
    } else {
        command
    };

    let Some(cmdlet) = rest.first().map(String::as_str) else {
        return false;
    };
    #[rustfmt::skip]
    const SAFE_CMDLETS: &[&str] = &[
        "Get-Location", "Get-ChildItem", "Get-Content", "Get-Item", "Get-Date",
        "Get-Command", "Get-Process", "Get-Host", "Write-Output", "Select-Object",
        "Measure-Object", "Get-Member", "Format-List", "Format-Table",
    ];
    SAFE_CMDLETS.iter().any(|c| cmdlet.eq_ignore_ascii_case(c))
}

// ============================================================================
//  Known-dangerous classification
// ============================================================================

/// Returns `true` if `command` matches a known-destructive pattern that must
/// never be auto-run. Recurses into `bash -lc "<script>"` segments.
#[must_use]
pub fn is_dangerous_command(command: &[String]) -> bool {
    if exec_is_dangerous(command) {
        return true;
    }
    // A script we cannot prove safe but CAN prove dangerous: split conservatively
    // and flag if any segment is dangerous. We reuse the plain-command splitter,
    // but the splitter returns `None` on structural hazards (redirects, subshells,
    // substitutions). For danger detection we still want to look inside, so fall
    // back to a loose split on the same operators when the strict parse fails.
    if let Some(segments) = parse_shell_lc_plain_commands(command) {
        return segments.iter().any(|c| exec_is_dangerous(c));
    }
    if let Some(script) = shell_lc_script(command) {
        // Loose check: the strict tokenizer bails on redirections / subshells,
        // but a destructive command can hide behind exactly those. Split loosely
        // (respecting quotes) on command separators and scan each segment.
        for seg in loose_segments(script) {
            if exec_is_dangerous(&seg) {
                return true;
            }
        }
    }
    false
}

/// Best-effort split of a script into command segments for *danger* detection
/// only. Unlike [`tokenize`], this never bails: it respects single/double quotes
/// so separators inside strings are literal, splits on `;`, `|`, `&`, and
/// newlines, and whitespace-tokenises each segment (redirection operators are
/// left as harmless words). Never used to grant `Allow`.
fn loose_segments(script: &str) -> Vec<Vec<String>> {
    let mut segments = Vec::new();
    let mut words: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut in_word = false;
    let mut quote: Option<char> = None;

    let push_word = |cur: &mut String, in_word: &mut bool, words: &mut Vec<String>| {
        if *in_word {
            words.push(std::mem::take(cur));
            *in_word = false;
        }
    };

    let chars: Vec<char> = script.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if let Some(q) = quote {
            if c == q {
                quote = None;
            } else {
                cur.push(c);
            }
            in_word = true;
            i += 1;
            continue;
        }
        match c {
            '\'' | '"' => {
                quote = Some(c);
                in_word = true;
            }
            ' ' | '\t' | '>' | '<' => push_word(&mut cur, &mut in_word, &mut words),
            ';' | '|' | '&' | '\n' | '\r' => {
                push_word(&mut cur, &mut in_word, &mut words);
                if !words.is_empty() {
                    segments.push(std::mem::take(&mut words));
                }
            }
            other => {
                cur.push(other);
                in_word = true;
            }
        }
        i += 1;
    }
    push_word(&mut cur, &mut in_word, &mut words);
    if !words.is_empty() {
        segments.push(words);
    }
    segments
}

fn exec_is_dangerous(command: &[String]) -> bool {
    let Some(cmd0) = command.first().map(String::as_str) else {
        return false;
    };
    let key = executable_name_lookup_key(cmd0).unwrap_or_else(|| cmd0.to_string());
    let args: Vec<&str> = command.iter().skip(1).map(String::as_str).collect();
    let has = |needle: &str| args.contains(&needle);
    let has_prefix = |p: &str| args.iter().any(|a| a.starts_with(p));

    match key.as_str() {
        // Recursive/forced removal.
        "rm" => {
            args.iter().any(|a| {
                a.starts_with('-')
                    && !a.starts_with("--")
                    && (a.contains('r') || a.contains('R') || a.contains('f'))
            }) || has("--recursive")
                || has("--force")
        }
        // Raw disk / filesystem destroyers.
        "dd" => has_prefix("of=") || has("of"),
        "mkfs" | "shred" | "wipefs" | "fdisk" | "parted" | "blkdiscard" => true,
        "mkfs.ext4" | "mkfs.xfs" | "mkfs.btrfs" | "mkfs.vfat" => true,
        // Mass permission/ownership changes.
        "chmod" | "chown" | "chgrp" => has("-R") || has("--recursive"),
        // Privilege escalation wrappers — never auto-run.
        "sudo" | "doas" | "su" | "runas" => true,
        // Fork bomb / arbitrary eval entry points handled via tokens below.
        "kill" | "killall" | "pkill" => has("-9") || has("-KILL") || has_prefix("-"),
        // git history/remote mutation.
        "git" => git_is_dangerous(command),
        // Package managers performing system mutation.
        "apt" | "apt-get" | "dnf" | "yum" | "pacman" | "zypper" => {
            args.iter().any(|a| matches!(*a, "remove" | "purge" | "autoremove"))
        }
        _ => false,
    }
}

fn git_is_dangerous(command: &[String]) -> bool {
    // Look at the first non-option subcommand.
    let mut i = 1;
    while i < command.len() && command[i].starts_with('-') {
        i += 1;
    }
    let Some(sub) = command.get(i).map(String::as_str) else {
        return false;
    };
    let rest: Vec<&str> = command.iter().skip(i + 1).map(String::as_str).collect();
    let force = rest.iter().any(|a| *a == "-f" || *a == "--force" || a.starts_with("--force-"));
    match sub {
        "push" => force,
        "reset" => rest.contains(&"--hard"),
        "clean" => rest.iter().any(|a| a.starts_with("-") && a.contains('f')),
        "checkout" => rest.iter().any(|a| *a == "-f" || *a == "--force"),
        _ => false,
    }
}

// ============================================================================
//  Dependency-free shell tokenizer / splitter
// ============================================================================

/// Extract the script body of a `bash -lc "<script>"` / `sh -c` / `zsh -lc`
/// invocation, if the command has exactly that shape.
fn shell_lc_script(command: &[String]) -> Option<&str> {
    if command.len() != 3 {
        return None;
    }
    let shell = executable_name_lookup_key(&command[0])?;
    if !matches!(shell.as_str(), "bash" | "sh" | "zsh") {
        return None;
    }
    // Accept the common option clusters: -c, -lc, -lic, -ic …
    let opt = command[1].as_str();
    let ok = opt.starts_with('-') && opt.ends_with('c') && opt[1..].chars().all(|c| "lic".contains(c));
    ok.then(|| command[2].as_str())
}

/// Parse `bash -lc "<script>"` into a sequence of plain-command argv vectors,
/// returning `None` if the command is not a shell `-c` form, or the script
/// contains any construct we cannot prove side-effect-free (redirection,
/// subshell, command/process substitution, backgrounding, here-docs, newlines).
fn parse_shell_lc_plain_commands(command: &[String]) -> Option<Vec<Vec<String>>> {
    let script = shell_lc_script(command)?;
    let words = tokenize(script)?;
    if words.is_empty() {
        return Some(Vec::new());
    }
    Some(split_on_operators(&words))
}

/// A token from the shell tokenizer: either a literal word, or a control
/// operator we allow between commands.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Tok {
    Word(String),
    /// `&&`, `||`, `;`, `|`
    Op,
}

/// Tokenise a shell script into words and the four allowed separators, returning
/// `None` if any disallowed metacharacter is present.
fn tokenize(script: &str) -> Option<Vec<Tok>> {
    let mut out: Vec<Tok> = Vec::new();
    // `None` = no word in progress; `Some(_)` = building a word (possibly empty,
    // e.g. from `""`). This avoids a separate "has word" flag whose final reset
    // would read as a dead assignment.
    let mut cur: Option<String> = None;
    let chars: Vec<char> = script.chars().collect();
    let mut i = 0;

    let flush = |out: &mut Vec<Tok>, cur: &mut Option<String>| {
        if let Some(word) = cur.take() {
            out.push(Tok::Word(word));
        }
    };

    while i < chars.len() {
        let c = chars[i];
        match c {
            // Quoting: consume verbatim into the current word.
            '\'' => {
                let word = cur.get_or_insert_with(String::new);
                i += 1;
                while i < chars.len() && chars[i] != '\'' {
                    word.push(chars[i]);
                    i += 1;
                }
                if i >= chars.len() {
                    return None; // unterminated
                }
                i += 1;
            }
            '"' => {
                let word = cur.get_or_insert_with(String::new);
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    // No command substitution allowed inside double quotes.
                    if chars[i] == '`' || (chars[i] == '$' && chars.get(i + 1) == Some(&'(')) {
                        return None;
                    }
                    if chars[i] == '\\' && i + 1 < chars.len() {
                        word.push(chars[i + 1]);
                        i += 2;
                        continue;
                    }
                    word.push(chars[i]);
                    i += 1;
                }
                if i >= chars.len() {
                    return None;
                }
                i += 1;
            }
            '\\' if i + 1 < chars.len() => {
                cur.get_or_insert_with(String::new).push(chars[i + 1]);
                i += 2;
            }
            ' ' | '\t' => {
                flush(&mut out, &mut cur);
                i += 1;
            }
            // Disallowed: newlines, redirection, subshell, substitution, expansion.
            '\n' | '\r' | '<' | '>' | '(' | ')' | '{' | '}' | '`' => return None,
            // Reject `$(...)` / `${...}`; conservatively reject any expansion.
            '$' => return None,
            '&' => {
                flush(&mut out, &mut cur);
                if chars.get(i + 1) == Some(&'&') {
                    out.push(Tok::Op);
                    i += 2;
                } else {
                    return None; // backgrounding
                }
            }
            '|' => {
                flush(&mut out, &mut cur);
                if chars.get(i + 1) == Some(&'|') {
                    i += 2;
                } else {
                    i += 1;
                }
                out.push(Tok::Op);
            }
            ';' => {
                flush(&mut out, &mut cur);
                out.push(Tok::Op);
                i += 1;
            }
            other => {
                cur.get_or_insert_with(String::new).push(other);
                i += 1;
            }
        }
    }
    flush(&mut out, &mut cur);
    Some(out)
}

/// Split a token stream on operators into per-command argv vectors. Empty
/// segments (e.g. trailing `;`) are dropped.
fn split_on_operators(tokens: &[Tok]) -> Vec<Vec<String>> {
    let mut segments = Vec::new();
    let mut cur: Vec<String> = Vec::new();
    for t in tokens {
        match t {
            Tok::Word(w) => cur.push(w.clone()),
            Tok::Op => {
                if !cur.is_empty() {
                    segments.push(std::mem::take(&mut cur));
                }
            }
        }
    }
    if !cur.is_empty() {
        segments.push(cur);
    }
    segments
}

/// Normalise an executable token to a lookup key: drop any directory component
/// and, on Windows, a trailing `.exe`/`.cmd`/`.bat` and case.
fn executable_name_lookup_key(cmd: &str) -> Option<String> {
    let base = cmd.rsplit(['/', '\\']).next().unwrap_or(cmd);
    if base.is_empty() {
        return None;
    }
    if cfg!(windows) {
        let lower = base.to_ascii_lowercase();
        let stripped = lower
            .strip_suffix(".exe")
            .or_else(|| lower.strip_suffix(".cmd"))
            .or_else(|| lower.strip_suffix(".bat"))
            .unwrap_or(&lower);
        Some(stripped.to_string())
    } else {
        Some(base.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(args: &[&str]) -> Vec<String> {
        args.iter().map(|s| (*s).to_string()).collect()
    }

    // ---- safe direct commands ------------------------------------------------

    #[test]
    fn safe_direct_commands() {
        assert!(is_known_safe_command(&v(&["ls"])));
        assert!(is_known_safe_command(&v(&["ls", "-la"])));
        assert!(is_known_safe_command(&v(&["git", "status"])));
        assert!(is_known_safe_command(&v(&["git", "log", "-p", "-1"])));
        assert!(is_known_safe_command(&v(&["git", "branch", "--show-current"])));
        assert!(is_known_safe_command(&v(&["sed", "-n", "1,5p", "f.txt"])));
        assert!(is_known_safe_command(&v(&["rg", "needle", "-n"])));
        assert!(is_known_safe_command(&v(&["find", ".", "-name", "x"])));
    }

    #[test]
    fn unsafe_or_unknown_direct_commands() {
        assert!(!is_known_safe_command(&v(&["cargo", "check"])));
        assert!(!is_known_safe_command(&v(&["git", "fetch"])));
        assert!(!is_known_safe_command(&v(&["git", "-C", ".", "status"])));
        assert!(!is_known_safe_command(&v(&["git", "log", "--output=/tmp/x"])));
        assert!(!is_known_safe_command(&v(&["find", ".", "-delete"])));
        assert!(!is_known_safe_command(&v(&["rg", "-z", "x"])));
        assert!(!is_known_safe_command(&v(&["base64", "-o", "out"])));
        assert!(!is_known_safe_command(&v(&["sed", "-n", "xp", "f"])));
    }

    // ---- bash -lc parsing ----------------------------------------------------

    #[test]
    fn bash_lc_safe_pipelines() {
        assert!(is_known_safe_command(&v(&["bash", "-lc", "ls"])));
        assert!(is_known_safe_command(&v(&["bash", "-lc", "ls -1"])));
        assert!(is_known_safe_command(&v(&["bash", "-lc", "git status"])));
        assert!(is_known_safe_command(&v(&["bash", "-lc", "ls && pwd"])));
        assert!(is_known_safe_command(&v(&["bash", "-lc", "echo hi ; ls"])));
        assert!(is_known_safe_command(&v(&["bash", "-lc", "ls | wc -l"])));
        assert!(is_known_safe_command(&v(&["zsh", "-lc", "ls"])));
        assert!(is_known_safe_command(&v(&["bash", "-lc", "grep -R foo -n || true"])));
    }

    #[test]
    fn bash_lc_unsafe_constructs() {
        assert!(!is_known_safe_command(&v(&["bash", "-lc", "ls > out"])));
        assert!(!is_known_safe_command(&v(&["bash", "-lc", "(ls)"])));
        assert!(!is_known_safe_command(&v(&["bash", "-lc", "echo $(whoami)"])));
        assert!(!is_known_safe_command(&v(&["bash", "-lc", "cat `which ls`"])));
        assert!(!is_known_safe_command(&v(&["bash", "-lc", "ls & "])));
        assert!(!is_known_safe_command(&v(&["bash", "-lc", "'git status'"])));
        // Wrong arg count: not the 3-token -c form.
        assert!(!is_known_safe_command(&v(&["bash", "-lc", "git", "status"])));
    }

    // ---- dangerous detection -------------------------------------------------

    #[test]
    fn dangerous_direct() {
        assert!(is_dangerous_command(&v(&["rm", "-rf", "/"])));
        assert!(is_dangerous_command(&v(&["rm", "-r", "build"])));
        assert!(is_dangerous_command(&v(&["dd", "if=/dev/zero", "of=/dev/sda"])));
        assert!(is_dangerous_command(&v(&["mkfs.ext4", "/dev/sda1"])));
        assert!(is_dangerous_command(&v(&["chmod", "-R", "777", "/"])));
        assert!(is_dangerous_command(&v(&["sudo", "rm", "x"])));
        assert!(is_dangerous_command(&v(&["git", "push", "--force"])));
        assert!(is_dangerous_command(&v(&["git", "reset", "--hard", "HEAD~5"])));
        assert!(is_dangerous_command(&v(&["apt-get", "remove", "coreutils"])));
    }

    #[test]
    fn dangerous_not_flagged_for_safe() {
        assert!(!is_dangerous_command(&v(&["ls", "-la"])));
        assert!(!is_dangerous_command(&v(&["git", "push"])));
        assert!(!is_dangerous_command(&v(&["git", "status"])));
        assert!(!is_dangerous_command(&v(&["rm", "file.txt"])));
        assert!(!is_dangerous_command(&v(&["chmod", "644", "f"])));
    }

    #[test]
    fn dangerous_inside_script() {
        assert!(is_dangerous_command(&v(&["bash", "-lc", "ls && rm -rf /"])));
        assert!(is_dangerous_command(&v(&["bash", "-lc", "echo hi > log && rm -rf build"])));
    }

    // ---- top-level classifier ------------------------------------------------

    #[test]
    fn classify_three_way() {
        assert_eq!(classify_command(&v(&["ls"])), Decision::Allow);
        assert_eq!(classify_command(&v(&["cargo", "build"])), Decision::Prompt);
        assert_eq!(classify_command(&v(&["rm", "-rf", "/"])), Decision::Forbidden);
        // A script mixing a safe and a destructive command is Forbidden, not Allow.
        assert_eq!(
            classify_command(&v(&["bash", "-lc", "ls && rm -rf /"])),
            Decision::Forbidden
        );
    }

    #[test]
    fn enforced_classification_allows_everything_when_guard_disabled() {
        // SAFETY: single-threaded test; we set and restore the env var locally.
        // By default (guardrails off) even a destructive command is reported
        // Allow by the *enforced* classifier — full autonomy, no guardrail.
        unsafe {
            std::env::remove_var(crate::GUARD_ENV);
        }
        assert_eq!(classify_command_enforced(&v(&["rm", "-rf", "/"])), Decision::Allow);
        assert_eq!(classify_command_enforced(&v(&["cargo", "build"])), Decision::Allow);
        // The pure classifier still tells the truth regardless of the env.
        assert_eq!(classify_command(&v(&["rm", "-rf", "/"])), Decision::Forbidden);
    }

    #[test]
    fn lookup_key_strips_path_and_exe() {
        assert_eq!(executable_name_lookup_key("/usr/bin/git").as_deref(), Some("git"));
        if cfg!(windows) {
            assert_eq!(
                executable_name_lookup_key(r"C:\Program Files\Git\cmd\git.exe").as_deref(),
                Some("git")
            );
        }
    }
}
