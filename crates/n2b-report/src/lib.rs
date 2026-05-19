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

use colored::Colorize;
use n2b_ai::{
    Context, SCHEMA_URL, SCHEMA_VERSION, byte_offset, category, confidence, context_lines, docs_url,
};
use n2b_types::types::{CompatInfo, CompatStatus, FileFix, Finding, Mode, RunOptions, Severity};
use n2b_util::line_offsets;
use serde::Serialize;
use serde_json::{Value, json};

pub const TOOL: &str = "node2bun";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

fn ellipsis(s: &str, n: usize) -> String {
    if s.chars().count() > n {
        let mut t: String = s.chars().take(n - 1).collect();
        t.push('…');
        t
    } else {
        s.to_string()
    }
}

fn sev_mark(s: Severity) -> String {
    match s {
        Severity::Error => "✗".red().to_string(),
        Severity::Warn => "!".yellow().to_string(),
        Severity::Info => "i".dimmed().to_string(),
    }
}

pub fn render_text(fixes: &[FileFix], opts: &RunOptions) -> String {
    let mut out = String::new();
    let total_findings: usize = fixes.iter().map(|f| f.findings.len()).sum();
    let changed: usize = fixes.iter().filter(|f| f.before != f.after).count();

    let mode_str = match opts.mode {
        Mode::Check => "check",
        Mode::Fix => "fix",
        Mode::Aggressive => "aggressive",
    };

    out.push_str(&format!(
        "{} {}\n",
        "node2bun".bold(),
        format!("(mode: {mode_str})").dimmed()
    ));
    out.push_str(&format!("  racine : {}\n", opts.root.display()));
    out.push_str(&format!("  fichiers scannés : {}\n", fixes.len()));
    out.push_str(&format!("  findings : {total_findings}\n"));
    out.push_str(&format!("  fichiers modifiés : {changed}\n\n"));

    let (mut errors, mut warns, mut infos) = (0usize, 0usize, 0usize);
    for fx in fixes {
        for f in &fx.findings {
            match f.severity {
                Severity::Error => errors += 1,
                Severity::Warn => warns += 1,
                Severity::Info => infos += 1,
            }
        }
    }

    for fix in fixes {
        if fix.findings.is_empty() {
            continue;
        }
        if opts.quiet && fix.findings.iter().all(|f| f.severity == Severity::Info) {
            continue;
        }
        let mark = if fix.before != fix.after {
            "✓".green().to_string()
        } else {
            "•".yellow().to_string()
        };
        out.push_str(&format!("{} {}\n", mark, fix.file.bold()));
        for f in &fix.findings {
            if opts.quiet && f.severity == Severity::Info {
                continue;
            }
            let loc = format!("{}:{}", f.line, f.col).dimmed().to_string();
            let tag = f.rule_id.cyan().to_string();
            let fix_str = match &f.replacement {
                Some(r) => format!(" {} {}", "→".dimmed(), ellipsis(r, 60).green()),
                None => String::new(),
            };
            out.push_str(&format!(
                "    {} {} {} {}{}\n",
                sev_mark(f.severity),
                loc,
                tag,
                f.message,
                fix_str,
            ));
            if let Some(c) = &f.compat {
                out.push_str(&format!(
                    "        {} {}\n",
                    "↳".dimmed(),
                    render_compat_text(c).dimmed(),
                ));
            }
        }
    }

    out.push('\n');
    out.push_str(&format!(
        "{} : {}, {}, {}\n",
        "Bilan".bold(),
        format!("{errors} errors").red(),
        format!("{warns} warns").yellow(),
        format!("{infos} infos").dimmed(),
    ));
    if opts.mode == Mode::Check && total_findings > 0 {
        out.push_str(
            &"(relancer avec --fix pour appliquer les corrections sûres, --aggressive pour migrer les APIs)"
                .dimmed()
                .to_string(),
        );
        out.push('\n');
    }
    out
}

#[derive(Debug, Serialize)]
struct EnrichedFinding<'a> {
    rule_id: &'a str,
    category: &'static str,
    severity: Severity,
    confidence: f32,
    message: &'a str,
    line: u32,
    col: u32,
    start_byte: u32,
    end_byte: u32,
    original: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    replacement: Option<&'a str>,
    autofix: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    aggressive: Option<bool>,
    docs_url: &'static str,
    context: Context,
    /// Phase 3+ : compat du module hôte (pour `imports/node-*` et
    /// `api/node-*`). Optionnel — rétro-compat schema_version=2.
    #[serde(skip_serializing_if = "Option::is_none")]
    compat: Option<&'a CompatInfo>,
}

