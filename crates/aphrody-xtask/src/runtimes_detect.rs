// runtimes_detect.rs — Rust port of scripts/runtimes-detect.ts.
//
// Detects external CLI binaries (coding agents + Rust toolchain + browser)
// on PATH. Emits a JSON manifest consumable by `aphrody runtimes`.
//
// Diverges from the .ts source by dropping the Bun/Node-only entries from
// the aphrody-internal block per the 100% Rust policy
// (memory `feedback_aphrody_rust_only`) :
//   - removed : bun, lightpanda, n2b (Node-based wrapper)
//   - added   : rustc, rustup
//   - mandatory-for-check : [cargo, rustc, gh] (was [bun, cargo, gh])
//
// Usage:
//     cargo xtask runtimes-detect
//     cargo xtask runtimes-detect --json
//     cargo xtask runtimes-detect --output path/manifest.json
//     cargo xtask runtimes-detect --check          # exit 1 if mandatory missing
//     cargo xtask runtimes-detect --write          # default path under var/

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use clap::Args as ClapArgs;
use serde::Serialize;

const PROBE_TIMEOUT: Duration = Duration::from_secs(5);
const MANDATORY_FOR_CHECK: &[&str] = &["cargo", "rustc", "gh"];

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
enum Category {
    CodingAgent,
    Toolchain,
    Browser,
    AphrodyInternal,
}

#[derive(Debug, Clone, Copy, Serialize)]
enum Source {
    #[serde(rename = "open-design")]
    OpenDesign,
    #[serde(rename = "aphrody")]
    Aphrody,
}

struct Spec {
    id: &'static str,
    name: &'static str,
    bin: &'static str,
    fallback_bins: &'static [&'static str],
    version_args: &'static [&'static str],
    category: Category,
    source: Source,
}

const RUNTIMES: &[Spec] = &[
    // --- open-design coding agents (16 entries, ported 1:1) -----------
    Spec { id: "claude", name: "Claude Code", bin: "claude", fallback_bins: &["openclaude"], version_args: &["--version"], category: Category::CodingAgent, source: Source::OpenDesign },
    Spec { id: "codex", name: "Codex CLI", bin: "codex", fallback_bins: &[], version_args: &["--version"], category: Category::CodingAgent, source: Source::OpenDesign },
    Spec { id: "copilot", name: "GitHub Copilot CLI", bin: "copilot", fallback_bins: &[], version_args: &["--version"], category: Category::CodingAgent, source: Source::OpenDesign },
    Spec { id: "cursor-agent", name: "Cursor Agent", bin: "cursor-agent", fallback_bins: &[], version_args: &["--version"], category: Category::CodingAgent, source: Source::OpenDesign },
    Spec { id: "deepseek", name: "DeepSeek TUI", bin: "deepseek", fallback_bins: &[], version_args: &["--version"], category: Category::CodingAgent, source: Source::OpenDesign },
    Spec { id: "devin", name: "Devin for Terminal", bin: "devin", fallback_bins: &[], version_args: &["--version"], category: Category::CodingAgent, source: Source::OpenDesign },
    Spec { id: "gemini", name: "Gemini CLI", bin: "gemini", fallback_bins: &[], version_args: &["--version"], category: Category::CodingAgent, source: Source::OpenDesign },
    Spec { id: "hermes", name: "Hermes", bin: "hermes", fallback_bins: &[], version_args: &["--version"], category: Category::CodingAgent, source: Source::OpenDesign },
    Spec { id: "kilo", name: "Kilo", bin: "kilo", fallback_bins: &[], version_args: &["--version"], category: Category::CodingAgent, source: Source::OpenDesign },
    Spec { id: "kimi", name: "Kimi CLI", bin: "kimi", fallback_bins: &[], version_args: &["--version"], category: Category::CodingAgent, source: Source::OpenDesign },
    Spec { id: "kiro", name: "Kiro CLI", bin: "kiro-cli", fallback_bins: &[], version_args: &["--version"], category: Category::CodingAgent, source: Source::OpenDesign },
    Spec { id: "opencode", name: "OpenCode", bin: "opencode-cli", fallback_bins: &["opencode"], version_args: &["--version"], category: Category::CodingAgent, source: Source::OpenDesign },
    Spec { id: "pi", name: "Pi", bin: "pi", fallback_bins: &[], version_args: &["--version"], category: Category::CodingAgent, source: Source::OpenDesign },
    Spec { id: "qoder", name: "Qoder CLI", bin: "qodercli", fallback_bins: &[], version_args: &["--version"], category: Category::CodingAgent, source: Source::OpenDesign },
    Spec { id: "qwen", name: "Qwen Code", bin: "qwen", fallback_bins: &[], version_args: &["--version"], category: Category::CodingAgent, source: Source::OpenDesign },
    Spec { id: "vibe", name: "Mistral Vibe CLI", bin: "vibe-acp", fallback_bins: &[], version_args: &["--version"], category: Category::CodingAgent, source: Source::OpenDesign },

    // --- aphrody-internal (post Rust-only purge) ----------------------
    // Removed: bun, lightpanda, n2b (Node-based wrapper).
    // Added: rustc, rustup.
    Spec { id: "cargo", name: "Rust cargo", bin: "cargo", fallback_bins: &[], version_args: &["--version"], category: Category::Toolchain, source: Source::Aphrody },
    Spec { id: "rustc", name: "Rust compiler", bin: "rustc", fallback_bins: &[], version_args: &["--version"], category: Category::Toolchain, source: Source::Aphrody },
    Spec { id: "rustup", name: "Rust toolchain installer", bin: "rustup", fallback_bins: &[], version_args: &["--version"], category: Category::Toolchain, source: Source::Aphrody },
    Spec { id: "gh", name: "GitHub CLI", bin: "gh", fallback_bins: &[], version_args: &["--version"], category: Category::Toolchain, source: Source::Aphrody },
    Spec { id: "msedge", name: "Microsoft Edge", bin: "msedge", fallback_bins: &["msedge.exe"], version_args: &["--version"], category: Category::Browser, source: Source::Aphrody },
    Spec { id: "bxc", name: "bxc scraper", bin: "bxc", fallback_bins: &[], version_args: &["--version"], category: Category::AphrodyInternal, source: Source::Aphrody },
];

