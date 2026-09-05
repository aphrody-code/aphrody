// SPDX-License-Identifier: Apache-2.0
// `aphrody scan` — repo-analysis subcommands (native Rust replacement for
// scripts/scan-tree.ps1 + scripts/scan-manifests.ps1).
//
// Phase 3.6 of docs/PLAN_RUST_ONLY.md.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Mutex,
    time::Instant,
};

use async_trait::async_trait;
use miette::{IntoDiagnostic, WrapErr};
use rayon::prelude::*;
use serde_json::{Value, json};
use walkdir::WalkDir;

use crate::context::{GoogleContext, TerminalCommand};

// ── aphrody scan tree ──────────────────────────────────────────────────────
//
// Recursive size + file-count breakdown of top-level groups (vendor, packages,
// crates by default; --groups to override). Parallel walk per top-level
// directory via rayon. Emits a JSON report and a console summary.

pub(crate) struct TreeCommand {
    /// Root of the project. Defaults to the current working directory.
    pub root: Option<PathBuf>,
    /// Comma-separated list of top-level group directories to inspect.
    /// Defaults to `vendor,packages,crates`.
    pub groups: Vec<String>,
    /// Output JSON file path. `-` writes to stdout.
    pub output: Option<PathBuf>,
    /// Number of top extensions to include per directory.
    pub top_ext_count: usize,
}

#[async_trait]
impl TerminalCommand for TreeCommand {
    async fn execute(&self, _ctx: &GoogleContext) -> miette::Result<()> {
        let root = resolve_root(self.root.clone())?;
        let groups: Vec<String> = if self.groups.is_empty() {
            vec!["vendor".into(), "packages".into(), "crates".into()]
        } else {
            self.groups.clone()
        };

        eprintln!("scan tree");
        eprintln!("  root   : {}", root.display());
        eprintln!("  groups : {}", groups.join(", "));

        let started = Instant::now();

        // Collect direct subdirs of each group into a flat work-list.
        let mut tasks: Vec<(String, PathBuf, String)> = Vec::new();
        for group in &groups {
            let base = root.join(group);
            if !base.is_dir() {
                continue;
            }
            for entry in std::fs::read_dir(&base)
                .into_diagnostic()
                .wrap_err_with(|| format!("failed to enumerate {}", base.display()))?
            {
                let entry = entry.into_diagnostic()?;
                let p = entry.path();
                if p.is_dir() {
                    let name =
                        p.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
                    tasks.push((group.clone(), p, name));
                }
            }
        }

        eprintln!("  tasks  : {} top-level directories", tasks.len());

        let top_n = self.top_ext_count;
        let entries: Vec<Value> = tasks
            .par_iter()
            .map(|(group, path, name)| scan_dir(group, path, name, top_n))
            .collect();

        let elapsed = started.elapsed();

        // Per-group totals.
        let mut by_group: HashMap<String, (u64, u64, u64)> = HashMap::new();
        for e in &entries {
            let g = e["group"].as_str().unwrap_or("").to_string();
            let files = e["files"].as_u64().unwrap_or(0);
            let bytes = e["bytes"].as_u64().unwrap_or(0);
            let slot = by_group.entry(g).or_insert((0, 0, 0));
            slot.0 += 1; // dirs
            slot.1 += files;
            slot.2 += bytes;
        }
        let mut by_group_json: Vec<Value> = by_group
            .iter()
            .map(|(g, (dirs, files, bytes))| {
                json!({
                    "group": g,
                    "dirs": dirs,
                    "files": files,
                    "bytes": bytes,
                    "mb": (*bytes as f64 / 1_048_576.0).round() / 1.0,
                    "gb": (*bytes as f64 / 1_073_741_824.0 * 1000.0).round() / 1000.0,
                })
            })
            .collect();
        by_group_json.sort_by_key(|v| std::cmp::Reverse(v["bytes"].as_u64().unwrap_or(0)));

        // Sort entries by bytes desc for the directories array.
        let mut sorted = entries.clone();
        sorted.sort_by_key(|v| std::cmp::Reverse(v["bytes"].as_u64().unwrap_or(0)));

        let report = json!({
            "generatedAt": chrono_iso8601_now(),
            "root": root.display().to_string(),
            "elapsedMs": elapsed.as_millis() as u64,
            "byGroup": by_group_json,
            "directories": sorted,
        });

        write_report(self.output.as_deref(), &report)?;

        // Console summary.
        println!();
        println!("=== PER-GROUP TOTALS ===");
        for v in report["byGroup"].as_array().unwrap_or(&vec![]) {
            println!(
                "  {:<10} {:>5} dirs  {:>10} files  {:>8.2} GB",
                v["group"].as_str().unwrap_or(""),
                v["dirs"].as_u64().unwrap_or(0),
                v["files"].as_u64().unwrap_or(0),
                v["gb"].as_f64().unwrap_or(0.0),
            );
        }
        println!();
        println!("=== TOP 20 DIRECTORIES BY SIZE ===");
        for v in report["directories"].as_array().unwrap_or(&vec![]).iter().take(20) {
            println!(
                "  {:<10} {:<40} {:>10} files  {:>10.2} MB",
                v["group"].as_str().unwrap_or(""),
                v["name"].as_str().unwrap_or(""),
                v["files"].as_u64().unwrap_or(0),
                v["mb"].as_f64().unwrap_or(0.0),
            );
        }
        println!();
        println!("done in {:.1} s", elapsed.as_secs_f64());
        Ok(())
    }
}

