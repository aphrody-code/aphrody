// Copyright 2026 aphrody-code
//
// Licensed under the Apache License, Version 2.0.

//! `n2b dotnet` sub-command — applies the four `dotnet` branch rule modules
//! (DN0xx, WN0xx, NA0xx, WC0xx) against a project tree.
//!
//! This is **additive** : it does NOT touch the main scan engine or the
//! frozen JSON/JSONL/SARIF baselines on `main`. The findings produced here
//! are printed independently and never feed into `--migrate` / `--fix` of
//! the default scan command.
//!
//! Usage:
//!   n2b dotnet <root> [--fix] [--aggressive] [--report text|json|markdown]

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;

use ignore::WalkBuilder;

use crate::cli::args::ReportArg;
use n2b_core::rules::{dotnet as dn, node_api_dotnet as na, winclean as wc, windows as wn};
use n2b_core::types::{Finding, Severity};

/// File extensions / names the dotnet rule set inspects.
fn is_relevant_file(name: &str, ext: &str) -> bool {
    matches!(
        ext,
        "cs" | "csproj"
            | "ps1"
            | "psm1"
            | "psd1"
            | "ts"
            | "tsx"
            | "js"
            | "mjs"
            | "yml"
            | "yaml"
            | "cmd"
            | "bat"
    ) || name.eq_ignore_ascii_case("Directory.Build.props")
        || name.eq_ignore_ascii_case("Directory.Packages.props")
}

pub fn run(root: PathBuf, fix: bool, aggressive: bool, report: ReportArg) -> Result<ExitCode> {
    let aggressive = aggressive || fix; // fix alone enables non-aggressive autofix
    let root_abs = root.canonicalize().unwrap_or_else(|_| root.clone());

    let mut all_findings: Vec<Finding> = Vec::new();
    let mut files_scanned: usize = 0;
    let mut files_rewritten: usize = 0;

    let walker = WalkBuilder::new(&root_abs)
        .standard_filters(true)
        .add_custom_ignore_filename(".oxlintignore")
        .build();
    for entry in walker.filter_map(|e| e.ok()) {
        if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }
        let path = entry.path();
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        if !is_relevant_file(name, &ext) {
            continue;
        }

        // Skip vendored / generated / submodule paths — same instincts as `.oxlintignore`.
        let path_s = path.to_string_lossy();
        if path_s.contains("\\node_modules\\")
            || path_s.contains("/node_modules/")
            || path_s.contains("\\bin\\")
            || path_s.contains("/bin/")
            || path_s.contains("\\obj\\")
            || path_s.contains("/obj/")
            || path_s.contains("\\target\\")
            || path_s.contains("/target/")
            || path_s.contains("\\vendor\\")
            || path_s.contains("/vendor/")
            || path_s.contains("\\.git\\")
            || path_s.contains("/.git/")
        {
            continue;
        }

        let Ok(source) = std::fs::read_to_string(path) else {
            continue;
        };
        files_scanned += 1;
        let original = source.clone();
        let display_path = path_s.to_string();

        // Run all 4 rule modules in sequence so later rules see prior fixes.
        let (mut findings, mut working) =
            dn::apply_dotnet_rules(&display_path, &source, aggressive);
        let (f2, working2) = wn::apply_windows_rules(&display_path, &working, aggressive);
        findings.extend(f2);
        working = working2;
        let (f3, working3) = na::apply_node_api_dotnet_rules(&display_path, &working, aggressive);
        findings.extend(f3);
        working = working3;
        let (f4, working4) = wc::apply_winclean_rules(&display_path, &working, aggressive);
        findings.extend(f4);
        working = working4;

        all_findings.extend(findings);

        // Persist fixes if --fix and content actually changed.
        if fix && working != original {
            if let Err(e) = std::fs::write(path, &working) {
                eprintln!("note: failed to write {}: {e}", display_path);
            } else {
                files_rewritten += 1;
            }
        }
    }

    // Report.
    match report {
        ReportArg::Json => {
            let summary = serde_json::json!({
                "tool": "n2b dotnet",
                "rules": ["DN0xx", "WN0xx", "NA0xx", "WC0xx"],
                "files_scanned": files_scanned,
                "files_rewritten": files_rewritten,
                "findings": all_findings,
            });
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        ReportArg::Jsonl => {
            for f in &all_findings {
                println!("{}", serde_json::to_string(f)?);
            }
        }
        ReportArg::Md | ReportArg::Markdown => {
            println!("# n2b dotnet — branch rules report");
            println!();
            println!("- Files scanned: {files_scanned}");
            if fix {
                println!("- Files rewritten: {files_rewritten}");
            }
            println!("- Total findings: {}", all_findings.len());
            println!();
            for f in &all_findings {
                println!(
                    "- `{}` line {} col {} — **{}**: {}",
                    f.file, f.line, f.col, f.rule_id, f.message
                );
            }
        }
        ReportArg::Sarif => {
            // Lean SARIF skeleton — sufficient for GitHub Code Scanning ingest.
            let sarif = serde_json::json!({
                "version": "2.1.0",
                "$schema": "https://schemastore.azurewebsites.net/schemas/json/sarif-2.1.0.json",
                "runs": [{
                    "tool": {
                        "driver": {
                            "name": "n2b-dotnet",
                            "informationUri": "https://github.com/aphrody-code/n2b",
                            "version": env!("CARGO_PKG_VERSION"),
                        }
                    },
                    "results": all_findings.iter().map(|f| serde_json::json!({
                        "ruleId": f.rule_id,
                        "level": match f.severity {
                            Severity::Error => "error",
                            Severity::Warn => "warning",
                            Severity::Info => "note",
                        },
                        "message": { "text": f.message },
                        "locations": [{
                            "physicalLocation": {
                                "artifactLocation": { "uri": f.file },
                                "region": { "startLine": f.line, "startColumn": f.col }
                            }
                        }],
                    })).collect::<Vec<_>>(),
                }]
            });
            println!("{}", serde_json::to_string(&sarif)?);
        }
        ReportArg::Text => {
            println!(
                "n2b dotnet branch rules — {} findings across {} files",
                all_findings.len(),
                files_scanned
            );
            if fix {
                println!("Rewrote {} files in place.", files_rewritten);
            }
            // Group by rule_id for readability.
            let mut by_rule: std::collections::BTreeMap<String, Vec<&Finding>> = Default::default();
            for f in &all_findings {
                by_rule.entry(f.rule_id.clone()).or_default().push(f);
            }
            for (rule, items) in by_rule {
                println!("\n[{}] ({} findings)", rule, items.len());
                for f in items.iter().take(5) {
                    println!("  {}:{}:{}  {}", f.file, f.line, f.col, f.message);
                }
                if items.len() > 5 {
                    println!("  … and {} more", items.len() - 5);
                }
            }
        }
    }

    Ok(if all_findings.is_empty() {
        ExitCode::SUCCESS
    } else if fix {
        ExitCode::SUCCESS // findings were applied
    } else {
        ExitCode::from(1) // findings remain
    })
}