const ABSOLUTE_FALLBACKS: &[(&str, &[&str])] = &[
    ("msedge", &[
        "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe",
        "C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe",
        "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
        "/usr/bin/microsoft-edge",
    ]),
];

#[derive(Debug, Serialize)]
struct Detected {
    id: String,
    name: String,
    bin: String,
    #[serde(rename = "resolvedBin")]
    resolved_bin: Option<String>,
    category: Category,
    source: Source,
    found: bool,
    path: Option<String>,
    version: Option<String>,
    ok: bool,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct Manifest {
    #[serde(rename = "schemaVersion")]
    schema_version: u32,
    #[serde(rename = "generatedAt")]
    generated_at: String,
    platform: String,
    arch: String,
    #[serde(rename = "runtimeCount")]
    runtime_count: usize,
    #[serde(rename = "foundCount")]
    found_count: usize,
    runtimes: Vec<Detected>,
}

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    /// Emit manifest JSON to stdout (skips the pretty table).
    #[arg(long)]
    json: bool,

    /// Write manifest JSON to a custom path.
    #[arg(long)]
    output: Option<PathBuf>,

    /// Write manifest to the default path (`var/data/runtimes/manifest.json`).
    #[arg(long)]
    write: bool,

    /// Exit with code 1 if any of [cargo, rustc, gh] is missing.
    #[arg(long)]
    check: bool,
}

pub(crate) fn run(args: Args) -> Result<()> {
    let runtimes: Vec<Detected> = RUNTIMES.iter().map(detect_one).collect();
    let manifest = build_manifest(runtimes);

    let target = args.output.clone().or_else(|| {
        if args.write {
            Some(default_output()?)
        } else {
            None
        }
    });

    if let Some(p) = &target {
        write_manifest(p, &manifest)?;
        if !args.json {
            eprintln!("runtimes-detect: wrote {}", p.display());
        }
    }

    if args.json {
        println!("{}", serde_json::to_string_pretty(&manifest)?);
    } else {
        println!("{}", render_table(&manifest.runtimes));
        println!(
            "\nplatform={} arch={} found={}/{}",
            manifest.platform, manifest.arch, manifest.found_count, manifest.runtime_count
        );
    }

    if args.check {
        let missing: Vec<&str> = MANDATORY_FOR_CHECK
            .iter()
            .filter(|id| !manifest.runtimes.iter().any(|r| r.id == **id && r.found))
            .copied()
            .collect();
        if !missing.is_empty() {
            eprintln!(
                "runtimes-detect: mandatory runtimes missing: {}",
                missing.join(", ")
            );
            std::process::exit(1);
        }
    }
    Ok(())
}

fn default_output() -> Option<PathBuf> {
    let root = repo_root().ok()?;
    Some(root.join("var").join("data").join("runtimes").join("manifest.json"))
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

fn resolve_bin(spec: &Spec) -> Option<String> {
    let mut candidates: Vec<&str> = vec![spec.bin];
    candidates.extend(spec.fallback_bins);
    for cand in candidates {
        if let Ok(path) = which(cand) {
            return Some(path);
        }
    }
    for (id, abs) in ABSOLUTE_FALLBACKS {
        if *id == spec.id {
            for candidate in *abs {
                if Path::new(candidate).is_file() {
                    return Some((*candidate).to_string());
                }
            }
        }
    }
    None
}

fn which(name: &str) -> Result<String> {
    let var_name = if cfg!(windows) { "PATH" } else { "PATH" };
    let path = env::var(var_name).context("read PATH")?;
    let sep = if cfg!(windows) { ';' } else { ':' };
    let exts: Vec<String> = if cfg!(windows) {
        env::var("PATHEXT")
            .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_string())
            .split(';')
            .map(|s| s.trim().to_ascii_lowercase())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        vec![String::new()]
    };
    for dir in path.split(sep) {
        let base = Path::new(dir).join(name);
        for ext in &exts {
            let cand = if ext.is_empty() {
                base.clone()
            } else if name.to_ascii_lowercase().ends_with(ext) {
                base.clone()
            } else {
                base.with_extension(ext.trim_start_matches('.'))
            };
            if cand.is_file() {
                return Ok(cand.to_string_lossy().into_owned());
            }
        }
    }
    anyhow::bail!("not found: {name}")
}

