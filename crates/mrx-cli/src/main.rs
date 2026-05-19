// SPDX-License-Identifier: Apache-2.0
// mrx — cross-platform, serverless-friendly monorepo mapper + watcher.
//
// Sub-commands:
//   scan   — one-shot audit, writes path.json + monorepo-map.json, exits.
//            Use in CI, cron, Docker entrypoints, AWS Lambda, etc.
//   watch  — long-running daemon, re-runs scan on FS events (debounced).
//            Use on workstation / VPS for live updates.
//   check  — like `scan`, but exit non-zero when findings are detected.
//            Use as pre-commit / pre-push / CI gate.
//
// All modes write to <root>/path.json + <root>/monorepo-map.json by default,
// overridable with --out / --map.

#![forbid(unsafe_code)]

use std::{
    path::{Path, PathBuf},
    process::ExitCode,
};

use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(name = "mrx", version, about, long_about = None)]
struct Cli {
    /// Root of the monorepo. Defaults to $VPS_ROOT or $HOME/vps.
    #[arg(long, env = "VPS_ROOT", global = true)]
    root: Option<PathBuf>,

    /// Output path for the audit JSON. Default: <root>/path.json
    #[arg(long, global = true)]
    out: Option<PathBuf>,

    /// Output path for the full monorepo map JSON. Default: <root>/monorepo-map.json
    #[arg(long, global = true)]
    map: Option<PathBuf>,

    /// Emit logs as JSON (machine-readable).
    #[arg(long, global = true)]
    log_json: bool,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// One-shot audit + map (serverless-friendly).
    Scan,
    /// Long-running watcher (notify, debounced).
    Watch {
        /// Debounce window in milliseconds.
        #[arg(long, default_value_t = 1500)]
        debounce_ms: u64,
    },
    /// Like `scan`, but exit non-zero if findings are detected.
    Check,
}

fn init_tracing(json: bool) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("mrx_cli=info,mrx_audit=info,mrx_watch=info,warn"));
    if json {
        tracing_subscriber::fmt().json().with_env_filter(filter).init();
    } else {
        tracing_subscriber::fmt().with_env_filter(filter).with_target(false).compact().init();
    }
}

fn resolve_root(arg: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    if let Some(p) = arg {
        return Ok(p);
    }
    if let Ok(p) = std::env::var("VPS_ROOT") {
        return Ok(PathBuf::from(p));
    }
    if let Ok(p) = std::env::var("APHRODY_ROOT") {
        return Ok(PathBuf::from(p));
    }
    std::env::current_dir().map_err(Into::into)
}

fn default_out(root: &Path, arg: Option<PathBuf>, name: &str) -> PathBuf {
    arg.unwrap_or_else(|| root.join(name))
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    init_tracing(cli.log_json);

    let root = match resolve_root(cli.root.clone()) {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("{e:#}");
            return ExitCode::from(2);
        },
    };
    if !root.is_dir() {
        tracing::error!("root does not exist: {}", root.display());
        return ExitCode::from(2);
    }
    let out = default_out(&root, cli.out.clone(), "path.json");
    let map = default_out(&root, cli.map.clone(), "monorepo-map.json");

    match cli.cmd {
        Cmd::Scan => match mrx_audit::run(&root, &out, &map) {
            Ok(r) => {
                tracing::info!(
                    status = ?r.status,
                    submodules = r.submodules,
                    workspaces = r.workspaces,
                    duration_ms = r.duration_ms,
                    "scan complete"
                );
                ExitCode::SUCCESS
            },
            Err(e) => {
                tracing::error!("scan failed: {e:#}");
                ExitCode::from(1)
            },
        },
        Cmd::Check => match mrx_audit::run(&root, &out, &map) {
            Ok(r) if r.status == mrx_core::Status::ProductionReady => ExitCode::SUCCESS,
            Ok(r) => {
                tracing::error!(
                    "findings detected: status={:?} ({} total)",
                    r.status,
                    r.total_findings
                );
                ExitCode::from(1)
            },
            Err(e) => {
                tracing::error!("check failed: {e:#}");
                ExitCode::from(1)
            },
        },
        Cmd::Watch { debounce_ms } => match mrx_watch::run(&root, &out, &map, debounce_ms) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                tracing::error!("watch failed: {e:#}");
                ExitCode::from(1)
            },
        },
    }
}
