// Copyright 2026 aphrody-code
//
// Licensed under the Apache License, Version 2.0.

//! WinClean project-specific migration rules.
//!
//! Detects patterns that should be rewritten when WinClean code is touched:
//! - `Bun.spawn([Winclean.Mcp.exe])` IPC → direct in-process call via
//!   `scripts/ipc/winclean-native.ts`.
//! - Hardcoded `C:\\winclean` paths → `WINCLEAN_ROOT`.
//! - `Stop-Process` without try/catch → PPL-protected processes throw.
//! - JsonSerializer without context → NativeAOT violation.
//!
//! Rule ID space: `WC0xx`.

use n2b_types::types::{Finding, MakeFindingOpts};
use n2b_util::{Edit, apply_edits, line_offsets, make_finding};
use once_cell::sync::Lazy;
use regex::Regex;

struct Mapping {
    re: Regex,
    replace: String,
    rule_id: String,
    message: String,
    aggressive: bool,
}

static MAPPINGS: Lazy<Vec<Mapping>> = Lazy::new(|| {
    vec![
        Mapping {
            re: Regex::new(r#"Bun\.spawn\(\s*\[\s*['"][^'"]*Winclean\.Mcp\.exe['"]"#)
                .expect("WC001 regex literal must be valid"),
            replace: "/* prefer scripts/ipc/winclean-native.ts in-process call */ Bun.spawn([".to_string(),
            rule_id: "WC001".to_string(),
            message: "Bun.spawn(Winclean.Mcp.exe) crosses a JSON/stdio boundary — use Winclean.Bun [JSExport] in-process (zero IPC overhead).".to_string(),
            aggressive: false,
        },
        Mapping {
            re: Regex::new(r#"["']C:[\\\\/]+winclean[\\\\/]+"#)
                .expect("WC002 regex literal must be valid"),
            replace: r#"`${process.env.WINCLEAN_ROOT ?? "C:/winclean"}/"#.to_string(),
            rule_id: "WC002".to_string(),
            message: "Hardcoded C:\\\\winclean path — use WINCLEAN_ROOT env var (cf CLAUDE.md FHS section).".to_string(),
            aggressive: true,
        },
        // WC003 ("Stop-Process without -ErrorAction") removed — negative lookahead
        // unsupported by Rust regex crate. Re-introduce via post-match filter.
        Mapping {
            re: Regex::new(r"\bJsonSerializer\.Serialize\(\s*(\w+)\s*\)")
                .expect("WC004 regex literal must be valid"),
            replace: "JsonSerializer.Serialize($1, McpJsonContext.Default./* DTO type */)".to_string(),
            rule_id: "WC004".to_string(),
            message: "JsonSerializer.Serialize without explicit context = reflection = NativeAOT violation. Pass a JsonSerializerContext (see McpJsonContext.cs).".to_string(),
            aggressive: false,
        },
        Mapping {
            re: Regex::new(r"\bProcess\.GetProcesses\(\)")
                .expect("WC005 regex literal must be valid"),
            replace: "/* prefer ProcessManager.ScanProcesses() — single NtQuerySystemInformation syscall */ Process.GetProcesses()".to_string(),
            rule_id: "WC005".to_string(),
            message: "Process.GetProcesses() opens N handles + N queries; prefer Winclean.Core.System.ProcessManager.ScanProcesses() (1 syscall).".to_string(),
            aggressive: false,
        },
        Mapping {
            re: Regex::new(r#"Set-Service\s+-Name\s+["']?(WdiSystemHost|hns|HvHost|RasMan|LicenseManager|LSM|Themes|BFE|RpcSs|DcomLaunch|PlugPlay|Power|Schedule)["']?\s+-StartupType\s+Disabled"#)
                .expect("WC006 regex literal must be valid"),
            replace: "# BLOCKED: $1 is in WinClean never-disable list (see CLAUDE.md / feedback_never-disable-windows-services)".to_string(),
            rule_id: "WC006".to_string(),
            message: "Disabling this Windows service breaks WSL/Docker/RDP/diagnostics — see feedback_never-disable-windows-services memory.".to_string(),
            aggressive: false,
        },
    ]
});

static COMMENT_PREFIX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\s*(//|#|<!--)").expect("COMMENT_PREFIX must be a valid regex"));

/// Apply every `WC0xx` rule on the given source.
pub fn apply_winclean_rules(path: &str, source: &str, aggressive: bool) -> (Vec<Finding>, String) {
    let mut out = source.to_string();
    let mut findings: Vec<Finding> = Vec::new();
    let mut offsets = line_offsets(&out);
    let mut offsets_stale = false;

    for rule in MAPPINGS.iter() {
        if rule.aggressive && !aggressive {
            continue;
        }
        if offsets_stale {
            offsets = line_offsets(&out);
            offsets_stale = false;
        }
        let mut edits: Vec<Edit> = Vec::new();
        for mat in rule.re.find_iter(&out) {
            let line_start = out[..mat.start()].rfind('\n').map(|p| p + 1).unwrap_or(0);
            let line_end = out[mat.start()..]
                .find('\n')
                .map(|p| mat.start() + p)
                .unwrap_or(out.len());
            let line = &out[line_start..line_end];
            if COMMENT_PREFIX.is_match(line) {
                continue;
            }
            let text = mat.as_str().to_string();
            let rewritten = rule.re.replace(&text, rule.replace.as_str()).to_string();
            findings.push(make_finding(
                path,
                &offsets,
                mat.start(),
                &rule.rule_id,
                rule.message.clone(),
                text.clone(),
                Some(rewritten.clone()),
                MakeFindingOpts {
                    autofix: Some(true),
                    aggressive: Some(rule.aggressive),
                    ..Default::default()
                },
            ));
            edits.push(Edit {
                index: mat.start(),
                len: text.len(),
                replacement: rewritten,
            });
        }
        if !edits.is_empty() {
            out = apply_edits(&out, edits);
            offsets_stale = true;
        }
    }
    (findings, out)
}
