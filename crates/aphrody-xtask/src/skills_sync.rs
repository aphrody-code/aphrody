// skills_sync.rs — pull a remote SKILL.md catalog into ./.claude/skills/.
//
// Rust port of scripts/skills-sync.ts (Bun). Same CLI surface:
//
//     cargo xtask skills-sync <owner>/<repo>[#<ref>] [--force]
//
// Strategy:
//   1. Shallow-clone the repo into a temp directory (system `git`).
//   2. Walk the clone, find every `SKILL.md`.
//   3. Parse YAML frontmatter for `name:` (falls back to dir basename).
//   4. Copy the containing folder into `.claude/skills/<name>/`
//      unless it already exists (use `--force` to overwrite).
//   5. Print a summary table to stdout.

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{anyhow, bail, Context, Result};
use clap::Args as ClapArgs;
use serde::Deserialize;
use walkdir::WalkDir;

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    /// Remote source as `<owner>/<repo>[#<ref>]`.
    /// Examples: `vercel-labs/agent-skills`, `anthropics/skills#main`.
    source: String,

    /// Overwrite existing skills locally.
    #[arg(long)]
    force: bool,
}

#[derive(Debug, Deserialize)]
struct Frontmatter {
    name: Option<String>,
}

enum Status {
    Installed,
    Skipped,
    Overwritten,
}

impl Status {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Installed => "installed",
            Self::Skipped => "skipped",
            Self::Overwritten => "overwritten",
        }
    }
}

pub(crate) fn run(args: Args) -> Result<()> {
    let (repo_spec, refname) = split_source(&args.source)?;
    let url = format!("https://github.com/{repo_spec}.git");

    let workdir = tempfile::tempdir().context("create temp dir for clone")?;
    let workdir_path = workdir.path();

    println!("\u{25b8} Cloning {url}#{refname} (shallow) ...");
    let branch_for_initial = if refname == "HEAD" { "main" } else { &refname };
    let initial = Command::new("git")
        .args(["clone", "--depth", "1", "--branch", branch_for_initial])
        .arg(&url)
        .arg(workdir_path)
        .status()
        .context("spawn git for initial clone")?;
    if !initial.success() {
        // Fallback: default branch.
        for entry in fs::read_dir(workdir_path).context("read tmp dir for cleanup")? {
            let entry = entry?;
            let p = entry.path();
            if p.is_dir() {
                fs::remove_dir_all(&p).ok();
            } else {
                fs::remove_file(&p).ok();
            }
        }
        let fallback = Command::new("git")
            .args(["clone", "--depth", "1"])
            .arg(&url)
            .arg(workdir_path)
            .status()
            .context("spawn git for fallback clone")?;
        if !fallback.success() {
            bail!("git clone failed for {url}");
        }
    }

    println!("\u{25b8} Discovering SKILL.md files ...");
    let mut candidates = Vec::new();
    for entry in WalkDir::new(workdir_path)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            name != ".git" && name != "node_modules"
        })
        .filter_map(Result::ok)
    {
        if entry.file_type().is_file() && entry.file_name() == "SKILL.md" {
            candidates.push(entry.into_path());
        }
    }
    println!("  found {} skill(s).", candidates.len());

    let target_root = repo_root()?.join(".claude").join("skills");
    fs::create_dir_all(&target_root).context("create .claude/skills/")?;

    let mut report: Vec<(String, Status, String)> = Vec::new();
    for skill_md in &candidates {
        let src_dir = skill_md.parent().ok_or_else(|| anyhow!("SKILL.md has no parent"))?;
        let name = skill_name(skill_md, src_dir)?;

        let dest = target_root.join(&name);
        let from_rel = src_dir
            .strip_prefix(workdir_path)
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| src_dir.to_string_lossy().into_owned());

        if dest.exists() {
            if args.force {
                fs::remove_dir_all(&dest).context("remove existing skill dir")?;
                copy_dir(src_dir, &dest)?;
                report.push((name, Status::Overwritten, from_rel));
            } else {
                report.push((name, Status::Skipped, from_rel));
            }
        } else {
            copy_dir(src_dir, &dest)?;
            report.push((name, Status::Installed, from_rel));
        }
    }

    print_summary(&report);
    Ok(())
}