fn enrich<'a>(f: &'a Finding, source: &str, offsets: &[u32]) -> EnrichedFinding<'a> {
    let start = byte_offset(offsets, f.line, f.col);
    let end = start + f.original.len() as u32;
    EnrichedFinding {
        rule_id: &f.rule_id,
        category: category(&f.rule_id),
        severity: f.severity,
        confidence: confidence(&f.rule_id, f.replacement.is_some()),
        message: &f.message,
        line: f.line,
        col: f.col,
        start_byte: start,
        end_byte: end,
        original: &f.original,
        replacement: f.replacement.as_deref(),
        autofix: f.autofix,
        aggressive: f.aggressive,
        docs_url: docs_url(&f.rule_id),
        context: context_lines(source, f.line),
        compat: f.compat.as_ref(),
    }
}

fn compat_status_label(s: CompatStatus) -> &'static str {
    match s {
        CompatStatus::Full => "🟢 full",
        CompatStatus::Partial => "🟡 partial",
        CompatStatus::Missing => "🔴 missing",
    }
}

fn render_compat_text(c: &CompatInfo) -> String {
    let mut s = format!(
        "compat: {} (node:{})",
        compat_status_label(c.status),
        c.module
    );
    if !c.missing_apis.is_empty() {
        s.push_str(" — manque ");
        s.push_str(&c.missing_apis.join(", "));
    }
    if let Some(eq) = &c.equivalent {
        s.push_str(&format!(" → {eq}"));
    }
    if let Some(bunpp) = &c.bunpp {
        s.push_str(&format!(" / polyfill: {bunpp}"));
    }
    s
}

fn meta(opts: &RunOptions, fixes: &[FileFix]) -> Value {
    json!({
        "schema_version": SCHEMA_VERSION,
        "$schema": SCHEMA_URL,
        "tool": TOOL,
        "version": VERSION,
        "mode": opts.mode,
        "root": opts.root.display().to_string(),
        "files_scanned": fixes.len(),
        "findings_total": fixes.iter().map(|f| f.findings.len()).sum::<usize>(),
    })
}

pub fn render_json(fixes: &[FileFix], opts: &RunOptions) -> String {
    render_json_with_card(fixes, opts, None)
}

/// Variante consommée par `--migrate` — ajoute le `report_card` (Phase 5 §5.4)
/// sans toucher à la sortie pour les autres modes. Le card est passé en
/// `serde_json::Value` (sérialisé en amont) pour rester decoupled de
/// `n2b-core::report_card` au niveau du crate `n2b-report`.
pub fn render_json_with_card(fixes: &[FileFix], opts: &RunOptions, card: Option<&Value>) -> String {
    let files: Vec<Value> = fixes
        .iter()
        .map(|f| {
            let offsets = line_offsets(&f.before);
            let findings: Vec<Value> = f
                .findings
                .iter()
                .map(|fi| {
                    serde_json::to_value(enrich(fi, &f.before, &offsets)).unwrap_or(json!({}))
                })
                .collect();
            json!({
                "path": f.file,
                "changed": f.before != f.after,
                "findings": findings,
            })
        })
        .collect();

    let mut m = meta(opts, fixes);
    if let Some(obj) = m.as_object_mut() {
        obj.insert("files".to_string(), Value::Array(files));
        if let Some(c) = card {
            if let Ok(v) = serde_json::to_value(c) {
                obj.insert("report_card".to_string(), v);
            }
        }
    }
    serde_json::to_string_pretty(&m).unwrap_or_default()
}

