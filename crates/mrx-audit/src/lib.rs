//! mrx-audit — native Rust monorepo audit.
//!
//! Single `run()` performs, in this order:
//!   1. Parse `.gitmodules` + enrich via `git submodule status`.
//!   2. Detect root kind via `mrx_detect::detect_root` (Bun? Turbo? pnpm?).
//!   3. Walk `apps/` + `packages/` in parallel using `ignore::WalkBuilder`
//!      (ripgrep's engine — respects .gitignore, .ignore, global excludes).
//!   4. Per-file in parallel via rayon:
//!        - pattern matches (`/home/ubuntu`, `/var/www`, `../../../../`),
//!        - language detection by extension,
//!        - size aggregation,
//!        - per-workspace stat bucketing.
//!   5. Compute a blake3 content hash of root config files
//!      (turbo.json, package.json, bun.lock, Cargo.toml, .gitmodules, …).
//!   6. Atomically write `path.json` + `monorepo-map.json` (.tmp + rename).

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use chrono::Utc;
use dashmap::DashMap;
use globset::GlobSetBuilder;
use ignore::{DirEntry, WalkBuilder, WalkState};
use parking_lot::Mutex;
use serde::Deserialize;

use mrx_core::*;

const SCAN_DIRS: &[&str] = &["apps", "packages"];
const IGNORE_SEGMENTS: &[&str] = &[
    "node_modules",
    ".next",
    ".turbo",
    ".bun-cache",
    "target",
    "dist",
    "build",
    ".git",
    ".cache",
];

/// Root-level config files that feed the blake3 monorepo content hash.
/// Mirror Turborepo's notion of "if these change, downstream cache busts".
const ROOT_HASH_FILES: &[&str] = &[
    "turbo.json",
    "turbo.jsonc",
    "package.json",
    "bun.lock",
    "bun.lockb",
    "package-lock.json",
    "pnpm-lock.yaml",
    "pnpm-workspace.yaml",
    "yarn.lock",
    "lerna.json",
    "nx.json",
    "deno.json",
    "deno.lock",
    "Cargo.toml",
    "Cargo.lock",
    ".gitmodules",
];

fn lang_for(ext: &str) -> Option<&'static str> {
    match ext {
        "ts" | "tsx" | "mts" | "cts" => Some("TypeScript"),
        "js" | "jsx" | "mjs" | "cjs" => Some("JavaScript"),
        "rs" => Some("Rust"),
        "py" => Some("Python"),
        "go" => Some("Go"),
        "sh" | "bash" => Some("Shell"),
        "toml" => Some("TOML"),
        "yaml" | "yml" => Some("YAML"),
        "json" | "jsonc" => Some("JSON"),
        "md" | "mdx" => Some("Markdown"),
        "css" | "scss" => Some("CSS"),
        "html" | "htm" => Some("HTML"),
        "svelte" => Some("Svelte"),
        "vue" => Some("Vue"),
        "sql" => Some("SQL"),
        _ => None,
    }
}

#[derive(Debug, Default)]
struct WorkspaceAcc {
    kind: Option<WorkspaceKind>,
    name: Option<String>,
    version: Option<String>,
    file_count: usize,
    bytes: u64,
    languages: BTreeMap<String, LangStat>,
}

#[derive(Debug, Default)]
struct ScanAccumulator {
    hardcoded_home: Mutex<Vec<String>>,
    hardcoded_var_www: Mutex<Vec<String>>,
    deep_relative: Mutex<Vec<String>>,
    total_files: std::sync::atomic::AtomicUsize,
    total_bytes: std::sync::atomic::AtomicU64,
    languages: DashMap<String, LangStat>,
    workspaces: DashMap<String, WorkspaceAcc>,
}

