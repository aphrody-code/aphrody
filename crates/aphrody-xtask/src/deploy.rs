// `cargo xtask deploy` — build every workspace binary in release mode and
// install the resulting executables into `~/.local/bin/`.
//
// Rationale: per memory `project_aphrody_install_convention`, the canonical
// Windows install path is `%USERPROFILE%\.local\bin\` (NOT HKCU PATH
// edition via `aphrody self install-path`). This task automates the build
// + copy step that every workspace contributor would otherwise do manually
// after touching any binary crate.
//
// Discovery is dynamic via `cargo metadata --format-version 1` so newly
// added bins (notebooklm-tools, future a2a-server-bin, etc.) are picked
// up without editing this file.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use serde::Deserialize;

#[derive(Debug, Parser)]
pub(crate) struct Args {
    /// Skip the `cargo build` step — only copy already-built artefacts.
    /// Useful when the caller chained `cargo build && cargo xtask deploy`.
    #[arg(long)]
    pub no_build: bool,

    /// Restrict to binaries whose name starts with one of these prefixes
    /// (comma-separated). Default = `aphrody,bxc,mrx,notebooklm` (all the
    /// user-facing CLIs aphrody ships).
    #[arg(long, default_value = "aphrody,bxc,mrx,notebooklm")]
    pub prefixes: String,

    /// Override the install root. Defaults to `$HOME/.local/bin` (or
    /// `%USERPROFILE%\.local\bin` on Windows).
    #[arg(long)]
    pub dest: Option<PathBuf>,

    /// Target triple to use for the build (passed verbatim to `cargo build
    /// --target <triple>`). Default = host triple via `rustc -vV`.
    #[arg(long)]
    pub target: Option<String>,

    /// Pretend mode: print the planned actions without copying anything.
    #[arg(long)]
    pub dry_run: bool,

    /// Skip the Claude Code plugin install / auto-update step (default ON).
    /// When enabled, this refreshes `~/.claude/plugins/known_marketplaces.json`
    /// + `installed_plugins.json` so the `aphrody` plugin is always registered
    /// at the version read from `plugin.json`.
    #[arg(long)]
    pub no_install_plugin: bool,
}

