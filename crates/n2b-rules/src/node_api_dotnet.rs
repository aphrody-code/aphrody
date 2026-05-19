// Copyright 2026 aphrody-code
//
// Licensed under the Apache License, Version 2.0.

//! node-api-dotnet specific rules — Bun/Node ⇄ C# interop best-practices.
//!
//! Catches common mistakes when wiring [JSExport] C# code to JS:
//! - static `import dotnet from 'node-api-dotnet/net10.0'` failures (npm pkg
//!   only ships net8.0/9.0 currently — direct dlopen is more robust).
//! - missing Generator companion PackageReference.
//! - C# returning `object` instead of typed DTO across the boundary.
//!
//! Rule ID space: `NA0xx`.

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
            re: Regex::new(r#"import\s+dotnet\s+from\s+['"]node-api-dotnet/net10\.0['"]"#)
                .expect("NA001 regex literal must be valid"),
            replace: "/* prefer direct process.dlopen on .node, see scripts/ipc/winclean-native.ts */".to_string(),
            rule_id: "NA001".to_string(),
            message: "node-api-dotnet npm 0.9.x ships net472/net8.0/net9.0 only — net10.0 will throw. Prefer process.dlopen() on the AOT .node binary.".to_string(),
            aggressive: true,
        },
        Mapping {
            re: Regex::new(r#"require\(\s*['"]node-api-dotnet/net10\.0['"]\s*\)"#)
                .expect("NA002 regex literal must be valid"),
            replace: r#"require('node-api-dotnet/net9.0')"#.to_string(),
            rule_id: "NA002".to_string(),
            message: "node-api-dotnet/net10.0 is not published yet — downgrade to net9.0 runtime (still loads net10.0 assemblies in many cases).".to_string(),
            aggressive: false,
        },
        Mapping {
            re: Regex::new(
                r#"PackageReference\s+Include="Microsoft\.JavaScript\.NodeApi"\s+Version="0\.10\.[^"]+""#,
            )
            .expect("NA003 regex literal must be valid"),
            replace: r#"PackageReference Include="Microsoft.JavaScript.NodeApi" Version="0.9.19""#
                .to_string(),
            rule_id: "NA003".to_string(),
            message: "Microsoft.JavaScript.NodeApi 0.10.x is not published on nuget.org (latest stable 0.9.19).".to_string(),
            aggressive: false,
        },
        Mapping {
            re: Regex::new(r"\[JSExport\]\s*\n\s*public\s+\w+\s+\w+\s*\([^)]*\)\s*=>\s*object\b")
                .expect("NA004 regex literal must be valid"),
            replace: "/* TODO: return a sealed DTO class, not 'object' — see Winclean.Bun ProcessSummary */".to_string(),
            rule_id: "NA004".to_string(),
            message: "[JSExport] methods should return primitives / arrays / sealed DTO classes — opaque 'object' is not marshaled cleanly.".to_string(),
            aggressive: false,
        },
        Mapping {
            re: Regex::new(r#"PackageReference\s+Include="Microsoft\.JavaScript\.NodeApi"(?:\s+Version="[^"]+")?\s*/>"#)
                .expect("NA005 regex literal must be valid"),
            replace: r#"PackageReference Include="Microsoft.JavaScript.NodeApi" Version="0.9.19" />
    <PackageReference Include="Microsoft.JavaScript.NodeApi.Generator" Version="0.9.19" PrivateAssets="all" />"#
                .to_string(),
            rule_id: "NA005".to_string(),
            message: "Microsoft.JavaScript.NodeApi must be paired with Microsoft.JavaScript.NodeApi.Generator (source-gen produces .d.ts + .mjs).".to_string(),
            aggressive: false,
        },
    ]
});

static COMMENT_PREFIX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\s*(<!--|//|#)").expect("COMMENT_PREFIX must be a valid regex"));

/// Apply every `NA0xx` rule on the given source.
pub fn apply_node_api_dotnet_rules(
    path: &str,
    source: &str,
    aggressive: bool,
) -> (Vec<Finding>, String) {
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