pub fn run(root: &Path, audit_out: &Path, map_out: &Path) -> Result<RunResult> {
    let started = Instant::now();
    let root = dunce::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let now_utc = Utc::now();
    let now = now_utc.format("%Y-%m-%dT%H:%MZ").to_string();
    let today = now_utc.format("%Y-%m-%d").to_string();
    let os = std::env::consts::OS.to_string();
    let host = hostname();

    let submodules = parse_submodules(&root).unwrap_or_default();
    tracing::debug!("parsed {} submodules", submodules.len());

    let root_kind = mrx_detect::detect_root(&root);
    let content_hash = compute_root_hash(&root);

    let acc = Arc::new(ScanAccumulator::default());
    let ignore_set = build_ignore_globset()?;

    for dir in SCAN_DIRS {
        let abs = root.join(dir);
        if !abs.is_dir() {
            continue;
        }
        let walker = WalkBuilder::new(&abs)
            .hidden(false)
            .git_ignore(true)
            .git_exclude(true)
            .git_global(true)
            .standard_filters(true)
            .threads(num_threads())
            .build_parallel();

        let acc_cl = Arc::clone(&acc);
        let ignore_set_cl = ignore_set.clone();
        let root_cl = root.clone();
        walker.run(move || {
            let acc = Arc::clone(&acc_cl);
            let ignore_set = ignore_set_cl.clone();
            let root = root_cl.clone();
            Box::new(move |result| {
                let entry = match result {
                    Ok(e) => e,
                    Err(_) => return WalkState::Continue,
                };
                if should_skip(&entry, &ignore_set) {
                    return WalkState::Continue;
                }
                if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                    process_file(&entry, &root, &acc);
                }
                WalkState::Continue
            })
        });
    }

    let total_files = acc.total_files.load(std::sync::atomic::Ordering::Relaxed);
    let total_bytes = acc.total_bytes.load(std::sync::atomic::Ordering::Relaxed);

    let mut findings: BTreeMap<String, FindingGroup> = BTreeMap::new();
    let abs_paths = std::mem::take(&mut *acc.hardcoded_home.lock());
    let var_www = std::mem::take(&mut *acc.hardcoded_var_www.lock());
    let deep_rel = std::mem::take(&mut *acc.deep_relative.lock());
    findings.insert(
        "absolute_paths".into(),
        FindingGroup {
            pattern: "/home/ubuntu".into(),
            status: if abs_paths.is_empty() {
                Status::ProductionReady
            } else {
                Status::Findings
            },
            matches: abs_paths,
        },
    );
    findings.insert(
        "system_paths".into(),
        FindingGroup {
            pattern: "/var/www".into(),
            status: if var_www.is_empty() {
                Status::ProductionReady
            } else {
                Status::Findings
            },
            matches: var_www,
        },
    );
    findings.insert(
        "fragile_relative_paths".into(),
        FindingGroup {
            pattern: "../../../../".into(),
            status: if deep_rel.is_empty() {
                Status::ProductionReady
            } else {
                Status::Findings
            },
            matches: deep_rel,
        },
    );

    let total_findings: usize = findings.values().map(|f| f.matches.len()).sum();
    let overall = if total_findings == 0 {
        Status::ProductionReady
    } else {
        Status::Findings
    };

    let audit = AuditReport {
        audit_name: "VPS Monorepo Path Hardening",
        status: overall,
        date: today.clone(),
        last_updated: now.clone(),
        generator: "mrx (rust)",
        scope: Scope {
            directories_audited: SCAN_DIRS.iter().map(|s| format!("vps/{s}")).collect(),
            excluded: IGNORE_SEGMENTS.iter().map(|s| s.to_string()).collect(),
            host: host.clone(),
            os: os.clone(),
        },
        findings,
        infrastructure_exceptions: InfraExceptions {
            note: "Absolute paths are intentionally preserved in infrastructure configuration files and documentation.".into(),
            directories: vec![
                "vps/infra/systemd".into(),
                "vps/infra/nginx".into(),
                "vps/docs".into(),
                "vps/CLAUDE.md".into(),
            ],
        },
        submodules: SubmoduleSection {
            tracked: submodules.clone(),
        },
        conclusion: if overall == Status::ProductionReady {
            "All source code (apps and packages) exclusively relies on workspace-aware utilities and environment variables for file resolution.".into()
        } else {
            format!("{total_findings} finding(s) detected — review `findings.*.matches` and remediate.")
        },
    };

    let mut languages: BTreeMap<String, LangStat> = BTreeMap::new();
    for entry in acc.languages.iter() {
        languages.insert(entry.key().clone(), entry.value().clone());
    }
    let mut workspaces: Vec<Workspace> = acc
        .workspaces
        .iter()
        .filter_map(|entry| {
            let path = entry.key().clone();
            let w = entry.value();
            let kind = w.kind?;
            let runtimes = mrx_detect::detect_workspace_runtimes(&root.join(&path));
            Some(Workspace {
                path,
                kind,
                runtimes,
                name: w.name.clone(),
                version: w.version.clone(),
                file_count: w.file_count,
                bytes: w.bytes,
                languages: w.languages.clone(),
            })
        })
        .collect();
    workspaces.sort_by(|a, b| a.path.cmp(&b.path));

    let duration = started.elapsed();
    let map = MonorepoMap {
        generated_at: now.clone(),
        root: root.display().to_string(),
        host: host.clone(),
        os: os.clone(),
        root_kind,
        content_hash,
        stats: MapStats {
            total_files,
            total_workspaces: workspaces.len(),
            total_submodules: submodules.len(),
            bytes_scanned: total_bytes,
            scan_duration_ms: duration.as_millis(),
            languages,
        },
        workspaces,
        submodules: submodules.clone(),
    };

    write_atomic(audit_out, &audit)?;
    write_atomic(map_out, &map)?;

    Ok(RunResult {
        status: overall,
        submodules: submodules.len(),
        workspaces: map.stats.total_workspaces,
        total_findings,
        duration_ms: duration.as_millis(),
    })
}