fn scan_dir(group: &str, path: &Path, name: &str, top_n: usize) -> Value {
    let mut bytes: u64 = 0;
    let mut files: u64 = 0;
    let mut ext_count: HashMap<String, u64> = HashMap::new();

    for entry in WalkDir::new(path).follow_links(false).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        if let Ok(md) = entry.metadata() {
            bytes += md.len();
            files += 1;
        }
        let ext = entry
            .path()
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_else(|| "<noext>".into());
        *ext_count.entry(ext).or_insert(0) += 1;
    }

    let mut sorted: Vec<(String, u64)> = ext_count.into_iter().collect();
    sorted.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
    let top_exts: Vec<Value> =
        sorted.into_iter().take(top_n).map(|(e, c)| json!({"ext": e, "count": c})).collect();

    json!({
        "group": group,
        "name": name,
        "files": files,
        "bytes": bytes,
        "mb": (bytes as f64 / 1_048_576.0 * 100.0).round() / 100.0,
        "gb": (bytes as f64 / 1_073_741_824.0 * 1000.0).round() / 1000.0,
        "topExt": top_exts,
    })
}

// ── aphrody scan manifests ─────────────────────────────────────────────────
//
// Walks the repo for Cargo.toml / package.json / pyproject.toml / etc.,
// extracts {name, version, dependencyCount, workspaceMembers} and emits a
// consolidated JSON report.

const DEFAULT_EXCLUDES: &[&str] = &[
    "node_modules",
    "target",
    ".git",
    "dist",
    "build",
    ".cargo/registry",
    ".cache",
    ".next",
    ".turbo",
    "__pycache__",
    ".venv",
    ".mypy_cache",
    ".ruff_cache",
];

const MANIFEST_NAMES: &[&str] = &[
    "Cargo.toml",
    "package.json",
    "turbo.json",
    "tsconfig.json",
    "bunfig.toml",
    "pyproject.toml",
    "uv.toml",
    "deno.json",
    "deno.jsonc",
    "bun.lock",
    "requirements.txt",
];

pub(crate) struct ManifestsCommand {
    /// Root of the project. Defaults to the current working directory.
    pub root: Option<PathBuf>,
    /// Output JSON file path. `-` writes to stdout.
    pub output: Option<PathBuf>,
}

