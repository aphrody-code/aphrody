// Copyright 2026 aphrody-code
//
// Licensed under the Apache License, Version 2.0.

//! `dotnet` toolchain migration rules — dotnet 10 / NativeAOT idioms.
//!
//! These rules detect legacy patterns (NuGet CLI, Newtonsoft.Json, MSBuild.exe)
//! and rewrite them toward modern dotnet 10 NativeAOT-compatible equivalents.
//! Activated when the scanner sees `.csproj`, `.cs`, `.ps1`, `.sh`, or `.yml`
//! files referencing the dotnet toolchain.
//!
//! Rule ID space: `DN0xx`.

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
            re: Regex::new(r"\bnuget\.exe\s+restore\b|\bnuget\s+restore\b")
                .expect("DN001 regex literal must be valid"),
            replace: "dotnet restore".to_string(),
            rule_id: "DN001".to_string(),
            message: "Use 'dotnet restore' (modern, cross-platform) instead of legacy 'nuget restore'.".to_string(),
            aggressive: false,
        },
        Mapping {
            re: Regex::new(r"\bMSBuild\.exe\b")
                .expect("DN002 regex literal must be valid"),
            replace: "dotnet msbuild".to_string(),
            rule_id: "DN002".to_string(),
            message: "Replace 'MSBuild.exe' with 'dotnet msbuild' — cross-platform, no VS install required.".to_string(),
            aggressive: false,
        },
        Mapping {
            re: Regex::new(r#"Include="Newtonsoft\.Json"[^/>]*/?>"#)
                .expect("DN003 regex literal must be valid"),
            replace: r#"Include="System.Text.Json" />"#.to_string(),
            rule_id: "DN003".to_string(),
            message: "Prefer System.Text.Json + JsonSerializerContext (NativeAOT-safe source-gen) over Newtonsoft.Json (reflection).".to_string(),
            aggressive: true,
        },
        Mapping {
            re: Regex::new(r"\bJsonConvert\.DeserializeObject<([^>]+)>\(")
                .expect("DN004 regex literal must be valid"),
            replace: "JsonSerializer.Deserialize<$1>(".to_string(),
            rule_id: "DN004".to_string(),
            message: "Replace Newtonsoft JsonConvert with System.Text.Json.JsonSerializer (declare context for AOT).".to_string(),
            aggressive: true,
        },
        Mapping {
            re: Regex::new(r"\bJsonConvert\.SerializeObject\(")
                .expect("DN005 regex literal must be valid"),
            replace: "JsonSerializer.Serialize(".to_string(),
            rule_id: "DN005".to_string(),
            message: "Replace Newtonsoft serialization with System.Text.Json (NativeAOT-safe, source-gen).".to_string(),
            aggressive: true,
        },
        Mapping {
            re: Regex::new(r"<TargetFramework>net6\.0</TargetFramework>")
                .expect("DN006 regex literal must be valid"),
            replace: "<TargetFramework>net10.0</TargetFramework>".to_string(),
            rule_id: "DN006".to_string(),
            message: "WinClean targets net10.0 (LTS until Nov 2028) — upgrade from net6.0.".to_string(),
            aggressive: false,
        },
        Mapping {
            re: Regex::new(r"<TargetFramework>net7\.0</TargetFramework>")
                .expect("DN007 regex literal must be valid"),
            replace: "<TargetFramework>net10.0</TargetFramework>".to_string(),
            rule_id: "DN007".to_string(),
            message: "net7.0 is out of support — upgrade to net10.0 (LTS).".to_string(),
            aggressive: false,
        },
        // DN008 was here ("dotnet publish without -r/--runtime") — removed because
        // Rust regex crate has no negative lookahead, so we cannot distinguish
        // "publish without -r" from "publish with -r" cheaply. Re-introduce
        // later via a post-match Rust filter or by switching to fancy-regex.
    ]
});

static COMMENT_PREFIX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\s*(<!--|#|//)").expect("COMMENT_PREFIX must be a valid regex"));

/// Apply every `DN0xx` rule on the given source.
/// `aggressive` enables the `aggressive: true` rules (JSON migration, AOT runtime).
pub fn apply_dotnet_rules(path: &str, source: &str, aggressive: bool) -> (Vec<Finding>, String) {
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