/// blake3 hash of the concatenated content of root config files (in fixed order).
/// Missing files contribute their name only — so add/remove still busts the hash.
fn compute_root_hash(root: &Path) -> String {
    let mut hasher = blake3::Hasher::new();
    for name in ROOT_HASH_FILES {
        hasher.update(name.as_bytes());
        hasher.update(b"\0");
        if let Ok(bytes) = std::fs::read(root.join(name)) {
            hasher.update(&bytes);
        }
        hasher.update(b"\n");
    }
    hasher.finalize().to_hex().to_string()
}

fn num_threads() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

fn hostname() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .unwrap_or_else(|_| "unknown".to_string())
}

fn should_skip(entry: &DirEntry, ignore_set: &globset::GlobSet) -> bool {
    let path = entry.path();
    for c in path.components() {
        let s = c.as_os_str().to_string_lossy();
        if IGNORE_SEGMENTS.iter().any(|p| *p == s) {
            return true;
        }
    }
    if ignore_set.is_match(path) {
        return true;
    }
    false
}

fn build_ignore_globset() -> Result<globset::GlobSet> {
    let mut b = GlobSetBuilder::new();
    for pat in [
        "**/*.log",
        "**/*.tmp",
        "**/*.swp",
        "**/*.swx",
        "**/*.lock",
        "**/*.min.js",
        "**/*.min.css",
    ] {
        b.add(globset::Glob::new(pat)?);
    }
    Ok(b.build()?)
}

#[derive(Deserialize)]
struct PkgJson {
    name: Option<String>,
    version: Option<String>,
}

