//! mrx-watch — long-running watcher that re-runs the native audit on FS events.
//!
//! Backends per OS (via notify v8):
//!   Linux   → inotify
//!   macOS   → FSEvents
//!   Windows → ReadDirectoryChangesW
//!   *BSD    → kqueue
//!   else    → polling fallback
//!
//! Events are debounced by `notify-debouncer-full`, then coalesced and filtered
//! by a noisy-path predicate (target/, node_modules/, …) before triggering a
//! full re-audit. The audit itself runs in `tokio::task::spawn_blocking` so the
//! watcher loop keeps draining events while we scan.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use notify::RecursiveMode;
use notify_debouncer_full::{DebounceEventResult, new_debouncer};
use parking_lot::Mutex;

const WATCH_DIRS: &[&str] = &["apps", "packages"];
const WATCH_FILES: &[&str] = &[
    ".gitmodules",
    "turbo.json",
    "package.json",
    "bun.lock",
    "Cargo.toml",
    "pnpm-workspace.yaml",
];
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

pub fn run(root: &Path, audit_out: &Path, map_out: &Path, debounce_ms: u64) -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("mrx-watch")
        .build()?;
    rt.block_on(async move { run_async(root, audit_out, map_out, debounce_ms).await })
}

async fn run_async(root: &Path, audit_out: &Path, map_out: &Path, debounce_ms: u64) -> Result<()> {
    tracing::info!(
        root = %root.display(),
        debounce_ms,
        os = std::env::consts::OS,
        "watcher starting"
    );

    let root = Arc::new(root.to_path_buf());
    let audit_out = Arc::new(audit_out.to_path_buf());
    let map_out = Arc::new(map_out.to_path_buf());

    // Initial pass so the JSON exists on startup.
    let r0 = Arc::clone(&root);
    let a0 = Arc::clone(&audit_out);
    let m0 = Arc::clone(&map_out);
    tokio::task::spawn_blocking(move || mrx_audit::run(&r0, &a0, &m0))
        .await
        .map_err(|e| anyhow::anyhow!("initial audit join: {e}"))?
        .map_err(|e| anyhow::anyhow!("initial audit: {e:#}"))?;

    let (tx, rx) = flume::unbounded::<DebounceEventResult>();
    let mut debouncer = new_debouncer(Duration::from_millis(debounce_ms), None, move |res| {
        let _ = tx.send(res);
    })?;
    for dir in WATCH_DIRS {
        let p = root.join(dir);
        if p.is_dir() {
            debouncer.watch(&p, RecursiveMode::Recursive)?;
            tracing::info!(path = %p.display(), "watching dir");
        }
    }
    for f in WATCH_FILES {
        let p = root.join(f);
        if p.exists() {
            debouncer.watch(&p, RecursiveMode::NonRecursive)?;
            tracing::info!(path = %p.display(), "watching file");
        }
    }

    let in_flight = Arc::new(Mutex::new(false));

    let shutdown = async {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{SignalKind, signal};
            let mut term = signal(SignalKind::terminate()).ok();
            let mut int = signal(SignalKind::interrupt()).ok();
            tokio::select! {
                _ = async { if let Some(s) = term.as_mut() { s.recv().await; } } => {},
                _ = async { if let Some(s) = int.as_mut() { s.recv().await; } } => {},
            }
        }
        #[cfg(not(unix))]
        {
            let _ = tokio::signal::ctrl_c().await;
        }
        tracing::info!("shutdown signal received");
    };

    let event_loop = async {
        loop {
            let recv = rx.recv_async().await;
            match recv {
                Ok(Ok(events)) => {
                    let relevant = events
                        .into_iter()
                        .any(|e| e.event.paths.iter().any(|p| !is_noisy(p)));
                    if !relevant {
                        continue;
                    }
                    let mut guard = in_flight.lock();
                    if *guard {
                        continue;
                    }
                    *guard = true;
                    drop(guard);

                    let r = Arc::clone(&root);
                    let a = Arc::clone(&audit_out);
                    let m = Arc::clone(&map_out);
                    let in_flight = Arc::clone(&in_flight);
                    tokio::task::spawn_blocking(move || {
                        let res = mrx_audit::run(&r, &a, &m);
                        *in_flight.lock() = false;
                        match res {
                            Ok(r) => tracing::info!(
                                status = ?r.status,
                                submodules = r.submodules,
                                workspaces = r.workspaces,
                                duration_ms = r.duration_ms,
                                "audit complete"
                            ),
                            Err(e) => tracing::error!("audit failed: {e:#}"),
                        }
                    });
                }
                Ok(Err(errs)) => {
                    for e in errs {
                        tracing::warn!("watch error: {e:?}");
                    }
                }
                Err(_) => break,
            }
        }
    };

    tokio::select! {
        _ = shutdown => {},
        _ = event_loop => {},
    }
    Ok(())
}

fn is_noisy(path: &Path) -> bool {
    for c in path.components() {
        let s = c.as_os_str().to_string_lossy();
        if IGNORE_SEGMENTS.iter().any(|p| *p == s) {
            return true;
        }
    }
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("log" | "tmp" | "swp" | "swx")
    )
}
