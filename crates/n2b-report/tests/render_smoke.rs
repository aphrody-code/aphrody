// Copyright 2026 Yohan Pierre
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Smoke tests for the n2b-report renderers — exercices NDJSON, SARIF, JSON,
//! Markdown et le prompt LLM contre un mini-set de findings synthétiques.
//!
//! Aligne le contrat avec l'aval (rpb-dashboard, GitHub Code Scanning, agent
//! LLM) sans dépendre d'un scan réel — fixtures inline, zéro I/O, zéro net.

use n2b_report::{
    render_json, render_jsonl, render_markdown, render_prompt, render_sarif, render_text,
};
use n2b_types::types::{FileFix, Finding, Mode, Report, RunOptions, Severity};
use serde_json::Value;
use std::path::PathBuf;

fn finding(rule: &str, line: u32, col: u32, sev: Severity, original: &str) -> Finding {
    Finding {
        file: "src/lib.ts".into(),
        line,
        col,
        rule_id: rule.into(),
        severity: sev,
        message: format!("synthetic {rule}"),
        original: original.into(),
        replacement: Some(format!("/* fixed: {original} */")),
        autofix: true,
        aggressive: Some(false),
        compat: None,
    }
}

fn fixture_fix() -> FileFix {
    // Use the actual finding offsets so byte_offset() in n2b-ai is exercised.
    let before = "import fs from \"fs\"\nconst x = require(\"node:path\")\n".to_string();
    let after = "import fs from \"node:fs\"\nconst x = require(\"node:path\")\n".to_string();
    FileFix {
        file: "src/lib.ts".into(),
        before,
        after,
        findings: vec![
            finding("imports/node-fs", 1, 16, Severity::Warn, "\"fs\""),
            finding("api/process-cwd", 2, 11, Severity::Info, "require"),
        ],
    }
}

fn fixture_opts() -> RunOptions {
    RunOptions {
        root: PathBuf::from("/tmp/n2b-test"),
        mode: Mode::Check,
        report: Report::Jsonl,
        quiet: false,
        ignore: vec![],
        agent: true,
        dry_run: false,
    }
}

#[test]
fn jsonl_emits_one_meta_then_one_finding_per_line() {
    let fixes = vec![fixture_fix()];
    let opts = fixture_opts();
    let out = render_jsonl(&fixes, &opts);

    let lines: Vec<&str> = out.lines().collect();
    // 1 meta line + 2 findings = 3 lines.
    assert_eq!(lines.len(), 3, "expected meta + 2 findings, got: {out}");

    let meta: Value = serde_json::from_str(lines[0]).expect("meta line is valid JSON");
    assert_eq!(meta["type"], "meta");
    assert_eq!(meta["tool"], "node2bun");
    assert_eq!(meta["files_scanned"], 1);
    assert_eq!(meta["findings_total"], 2);
    assert_eq!(meta["mode"], "check");

    let f1: Value = serde_json::from_str(lines[1]).expect("finding line is valid JSON");
    assert_eq!(f1["type"], "finding");
    assert_eq!(f1["file"], "src/lib.ts");
    assert_eq!(f1["rule_id"], "imports/node-fs");
    assert_eq!(f1["line"], 1);
    assert!(
        f1["docs_url"].as_str().is_some_and(|s| !s.is_empty()),
        "docs_url should be populated"
    );
    assert!(
        f1["context"].is_object(),
        "context_lines should serialize as object"
    );

    let f2: Value = serde_json::from_str(lines[2]).expect("second finding parses");
    assert_eq!(f2["rule_id"], "api/process-cwd");
    assert_eq!(f2["severity"], "info");
}

#[test]
fn sarif_envelope_conforms_to_2_1_0_shape() {
    let fixes = vec![fixture_fix()];
    let opts = fixture_opts();
    let out = render_sarif(&fixes, &opts);
    let v: Value = serde_json::from_str(&out).expect("SARIF output parses as JSON");

    assert_eq!(v["version"], "2.1.0");
    assert!(v["$schema"].as_str().unwrap().contains("sarif"));
    let runs = v["runs"].as_array().expect("runs is array");
    assert_eq!(runs.len(), 1);
    let run = &runs[0];
    assert_eq!(run["tool"]["driver"]["name"], "node2bun");

    // Both findings must be present and have rule ids.
    let results = run["results"].as_array().expect("results array");
    assert_eq!(results.len(), 2);
    let ids: Vec<&str> = results
        .iter()
        .filter_map(|r| r["ruleId"].as_str())
        .collect();
    assert!(ids.contains(&"imports/node-fs"));
    assert!(ids.contains(&"api/process-cwd"));

    // Replacement must surface as a `fixes` entry.
    let with_fix = results
        .iter()
        .find(|r| r["fixes"].is_array())
        .expect("at least one result has fixes");
    assert!(
        with_fix["fixes"][0]["artifactChanges"][0]["replacements"][0]["insertedContent"]["text"]
            .as_str()
            .is_some()
    );
}

#[test]
fn json_envelope_aggregates_findings_per_file() {
    let fixes = vec![fixture_fix()];
    let opts = fixture_opts();
    let out = render_json(&fixes, &opts);
    let v: Value = serde_json::from_str(&out).expect("JSON output parses");

    assert_eq!(v["tool"], "node2bun");
    assert_eq!(v["files_scanned"], 1);
    let files = v["files"].as_array().expect("files array");
    assert_eq!(files.len(), 1);
    assert_eq!(files[0]["path"], "src/lib.ts");
    assert_eq!(files[0]["changed"], true);
    let findings = files[0]["findings"].as_array().expect("findings array");
    assert_eq!(findings.len(), 2);
}

#[test]
fn markdown_emits_table_rows_per_finding() {
    let fixes = vec![fixture_fix()];
    let opts = fixture_opts();
    let out = render_markdown(&fixes, &opts);
    assert!(out.starts_with("# node2bun report"));
    // 1 header + 1 separator + 2 finding rows
    let table_rows = out
        .lines()
        .filter(|l| l.starts_with("| 1:16") || l.starts_with("| 2:11"))
        .count();
    assert_eq!(table_rows, 2, "expected 2 finding rows, got: {out}");
}

#[test]
fn prompt_truncates_at_max_findings() {
    let fixes = vec![fixture_fix()];
    let opts = fixture_opts();
    let out = render_prompt(&fixes, &opts, 1);
    assert!(out.contains("# Node.js → Bun migration task"));
    assert!(
        out.contains("truncated"),
        "max_findings=1 should trigger truncation footer: {out}"
    );
}

#[test]
fn text_render_includes_summary_counts() {
    let fixes = vec![fixture_fix()];
    let mut opts = fixture_opts();
    opts.agent = false; // colored output but inert in tests (no ANSI assertion)
    let out = render_text(&fixes, &opts);
    assert!(out.contains("findings : 2"));
    assert!(out.contains("imports/node-fs"));
    assert!(out.contains("api/process-cwd"));
}