fn process_file(entry: &DirEntry, root: &Path, acc: &ScanAccumulator) {
    use std::sync::atomic::Ordering;

    let path = entry.path();
    let rel = path.strip_prefix(root).unwrap_or(path);
    let rel_str = rel.display().to_string();

    let meta = match entry.metadata() {
        Ok(m) => m,
        Err(_) => return,
    };
    let size = meta.len();
    acc.total_files.fetch_add(1, Ordering::Relaxed);
    acc.total_bytes.fetch_add(size, Ordering::Relaxed);

    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if let Some(lang) = lang_for(ext) {
            let mut entry = acc.languages.entry(lang.to_string()).or_default();
            entry.files += 1;
            entry.bytes += size;
        }
    }

    let file_name = path.file_name().and_then(|n| n.to_str());
    if let Some(name) = file_name {
        let ws_root = path.parent().and_then(|p| p.strip_prefix(root).ok());
        if let Some(ws_rel) = ws_root {
            let ws_key = ws_rel.display().to_string();
            match name {
                "package.json" => {
                    if let Some((n, v)) = parse_pkg_json(path) {
                        let mut e = acc.workspaces.entry(ws_key).or_default();
                        e.name = e.name.clone().or(n);
                        e.version = e.version.clone().or(v);
                        e.kind = Some(match e.kind {
                            Some(WorkspaceKind::Rust) => WorkspaceKind::Hybrid,
                            _ => WorkspaceKind::Node,
                        });
                    }
                }
                "Cargo.toml" => {
                    if let Some((n, v)) = parse_cargo_toml(path) {
                        let mut e = acc.workspaces.entry(ws_key).or_default();
                        e.name = e.name.clone().or(n);
                        e.version = e.version.clone().or(v);
                        e.kind = Some(match e.kind {
                            Some(WorkspaceKind::Node) => WorkspaceKind::Hybrid,
                            _ => WorkspaceKind::Rust,
                        });
                    }
                }
                _ => {}
            }
        }
    }

    if let Some(ws_key) = workspace_key(rel) {
        let mut e = acc.workspaces.entry(ws_key).or_default();
        e.file_count += 1;
        e.bytes += size;
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if let Some(lang) = lang_for(ext) {
                let l = e.languages.entry(lang.to_string()).or_default();
                l.files += 1;
                l.bytes += size;
            }
        }
    }

    let is_textual = match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => matches!(
            ext,
            "ts" | "tsx"
                | "mts"
                | "cts"
                | "js"
                | "jsx"
                | "mjs"
                | "cjs"
                | "rs"
                | "py"
                | "go"
                | "sh"
                | "bash"
                | "toml"
                | "yaml"
                | "yml"
                | "json"
                | "jsonc"
                | "md"
                | "mdx"
                | "html"
                | "htm"
                | "svelte"
                | "vue"
                | "sql"
                | "css"
                | "scss"
        ),
        None => false,
    };
    if !is_textual || size > 1_000_000 {
        return;
    }

    let mut buf = String::new();
    if let Ok(mut f) = File::open(path) {
        if f.read_to_string(&mut buf).is_err() {
            return;
        }
    } else {
        return;
    }

    if buf.contains("/home/ubuntu") {
        acc.hardcoded_home.lock().push(rel_str.clone());
    }
    if buf.contains("/var/www") {
        acc.hardcoded_var_www.lock().push(rel_str.clone());
    }
    if buf.contains("../../../../") {
        acc.deep_relative.lock().push(rel_str);
    }
}

fn workspace_key(rel: &Path) -> Option<String> {
    let mut comps = rel.components();
    let top = comps.next()?.as_os_str().to_string_lossy().into_owned();
    if top != "apps" && top != "packages" {
        return None;
    }
    let name = comps.next()?.as_os_str().to_string_lossy().into_owned();
    Some(format!("{top}/{name}"))
}

fn parse_pkg_json(path: &Path) -> Option<(Option<String>, Option<String>)> {
    let s = std::fs::read_to_string(path).ok()?;
    let p: PkgJson = serde_json::from_str(&s).ok()?;
    Some((p.name, p.version))
}

