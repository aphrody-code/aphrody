// audit_openclaw_ext.rs — Rust port of scripts/openclaw-extensions-audit.ts.
//
// Enumerates every extension under `C:/src/openclaw/extensions/`,
// extracts metadata (name, description, version, LOC, test count, deps),
// scores each by port-priority for aphrody, and emits a ranked roadmap.

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result};
use clap::Args as ClapArgs;
use regex::Regex;
use serde::Serialize;
use walkdir::WalkDir;

const DEFAULT_SOURCE: &str = "C:/src/openclaw/extensions";

const CHANNEL_ADAPTERS: &[&str] = &[
    "discord", "slack", "telegram", "whatsapp", "signal", "imessage", "irc",
    "teams", "matrix", "feishu", "line", "mattermost", "nextcloud", "nextcloud-talk",
    "nostr", "synology", "synology-chat", "tlon", "twitch", "zalo", "zalouser",
    "wechat", "webchat", "qq", "qqbot", "qa-channel",
];

const CODE_EXTS: &[&str] = &["ts", "tsx", "js", "jsx", "mjs", "cjs"];
const SKIP_DIRS: &[&str] = &[
    "node_modules", ".git", "dist", "build", "out", ".turbo", ".next",
    ".cache", "coverage", ".vite",
];

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum Priority {
    High,
    Medium,
    Low,
}

impl Priority {
    fn rank(self) -> u8 {
        match self { Self::High => 3, Self::Medium => 2, Self::Low => 1 }
    }
    fn as_str(self) -> &'static str {
        match self { Self::High => "high", Self::Medium => "medium", Self::Low => "low" }
    }
}

#[derive(Debug, Serialize)]
struct ExtensionRecord {
    dir: String,
    name: String,
    #[serde(rename = "bareName")]
    bare_name: String,
    description: String,
    version: String,
    keywords: Vec<String>,
    loc: usize,
    #[serde(rename = "testCount")]
    test_count: usize,
    #[serde(rename = "depsCount")]
    deps_count: usize,
    deps: Vec<String>,
    priority: Priority,
    #[serde(rename = "priorityReason")]
    priority_reason: String,
    #[serde(rename = "hasReadme")]
    has_readme: bool,
    #[serde(rename = "hasSrc")]
    has_src: bool,
}

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    /// Override the extensions source directory.
    #[arg(long, default_value = DEFAULT_SOURCE)]
    source: PathBuf,

    /// Emit JSON to stdout (no files written).
    #[arg(long)]
    json: bool,

    /// Restrict roadmap rows to top-N.
    #[arg(long)]
    top: Option<usize>,
}

pub(crate) fn run(args: Args) -> Result<()> {
    if !args.source.exists() {
        eprintln!("source directory not found: {}", args.source.display());
        std::process::exit(1);
    }

    let records = audit(&args.source)?;

    if args.json {
        let payload = serde_json::json!({
            "source": args.source.to_string_lossy(),
            "count": records.len(),
            "records": records,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    let root = repo_root()?;
    let out_json = root.join("var").join("data").join("openclaw-extensions.json");
    let out_md = root.join("docs").join("audits").join("openclaw-extensions-roadmap.md");

    if let Some(p) = out_json.parent() { fs::create_dir_all(p).ok(); }
    if let Some(p) = out_md.parent() { fs::create_dir_all(p).ok(); }

    let payload = serde_json::json!({
        "source": args.source.to_string_lossy(),
        "generatedAt": now_iso8601(),
        "count": records.len(),
        "records": records,
    });
    fs::write(&out_json, format!("{}\n", serde_json::to_string_pretty(&payload)?))
        .context("write openclaw-extensions.json")?;
    fs::write(&out_md, render_markdown(&records, &args)).context("write roadmap.md")?;

    let mut dist = (0usize, 0usize, 0usize);
    for r in &records {
        match r.priority { Priority::High => dist.0 += 1, Priority::Medium => dist.1 += 1, Priority::Low => dist.2 += 1 }
    }
    println!("wrote {}", out_json.display());
    println!("wrote {}", out_md.display());
    println!("extensions: {}", records.len());
    println!("priority: high={} medium={} low={}", dist.0, dist.1, dist.2);
    println!("top-10 by rank:");
    for (i, r) in records.iter().take(10).enumerate() {
        println!(
            "  {:>2}. [{}] {} (LOC={}, tests={}, deps={})",
            i + 1, r.priority.as_str(), r.bare_name, r.loc, r.test_count, r.deps_count
        );
    }
    Ok(())
}

fn audit(source: &Path) -> Result<Vec<ExtensionRecord>> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    for entry in fs::read_dir(source).with_context(|| format!("read_dir {}", source.display()))? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let p = entry.path();
            if p.join("package.json").exists() {
                dirs.push(p);
            }
        }
    }
    dirs.sort();

    let high_re = Regex::new(r"(?i)memory|gateway|voice|sdk").expect("static");

    let mut records: Vec<ExtensionRecord> = Vec::new();
    for dir in &dirs {
        let pkg_path = dir.join("package.json");
        let Some(pkg) = read_json_obj(&pkg_path) else { continue; };
        let folder = dir.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let name = pkg.get("name").and_then(|v| v.as_str()).unwrap_or(&folder).to_string();
        let bare = strip_scope(&name);
        let description = pkg.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let version = pkg.get("version").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let keywords: Vec<String> = pkg.get("keywords")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let mut deps: Vec<String> = pkg.get("dependencies")
            .and_then(|v| v.as_object())
            .map(|o| o.keys().cloned().collect())
            .unwrap_or_default();
        deps.sort();
        let (loc, test_count) = count_lines_and_tests(dir);
        let (priority, reason) = classify(&bare, &keywords, &high_re);

        records.push(ExtensionRecord {
            dir: dir.to_string_lossy().into_owned(),
            name,
            bare_name: bare,
            description,
            version,
            keywords,
            loc,
            test_count,
            deps_count: deps.len(),
            deps,
            priority,
            priority_reason: reason,
            has_readme: dir.join("README.md").exists(),
            has_src: dir.join("src").exists(),
        });
    }

    records.sort_by(|a, b| {
        b.priority.rank().cmp(&a.priority.rank())
            .then_with(|| b.loc.cmp(&a.loc))
            .then_with(|| a.name.cmp(&b.name))
    });
    Ok(records)
}

