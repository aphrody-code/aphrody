// worktree_setup.rs — Rust port of scripts/setup-worktrees.ts.
//
// Bootstraps the local `~/worktree/` (or `C:/worktree/`) clones aphrody
// expects for cross-repo references. The .ts source delegates entirely to
// `git clone --depth 1` per slug.
//
// NOTE: this Rust port covers the standard "clone-if-missing" flow via
// system git (`Command::new("git")`). The bespoke .ts behaviour around
// branch pinning, post-clone hooks, and per-slug `--only=` filtering is
// preserved at the CLI surface; deeper hooks (e.g. `bun install` inside
// gemini-cli) are intentionally dropped per the 100%-Rust policy.

use std::{
    env,
    path::PathBuf,
    process::Command,
};

use anyhow::{Context, Result};
use clap::Args as ClapArgs;

struct WorktreeSpec {
    slug: &'static str,
    repo: &'static str,
    branch: Option<&'static str>,
}

const SPECS: &[WorktreeSpec] = &[
    WorktreeSpec { slug: "bxc", repo: "https://github.com/aphrody-code/bxc.git", branch: Some("aphrody") },
    WorktreeSpec { slug: "n2b", repo: "https://github.com/aphrody-code/n2b.git", branch: Some("aphrody") },
    WorktreeSpec { slug: "open-design", repo: "https://github.com/aphrody-code/open-design.git", branch: None },
    WorktreeSpec { slug: "openclaw", repo: "https://github.com/openclaw/openclaw.git", branch: None },
    WorktreeSpec { slug: "gemini-cli", repo: "https://github.com/google-gemini/gemini-cli.git", branch: None },
    WorktreeSpec { slug: "components", repo: "https://github.com/angular/components.git", branch: None },
    WorktreeSpec { slug: "whisper", repo: "https://github.com/ggerganov/whisper.cpp.git", branch: None },
    WorktreeSpec { slug: "live-api-web-console", repo: "https://github.com/google-gemini/live-api-web-console.git", branch: None },
    WorktreeSpec { slug: "agent-browser", repo: "https://github.com/vercel-labs/agent-browser.git", branch: None },
    WorktreeSpec { slug: "vercel-agent-skills", repo: "https://github.com/vercel-labs/agent-skills.git", branch: None },
    WorktreeSpec { slug: "vercel-skills", repo: "https://github.com/vercel-labs/skills.git", branch: None },
    WorktreeSpec { slug: "open-agents", repo: "https://github.com/vercel-labs/open-agents.git", branch: None },
    WorktreeSpec { slug: "wterm", repo: "https://github.com/vercel-labs/wterm.git", branch: None },
    WorktreeSpec { slug: "terminal", repo: "https://github.com/microsoft/terminal.git", branch: None },
];

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    /// Override default worktree root (`C:/worktree` on Windows, `~/worktree` elsewhere).
    #[arg(long)]
    root: Option<PathBuf>,

    /// Comma-separated list of slugs to clone (e.g. `bxc,n2b`).
    #[arg(long)]
    only: Option<String>,

    /// Show planned actions without cloning.
    #[arg(long)]
    dry_run: bool,
}

fn default_root() -> PathBuf {
    if cfg!(windows) {
        PathBuf::from("C:/worktree")
    } else {
        let home = env::var("HOME")
            .or_else(|_| env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join("worktree")
    }
}

pub(crate) fn run(args: Args) -> Result<()> {
    let root = args.root.unwrap_or_else(default_root);
    let filter: Option<Vec<String>> = args.only.map(|s| {
        s.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect()
    });

    std::fs::create_dir_all(&root).with_context(|| format!("mkdir {}", root.display()))?;

    let mut cloned = 0usize;
    let mut skipped = 0usize;
    let mut failed = 0usize;
    for spec in SPECS {
        if let Some(filt) = &filter {
            if !filt.iter().any(|s| s == spec.slug) {
                continue;
            }
        }
        let dest = root.join(spec.slug);
        if dest.exists() {
            println!("[setup-worktrees] {:<22} present at {}", spec.slug, dest.display());
            skipped += 1;
            continue;
        }
        if args.dry_run {
            println!(
                "[setup-worktrees] DRY {:<22} would clone {} {} -> {}",
                spec.slug,
                spec.repo,
                spec.branch.unwrap_or("HEAD"),
                dest.display()
            );
            continue;
        }
        println!(
            "[setup-worktrees] cloning {:<22} {} ...",
            spec.slug, spec.repo
        );
        let mut cmd = Command::new("git");
        cmd.args(["clone", "--depth", "1"]);
        if let Some(b) = spec.branch {
            cmd.args(["--branch", b]);
        }
        cmd.arg(spec.repo).arg(&dest);
        match cmd.status() {
            Ok(s) if s.success() => {
                cloned += 1;
            }
            Ok(s) => {
                eprintln!("[setup-worktrees] {} clone failed (exit {})", spec.slug, s.code().unwrap_or(-1));
                failed += 1;
            }
            Err(e) => {
                eprintln!("[setup-worktrees] {} spawn git failed: {e}", spec.slug);
                failed += 1;
            }
        }
    }

    println!(
        "[setup-worktrees] done. {cloned} cloned, {skipped} skipped, {failed} failed."
    );
    if failed > 0 {
        std::process::exit(1);
    }
    Ok(())
}
