// audit_openclaw.rs — Rust port of scripts/aphrody-vs-open-design-openclaw.audit.ts.
//
// Cross-repo diff of aphrody / open-design / openclaw inventories. Emits
// a markdown audit under docs/audits/openclaw-*.md.
//
// TODO Phase 2 follow-up: full diff pipeline. The TS source (700 LOC)
// walks all three repos, normalises package metadata, computes per-axis
// coverage (skills, hooks, components), and renders a multi-section
// audit. This stub validates clone presence and emits a placeholder
// audit referencing the openclaw-extensions-audit output.

use std::{
    env, fs,
    path::PathBuf,
    process::Command,
};

use anyhow::{Context, Result};
use clap::Args as ClapArgs;

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    /// Path to the openclaw clone.
    #[arg(long, default_value = "C:/worktree/openclaw")]
    openclaw: PathBuf,

    /// Path to the open-design clone.
    #[arg(long, default_value = "C:/worktree/open-design")]
    open_design: PathBuf,

    /// Output markdown path.
    #[arg(long)]
    out: Option<PathBuf>,
}

pub(crate) fn run(args: Args) -> Result<()> {
    let root = repo_root()?;
    let out = args.out.unwrap_or_else(|| {
        root.join("docs").join("audits").join(format!("openclaw-cross-{}.md", today_yyyymmdd()))
    });
    if let Some(p) = out.parent() { fs::create_dir_all(p).ok(); }

    println!(
        "[audit-openclaw] openclaw={} open-design={} out={}",
        args.openclaw.display(), args.open_design.display(), out.display()
    );

    let oc_present = args.openclaw.exists();
    let od_present = args.open_design.exists();
    if !oc_present {
        eprintln!("WARN: openclaw clone missing at {}", args.openclaw.display());
    }
    if !od_present {
        eprintln!("WARN: open-design clone missing at {}", args.open_design.display());
    }

    let body = format!(
        "<!-- SPDX-License-Identifier: Apache-2.0 -->\n\n\
         # aphrody vs open-design vs openclaw \u{2014} cross-repo audit\n\n\
         Generated: {generated}\n\n\
         **Status:** stub \u{2014} Rust port pending Phase 2 follow-up. \
         Full diff pipeline is in the TS source under git history; this \
         placeholder verifies clone presence and links to richer reports.\n\n\
         ## Clones\n\n\
         | Repo | Path | Present |\n\
         |---|---|---|\n\
         | openclaw    | `{oc}` | {ocp} |\n\
         | open-design | `{od}` | {odp} |\n\
         | aphrody     | `{ap}` | yes |\n\n\
         ## Companion reports\n\n\
         - `cargo xtask audit-openclaw-ext` \u{2192} `docs/audits/openclaw-extensions-roadmap.md`\n\
         - `cargo xtask skills-harvest`     \u{2192} `assets/skills-harvest/*.json`\n\
         - `cargo xtask skill-schema-check` \u{2192} per-skill `.od.yaml`\n",
        generated = now_iso8601(),
        oc = args.openclaw.display(), ocp = if oc_present {"yes"} else {"MISSING"},
        od = args.open_design.display(), odp = if od_present {"yes"} else {"MISSING"},
        ap = root.display(),
    );
    fs::write(&out, body).context("write audit")?;
    println!("[audit-openclaw] wrote {}", out.display());
    Ok(())
}

fn repo_root() -> Result<PathBuf> {
    if let Ok(dir) = env::var("CARGO_WORKSPACE_DIR") { return Ok(PathBuf::from(dir)); }
    let out = Command::new("git").args(["rev-parse", "--show-toplevel"]).output().context("git")?;
    if out.status.success() { return Ok(PathBuf::from(String::from_utf8(out.stdout)?.trim())); }
    Ok(env::current_dir()?)
}

fn today_yyyymmdd() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let (y, m, d, _, _, _) = ymdhms(secs);
    format!("{y:04}-{m:02}-{d:02}")
}

fn now_iso8601() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let (y, mo, d, h, mi, s) = ymdhms(secs);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

fn ymdhms(mut t: u64) -> (i32, u32, u32, u32, u32, u32) {
    let s = (t % 60) as u32; t /= 60;
    let mi = (t % 60) as u32; t /= 60;
    let h = (t % 24) as u32; t /= 24;
    let mut days = t as i64; days += 719_468;
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