pub(crate) fn run(args: Args) -> Result<()> {
    let prefixes: Vec<&str> =
        args.prefixes.split(',').map(str::trim).filter(|s| !s.is_empty()).collect();
    if prefixes.is_empty() {
        bail!("--prefixes must contain at least one non-empty entry");
    }

    let metadata = read_workspace_metadata()?;
    let bins = collect_matching_bins(&metadata, &prefixes);
    if bins.is_empty() {
        bail!(
            "no binary target in the workspace matched any prefix in {:?}",
            prefixes
        );
    }

    let target_triple = match args.target.clone() {
        Some(t) => t,
        None => host_triple()?,
    };

    if !args.no_build {
        build_release(&bins, &target_triple)?;
    }

    let release_dirs = release_search_paths(&metadata.target_directory, &target_triple);
    let dest_root = resolve_dest(args.dest.as_deref())?;
    if !args.dry_run {
        std::fs::create_dir_all(&dest_root)
            .with_context(|| format!("failed to create destination {}", dest_root.display()))?;
    }

    let mut deployed = Vec::with_capacity(bins.len());
    let mut missing = Vec::new();
    let mut locked = Vec::new();
    for bin in &bins {
        match locate_binary(bin, &release_dirs)? {
            Some(src) => {
                let dst = dest_root.join(file_name_with_ext(bin));
                if args.dry_run {
                    tracing::info!("[dry-run] would copy {} -> {}", src.display(), dst.display());
                } else {
                    match copy_with_fallback(&src, &dst) {
                        Ok(()) => {
                            let size = std::fs::metadata(&dst).map(|m| m.len()).unwrap_or(0);
                            deployed.push((bin.clone(), dst, size));
                        },
                        Err(CopyError::Locked) => locked.push((bin.clone(), dst)),
                        Err(CopyError::Other(e)) => {
                            return Err(e).with_context(|| {
                                format!("failed to copy {} -> {}", src.display(), bin)
                            });
                        },
                    }
                }
            },
            None => missing.push(bin.clone()),
        }
    }

    if !deployed.is_empty() {
        println!("=== aphrody deploy : {} binar{} installé{} ===", deployed.len(),
            if deployed.len() > 1 { "ies" } else { "y" },
            if deployed.len() > 1 { "s" } else { "" });
        for (name, dst, size) in &deployed {
            println!("  [ok] {:<32} -> {} ({} bytes)", name, dst.display(), size);
        }
    }
    if !missing.is_empty() {
        println!("=== {} binar{} introuvable{} (build manqué ou nom différent) ===",
            missing.len(),
            if missing.len() > 1 { "ies" } else { "y" },
            if missing.len() > 1 { "s" } else { "" });
        for name in &missing {
            println!("  [skip] {}", name);
        }
    }
    if !locked.is_empty() {
        println!("=== {} binar{} verrouillé{} (process actif — kill puis re-deploy) ===",
            locked.len(),
            if locked.len() > 1 { "ies" } else { "y" },
            if locked.len() > 1 { "s" } else { "" });
        for (name, dst) in &locked {
            println!("  [locked] {:<32} -> {} (en cours d'exécution)", name, dst.display());
        }
    }

    if !args.dry_run {
        ensure_dest_in_path(&dest_root);
    }

    if !args.no_install_plugin {
        match install_or_update_plugin(args.dry_run) {
            Ok(report) => println!("{report}"),
            Err(e) => eprintln!("[warn] plugin install/update skipped: {e:#}"),
        }
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
struct CargoMetadata {
    packages: Vec<CargoPackage>,
    target_directory: PathBuf,
    workspace_members: Vec<String>,
    workspace_root: PathBuf,
}

#[derive(Debug, Deserialize)]
struct CargoPackage {
    id: String,
    targets: Vec<CargoTarget>,
}

#[derive(Debug, Deserialize)]
struct CargoTarget {
    name: String,
    kind: Vec<String>,
}

fn read_workspace_metadata() -> Result<CargoMetadata> {
    let out = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .output()
        .context("failed to spawn `cargo metadata`")?;
    if !out.status.success() {
        bail!(
            "`cargo metadata` exited with status {}: {}",
            out.status,
            String::from_utf8_lossy(&out.stderr)
        );
    }
    serde_json::from_slice::<CargoMetadata>(&out.stdout)
        .context("failed to parse `cargo metadata` JSON output")
}

fn collect_matching_bins(meta: &CargoMetadata, prefixes: &[&str]) -> Vec<String> {
    let workspace: BTreeSet<&String> = meta.workspace_members.iter().collect();
    let mut bins: BTreeSet<String> = BTreeSet::new();
    for pkg in &meta.packages {
        if !workspace.contains(&pkg.id) {
            continue;
        }
        for tgt in &pkg.targets {
            let is_bin = tgt.kind.iter().any(|k| k == "bin");
            if !is_bin {
                continue;
            }
            if prefixes.iter().any(|p| tgt.name.starts_with(p)) {
                bins.insert(tgt.name.clone());
            }
        }
    }
    bins.into_iter().collect()
}

fn build_release(bins: &[String], target: &str) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.arg("build").arg("--release").arg("--locked").arg("--target").arg(target);
    for name in bins {
        cmd.arg("--bin").arg(name);
    }
    tracing::info!("building {} bin(s) for {}", bins.len(), target);
    let status = cmd.status().context("failed to spawn `cargo build`")?;
    if !status.success() {
        bail!("`cargo build --release` exited with status {}", status);
    }
    Ok(())
}

fn release_search_paths(target_dir: &Path, triple: &str) -> Vec<PathBuf> {
    vec![
        target_dir.join(triple).join("release"),
        target_dir.join("release"),
    ]
}

fn locate_binary(name: &str, dirs: &[PathBuf]) -> Result<Option<PathBuf>> {
    let fname = file_name_with_ext(name);
    for dir in dirs {
        let candidate = dir.join(&fname);
        if candidate.is_file() {
            return Ok(Some(candidate));
        }
    }
    Ok(None)
}

fn file_name_with_ext(name: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

enum CopyError {
    Locked,
    Other(anyhow::Error),
}

fn copy_with_fallback(src: &Path, dst: &Path) -> Result<(), CopyError> {
    // Try direct copy first.
    match std::fs::copy(src, dst) {
        Ok(_) => Ok(()),
        Err(e) if is_sharing_violation(&e) => {
            // Windows: target is running. Try atomic temp+rename — the OS lets
            // us rename-over a locked file as long as no one has an open handle
            // *to the new name* (the running process holds the old name).
            let tmp = dst.with_extension("xtask-new");
            if std::fs::copy(src, &tmp).is_err() {
                return Err(CopyError::Locked);
            }
            match std::fs::rename(&tmp, dst) {
                Ok(()) => Ok(()),
                Err(_) => {
                    let _ = std::fs::remove_file(&tmp);
                    Err(CopyError::Locked)
                },
            }
        },
        Err(e) => Err(CopyError::Other(e.into())),
    }
}

fn is_sharing_violation(e: &std::io::Error) -> bool {
    // Windows "sharing violation" maps to ErrorKind::PermissionDenied (kind)
    // and raw OS error 32 (ERROR_SHARING_VIOLATION). On Unix this almost
    // never happens for ordinary file overwrites; we still treat
    // PermissionDenied as "locked" so the loop can keep going.
    matches!(e.kind(), std::io::ErrorKind::PermissionDenied)
        || e.raw_os_error() == Some(32)
}

// ---------------------------------------------------------------------------
// Claude Code plugin install / auto-update
// ---------------------------------------------------------------------------

/// Refreshes the marketplace + installed_plugins registries so the local
/// `aphrody` plugin is always pinned to the version read from the manifest.
/// Idempotent: re-running upserts entries instead of duplicating them.
fn install_or_update_plugin(dry_run: bool) -> Result<String> {
    let meta = read_workspace_metadata()?;
    let plugin_dir = meta.workspace_root.join(".claude").join("plugins").join("aphrody");
    let manifest_path = plugin_dir.join(".claude-plugin").join("plugin.json");
    let manifest = read_plugin_manifest(&manifest_path)
        .with_context(|| format!("failed to read {}", manifest_path.display()))?;

    let marketplace_root = meta.workspace_root.join(".claude").join("plugins");
    let marketplace_manifest = marketplace_root.join(".claude-plugin").join("marketplace.json");
    let now = iso8601_now();
    let marketplace_name = "aphrody-local";

    if dry_run {
        return Ok(format!(
            "=== [dry-run] plugin {}@{} v{} (marketplace {}) would be refreshed ===\n  \
             - marketplace manifest: {}\n  \
             - known_marketplaces.json: $HOME/.claude/plugins/known_marketplaces.json\n  \
             - installed_plugins.json: $HOME/.claude/plugins/installed_plugins.json",
            manifest.name,
            marketplace_name,
            manifest.version,
            marketplace_name,
            marketplace_manifest.display(),
        ));
    }

    // 1) Ensure the in-repo marketplace.json reflects the plugin manifest.
    write_marketplace_index(
        &marketplace_manifest,
        marketplace_name,
        &manifest,
    )?;

    // 2) Resolve the user-global Claude Code plugin store.
    let claude_plugins_dir = resolve_claude_plugins_dir()?;
    let known_path = claude_plugins_dir.join("known_marketplaces.json");
    let installed_path = claude_plugins_dir.join("installed_plugins.json");
    std::fs::create_dir_all(&claude_plugins_dir).with_context(|| {
        format!("failed to mkdir {}", claude_plugins_dir.display())
    })?;

    upsert_known_marketplace(&known_path, marketplace_name, &marketplace_root, &now)?;
    upsert_installed_plugin(
        &installed_path,
        &manifest.name,
        marketplace_name,
        &plugin_dir,
        &manifest.version,
        &now,
    )?;

    Ok(format!(
        "=== plugin {}@{} v{} (marketplace {}) refreshed ===\n  \
         [ok] {}\n  \
         [ok] {}\n  \
         [ok] {}",
        manifest.name,
        marketplace_name,
        manifest.version,
        marketplace_name,
        marketplace_manifest.display(),
        known_path.display(),
        installed_path.display(),
    ))
}

#[derive(Debug, Deserialize)]
struct PluginManifest {
    name: String,
    #[serde(default = "default_version")]
    version: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    author: Option<serde_json::Value>,
    #[serde(default)]
    homepage: Option<String>,
    #[serde(default)]
    category: Option<String>,
}

fn default_version() -> String {
    "0.0.0".to_string()
}

fn read_plugin_manifest(path: &Path) -> Result<PluginManifest> {
    let body =
        std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    serde_json::from_str(&body).with_context(|| format!("parse {}", path.display()))
}

fn write_marketplace_index(
    path: &Path,
    marketplace_name: &str,
    plugin: &PluginManifest,
) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let owner = plugin
        .author
        .clone()
        .unwrap_or_else(|| serde_json::json!({ "name": "aphrody-code" }));
    let description = plugin
        .description
        .clone()
        .unwrap_or_else(|| format!("Local marketplace for the {} plugin.", plugin.name));
    let body = serde_json::json!({
        "$schema": "https://anthropic.com/claude-code/marketplace.schema.json",
        "name": marketplace_name,
        "description": format!(
            "Local marketplace for the {} plugin (development install).", plugin.name
        ),
        "owner": owner,
        "plugins": [
            {
                "name": plugin.name,
                "description": description,
                "author": owner,
                "category": plugin.category.clone().unwrap_or_else(|| "productivity".to_string()),
                "source": format!("./{}", plugin.name),
                "homepage": plugin.homepage.clone().unwrap_or_default(),
            }
        ]
    });
    write_json_pretty(path, &body)
}

fn upsert_known_marketplace(
    path: &Path,
    marketplace_name: &str,
    marketplace_root: &Path,
    now: &str,
) -> Result<()> {
    let mut doc: serde_json::Map<String, serde_json::Value> = match std::fs::read_to_string(path) {
        Ok(s) => serde_json::from_str(&s)
            .with_context(|| format!("parse {}", path.display()))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => serde_json::Map::new(),
        Err(e) => return Err(e).with_context(|| format!("read {}", path.display())),
    };
    doc.insert(
        marketplace_name.to_string(),
        serde_json::json!({
            "source": {
                "source": "directory",
                "path": marketplace_root.to_string_lossy(),
            },
            "installLocation": marketplace_root.to_string_lossy(),
            "lastUpdated": now,
        }),
    );
    write_json_pretty(path, &serde_json::Value::Object(doc))
}

fn upsert_installed_plugin(
    path: &Path,
    plugin_name: &str,
    marketplace_name: &str,
    install_path: &Path,
    version: &str,
    now: &str,
) -> Result<()> {
    let mut doc: serde_json::Value = match std::fs::read_to_string(path) {
        Ok(s) => serde_json::from_str(&s)
            .with_context(|| format!("parse {}", path.display()))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => serde_json::json!({
            "version": 2,
            "plugins": {}
        }),
        Err(e) => return Err(e).with_context(|| format!("read {}", path.display())),
    };

    let plugins = doc
        .get_mut("plugins")
        .and_then(|v| v.as_object_mut())
        .ok_or_else(|| anyhow!("{}: missing `plugins` object", path.display()))?;

    let key = format!("{plugin_name}@{marketplace_name}");
    let entries = plugins
        .entry(key.clone())
        .or_insert_with(|| serde_json::Value::Array(Vec::new()));
    let array = entries
        .as_array_mut()
        .ok_or_else(|| anyhow!("{}: `{}` is not an array", path.display(), key))?;

    let existing_pos = array.iter().position(|e| {
        e.get("scope").and_then(|s| s.as_str()) == Some("user")
            && e.get("installPath")
                .and_then(|p| p.as_str())
                .map(|p| canon_path_eq(p, install_path))
                .unwrap_or(false)
    });

    let installed_at = existing_pos
        .and_then(|i| array[i].get("installedAt").cloned())
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| now.to_string());

    let entry = serde_json::json!({
        "scope": "user",
        "installPath": install_path.to_string_lossy(),
        "version": version,
        "installedAt": installed_at,
        "lastUpdated": now,
    });

    if let Some(i) = existing_pos {
        array[i] = entry;
    } else {
        array.push(entry);
    }

    write_json_pretty(path, &doc)
}

