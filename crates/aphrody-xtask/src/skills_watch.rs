// skills_watch.rs — Rust port of scripts/skills-hot-reload.ts.
//
// Hot-reload watcher for `.claude/skills/` (and `.claude/agents/`). On every
// change writes a flat JSON manifest of parsed skills/agents and touches a
// reload-signal file consumable by downstream daemons.
//
// TODO Phase 2 follow-up: switch from polling loop to `notify` watcher
// crate (already in workspace.dependencies) for sub-100ms event latency.
// This stub implements `--once` mode (single scan + emit) which is the CI
// path. Continuous mode falls back to a coarse 1.5s poll.

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::Duration,
};

use anyhow::{Context, Result};
use clap::Args as ClapArgs;
use serde::Serialize;
use walkdir::WalkDir;

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    /// Run a single full scan then exit (CI mode).
    #[arg(long)]
    once: bool,

    /// Signal-file path written on each reload.
    #[arg(long)]
    reload_signal: Option<PathBuf>,

    /// Manifest output path.
    #[arg(long)]
    manifest: Option<PathBuf>,

    /// Poll interval in milliseconds (continuous mode).
    #[arg(long, default_value = "1500")]
    poll_ms: u64,
}

#[derive(Debug, Serialize)]
struct SkillRecord {
    kind: String,
    path: String,
    name: String,
    size: u64,
}

pub(crate) fn run(args: Args) -> Result<()> {
    let root = repo_root()?;
    let skills_root = root.join(".claude").join("skills");
    let agents_root = root.join(".claude").join("agents");
    let signal_path = args
        .reload_signal
        .unwrap_or_else(|| root.join("var").join("data").join("skills-reload.signal"));
    let manifest_path = args
        .manifest
        .unwrap_or_else(|| root.join("var").join("data").join("skills-manifest.json"));

    if let Some(p) = signal_path.parent() { fs::create_dir_all(p).ok(); }
    if let Some(p) = manifest_path.parent() { fs::create_dir_all(p).ok(); }

    println!(
        "[skills-watch] skills={} agents={} signal={} manifest={}",
        skills_root.display(), agents_root.display(),
        signal_path.display(), manifest_path.display()
    );

    if args.once {
        return scan_and_emit(&skills_root, &agents_root, &signal_path, &manifest_path);
    }

    println!("[skills-watch] entering poll loop ({}ms interval). Ctrl-C to stop.", args.poll_ms);
    loop {
        if let Err(e) = scan_and_emit(&skills_root, &agents_root, &signal_path, &manifest_path) {
            eprintln!("[skills-watch] scan error: {e:#}");
        }
        thread::sleep(Duration::from_millis(args.poll_ms));
    }
}

fn scan_and_emit(skills_root: &Path, agents_root: &Path, signal: &Path, manifest: &Path) -> Result<()> {
    let mut records: Vec<SkillRecord> = Vec::new();
    if skills_root.exists() {
        records.extend(scan_dir(skills_root, "skill"));
    }
    if agents_root.exists() {
        records.extend(scan_dir(agents_root, "agent"));
    }
    let payload = serde_json::json!({
        "$schema": "https://aphrody-code.dev/schemas/skills-manifest/v1",
        "generatedAt": now_iso8601(),
        "count": records.len(),
        "records": records,
    });
    fs::write(manifest, format!("{}\n", serde_json::to_string_pretty(&payload)?))
        .context("write manifest")?;
    fs::write(signal, format!("{}\n", now_iso8601())).context("write signal")?;
    println!("[skills-watch] scanned {} entries", records.len());
    Ok(())
}

fn scan_dir(root: &Path, kind: &str) -> Vec<SkillRecord> {
    let mut out = Vec::new();
    for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
        if entry.file_type().is_file() && entry.file_name() == "SKILL.md" {
            let dir = entry.path().parent().unwrap_or(entry.path());
            let name = dir.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            out.push(SkillRecord {
                kind: kind.to_string(),
                path: entry.path().to_string_lossy().into_owned(),
                name,
                size: entry.metadata().map(|m| m.len()).unwrap_or(0),
            });
        }
    }
    out
}

fn repo_root() -> Result<PathBuf> {
    if let Ok(dir) = env::var("CARGO_WORKSPACE_DIR") { return Ok(PathBuf::from(dir)); }
    let out = Command::new("git").args(["rev-parse", "--show-toplevel"]).output().context("git")?;
    if out.status.success() { return Ok(PathBuf::from(String::from_utf8(out.stdout)?.trim())); }
    Ok(env::current_dir()?)
}

fn now_iso8601() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let (y, mo, d, h, mi, s) = unix_to_ymdhms(secs);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

fn unix_to_ymdhms(mut t: u64) -> (i32, u32, u32, u32, u32, u32) {
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
