// `cargo xtask install-mcp` — auto-build + auto-install the unified
// `aphrody-mcp` Rust binary.
//
// Idempotent: rebuilds + redeploys only when any source file under
// `crates/google_mcp/src/` (or its `Cargo.toml`) is **newer** than the
// installed `~/.local/bin/aphrody-mcp[.exe]` binary. Safe to invoke
// from a hot path (SessionStart, PostToolUse) — the no-op case is a
// handful of `metadata()` calls (sub-millisecond) and a fast exit.
//
// Cross-platform :
//   - Linux / macOS  → `${HOME}/.local/bin/aphrody-mcp`
//   - Windows        → `%USERPROFILE%\.local\bin\aphrody-mcp.exe`
//
// Designed for the `.claude/plugins/aphrody/hooks/hooks.json` wiring.
// Pure Rust (no shell / bash / pwsh dependency).

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::SystemTime,
};

use anyhow::{Context, Result, anyhow, bail};
use clap::Parser;

#[derive(Debug, Parser)]
pub(crate) struct Args {
    /// Force a rebuild + reinstall even when the binary is up-to-date.
    #[arg(long)]
    pub force: bool,

    /// Suppress all stdout/stderr on the no-op path (default ON for hook
    /// usage). Disable with `--no-quiet` to surface the build trace.
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    pub quiet: bool,
}

pub(crate) fn run(args: Args) -> Result<()> {
    let repo_root = repo_root()?;
    let exe_name = exe_name();
    let install_dir = install_dir()?;
    fs::create_dir_all(&install_dir)
        .with_context(|| format!("create install dir {}", install_dir.display()))?;
    let install_path = install_dir.join(&exe_name);

    // 1. Up-to-date check.
    let reason = needs_rebuild(&repo_root, &install_path, args.force)?;
    let Some(reason) = reason else {
        // Silent no-op — keep hook noise minimal.
        if !args.quiet {
            println!("[install-mcp] up-to-date ({})", install_path.display());
        }
        return Ok(());
    };

    eprintln!("[install-mcp] rebuild trigger: {reason}");

    // 2. Rebuild — RUSTC_WRAPPER unset because sccache was dropping
    //    mid-build connections on this host (cf. 2026-05-19 session log).
    let status = Command::new("cargo")
        .args([
            "build",
            "--release",
            "-p",
            "google_mcp",
            "--bin",
            "aphrody-mcp",
            "--locked",
        ])
        .env_remove("RUSTC_WRAPPER")
        .current_dir(&repo_root)
        .status()
        .context("spawn `cargo build`")?;

    if !status.success() {
        bail!("cargo build -p google_mcp failed (exit {status})");
    }

    // 3. Locate the freshly-built binary across the standard Rust triples
    //    and promote it to ~/.local/bin/.
    let triples = [
        "x86_64-pc-windows-msvc",
        "x86_64-unknown-linux-gnu",
        "aarch64-unknown-linux-gnu",
        "aarch64-apple-darwin",
        "x86_64-apple-darwin",
    ];

    for triple in triples {
        let candidate = repo_root.join("target").join(triple).join("release").join(&exe_name);
        if candidate.is_file() {
            fs::copy(&candidate, &install_path)
                .with_context(|| format!("copy {} → {}", candidate.display(), install_path.display()))?;
            eprintln!(
                "[install-mcp] installed: {} → {}",
                candidate.display(),
                install_path.display()
            );
            return Ok(());
        }
    }

    // Fallback to the default target dir (no triple).
    let candidate = repo_root.join("target").join("release").join(&exe_name);
    if candidate.is_file() {
        fs::copy(&candidate, &install_path).with_context(|| {
            format!("copy {} → {}", candidate.display(), install_path.display())
        })?;
        eprintln!(
            "[install-mcp] installed: {} → {}",
            candidate.display(),
            install_path.display()
        );
        return Ok(());
    }

    bail!("built binary not found under target/ — expected aphrody-mcp[.exe]");
}

fn exe_name() -> String {
    if cfg!(windows) { "aphrody-mcp.exe".to_string() } else { "aphrody-mcp".to_string() }
}

fn install_dir() -> Result<PathBuf> {
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .ok_or_else(|| anyhow!("neither $HOME nor %USERPROFILE% is set"))?;
    Ok(PathBuf::from(home).join(".local").join("bin"))
}

/// Walks the workspace `Cargo.toml` upward from CWD to locate the repo root.
/// Stops at the first directory containing `[workspace]` in `Cargo.toml`.
fn repo_root() -> Result<PathBuf> {
    let cwd = std::env::current_dir().context("current_dir")?;
    let mut dir = cwd.as_path();
    loop {
        let cargo = dir.join("Cargo.toml");
        if cargo.is_file() {
            let body = fs::read_to_string(&cargo).unwrap_or_default();
            if body.contains("[workspace]") {
                return Ok(dir.to_path_buf());
            }
        }
        let Some(parent) = dir.parent() else {
            bail!("could not locate workspace root from {}", cwd.display());
        };
        dir = parent;
    }
}

/// Returns `Some(reason)` when a rebuild is needed, `None` when the
/// installed binary is fresher than every relevant source input.
fn needs_rebuild(repo_root: &Path, install_path: &Path, force: bool) -> Result<Option<String>> {
    if force {
        return Ok(Some("--force flag set".to_string()));
    }

    let installed_mtime = match install_path.metadata() {
        Ok(m) => m.modified().context("installed binary mtime")?,
        Err(_) => {
            return Ok(Some(format!("binary missing at {}", install_path.display())));
        }
    };

    let src_dir = repo_root.join("crates").join("google_mcp").join("src");
    let manifest = repo_root.join("crates").join("google_mcp").join("Cargo.toml");

    if let Some(reason) = newer_than(&manifest, installed_mtime)? {
        return Ok(Some(reason));
    }

    for entry in walk_rs_files(&src_dir)? {
        if let Some(reason) = newer_than(&entry, installed_mtime)? {
            return Ok(Some(reason));
        }
    }

    Ok(None)
}

fn newer_than(path: &Path, threshold: SystemTime) -> Result<Option<String>> {
    let Ok(meta) = path.metadata() else {
        return Ok(None);
    };
    let mtime = meta.modified().context("path mtime")?;
    if mtime > threshold {
        Ok(Some(format!("{} newer than installed binary", path.display())))
    } else {
        Ok(None)
    }
}

/// Iterative DFS yielding every `*.rs` file under `root`. Avoids the
/// `walkdir` crate to keep this single module light (and `aphrody-xtask`
/// already vendors `walkdir`, but a small custom walker is enough here).
fn walk_rs_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let ft = match entry.file_type() {
                Ok(t) => t,
                Err(_) => continue,
            };
            if ft.is_dir() {
                stack.push(path);
            } else if ft.is_file()
                && path.extension().and_then(|s| s.to_str()) == Some("rs")
            {
                out.push(path);
            }
        }
    }
    Ok(out)
}
