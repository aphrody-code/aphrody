// m3_audit.rs — Rust port of scripts/m3-coverage-audit.ts.
//
// Audits CSS/HTML files across assets/ and packages/ for Material Design 3
// token coverage (var(--md-sys-*) usage, shape/elevation/motion roles).
// Emits a coverage report to docs/audits/m3-coverage.md.
//
// TODO Phase 2 follow-up: full coverage analyser. The TS source (753 LOC)
// parses every CSS file with lightningcss, indexes token usage by role,
// and emits per-file + per-role coverage tables. This stub implements
// the directory walk + token-string detection; the per-role analysis
// is pending the lightningcss-rs integration.

use std::{
    env, fs,
    path::PathBuf,
    process::Command,
};

use anyhow::{Context, Result};
use clap::Args as ClapArgs;
use regex::Regex;
use serde::Serialize;
use walkdir::WalkDir;

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    /// Root directory to audit (default: workspace root).
    #[arg(long)]
    root: Option<PathBuf>,

    /// Emit JSON to stdout (skips markdown).
    #[arg(long)]
    json: bool,

    /// Exit 1 if coverage falls below threshold (0..=100).
    #[arg(long, default_value = "0")]
    min_coverage: u8,
}

#[derive(Debug, Default, Serialize)]
struct FileCoverage {
    path: String,
    tokens_used: usize,
    md_sys_count: usize,
    md_ref_count: usize,
}

#[derive(Debug, Default, Serialize)]
struct CoverageReport {
    files_scanned: usize,
    files_with_tokens: usize,
    total_token_uses: usize,
    coverage_percent: f64,
    files: Vec<FileCoverage>,
}

pub(crate) fn run(args: Args) -> Result<()> {
    let root = args.root.unwrap_or_else(|| repo_root().unwrap_or_else(|_| PathBuf::from(".")));

    let css_re = Regex::new(r"var\(--md-(sys|ref)-([a-z0-9-]+)\)").expect("static");

    let mut report = CoverageReport::default();
    for entry in WalkDir::new(&root)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !matches!(name.as_ref(), "node_modules" | "target" | ".git" | "dist" | "build" | "var")
        })
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() { continue; }
        let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) else { continue; };
        if !matches!(ext.to_ascii_lowercase().as_str(), "css" | "scss" | "html" | "tsx" | "jsx") {
            continue;
        }
        report.files_scanned += 1;
        let Ok(body) = fs::read_to_string(entry.path()) else { continue; };
        let mut sys = 0usize;
        let mut refer = 0usize;
        let mut total = 0usize;
        for cap in css_re.captures_iter(&body) {
            total += 1;
            match cap.get(1).map(|m| m.as_str()) {
                Some("sys") => sys += 1,
                Some("ref") => refer += 1,
                _ => {}
            }
        }
        if total > 0 {
            report.files_with_tokens += 1;
            report.total_token_uses += total;
            report.files.push(FileCoverage {
                path: entry.path().strip_prefix(&root).unwrap_or(entry.path())
                    .to_string_lossy().into_owned(),
                tokens_used: total,
                md_sys_count: sys,
                md_ref_count: refer,
            });
        }
    }

    if report.files_scanned > 0 {
        report.coverage_percent =
            (report.files_with_tokens as f64 / report.files_scanned as f64) * 100.0;
    }
    report.files.sort_by(|a, b| b.tokens_used.cmp(&a.tokens_used));

    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        let out = repo_root()?.join("docs").join("audits").join("m3-coverage.md");
        if let Some(p) = out.parent() { fs::create_dir_all(p).ok(); }
        let mut body = String::new();
        body.push_str("<!-- SPDX-License-Identifier: Apache-2.0 -->\n\n");
        body.push_str("# Material Design 3 \u{2014} token coverage audit\n\n");
        body.push_str(&format!(
            "Files scanned: **{}**  \nFiles with M3 tokens: **{}**  \nTotal token uses: **{}**  \nCoverage: **{:.1}%**\n\n",
            report.files_scanned, report.files_with_tokens, report.total_token_uses, report.coverage_percent
        ));
        body.push_str("## Top 50 files by token usage\n\n");
        body.push_str("| file | tokens | --md-sys-* | --md-ref-* |\n|---|---:|---:|---:|\n");
        for f in report.files.iter().take(50) {
            body.push_str(&format!("| `{}` | {} | {} | {} |\n", f.path, f.tokens_used, f.md_sys_count, f.md_ref_count));
        }
        body.push_str("\n_Generator: `cargo xtask m3-audit`. TODO Phase 2 follow-up: per-role coverage._\n");
        fs::write(&out, body).context("write m3-coverage.md")?;
        println!("[m3-audit] wrote {}", out.display());
        println!(
            "[m3-audit] files={} with-tokens={} total-uses={} coverage={:.1}%",
            report.files_scanned, report.files_with_tokens, report.total_token_uses, report.coverage_percent
        );
    }

    if args.min_coverage > 0 && report.coverage_percent < args.min_coverage as f64 {
        eprintln!("[m3-audit] coverage {:.1}% < min {}%", report.coverage_percent, args.min_coverage);
        std::process::exit(1);
    }
    Ok(())
}

fn repo_root() -> Result<PathBuf> {
    if let Ok(dir) = env::var("CARGO_WORKSPACE_DIR") { return Ok(PathBuf::from(dir)); }
    let out = Command::new("git").args(["rev-parse", "--show-toplevel"]).output().context("git")?;
    if out.status.success() { return Ok(PathBuf::from(String::from_utf8(out.stdout)?.trim())); }
    Ok(env::current_dir()?)
}
