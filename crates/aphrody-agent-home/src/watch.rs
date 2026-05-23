// SPDX-License-Identifier: Apache-2.0
//! Atomic hot-reload of the live [`AgentHome`] (AH-13).
//!
//! openclaw re-reads the workspace on every session. aphrody watches the
//! workspace root with `notify` and, on any change to a bootstrap file, re-runs
//! [`AgentHome::open`] and atomically swaps the shared handle via
//! [`arc_swap::ArcSwap`]. Readers never block and never see a torn state: a
//! [`SharedHome::load`] returns either the old `Arc<AgentHome>` or the new one,
//! never a half-applied edit.
//!
//! Host-only: the whole module is cfg-gated out of wasm (no filesystem
//! notifications there). On wasm the runtime simply holds an
//! [`arc_swap::ArcSwap`] and reloads on demand.

use std::path::PathBuf;
use std::sync::Arc;

use arc_swap::ArcSwap;
use notify::{Event, EventKind, RecursiveMode, Watcher};

use crate::filenames::BootstrapFile;
use crate::home::HomeOptions;
use crate::{AgentHome, HomeError};

/// A shared, hot-swappable [`AgentHome`]. Cheap to clone (`Arc`).
#[derive(Clone)]
pub struct SharedHome {
    cell: Arc<ArcSwap<AgentHome>>,
}

impl SharedHome {
    /// Wrap a home in a swap cell.
    #[must_use]
    pub fn new(home: AgentHome) -> Self {
        Self {
            cell: Arc::new(ArcSwap::from_pointee(home)),
        }
    }

    /// Load the current home (a cheap `Arc` clone; never blocks).
    #[must_use]
    pub fn load(&self) -> Arc<AgentHome> {
        self.cell.load_full()
    }

    /// Atomically replace the current home.
    pub fn store(&self, home: AgentHome) {
        self.cell.store(Arc::new(home));
    }
}

impl std::fmt::Debug for SharedHome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedHome")
            .field("root", &self.load().root().to_path_buf())
            .finish()
    }
}

/// An active filesystem watcher. Dropping it stops the watch.
pub struct HomeWatcher {
    // Held to keep the watch alive; never read after construction.
    _watcher: notify::RecommendedWatcher,
    shared: SharedHome,
}

impl HomeWatcher {
    /// The shared handle this watcher keeps fresh.
    #[must_use]
    pub fn shared(&self) -> &SharedHome {
        &self.shared
    }
}

impl std::fmt::Debug for HomeWatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // `_watcher` (the OS notify handle) has no useful Debug; mark the
        // struct non-exhaustive rather than printing a placeholder.
        f.debug_struct("HomeWatcher")
            .field("shared", &self.shared)
            .finish_non_exhaustive()
    }
}

/// Whether a changed path is a bootstrap file we should reload for.
fn is_bootstrap_path(path: &std::path::Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| BootstrapFile::from_basename(n).is_some())
}

/// Start watching `workspace` and hot-reload the [`AgentHome`] on bootstrap
/// file changes. Returns the [`HomeWatcher`] (keep it alive) plus the
/// [`SharedHome`] readers should load from.
///
/// The reload re-opens the home with the same `opts` (workspace forced to the
/// watched root). A reload that fails (e.g. a transient half-written file or a
/// strict SOUL lint) is logged and skipped — the previous good home stays
/// live, so an editor's intermediate save never breaks the runtime.
///
/// # Errors
/// [`HomeError::Watch`] when the watcher cannot start or register the path.
// `opts` is moved into the long-lived reload closure (which needs an owned,
// `'static` copy); the single `.clone()` is for the one-shot initial open.
#[allow(clippy::needless_pass_by_value)]
pub fn watch(workspace: PathBuf, opts: HomeOptions) -> Result<HomeWatcher, HomeError> {
    // Initial load consumes a clone; the original `opts` is moved into the
    // reload closure below (which needs an owned, `'static` copy).
    let mut initial_opts = opts.clone();
    initial_opts.workspace = Some(workspace.clone());
    let home = AgentHome::open(initial_opts)?;
    let shared = SharedHome::new(home);

    let cb_shared = shared.clone();
    let cb_workspace = workspace.clone();
    let cb_opts = opts;

    let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
        let Ok(event) = res else { return };
        if !matches!(
            event.kind,
            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
        ) {
            return;
        }
        if !event.paths.iter().any(|p| is_bootstrap_path(p)) {
            return;
        }
        let mut reload_opts = cb_opts.clone();
        reload_opts.workspace = Some(cb_workspace.clone());
        match AgentHome::open(reload_opts) {
            Ok(reloaded) => {
                cb_shared.store(reloaded);
                tracing::debug!(root = %cb_workspace.display(), "agent-home hot-reloaded");
            }
            Err(err) => {
                tracing::warn!(error = %err, "agent-home reload skipped (kept previous)");
            }
        }
    })
    .map_err(|e| HomeError::Watch(e.to_string()))?;

    watcher
        .watch(&workspace, RecursiveMode::NonRecursive)
        .map_err(|e| HomeError::Watch(e.to_string()))?;

    Ok(HomeWatcher {
        _watcher: watcher,
        shared,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn shared_home_loads_and_stores() {
        let td = tempdir().unwrap();
        let ws = td.path().join("workspace");
        let home = AgentHome::onboard(&crate::OnboardOptions::new(&ws)).unwrap();
        let shared = SharedHome::new(home);
        assert_eq!(shared.load().identity().name, "aphrody");

        // Store a home opened from a different workspace; load reflects it.
        let ws2 = td.path().join("workspace2");
        std::fs::create_dir_all(&ws2).unwrap();
        std::fs::write(ws2.join("IDENTITY.md"), "---\nname: Nova\n---\n").unwrap();
        let home2 = AgentHome::open(HomeOptions {
            workspace: Some(ws2),
            ..HomeOptions::default()
        })
        .unwrap();
        shared.store(home2);
        assert_eq!(shared.load().identity().name, "Nova");
    }

    #[test]
    fn is_bootstrap_path_matches_known_files() {
        assert!(is_bootstrap_path(std::path::Path::new("/ws/SOUL.md")));
        assert!(is_bootstrap_path(std::path::Path::new("/ws/AGENTS.md")));
        assert!(!is_bootstrap_path(std::path::Path::new("/ws/random.txt")));
    }

    #[test]
    fn watch_starts_and_exposes_initial_home() {
        let td = tempdir().unwrap();
        let ws = td.path().join("workspace");
        AgentHome::onboard(&crate::OnboardOptions::new(&ws)).unwrap();
        let watcher = watch(ws, HomeOptions::default()).unwrap();
        assert_eq!(watcher.shared().load().identity().name, "aphrody");
    }
}