fn split_source(s: &str) -> Result<(String, String)> {
    let (repo, refname) = match s.split_once('#') {
        Some((r, f)) => (r.to_string(), f.to_string()),
        None => (s.to_string(), "HEAD".to_string()),
    };
    if !repo.contains('/') {
        bail!("Invalid source \"{s}\" \u{2014} expected <owner>/<repo>");
    }
    Ok((repo, refname))
}

fn skill_name(skill_md: &Path, src_dir: &Path) -> Result<String> {
    let text = fs::read_to_string(skill_md).context("read SKILL.md")?;
    if let Some(stripped) = text.strip_prefix("---\n") {
        if let Some(end) = stripped.find("\n---") {
            let fm_text = &stripped[..end];
            if let Ok(fm) = serde_yaml::from_str::<Frontmatter>(fm_text) {
                if let Some(n) = fm.name {
                    let trimmed = n.trim();
                    if !trimmed.is_empty() {
                        return Ok(trimmed.to_string());
                    }
                }
            }
        }
    }
    Ok(src_dir
        .file_name()
        .ok_or_else(|| anyhow!("skill dir has no name"))?
        .to_string_lossy()
        .into_owned())
}

fn copy_dir(src: &Path, dest: &Path) -> Result<()> {
    let opts = fs_extra::dir::CopyOptions {
        copy_inside: true,
        ..Default::default()
    };
    fs_extra::dir::copy(src, dest, &opts)
        .with_context(|| format!("copy {} -> {}", src.display(), dest.display()))?;
    Ok(())
}

fn repo_root() -> Result<PathBuf> {
    // Honor CARGO_WORKSPACE_DIR if set (cargo nightly), else `cargo metadata`-equivalent
    // via `git rev-parse --show-toplevel`, else current dir.
    if let Ok(dir) = env::var("CARGO_WORKSPACE_DIR") {
        return Ok(PathBuf::from(dir));
    }
    let out = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("spawn git rev-parse")?;
    if out.status.success() {
        let path = String::from_utf8(out.stdout).context("git output utf-8")?;
        return Ok(PathBuf::from(path.trim()));
    }
    Ok(env::current_dir()?)
}

fn print_summary(report: &[(String, Status, String)]) {
    let name_w = report.iter().map(|(n, _, _)| n.len()).max().unwrap_or(4).max(4);
    let status_w = report
        .iter()
        .map(|(_, s, _)| s.as_str().len())
        .max()
        .unwrap_or(6)
        .max(6);

    println!();
    println!(
        "{:<name_w$}  {:<status_w$}  from",
        "name",
        "status",
        name_w = name_w,
        status_w = status_w
    );
    println!(
        "{:-<name_w$}  {:-<status_w$}  {}",
        "",
        "",
        "----",
        name_w = name_w,
        status_w = status_w
    );
    for (n, s, f) in report {
        println!(
            "{:<name_w$}  {:<status_w$}  {}",
            n,
            s.as_str(),
            f,
            name_w = name_w,
            status_w = status_w
        );
    }
    println!();
    let total = report.len();
    let installed = report.iter().filter(|(_, s, _)| matches!(s, Status::Installed)).count();
    let skipped = report.iter().filter(|(_, s, _)| matches!(s, Status::Skipped)).count();
    let overwritten = report
        .iter()
        .filter(|(_, s, _)| matches!(s, Status::Overwritten))
        .count();
    println!(
        "{total} skill(s) processed \u{2014} {installed} installed, {skipped} skipped, {overwritten} overwritten"
    );
}