/// Affiche le report card en mode text (Phase 5 §5.4 §5.6).
pub fn render_report_card_text<C: serde::Serialize>(card: &C) -> String {
    let v = serde_json::to_value(card).unwrap_or_default();
    let pct = v
        .get("auto_migratable_pct")
        .and_then(|x| x.as_f64())
        .unwrap_or(0.0);
    let total = v
        .get("total_findings")
        .and_then(|x| x.as_u64())
        .unwrap_or(0);
    let blocking = v
        .get("blocking_findings")
        .and_then(|x| x.as_u64())
        .unwrap_or(0);
    let migrated = v.get("auto_migrated").and_then(|x| x.as_u64()).unwrap_or(0);
    let residue = v
        .get("manual_residue")
        .and_then(|x| x.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let mut out = String::new();
    out.push_str(&format!(
        "\n{}\n  auto-migratable : {} ({}/{} blocking, {} info+)\n  résidu manuel   : {}\n",
        "Migration report card".bold(),
        format!("{:.1}%", pct * 100.0).cyan(),
        migrated,
        blocking,
        total - blocking,
        residue,
    ));
    out
}

/// JSON Lines : 1 objet par ligne, streamable.
/// Ligne 1 = `{"type":"meta", ...}`
/// Lignes suivantes = `{"type":"finding", "file":..., ...enriched}`
pub fn render_jsonl(fixes: &[FileFix], opts: &RunOptions) -> String {
    let mut out = String::new();
    let mut m = meta(opts, fixes);
    if let Some(obj) = m.as_object_mut() {
        obj.insert("type".to_string(), Value::String("meta".to_string()));
    }
    out.push_str(&serde_json::to_string(&m).unwrap_or_default());
    out.push('\n');

    for f in fixes {
        let offsets = line_offsets(&f.before);
        for fi in &f.findings {
            let enriched = enrich(fi, &f.before, &offsets);
            let mut v = serde_json::to_value(&enriched).unwrap_or(json!({}));
            if let Some(obj) = v.as_object_mut() {
                obj.insert("type".to_string(), Value::String("finding".to_string()));
                obj.insert("file".to_string(), Value::String(f.file.clone()));
            }
            out.push_str(&serde_json::to_string(&v).unwrap_or_default());
            out.push('\n');
        }
    }
    out
}

fn escape_md(s: &str) -> String {
    s.replace('|', "\\|").replace('\n', " ")
}

/// SARIF 2.1.0 — format de résultats pour GitHub Code Scanning /
/// Azure DevOps / VS Code Problems panel. Spec : https://sarifweb.azurewebsites.net
pub fn render_sarif(fixes: &[FileFix], opts: &RunOptions) -> String {
    use std::collections::BTreeSet;
    let mut rule_ids: BTreeSet<String> = BTreeSet::new();
    for fx in fixes {
        for f in &fx.findings {
            rule_ids.insert(f.rule_id.clone());
        }
    }
    let rules: Vec<Value> = rule_ids
        .iter()
        .map(|id| {
            json!({
                "id": id,
                "name": id,
                "shortDescription": { "text": id },
                "helpUri": docs_url(id),
                "properties": { "category": category(id) },
            })
        })
        .collect();

    let results: Vec<Value> = fixes
        .iter()
        .flat_map(|fx| {
            let offsets = line_offsets(&fx.before);
            let file = fx.file.clone();
            let before = fx.before.clone();
            fx.findings
                .iter()
                .map(|f| {
                    let start = byte_offset(&offsets, f.line, f.col);
                    let end = start + f.original.len() as u32;
                    let level = match f.severity {
                        Severity::Error => "error",
                        Severity::Warn => "warning",
                        Severity::Info => "note",
                    };
                    let mut result = json!({
                        "ruleId": f.rule_id,
                        "level": level,
                        "message": { "text": f.message },
                        "locations": [{
                            "physicalLocation": {
                                "artifactLocation": { "uri": file },
                                "region": {
                                    "startLine": f.line,
                                    "startColumn": f.col,
                                    "byteOffset": start,
                                    "byteLength": end - start,
                                }
                            }
                        }],
                    "properties": {
                            "category": category(&f.rule_id),
                            "confidence": confidence(&f.rule_id, f.replacement.is_some()),
                            "autofix": f.autofix,
                            "compat": f.compat,
                        },
                    });
                    if let Some(repl) = &f.replacement {
                        if let Some(obj) = result.as_object_mut() {
                            obj.insert(
                                "fixes".to_string(),
                                json!([{
                                    "description": { "text": format!("Replace with: {repl}") },
                                    "artifactChanges": [{
                                        "artifactLocation": { "uri": file },
                                        "replacements": [{
                                            "deletedRegion": {
                                                "byteOffset": start,
                                                "byteLength": end - start,
                                            },
                                            "insertedContent": { "text": repl },
                                        }]
                                    }]
                                }]),
                            );
                        }
                    }
                    let _ = before;
                    result
                })
                .collect::<Vec<_>>()
        })
        .collect();

    let sarif = json!({
        "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/main/sarif-2.1/schema/sarif-schema-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": TOOL,
                    "version": VERSION,
                    "informationUri": "https://github.com/aphrody-code/n2b",
                    "rules": rules,
                }
            },
            "results": results,
            "invocations": [{
                "executionSuccessful": true,
                "workingDirectory": { "uri": opts.root.display().to_string() },
            }],
        }]
    });
    serde_json::to_string_pretty(&sarif).unwrap_or_default()
}