fn parse_cargo_toml(path: &Path) -> Option<(Option<String>, Option<String>)> {
    let s = std::fs::read_to_string(path).ok()?;
    let mut name = None;
    let mut version = None;
    let mut in_package = false;
    for line in s.lines().take(120) {
        let t = line.trim();
        if t.starts_with('[') {
            in_package = t == "[package]";
            continue;
        }
        if !in_package {
            continue;
        }
        if let Some(rest) = t.strip_prefix("name") {
            if let Some(v) = parse_string_assign(rest) {
                name = Some(v);
            }
        } else if let Some(rest) = t.strip_prefix("version") {
            if let Some(v) = parse_string_assign(rest) {
                version = Some(v);
            }
        }
    }
    Some((name, version))
}

fn parse_string_assign(rest: &str) -> Option<String> {
    let rest = rest.trim_start();
    if !rest.starts_with('=') {
        return None;
    }
    let rest = rest[1..].trim_start();
    let bytes = rest.as_bytes();
    if bytes.first() != Some(&b'"') {
        return None;
    }
    let end = rest[1..].find('"')?;
    Some(rest[1..1 + end].to_string())
}

fn parse_submodules(root: &Path) -> Result<Vec<Submodule>> {
    let gm = root.join(".gitmodules");
    if !gm.exists() {
        return Ok(vec![]);
    }
    let s = std::fs::read_to_string(&gm)?;

    let mut out = Vec::new();
    let mut current_path: Option<String> = None;
    let mut current_url: Option<String> = None;

    let flush = |out: &mut Vec<Submodule>, path: &mut Option<String>, url: &mut Option<String>| {
        if let (Some(p), Some(u)) = (path.take(), url.take()) {
            out.push(Submodule {
                path: p,
                url: u,
                sha: None,
                pinned: None,
                added: None,
            });
        }
    };

    for line in s.lines() {
        let t = line.trim();
        if t.starts_with("[submodule") {
            flush(&mut out, &mut current_path, &mut current_url);
        } else if let Some(rest) = t.strip_prefix("path") {
            if let Some(v) = parse_string_or_value(rest) {
                current_path = Some(v);
            }
        } else if let Some(rest) = t.strip_prefix("url") {
            if let Some(v) = parse_string_or_value(rest) {
                current_url = Some(v);
            }
        }
    }
    flush(&mut out, &mut current_path, &mut current_url);

    if let Ok(output) = std::process::Command::new("git")
        .arg("submodule")
        .arg("status")
        .current_dir(root)
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut by_path: BTreeMap<String, (String, Option<String>)> = BTreeMap::new();
            for line in stdout.lines() {
                let trimmed = line.trim_start_matches([' ', '+', '-']);
                let mut parts = trimmed.splitn(2, ' ');
                let sha = parts.next().unwrap_or("").to_string();
                let rest = parts.next().unwrap_or("");
                let (path_part, pinned) = match rest.find(" (") {
                    Some(i) => (
                        rest[..i].to_string(),
                        Some(rest[i + 2..].trim_end_matches(')').to_string()),
                    ),
                    None => (rest.to_string(), None),
                };
                by_path.insert(path_part, (sha, pinned));
            }
            for sm in out.iter_mut() {
                if let Some((sha, pinned)) = by_path.get(&sm.path) {
                    sm.sha = Some(sha.clone());
                    sm.pinned = pinned.clone();
                }
            }
        }
    }

    Ok(out)
}

fn parse_string_or_value(rest: &str) -> Option<String> {
    let rest = rest.trim_start();
    if !rest.starts_with('=') {
        return None;
    }
    Some(rest[1..].trim().to_string())
}

fn write_atomic<T: serde::Serialize>(out: &Path, value: &T) -> Result<()> {
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let tmp = out.with_extension("json.tmp");
    let f = File::create(&tmp).with_context(|| format!("create {}", tmp.display()))?;
    let mut w = BufWriter::new(f);
    serde_json::to_writer_pretty(&mut w, value)?;
    w.write_all(b"\n")?;
    w.flush()?;
    std::fs::rename(&tmp, out).with_context(|| format!("rename -> {}", out.display()))?;
    Ok(())
}