fn canon_path_eq(a: &str, b: &Path) -> bool {
    let lhs = Path::new(a);
    let rhs: &Path = b;
    match (std::fs::canonicalize(lhs), std::fs::canonicalize(rhs)) {
        (Ok(l), Ok(r)) => l == r,
        _ => lhs == rhs,
    }
}

fn write_json_pretty(path: &Path, value: &serde_json::Value) -> Result<()> {
    let body = serde_json::to_string_pretty(value)
        .context("serialize JSON")?;
    std::fs::write(path, format!("{body}\n"))
        .with_context(|| format!("write {}", path.display()))
}

fn resolve_claude_plugins_dir() -> Result<PathBuf> {
    let home = std::env::var_os("CLAUDE_HOME")
        .or_else(|| std::env::var_os("HOME"))
        .or_else(|| std::env::var_os("USERPROFILE"))
        .ok_or_else(|| anyhow!("CLAUDE_HOME / HOME / USERPROFILE all unset"))?;
    Ok(PathBuf::from(home).join(".claude").join("plugins"))
}

fn iso8601_now() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

fn host_triple() -> Result<String> {
    let out = Command::new("rustc").arg("-vV").output().context("failed to spawn `rustc -vV`")?;
    if !out.status.success() {
        bail!("`rustc -vV` exited with status {}", out.status);
    }
    let text = String::from_utf8(out.stdout).context("`rustc -vV` returned non-UTF8")?;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("host: ") {
            return Ok(rest.trim().to_string());
        }
    }
    Err(anyhow!("could not parse `host:` line from rustc -vV output"))
}

fn resolve_dest(override_path: Option<&Path>) -> Result<PathBuf> {
    if let Some(p) = override_path {
        return Ok(p.to_path_buf());
    }
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .ok_or_else(|| anyhow!("neither $HOME nor %USERPROFILE% are set"))?;
    Ok(PathBuf::from(home).join(".local").join("bin"))
}

fn ensure_dest_in_path(dest: &Path) {
    let path_var = std::env::var_os("PATH").unwrap_or_default();
    let dest_canon = std::fs::canonicalize(dest).unwrap_or_else(|_| dest.to_path_buf());
    for entry in std::env::split_paths(&path_var) {
        if let Ok(canon) = std::fs::canonicalize(&entry) {
            if canon == dest_canon {
                return;
            }
        }
    }
    eprintln!(
        "[hint] {} n'est pas dans $PATH — ajoute-le pour invoquer les binaires \
         directement (Bash: `export PATH=\"$HOME/.local/bin:$PATH\"`, \
         PowerShell: `$env:PATH += \";$env:USERPROFILE\\.local\\bin\"`).",
        dest.display()
    );
}
