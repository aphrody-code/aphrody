// worktree_check.rs — Rust port of scripts/check-worktrees.ts.
//
// CI / pre-commit gate. Walks every aphrody source file that hardcodes a
// worktree path and verifies the path resolves on the running machine.
//
// Usage:
//     cargo xtask worktree-check
//     cargo xtask worktree-check --json
//     cargo xtask worktree-check --root /path/to/worktree
//
// Exit code:
//     0 — every referenced worktree exists.
//     1 — at least one missing worktree.

use std::{
    env, fs,
    path::PathBuf,
};

use anyhow::Result;
use clap::Args as ClapArgs;
use serde::Serialize;

struct ExpectedWorktree {
    slug: &'static str,
    dir_name: &'static str,
    consumers: &'static [&'static str],
}

const EXPECTED: &[ExpectedWorktree] = &[
    ExpectedWorktree { slug: "bxc", dir_name: "bxc", consumers: &["scripts/bxc-mass-scrape.ts", "scripts/scrape-m3-tokens.ts"] },
    ExpectedWorktree { slug: "n2b", dir_name: "n2b", consumers: &["Cargo.toml"] },
    ExpectedWorktree { slug: "open-design", dir_name: "open-design", consumers: &[
        "packages/aphrody-skills/src/sources.ts",
        "scripts/design-systems-import.ts",
        "scripts/design-templates-import.ts",
        "scripts/skills-harvest-open-design.ts",
    ] },
    ExpectedWorktree { slug: "openclaw", dir_name: "openclaw", consumers: &[
        "packages/aphrody-skills/src/sources.ts",
        "packages/plugin-package-contract/",
        "scripts/openclaw-extensions-audit.ts",
    ] },
    ExpectedWorktree { slug: "gemini-cli", dir_name: "gemini-cli", consumers: &[
        "packages/aphrody-skills/src/sources.ts",
        "packages/gemini-live-aphrody/src/auth.ts",
    ] },
    ExpectedWorktree { slug: "components", dir_name: "components", consumers: &["docs/audits/2026-05-17-angular-material-scrape.md"] },
    ExpectedWorktree { slug: "whisper", dir_name: "whisper", consumers: &["crates/aphrody-voice-stt/src/local_whisper.rs"] },
    ExpectedWorktree { slug: "live-api-web-console", dir_name: "live-api-web-console", consumers: &["packages/gemini-live-aphrody/"] },
    ExpectedWorktree { slug: "design.md", dir_name: "design.md", consumers: &["DESIGN.md"] },
    ExpectedWorktree { slug: "agent-browser", dir_name: "agent-browser", consumers: &["docs/audits/2026-05-17-vercel-agent-browser-vs-bxc.md"] },
    ExpectedWorktree { slug: "vercel-agent-skills", dir_name: "vercel-agent-skills", consumers: &["packages/aphrody-skills/src/sources.ts"] },
    ExpectedWorktree { slug: "vercel-skills", dir_name: "vercel-skills", consumers: &["packages/aphrody-skills/src/sources.ts"] },
    ExpectedWorktree { slug: "open-agents", dir_name: "open-agents", consumers: &["packages/aphrody-skills/src/sources.ts"] },
    ExpectedWorktree { slug: "wterm", dir_name: "wterm", consumers: &[
        "crates/aphrody-terminal-vt/",
        "crates/aphrody-terminal-wasm/",
        "crates/aphrody-terminal-backend/",
    ] },
    ExpectedWorktree { slug: "terminal", dir_name: "terminal", consumers: &[
        "crates/aphrody-terminal-vt/ (Buffer + Renderer reference)",
        "crates/aphrody-terminal-wasm/ (AtlasEngine reference)",
        "crates/aphrody-terminal-backend/ (ConPTY reference)",
        "crates/aphrody-terminal-settings/ (profiles.schema.json compat)",
    ] },
];

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    /// Override default worktree root (`C:/worktree` on Windows, `~/worktree` elsewhere).
    #[arg(long)]
    root: Option<PathBuf>,

    /// Emit machine-readable JSON to stdout.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Serialize)]
struct CheckResult {
    slug: String,
    #[serde(rename = "dirName")]
    dir_name: String,
    path: String,
    exists: bool,
    consumers: Vec<String>,
}

#[derive(Debug, Serialize)]
struct JsonOutput {
    #[serde(rename = "$schema")]
    schema: &'static str,
    #[serde(rename = "generatedAt")]
    generated_at: String,
    root: String,
    total: usize,
    present: usize,
    missing: usize,
    results: Vec<CheckResult>,
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

    let results: Vec<CheckResult> = EXPECTED
        .iter()
        .map(|spec| {
            let path = root.join(spec.dir_name);
            let exists = fs::metadata(&path).map(|m| m.is_dir()).unwrap_or(false);
            CheckResult {
                slug: spec.slug.to_string(),
                dir_name: spec.dir_name.to_string(),
                path: path.to_string_lossy().into_owned(),
                exists,
                consumers: spec.consumers.iter().map(|s| (*s).to_string()).collect(),
            }
        })
        .collect();

    let missing: Vec<&CheckResult> = results.iter().filter(|r| !r.exists).collect();
    let present = results.len() - missing.len();

    if args.json {
        let out = JsonOutput {
            schema: "https://aphrody-code.dev/schemas/check-worktrees/v1",
            generated_at: now_iso8601(),
            root: root.to_string_lossy().into_owned(),
            total: EXPECTED.len(),
            present,
            missing: missing.len(),
            results: results.iter().map(|r| CheckResult {
                slug: r.slug.clone(),
                dir_name: r.dir_name.clone(),
                path: r.path.clone(),
                exists: r.exists,
                consumers: r.consumers.clone(),
            }).collect(),
        };
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!(
            "[check-worktrees] root={}  total={}  present={}  missing={}",
            root.display(),
            EXPECTED.len(),
            present,
            missing.len(),
        );
        for r in &results {
            let mark = if r.exists { "ok    " } else { "MISS  " };
            println!("  {} {:<22} {}", mark, r.slug, r.path);
        }
        if !missing.is_empty() {
            println!();
            let slugs: Vec<&str> = missing.iter().map(|m| m.slug.as_str()).collect();
            println!(
                "Fix: cargo xtask worktree-setup --only={}",
                slugs.join(","),
            );
        }
    }

    if !missing.is_empty() {
        std::process::exit(1);
    }
    Ok(())
}

fn now_iso8601() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (y, mo, d, h, mi, s) = unix_to_ymdhms(secs);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

fn unix_to_ymdhms(mut t: u64) -> (i32, u32, u32, u32, u32, u32) {
    let s = (t % 60) as u32;
    t /= 60;
    let mi = (t % 60) as u32;
    t /= 60;
    let h = (t % 24) as u32;
    t /= 24;
    let mut days = t as i64;
    days += 719_468;
    let era = days.div_euclid(146_097);
    let doe = days.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = (yoe + era * 400) as i32;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let y = y + if m <= 2 { 1 } else { 0 };
    (y, m, d, h, mi, s)
}