fn detect_one(spec: &Spec) -> Detected {
    let resolved = resolve_bin(spec);
    let Some(rb) = resolved else {
        return Detected {
            id: spec.id.into(),
            name: spec.name.into(),
            bin: spec.bin.into(),
            resolved_bin: None,
            category: spec.category,
            source: spec.source,
            found: false,
            path: None,
            version: None,
            ok: false,
            error: Some("not found on PATH".into()),
        };
    };
    let (version, ok, error) = probe_version(&rb, spec.version_args);
    Detected {
        id: spec.id.into(),
        name: spec.name.into(),
        bin: spec.bin.into(),
        resolved_bin: Some(rb.clone()),
        category: spec.category,
        source: spec.source,
        found: true,
        path: Some(rb),
        version,
        ok,
        error,
    }
}

fn probe_version(bin: &str, args: &[&str]) -> (Option<String>, bool, Option<String>) {
    use std::io::Read;
    use std::sync::mpsc;
    use std::thread;

    let bin = bin.to_string();
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let mut child = match Command::new(&bin)
            .args(&args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .stdin(std::process::Stdio::null())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send((None, false, Some(e.to_string())));
                return;
            }
        };
        let status = child.wait();
        let mut stdout = String::new();
        if let Some(mut s) = child.stdout.take() {
            let _ = s.read_to_string(&mut stdout);
        }
        let mut stderr = String::new();
        if let Some(mut s) = child.stderr.take() {
            let _ = s.read_to_string(&mut stderr);
        }
        let blob = format!("{stdout}\n{stderr}");
        let first = blob
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty())
            .map(|s| s.to_string());
        match status {
            Ok(s) if s.success() => {
                let _ = tx.send((first, true, None));
            }
            Ok(s) => {
                let _ = tx.send((first, false, Some(format!("exit {}", s.code().unwrap_or(-1)))));
            }
            Err(e) => {
                let _ = tx.send((None, false, Some(e.to_string())));
            }
        }
    });

    match rx.recv_timeout(PROBE_TIMEOUT) {
        Ok((v, ok, e)) => (v, ok, e),
        Err(_) => (None, false, Some(format!("timeout after {}ms", PROBE_TIMEOUT.as_millis()))),
    }
}

fn build_manifest(runtimes: Vec<Detected>) -> Manifest {
    let runtime_count = runtimes.len();
    let found_count = runtimes.iter().filter(|r| r.found).count();
    Manifest {
        schema_version: 1,
        generated_at: now_iso8601(),
        platform: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        runtime_count,
        found_count,
        runtimes,
    }
}

fn now_iso8601() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Minimal RFC3339 — UTC, no fractional seconds. Enough for the manifest.
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
    // Civil from days algorithm (Howard Hinnant).
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

fn write_manifest(path: &Path, manifest: &Manifest) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("mkdir manifest parent")?;
    }
    let body = serde_json::to_string_pretty(manifest)? + "\n";
    fs::write(path, body).context("write manifest")?;
    Ok(())
}

fn pad(s: &str, n: usize) -> String {
    if s.len() >= n {
        s.chars().take(n).collect()
    } else {
        let mut out = s.to_string();
        out.push_str(&" ".repeat(n - s.len()));
        out
    }
}

fn render_table(runtimes: &[Detected]) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "{}| {}| {}| found",
        pad("ID", 20),
        pad("bin", 20),
        pad("version", 30)
    ));
    lines.push(format!("{}|{}|{}|------", "-".repeat(20), "-".repeat(21), "-".repeat(31)));
    for r in runtimes {
        let version = if r.found {
            r.version.clone().unwrap_or_else(|| {
                if r.ok {
                    "(empty)".to_string()
                } else {
                    format!("err: {}", r.error.clone().unwrap_or_else(|| "?".into()))
                }
            })
        } else {
            "-".to_string()
        };
        let truncated = if version.len() > 30 {
            format!("{}...", &version[..27])
        } else {
            version
        };
        lines.push(format!(
            "{}| {}| {}| {}",
            pad(&r.id, 20),
            pad(&r.bin, 20),
            pad(&truncated, 30),
            if r.found { "yes" } else { "no" }
        ));
    }
    lines.join("\n")
}
