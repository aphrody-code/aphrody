// Copyright 2026 aphrody-code
//
// Licensed under the Apache License, Version 2.0.

//! Windows 11 idiom migration rules.
//!
//! Detects deprecated PowerShell cmdlets, Windows-specific path mistakes, and
//! hardcoded user-profile paths. Active on `.ps1`, `.psm1`, `.psd1`, `.cmd`,
//! `.bat`, `.ts`, `.js`, `.yml` files.
//!
//! Rule ID space: `WN0xx`.

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
            re: Regex::new(r"\bGet-WmiObject\b")
                .expect("WN001 regex literal must be valid"),
            replace: "Get-CimInstance".to_string(),
            rule_id: "WN001".to_string(),
            message: "Get-WmiObject is deprecated since PS 6 — use Get-CimInstance (DCOM/WinRM transport, faster).".to_string(),
            aggressive: false,
        },
        Mapping {
            re: Regex::new(r"\bInvoke-WmiMethod\b")
                .expect("WN002 regex literal must be valid"),
            replace: "Invoke-CimMethod".to_string(),
            rule_id: "WN002".to_string(),
            message: "Invoke-WmiMethod is deprecated — use Invoke-CimMethod (CIM cmdlets).".to_string(),
            aggressive: false,
        },
        Mapping {
            re: Regex::new(r"\bGet-EventLog\b")
                .expect("WN003 regex literal must be valid"),
            replace: "Get-WinEvent".to_string(),
            rule_id: "WN003".to_string(),
            message: "Get-EventLog only reads classic Windows logs — Get-WinEvent supports modern ETW/EVTX channels.".to_string(),
            aggressive: false,
        },
        Mapping {
            re: Regex::new(r"\bpowershell\.exe\b")
                .expect("WN004 regex literal must be valid"),
            replace: "pwsh.exe".to_string(),
            rule_id: "WN004".to_string(),
            message: "Prefer pwsh.exe (PowerShell 7+) over Windows PowerShell 5.1 (powershell.exe) for cross-version consistency.".to_string(),
            aggressive: false,
        },
        Mapping {
            re: Regex::new(r#"["']C:\\Users\\[^"'\\\n]+\\"#)
                .expect("WN005 regex literal must be valid"),
            replace: r#""$env:USERPROFILE\\"#.to_string(),
            rule_id: "WN005".to_string(),
            message: "Hardcoded C:\\Users\\<name>\\ leak — use $env:USERPROFILE (PS) or %USERPROFILE% (cmd) or process.env.USERPROFILE (JS).".to_string(),
            aggressive: true,
        },
        Mapping {
            re: Regex::new(r"\$\w+\s*\+=\s*[\$@]\(")
                .expect("WN006 regex literal must be valid"),
            replace: "$_PLACEHOLDER_.Add(".to_string(),
            rule_id: "WN006".to_string(),
            message: "PowerShell `+=` on array rebuilds the array each iteration (O(n²)). Use `[List[T]]::new(); .Add($item)`.".to_string(),
            aggressive: false,
        },
        Mapping {
            re: Regex::new(r"\bGet-ChildItem\s+-Recurse\s+(-Path\s+)?(?:[A-Z]:\\|\\\\)")
                .expect("WN007 regex literal must be valid"),
            replace: "[System.IO.Directory]::EnumerateFiles(".to_string(),
            rule_id: "WN007".to_string(),
            message: "Get-ChildItem -Recurse buffers all results — use [System.IO.Directory]::EnumerateFiles + EnumerationOptions for streaming on large trees.".to_string(),
            aggressive: true,
        },
        Mapping {
            re: Regex::new(r"\bHKEY_LOCAL_MACHINE\\")
                .expect("WN008 regex literal must be valid"),
            replace: r"HKLM:\".to_string(),
            rule_id: "WN008".to_string(),
            message: "Use the PSDrive prefix 'HKLM:\\' instead of raw 'HKEY_LOCAL_MACHINE\\' in PowerShell.".to_string(),
            aggressive: false,
        },
        Mapping {
            re: Regex::new(r"\bHKEY_CURRENT_USER\\")
                .expect("WN009 regex literal must be valid"),
            replace: r"HKCU:\".to_string(),
            rule_id: "WN009".to_string(),
            message: "Use the PSDrive prefix 'HKCU:\\' instead of raw 'HKEY_CURRENT_USER\\' in PowerShell.".to_string(),
            aggressive: false,
        },
        Mapping {
            re: Regex::new(r"#!/usr/bin/env\s+node\b")
                .expect("WN010 regex literal must be valid"),
            replace: "#!/usr/bin/env bun".to_string(),
            rule_id: "WN010".to_string(),
            message: "Replace `node` shebang with `bun` for WinClean tooling (CLAUDE.md mandate: bun only).".to_string(),
            aggressive: false,
        },
    ]
});

static COMMENT_PREFIX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\s*(#|<!--|//)").expect("COMMENT_PREFIX must be a valid regex"));

/// Apply every `WN0xx` rule on the given source.
pub fn apply_windows_rules(path: &str, source: &str, aggressive: bool) -> (Vec<Finding>, String) {
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