#[async_trait]
impl TerminalCommand for ManifestsCommand {
    async fn execute(&self, _ctx: &GoogleContext) -> miette::Result<()> {
        let root = resolve_root(self.root.clone())?;

        eprintln!("scan manifests");
        eprintln!("  root : {}", root.display());

        let started = Instant::now();

        // Collect candidate paths first (single-threaded walk to honour
        // excludes consistently).
        let candidates = Mutex::new(Vec::<PathBuf>::new());
        let walker = ignore::WalkBuilder::new(&root)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .hidden(false)
            .follow_links(false)
            .build();

        for entry in walker.flatten() {
            if !entry.file_type().is_some_and(|t| t.is_file()) {
                continue;
            }
            let path = entry.path();
            let path_str = path.to_string_lossy().to_lowercase();
            if DEFAULT_EXCLUDES.iter().any(|ex| {
                path_str.contains(&format!(
                    "{}{}{}",
                    std::path::MAIN_SEPARATOR,
                    ex,
                    std::path::MAIN_SEPARATOR
                )) || path_str.contains(&format!("/{}/", ex))
            }) {
                continue;
            }
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            if MANIFEST_NAMES.iter().any(|m| m.eq_ignore_ascii_case(name))
                || name.starts_with("tsconfig.") && name.ends_with(".json")
            {
                candidates.lock().unwrap().push(path.to_path_buf());
            }
        }

        let candidates = candidates.into_inner().unwrap();
        eprintln!("  found: {} manifests", candidates.len());

        let manifests: Vec<Value> =
            candidates.par_iter().filter_map(|p| parse_manifest(&root, p)).collect();

        let elapsed = started.elapsed();

        // Counts by filename.
        let mut counts: HashMap<String, u64> = HashMap::new();
        for m in &manifests {
            *counts.entry(m["fileName"].as_str().unwrap_or("").to_string()).or_insert(0) += 1;
        }
        let mut by_type: Vec<Value> =
            counts.into_iter().map(|(k, v)| json!({"pattern": k, "count": v})).collect();
        by_type.sort_by_key(|v| std::cmp::Reverse(v["count"].as_u64().unwrap_or(0)));

        let mut sorted_manifests = manifests.clone();
        sorted_manifests
            .sort_by(|a, b| a["path"].as_str().unwrap_or("").cmp(b["path"].as_str().unwrap_or("")));

        let report = json!({
            "generatedAt": chrono_iso8601_now(),
            "root": root.display().to_string(),
            "elapsedMs": elapsed.as_millis() as u64,
            "totalCount": manifests.len() as u64,
            "byType": by_type,
            "manifests": sorted_manifests,
        });

        write_report(self.output.as_deref(), &report)?;

        println!();
        println!("=== MANIFEST COUNTS BY TYPE ===");
        for v in report["byType"].as_array().unwrap_or(&vec![]) {
            println!(
                "  {:<20} {:>5}",
                v["pattern"].as_str().unwrap_or(""),
                v["count"].as_u64().unwrap_or(0)
            );
        }

        println!();
        println!("=== WORKSPACE ROOTS ===");
        for m in report["manifests"].as_array().unwrap_or(&vec![]) {
            if let Some(members) = m["meta"]["workspaceMembers"].as_u64() {
                println!("  {:<60} members={}", m["path"].as_str().unwrap_or(""), members);
            }
        }

        println!();
        println!("done in {:.1} s", elapsed.as_secs_f64());
        Ok(())
    }
}

