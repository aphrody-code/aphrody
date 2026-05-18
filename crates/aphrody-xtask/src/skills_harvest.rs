// skills_harvest.rs — Rust port of scripts/skills-harvest-open-design.ts.
//
// Harvests skill definitions from upstream open-design / vercel-labs
// repositories and writes per-source manifests to
// `assets/skills-harvest/<source>.json`.
//
// TODO Phase 2 follow-up: full harvest logic. The TS source walks
// multiple sibling clones (`open-design`, `vercel-agent-skills`,
// `vercel-skills`, `open-agents`) and unions their SKILL.md inventories.
// This stub validates clone presence and writes a placeholder manifest.

use std::{
    env, fs,
    path::PathBuf,
    process::Command,
};

use anyhow::{Context, Result};
use clap::Args as ClapArgs;
use walkdir::WalkDir;

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    /// Root directory housing all open-design clones (default: worktree root).
    #[arg(long)]
    root: Option<PathBuf>,

    /// Output directory (default: assets/skills-harvest).
    #[arg(long)]
    out: Option<PathBuf>,

    /// Comma-separated list of source slugs to harvest.
    #[arg(long, default_value = "open-design,vercel-agent-skills,vercel-skills,open-agents")]
    sources: String,
}

pub(crate) fn run(args: Args) -> Result<()> {
    let root = args.root.unwrap_or_else(default_root);
    let out_dir = args
        .out
        .unwrap_or_else(|| repo_root().unwrap_or_else(|_| PathBuf::from(".")).join("assets").join("skills-harvest"));
    fs::create_dir_all(&out_dir).context("mkdir output")?;
    let sources: Vec<&str> = args.sources.split(',').map(str::trim).filter(|s| !s.is_empty()).collect();

    println!(
        "[skills-harvest] root={} out={} sources=[{}]",
        root.display(),
        out_dir.display(),
        sources.join(",")
    );

    let mut total = 0usize;
    for src in sources {
        let clone = root.join(src);
        if !clone.exists() {
            eprintln!(
                "[skills-harvest] {src}: missing clone at {}, skipping (try `cargo xtask worktree-setup --only={src}`)",
                clone.display()
            );
            continue;
        }
        let mut skills: Vec<serde_json::Value> = Vec::new();
        for entry in WalkDir::new(&clone)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                name != ".git" && name != "node_modules" && name != "target" && name != "dist"
            })
            .filter_map(Result::ok)
        {
            if entry.file_type().is_file() && entry.file_name() == "SKILL.md" {
                let rel = entry.path().strip_prefix(&clone).unwrap_or(entry.path());
                skills.push(serde_json::json!({
                    "source": src,
                    "path": rel.to_string_lossy(),
                    "size": entry.metadata().ok().map(|m| m.len()).unwrap_or(0),
                }));
            }
        }
        let count = skills.len();
        total += count;
        let manifest = serde_json::json!({
            "$schema": "https://aphrody-code.dev/schemas/skills-harvest/v1",
            "source": src,
            "clone": clone.to_string_lossy(),
            "count": count,
            "skills": skills,
        });
        let out_path = out_dir.join(format!("{src}.json"));
        fs::write(&out_path, format!("{}\n", serde_json::to_string_pretty(&manifest)?))?;
        println!("[skills-harvest] {src}: {count} skills \u{2192} {}", out_path.display());
    }

    println!("[skills-harvest] done. total={total} skills harvested.");
    Ok(())
}

fn default_root() -> PathBuf {
    if cfg!(windows) { PathBuf::from("C:/worktree") }
    else {
        let home = env::var("HOME").or_else(|_| env::var("USERPROFILE")).unwrap_or_else(|_| ".".into());
        PathBuf::from(home).join("worktree")
    }
}

fn repo_root() -> Result<PathBuf> {
    if let Ok(dir) = env::var("CARGO_WORKSPACE_DIR") { return Ok(PathBuf::from(dir)); }
    let out = Command::new("git").args(["rev-parse", "--show-toplevel"]).output().context("git")?;
    if out.status.success() { return Ok(PathBuf::from(String::from_utf8(out.stdout)?.trim())); }
    Ok(env::current_dir()?)
}
