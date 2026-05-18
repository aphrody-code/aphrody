// skill_schema_check.rs — Rust port of scripts/skill-schema-align.ts.
//
// Bridges aphrody's `.claude/skills/<name>/SKILL.md` files (Anthropic
// skill-loader schema: name + description + when_to_use) to the
// open-design daemon schema (name + multi-line description + triggers[]).
//
// Emits a companion `.od.yaml` sidecar next to every existing SKILL.md.
//
// Usage:
//     cargo xtask skill-schema-check
//     cargo xtask skill-schema-check --check

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result};
use clap::Args as ClapArgs;
use regex::Regex;
use serde::Deserialize;
use walkdir::WalkDir;

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    /// Exit 1 if any skill fails alignment.
    #[arg(long)]
    check: bool,

    /// Override the .claude/skills root.
    #[arg(long)]
    skills_root: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
struct Frontmatter {
    name: Option<String>,
    description: Option<String>,
    when_to_use: Option<String>,
    triggers: Option<Vec<String>>,
}

enum Status { Aligned, Partial, Failed }

impl Status {
    fn as_str(&self) -> &'static str {
        match self { Self::Aligned => "aligned", Self::Partial => "partial", Self::Failed => "failed" }
    }
}

struct Report {
    #[allow(dead_code)]
    skill: String,
    rel_path: String,
    status: Status,
    trigger_count: usize,
    missing: Vec<String>,
}

pub(crate) fn run(args: Args) -> Result<()> {
    let root = match args.skills_root {
        Some(p) => p,
        None => repo_root()?.join(".claude").join("skills"),
    };
    if !root.exists() {
        eprintln!("skills root not found: {}", root.display());
        std::process::exit(2);
    }

    let mut reports: Vec<Report> = Vec::new();
    for entry in WalkDir::new(&root)
        .into_iter()
        .filter_map(Result::ok)
    {
        if entry.file_type().is_file() && entry.file_name() == "SKILL.md" {
            match process_skill(entry.path(), &root) {
                Ok(r) => reports.push(r),
                Err(e) => {
                    let skill = entry
                        .path()
                        .parent()
                        .and_then(|p| p.file_name())
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    let rel = entry
                        .path()
                        .strip_prefix(&root)
                        .map(|p| p.to_string_lossy().into_owned())
                        .unwrap_or_else(|_| entry.path().to_string_lossy().into_owned());
                    eprintln!("[skill-schema] {} : {e:#}", entry.path().display());
                    reports.push(Report {
                        skill,
                        rel_path: rel,
                        status: Status::Failed,
                        trigger_count: 0,
                        missing: vec!["frontmatter-parse-error".to_string()],
                    });
                }
            }
        }
    }

    let total = reports.len();
    let aligned = reports.iter().filter(|r| matches!(r.status, Status::Aligned)).count();
    let partial = reports.iter().filter(|r| matches!(r.status, Status::Partial)).count();
    let failed = reports.iter().filter(|r| matches!(r.status, Status::Failed)).count();

    println!("[skill-schema] root={}", root.display());
    println!("[skill-schema] total={total} aligned={aligned} partial={partial} failed={failed}");
    for r in &reports {
        println!(
            "  {:<10} triggers={:<3} {} {}",
            r.status.as_str(), r.trigger_count, r.rel_path,
            if r.missing.is_empty() { String::new() } else { format!("missing=[{}]", r.missing.join(",")) }
        );
    }

    if args.check && failed > 0 {
        std::process::exit(1);
    }
    Ok(())
}

fn process_skill(skill_md: &Path, root: &Path) -> Result<Report> {
    let raw = fs::read_to_string(skill_md).context("read SKILL.md")?;
    let stripped = if raw.starts_with('\u{feff}') { &raw[3..] } else { raw.as_str() };
    let fm: Frontmatter = parse_frontmatter(stripped)?;

    let skill_dir = skill_md.parent().context("SKILL.md has no parent")?;
    let skill_name = fm
        .name
        .clone()
        .unwrap_or_else(|| {
            skill_dir.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
        });
    let rel = skill_md
        .strip_prefix(root)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| skill_md.to_string_lossy().into_owned());

    let mut missing: Vec<String> = Vec::new();
    if fm.name.as_deref().unwrap_or("").trim().is_empty() { missing.push("name".into()); }
    if fm.description.as_deref().unwrap_or("").trim().is_empty() { missing.push("description".into()); }

    let mut triggers: Vec<String> = Vec::new();
    if let Some(t) = &fm.triggers { triggers.extend(t.iter().cloned()); }
    if let Some(wtu) = &fm.when_to_use {
        let re = Regex::new(r#""([^"]+)""#).expect("static");
        for cap in re.captures_iter(wtu) {
            if let Some(m) = cap.get(1) {
                let s = m.as_str().trim();
                if !s.is_empty() && !triggers.iter().any(|t| t.eq_ignore_ascii_case(s)) {
                    triggers.push(s.to_string());
                }
            }
        }
    }
    if triggers.is_empty() && !skill_name.is_empty() {
        triggers.push(format!("/{}", skill_name));
    }

    let status = if !missing.is_empty() {
        Status::Failed
    } else if triggers.is_empty() {
        Status::Partial
    } else {
        Status::Aligned
    };

    // Emit .od.yaml sidecar.
    let sidecar_path = skill_dir.join(".od.yaml");
    let yaml = render_od_yaml(
        &skill_name,
        fm.description.as_deref().unwrap_or(""),
        &triggers,
    );
    if let Err(e) = fs::write(&sidecar_path, yaml) {
        eprintln!("[skill-schema] failed to write {}: {e}", sidecar_path.display());
    }

    Ok(Report {
        skill: skill_name,
        rel_path: rel,
        status,
        trigger_count: triggers.len(),
        missing,
    })
}

fn parse_frontmatter(s: &str) -> Result<Frontmatter> {
    let stripped = s.strip_prefix("---\n").or_else(|| s.strip_prefix("---\r\n"))
        .context("no leading --- delimiter")?;
    let end = stripped.find("\n---").context("no trailing --- delimiter")?;
    let fm_text = &stripped[..end];
    let fm: Frontmatter = serde_yaml::from_str(fm_text).context("parse YAML frontmatter")?;
    Ok(fm)
}

fn render_od_yaml(name: &str, description: &str, triggers: &[String]) -> String {
    let mut out = String::new();
    out.push_str("# Auto-generated by `cargo xtask skill-schema-check` \u{2014} DO NOT EDIT\n");
    out.push_str(&format!("name: {name}\n"));
    out.push_str("description: |\n");
    for line in description.lines() {
        out.push_str(&format!("  {line}\n"));
    }
    out.push_str("triggers:\n");
    for t in triggers {
        let escaped = t.replace('"', "\\\"");
        out.push_str(&format!("  - \"{escaped}\"\n"));
    }
    out
}

fn repo_root() -> Result<PathBuf> {
    if let Ok(dir) = env::var("CARGO_WORKSPACE_DIR") {
        return Ok(PathBuf::from(dir));
    }
    let out = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("spawn git rev-parse")?;
    if out.status.success() {
        return Ok(PathBuf::from(String::from_utf8(out.stdout)?.trim()));
    }
    Ok(env::current_dir()?)
}