fn parse_manifest(root: &Path, path: &Path) -> Option<Value> {
    let text = std::fs::read_to_string(path).ok()?;
    let file_name = path.file_name()?.to_str()?.to_string();
    let rel_path = path.strip_prefix(root).unwrap_or(path).display().to_string();
    let size_kb = (text.len() as f64 / 1024.0 * 100.0).round() / 100.0;

    let mut meta = serde_json::Map::new();

    if file_name == "bun.lock" {
        meta.insert("type".into(), json!("lockfile"));
    } else if file_name.ends_with(".json") || file_name.ends_with(".jsonc") {
        meta.insert("type".into(), json!("json"));
        // Permissive parse (tsconfig allows comments — strip line + block comments).
        let cleaned = strip_jsonc_comments(&text);
        match serde_json::from_str::<Value>(&cleaned) {
            Ok(obj) => {
                copy_str_field(&obj, "name", &mut meta);
                copy_str_field(&obj, "version", &mut meta);
                if let Some(d) = obj.get("dependencies").and_then(|v| v.as_object()) {
                    meta.insert("dependencyCount".into(), json!(d.len()));
                }
                if let Some(d) = obj.get("devDependencies").and_then(|v| v.as_object()) {
                    meta.insert("devDependencyCount".into(), json!(d.len()));
                }
                if let Some(ws) = obj.get("workspaces") {
                    let n = if let Some(arr) = ws.as_array() { arr.len() } else { 1 };
                    meta.insert("workspaceMembers".into(), json!(n));
                }
                if let Some(s) = obj.get("scripts").and_then(|v| v.as_object()) {
                    meta.insert("scriptCount".into(), json!(s.len()));
                }
                if let Some(p) = obj.get("pipeline").and_then(|v| v.as_object()) {
                    meta.insert("pipelineTasks".into(), json!(p.len()));
                }
            },
            Err(e) => {
                let msg = e.to_string();
                meta.insert("parseError".into(), json!(msg.chars().take(120).collect::<String>()));
            },
        }
    } else if file_name.ends_with(".toml") {
        meta.insert("type".into(), json!("toml"));
        // Light regex-style scan for [package] name/version — avoids pulling
        // a full TOML parser at this layer (already in workspace via `toml`
        // crate; can be upgraded if needed).
        if let Some(name) = find_toml_string(&text, "name") {
            meta.insert("name".into(), json!(name));
        }
        if let Some(version) = find_toml_string(&text, "version") {
            meta.insert("version".into(), json!(version));
        }
        let mut deps = 0usize;
        for section in &["dependencies", "dev-dependencies", "build-dependencies"] {
            deps += count_section_entries(&text, section);
        }
        meta.insert("dependencyCount".into(), json!(deps));
        if let Some(members) = count_workspace_members(&text) {
            meta.insert("workspaceMembers".into(), json!(members));
        }
    } else if file_name == "requirements.txt" {
        let n = text
            .lines()
            .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
            .count();
        meta.insert("type".into(), json!("pip-requirements"));
        meta.insert("dependencyCount".into(), json!(n));
    } else {
        meta.insert("type".into(), json!("unknown"));
    }

    Some(json!({
        "path": rel_path,
        "fileName": file_name,
        "sizeKB": size_kb,
        "meta": Value::Object(meta),
    }))
}

fn copy_str_field(src: &Value, key: &str, dst: &mut serde_json::Map<String, Value>) {
    if let Some(v) = src.get(key).and_then(|v| v.as_str()) {
        dst.insert(key.into(), json!(v));
    }
}

fn find_toml_string(text: &str, key: &str) -> Option<String> {
    // Look for `^<key>\s*=\s*"<value>"` on a non-comment line.
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            continue;
        }
        // Cheap split: KEY = "VALUE"
        let mut parts = trimmed.splitn(2, '=');
        let k = parts.next()?.trim();
        if k != key {
            continue;
        }
        let v = parts.next()?.trim();
        if let Some(s) = v.strip_prefix('"') {
            if let Some(end) = s.find('"') {
                return Some(s[..end].to_string());
            }
        }
    }
    None
}

fn count_section_entries(text: &str, section: &str) -> usize {
    // Find `^[section]` then count `^key = ...` lines until next `[...]` or EOF.
    let header = format!("[{section}]");
    let mut in_section = false;
    let mut count = 0usize;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_section = trimmed == header;
            continue;
        }
        if !in_section || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(eq_idx) = trimmed.find('=') {
            let key = trimmed[..eq_idx].trim();
            if !key.is_empty()
                && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            {
                count += 1;
            }
        }
    }
    count
}