fn classify(bare: &str, keywords: &[String], high_re: &Regex) -> (Priority, String) {
    if let Some(m) = high_re.find(bare) {
        return (Priority::High, format!("name matches /{}/", m.as_str()));
    }
    if CHANNEL_ADAPTERS.iter().any(|s| *s == bare) {
        return (Priority::Medium, "channel adapter".to_string());
    }
    let kw_lower: Vec<String> = keywords.iter().map(|k| k.to_ascii_lowercase()).collect();
    if kw_lower.iter().any(|k| high_re.is_match(k)) {
        return (Priority::High, "keyword match (memory|gateway|voice|sdk)".to_string());
    }
    if kw_lower.iter().any(|k| CHANNEL_ADAPTERS.iter().any(|c| *c == k.as_str())) {
        return (Priority::Medium, "keyword match (channel)".to_string());
    }
    (Priority::Low, "default".to_string())
}

fn strip_scope(name: &str) -> String {
    if let Some(rest) = name.strip_prefix('@') {
        if let Some(slash) = rest.find('/') {
            return rest[slash + 1..].to_string();
        }
    }
    name.to_string()
}

fn read_json_obj(path: &Path) -> Option<serde_json::Map<String, serde_json::Value>> {
    let raw = fs::read_to_string(path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
    v.as_object().cloned()
}

fn count_lines_and_tests(dir: &Path) -> (usize, usize) {
    let mut loc = 0usize;
    let mut tests = 0usize;
    for entry in WalkDir::new(dir)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !SKIP_DIRS.iter().any(|s| *s == name.as_ref())
        })
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() { continue; }
        let name = entry.file_name().to_string_lossy();
        let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) else { continue; };
        let ext = ext.to_ascii_lowercase();
        if !CODE_EXTS.iter().any(|s| *s == ext.as_str()) { continue; }
        if name.ends_with(".test.ts") || name.ends_with(".test.tsx") || name.ends_with(".test.js") {
            tests += 1;
        }
        if let Ok(raw) = fs::read_to_string(entry.path()) {
            if raw.is_empty() { continue; }
            let n = raw.bytes().filter(|b| *b == b'\n').count();
            loc += if raw.ends_with('\n') { n } else { n + 1 };
        }
    }
    (loc, tests)
}

fn render_markdown(records: &[ExtensionRecord], args: &Args) -> String {
    let mut dist = (0usize, 0usize, 0usize);
    for r in records {
        match r.priority { Priority::High => dist.0 += 1, Priority::Medium => dist.1 += 1, Priority::Low => dist.2 += 1 }
    }
    let rows: &[ExtensionRecord] = match args.top {
        Some(n) if n < records.len() => &records[..n],
        _ => records,
    };
    let mut out = String::new();
    out.push_str("<!-- SPDX-License-Identifier: Apache-2.0 -->\n\n");
    out.push_str("# Openclaw extensions \u{2014} port-priority roadmap\n\n");
    out.push_str(&format!("**Source:** `{}`\n", args.source.display()));
    out.push_str(&format!("**Generated:** {}\n", now_iso8601()));
    out.push_str(&format!("**Total extensions:** {}\n", records.len()));
    out.push_str(&format!("**Distribution:** high={}, medium={}, low={}\n\n", dist.0, dist.1, dist.2));
    out.push_str("## Ranked table\n\n");
    out.push_str("| priority | name | LOC | tests | deps | description |\n");
    out.push_str("|---|---|---:|---:|---:|---|\n");
    for r in rows {
        let desc = r.description.replace('|', "\\|").replace('\n', " ");
        let desc = if desc.is_empty() { "\u{2014}".to_string() } else { desc };
        out.push_str(&format!(
            "| {} | `{}` | {} | {} | {} | {} |\n",
            r.priority.as_str(), r.name, r.loc, r.test_count, r.deps_count, desc
        ));
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

fn now_iso8601() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
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