pub fn render_markdown(fixes: &[FileFix], opts: &RunOptions) -> String {
    let mut out = String::new();
    out.push_str("# node2bun report\n\n");
    let mode_str = match opts.mode {
        Mode::Check => "check",
        Mode::Fix => "fix",
        Mode::Aggressive => "aggressive",
    };
    out.push_str(&format!("- mode : `{mode_str}`\n"));
    out.push_str(&format!("- racine : `{}`\n\n", opts.root.display()));
    for fix in fixes {
        if fix.findings.is_empty() {
            continue;
        }
        out.push_str(&format!("## `{}`\n\n", fix.file));
        out.push_str("| ligne | règle | message | remplacement | compat |\n");
        out.push_str("| --- | --- | --- | --- | --- |\n");
        for f in &fix.findings {
            let repl = match &f.replacement {
                Some(r) => format!("`{}`", escape_md(r)),
                None => String::new(),
            };
            let compat = match &f.compat {
                Some(c) => escape_md(&render_compat_text(c)),
                None => String::new(),
            };
            out.push_str(&format!(
                "| {}:{} | `{}` | {} | {} | {} |\n",
                f.line, f.col, f.rule_id, f.message, repl, compat
            ));
        }
        out.push('\n');
    }
    out
}

/// Prompt prêt à coller dans un LLM, avec contexte et remplacement suggéré.
pub fn render_prompt(fixes: &[FileFix], opts: &RunOptions, max_findings: usize) -> String {
    let mut out = String::new();
    out.push_str(
        "# Node.js → Bun migration task\n\n\
         You are migrating a Node.js codebase to Bun. The static analyzer `node2bun` \
         produced the findings below. For each finding:\n\n\
         1. If a `Suggested replacement` is provided and it's safe in context, apply it.\n\
         2. If the suggestion requires wider refactoring (e.g. making a function `async`), \
         do the refactor consistently across the file.\n\
         3. If the finding has no replacement, add a short comment explaining the Bun-native \
         alternative and leave the code otherwise untouched.\n\
         4. Do not rewrite unrelated code.\n\n\
         Use the `docs_url` to confirm API behavior before large changes.\n\n",
    );
    out.push_str(&format!(
        "**Root**: `{}` · **Mode**: `{:?}` · **Tool**: node2bun {}\n\n---\n\n",
        opts.root.display(),
        opts.mode,
        VERSION
    ));

    let mut count = 0usize;
    'outer: for fix in fixes {
        for f in &fix.findings {
            if count >= max_findings {
                out.push_str(&format!(
                    "\n_... truncated, {} findings remaining._\n",
                    total_remaining(fixes, count)
                ));
                break 'outer;
            }
            let ctx = context_lines(&fix.before, f.line);
            out.push_str(&format!(
                "## `{}`:{}:{}\n\n- **rule**: `{}` ({})\n- **severity**: `{:?}`\n- **message**: {}\n- **docs**: {}\n\n",
                fix.file,
                f.line,
                f.col,
                f.rule_id,
                if f.autofix { "autofix" } else { "advisory" },
                f.severity,
                f.message,
                docs_url(&f.rule_id),
            ));
            out.push_str("**Before**:\n```ts\n");
            for l in &ctx.before {
                out.push_str(l);
                out.push('\n');
            }
            out.push_str(&ctx.line);
            out.push_str("   // <-- finding\n");
            for l in &ctx.after {
                out.push_str(l);
                out.push('\n');
            }
            out.push_str("```\n\n");
            if let Some(r) = &f.replacement {
                out.push_str(&format!(
                    "**Suggested replacement**:\n```ts\n{}\n```\n\n",
                    r
                ));
            }
            out.push_str("---\n\n");
            count += 1;
        }
    }
    out
}

fn total_remaining(fixes: &[FileFix], emitted: usize) -> usize {
    let total: usize = fixes.iter().map(|f| f.findings.len()).sum();
    total.saturating_sub(emitted)
}