fn count_workspace_members(text: &str) -> Option<usize> {
    // Look for `[workspace]` then `members = [ ... ]` and count quoted strings
    // before the closing bracket.
    let ws_idx = text.find("[workspace]")?;
    let tail = &text[ws_idx..];
    let members_idx = tail.find("members")?;
    let after = &tail[members_idx..];
    let open = after.find('[')?;
    let close = after[open..].find(']')?;
    let block = &after[open..open + close];
    let mut count = 0usize;
    let mut chars = block.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '"' {
            // skip until closing quote
            for c2 in chars.by_ref() {
                if c2 == '"' {
                    count += 1;
                    break;
                }
            }
        }
    }
    Some(count)
}

fn strip_jsonc_comments(input: &str) -> String {
    // Remove `//` line comments and `/* ... */` block comments. Safe-naive:
    // does not preserve string-literal slashes, which is fine for tsconfig-
    // shaped inputs where `//` inside strings is exceptionally rare.
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    let mut in_string = false;
    let mut escape = false;
    while let Some(c) = chars.next() {
        if in_string {
            out.push(c);
            if escape {
                escape = false;
            } else if c == '\\' {
                escape = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        if c == '"' {
            in_string = true;
            out.push(c);
            continue;
        }
        if c == '/' {
            match chars.peek() {
                Some('/') => {
                    // Skip until newline.
                    for c2 in chars.by_ref() {
                        if c2 == '\n' {
                            out.push('\n');
                            break;
                        }
                    }
                    continue;
                },
                Some('*') => {
                    chars.next();
                    while let Some(c2) = chars.next() {
                        if c2 == '*' && chars.peek() == Some(&'/') {
                            chars.next();
                            break;
                        }
                    }
                    continue;
                },
                _ => {},
            }
        }
        out.push(c);
    }
    out
}

// ── Shared helpers ─────────────────────────────────────────────────────────

fn resolve_root(explicit: Option<PathBuf>) -> miette::Result<PathBuf> {
    if let Some(p) = explicit {
        return Ok(p);
    }
    std::env::current_dir().into_diagnostic().wrap_err("failed to read current working directory")
}

fn write_report(output: Option<&Path>, report: &Value) -> miette::Result<()> {
    let json = serde_json::to_string_pretty(report)
        .into_diagnostic()
        .wrap_err("failed to serialise report")?;

    // None or "-" → stdout; anything else → write to that path.
    let to_stdout = match output {
        None => true,
        Some(p) => p == Path::new("-"),
    };
    if to_stdout {
        println!("{json}");
        return Ok(());
    }

    let path = output.unwrap();
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .into_diagnostic()
                .wrap_err_with(|| format!("failed to create {}", parent.display()))?;
        }
    }
    std::fs::write(path, &json)
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to write {}", path.display()))?;
    eprintln!("report: {}", path.display());
    Ok(())
}

/// Minimal ISO-8601 timestamp without pulling chrono. Uses SystemTime UNIX
/// seconds and formats them as UTC; sufficient for report metadata.
fn chrono_iso8601_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    // Conservative: emit Unix epoch seconds as `@<secs>` plus an RFC3339 UTC
    // expansion. We compute UTC manually (no leap-second precision) to avoid
    // adding a dep for one timestamp.
    let (year, month, day, hour, minute, second) = unix_to_ymd_hms(secs);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// Convert UNIX seconds → (year, month, day, hour, minute, second) UTC.
/// Algorithm: Howard Hinnant's civil_from_days (public domain).
fn unix_to_ymd_hms(secs: u64) -> (i64, u32, u32, u32, u32, u32) {
    let days = (secs / 86_400) as i64;
    let secs_of_day = (secs % 86_400) as u32;
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = if m <= 2 { y + 1 } else { y };
    let hour = secs_of_day / 3600;
    let minute = (secs_of_day % 3600) / 60;
    let second = secs_of_day % 60;
    (year, m, d, hour, minute, second)
}
