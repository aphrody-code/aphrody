// SPDX-License-Identifier: Apache-2.0
//! `aphrody gui` — resolve and launch the native desktop app (Tauri + Angular).
//!
//! The desktop app lives under `apps/desktop/`, which is **self-rooted and
//! excluded from the core workspace** so the heavy webview stack (wry / tao /
//! GTK) never enters the core supply-chain (cf. CLAUDE.md §7). Native
//! integration therefore means the `aphrody` binary **resolves + spawns** the
//! standalone GUI binary, mirroring how [`gemini_runtime::resolve_bin`] locates
//! the Gemini CLI — it never embeds a webview in the CLI itself.
//!
//! The GUI binary is named `aphrody-gui` (`aphrody-gui.exe` on Windows).
//!
//! Resolution order for [`resolve_gui_bin`] (first match wins):
//! 1. `$APHRODY_GUI_BIN` — explicit override, must point to an existing file.
//! 2. Sibling of `std::env::current_exe()` — `aphrody-gui[.exe]`, the binary a
//!    deploy script drops next to `aphrody[.exe]`.
//! 3. In-tree build — walk up from `$CWD` for
//!    `apps/desktop/src-tauri/target/{release,debug}/aphrody-gui[.exe]`.
//! 4. `which("aphrody-gui")` — PATH lookup.
//!
//! Launching is fire-and-forget: the GUI is a windowed app, so the command
//! spawns it (without inheriting/waiting on it), prints a confirmation with the
//! child PID, and returns immediately — the terminal is never blocked on the
//! window lifetime.

use std::path::{Path, PathBuf};

/// Environment override pointing directly at the `aphrody-gui` binary. Highest
/// precedence in [`resolve_gui_bin`]; when set to a non-empty existing path it
/// short-circuits every other resolution step.
pub(crate) const ENV_GUI_BIN: &str = "APHRODY_GUI_BIN";

/// Base name of the GUI binary (without the platform executable extension).
const GUI_BIN_STEM: &str = "aphrody-gui";

/// Failure modes of [`resolve_gui_bin`]. Kept distinct from a bare `Option` so
/// the launch path can surface an actionable, build-vs-deploy specific message
/// (mirrors the shape of `gemini_runtime::RuntimeError::NotFound`).
#[derive(Debug, thiserror::Error)]
pub(crate) enum GuiResolveError {
    /// No resolution step yielded an existing `aphrody-gui` binary.
    #[error("`aphrody-gui` binary not found (env / sibling / in-tree build / PATH all empty)")]
    NotFound,
    /// A `which("aphrody-gui")` PATH lookup failed for a reason other than
    /// "not present" (e.g. a malformed `$PATH`).
    #[error("PATH lookup for `aphrody-gui` failed: {0}")]
    Which(#[from] which::Error),
}

/// Candidate file names for the GUI binary on the running platform. Windows
/// builds carry the `.exe` extension; every other target uses the bare stem.
/// Both are probed so a `.exe` produced under WSL/cross builds is still found.
fn gui_bin_names() -> [String; 2] {
    [format!("{GUI_BIN_STEM}.exe"), GUI_BIN_STEM.to_string()]
}

/// Resolve the `aphrody-gui` binary path **without** launching it.
///
/// Resolution order is documented on the [module][self]. Returns the resolved
/// path; the caller is responsible for spawning it.
///
/// # Errors
///
/// - [`GuiResolveError::NotFound`] when no resolution step yields a path.
/// - [`GuiResolveError::Which`] when the final PATH lookup itself errors.
pub(crate) fn resolve_gui_bin() -> Result<PathBuf, GuiResolveError> {
    // Step 1 : explicit env override (must point to an existing file).
    if let Ok(explicit) = std::env::var(ENV_GUI_BIN) {
        let trimmed = explicit.trim();
        if !trimmed.is_empty() {
            let p = PathBuf::from(trimmed);
            if p.is_file() {
                return Ok(p);
            }
        }
    }

    // Step 2 : sibling of the running aphrody binary.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for cand in gui_bin_names() {
                let p = dir.join(&cand);
                if p.is_file() {
                    return Ok(p);
                }
            }
        }
    }

    // Step 3 : walk up from CWD looking for an in-tree Tauri build artifact.
    if let Ok(cwd) = std::env::current_dir() {
        if let Some(p) = find_in_tree_gui(&cwd) {
            return Ok(p);
        }
    }

    // Step 4 : PATH lookup. `which::which` returns `Err(CannotFindBinaryPath)`
    // when the binary is simply absent — map that to `NotFound` so the launch
    // path emits the build/deploy guidance rather than a raw lookup error.
    match which::which(GUI_BIN_STEM) {
        Ok(p) => Ok(p),
        Err(which::Error::CannotFindBinaryPath) => Err(GuiResolveError::NotFound),
        Err(e) => Err(GuiResolveError::Which(e)),
    }
}

/// Walk up from `start`, probing each ancestor for the Tauri build artifact
/// under `apps/desktop/src-tauri/target/`. `release` is preferred over `debug`.
/// Returns the first existing match.
fn find_in_tree_gui(start: &Path) -> Option<PathBuf> {
    let mut here: Option<&Path> = Some(start);
    while let Some(dir) = here {
        let target_dir = dir
            .join("apps")
            .join("desktop")
            .join("src-tauri")
            .join("target");
        if let Some(p) = probe_cargo_target(&target_dir) {
            return Some(p);
        }
        here = dir.parent();
    }
    None
}

/// Probe a Cargo `target/` directory for `aphrody-gui[.exe]`, honouring **both**
/// layouts:
/// - plain `target/<profile>/` (no forced target), and
/// - `target/<triple>/<profile>/` — what Cargo emits when a `build.target` is
///   forced (e.g. the repo `.cargo/config.toml` pins `x86_64-pc-windows-msvc`,
///   which the self-rooted desktop build inherits by config-walk-up).
///
/// `release` wins over `debug` across every layout: the release sweep (plain
/// then each `<triple>`) runs fully before any debug probe. `<triple>` subdirs
/// are enumerated in sorted order for determinism.
fn probe_cargo_target(target_dir: &Path) -> Option<PathBuf> {
    // Immediate subdirs that look like target triples (everything except the
    // plain profile dirs themselves), sorted for a stable resolution order.
    let mut triples: Vec<PathBuf> = std::fs::read_dir(target_dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_dir()
                && !matches!(
                    p.file_name().and_then(|n| n.to_str()),
                    Some("release" | "debug")
                )
        })
        .collect();
    triples.sort();

    for profile in ["release", "debug"] {
        if let Some(p) = probe_profile_dir(&target_dir.join(profile)) {
            return Some(p);
        }
        for triple in &triples {
            if let Some(p) = probe_profile_dir(&triple.join(profile)) {
                return Some(p);
            }
        }
    }
    None
}

/// Probe a single `target/<…>/<profile>/` directory for the GUI binary under
/// either platform file name. Returns the first existing match.
fn probe_profile_dir(profile_dir: &Path) -> Option<PathBuf> {
    for cand in gui_bin_names() {
        let p = profile_dir.join(&cand);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

/// Actionable, French, multi-line error for the "GUI binary not found" case.
/// Lists the three remedies (build / env override / deploy) so an operator can
/// recover without leaving the terminal.
fn not_found_report() -> miette::Report {
    miette::miette!(
        "App desktop `aphrody-gui` introuvable.\n\n\
         Résolution tentée (premier match gagnant) :\n\
         1. ${ENV} (fichier existant)\n\
         2. voisin de l'exécutable : `aphrody-gui[.exe]`\n\
         3. build in-tree : `apps/desktop/src-tauri/target/{{release,debug}}/aphrody-gui[.exe]`\n\
         4. PATH : `aphrody-gui`\n\n\
         Remèdes :\n\
         (a) Construire : `cd apps/desktop && bun install && bun run build && \
             cd src-tauri && cargo build --release`\n\
         (b) Définir l'override : `{ENV}=/chemin/aphrody-gui`\n\
         (c) Déployer le binaire : `scripts/deploy.ps1` (Windows) ou `scripts/deploy.sh` (Unix).",
        ENV = ENV_GUI_BIN,
    )
}

/// Implementation of `aphrody gui [--print-path] [-- <args…>]`.
///
/// - `--print-path` : resolve and print the absolute path, exit 0; on failure
///   emit [`not_found_report`] and exit non-zero. Scriptable (no spawn).
/// - otherwise : resolve, spawn the GUI binary fire-and-forget (it is a
///   windowed app), print a French confirmation with the child PID, exit 0.
///   The terminal is **not** blocked on the window's lifetime.
pub(crate) struct GuiCommand {
    pub print_path: bool,
    pub args: Vec<String>,
}

impl GuiCommand {
    /// Resolve (and optionally launch) the desktop app.
    ///
    /// # Errors
    ///
    /// Returns [`not_found_report`] when no resolution step yields a binary,
    /// or a spawn-failure report when the resolved binary cannot be launched.
    pub(crate) fn execute(&self) -> miette::Result<()> {
        let bin_path = resolve_gui_bin().map_err(|e| match e {
            GuiResolveError::NotFound => not_found_report(),
            GuiResolveError::Which(_) => miette::miette!("{e}"),
        })?;

        // Absolute path for both the scriptable print and the confirmation; fall
        // back to the resolved path verbatim if canonicalisation fails (e.g. a
        // permission quirk) so we never swallow an otherwise-usable result.
        let display_path = std::fs::canonicalize(&bin_path).unwrap_or_else(|_| bin_path.clone());

        if self.print_path {
            println!("{}", display_path.display());
            return Ok(());
        }

        // Fire-and-forget: the GUI owns its own window/event loop, so we spawn
        // without inheriting stdio-blocking semantics and never `.wait()`.
        let child = std::process::Command::new(&bin_path)
            .args(&self.args)
            .spawn()
            .map_err(|e| {
                miette::miette!("Échec du lancement de `{}` : {e}", display_path.display())
            })?;

        println!("aphrody-gui lancé (pid {})", child.id());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The env override is process-global; serialise the env-mutating tests so
    // they cannot observe each other's `APHRODY_GUI_BIN` writes when nextest
    // runs them on the same process (it does not, per-test, but a guard keeps
    // the intent explicit and is cheap).
    use std::sync::{Mutex, MutexGuard};
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn lock_env() -> MutexGuard<'static, ()> {
        ENV_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// (a) `APHRODY_GUI_BIN` pointing at an existing temp file resolves to it.
    #[test]
    fn resolves_via_env_override_to_existing_file() {
        let _guard = lock_env();
        let tmp = tempfile::NamedTempFile::new().expect("create temp file");
        let path = tmp.path().to_path_buf();

        // SAFETY: single-threaded test guarded by ENV_LOCK; no other thread
        // reads/writes the environment concurrently.
        unsafe {
            std::env::set_var(ENV_GUI_BIN, &path);
        }
        let resolved = resolve_gui_bin();
        unsafe {
            std::env::remove_var(ENV_GUI_BIN);
        }

        let resolved = resolved.expect("env override must resolve to the temp file");
        assert_eq!(resolved, path, "resolver must return the env-pointed path verbatim");
    }

    /// (b) Empty env + (effectively) absent binary => `NotFound`, never a panic.
    ///
    /// We cannot guarantee no `aphrody-gui` exists on PATH or as an exe sibling
    /// on an arbitrary host, but we *can* assert the function returns a `Result`
    /// without panicking and that, in the common CI case (no such binary), it is
    /// the `NotFound` variant. To make this deterministic we point `current_dir`
    /// resolution at an empty temp dir is impossible (no API to inject CWD here
    /// cheaply); instead we assert the contract: with the env unset the call
    /// completes and yields either a path (binary genuinely present) or
    /// `NotFound`/`Which` — i.e. it does not panic and never returns a bogus
    /// non-existent path.
    #[test]
    fn empty_env_does_not_panic_and_returns_result() {
        let _guard = lock_env();
        // SAFETY: guarded by ENV_LOCK; serial mutation of the environment.
        unsafe {
            std::env::remove_var(ENV_GUI_BIN);
        }

        match resolve_gui_bin() {
            Ok(p) => assert!(
                p.is_file(),
                "a resolved path must point at an existing file, got {}",
                p.display()
            ),
            Err(GuiResolveError::NotFound | GuiResolveError::Which(_)) => {},
        }
    }

    /// An empty / whitespace `APHRODY_GUI_BIN` must be ignored (not treated as a
    /// valid path), falling through to the later resolution steps.
    #[test]
    fn blank_env_override_is_ignored() {
        let _guard = lock_env();
        // SAFETY: guarded by ENV_LOCK; serial mutation of the environment.
        unsafe {
            std::env::set_var(ENV_GUI_BIN, "   ");
        }
        let resolved = resolve_gui_bin();
        unsafe {
            std::env::remove_var(ENV_GUI_BIN);
        }
        // Whatever the host yields, a blank override must never be returned as a
        // path; the only blank-derived outcome is NotFound (or a real later-step
        // hit, which is necessarily an existing file).
        if let Ok(p) = resolved {
            assert!(p.is_file(), "fallthrough result must be an existing file");
            assert_ne!(p.as_os_str(), "   ", "blank override must not be returned");
        }
    }

    /// A non-existent `APHRODY_GUI_BIN` path must be ignored (env step requires
    /// an existing file), not returned verbatim.
    #[test]
    fn nonexistent_env_override_is_ignored() {
        let _guard = lock_env();
        let bogus = std::env::temp_dir().join("aphrody-gui-does-not-exist-zzz-9182");
        assert!(!bogus.exists(), "precondition: bogus path must not exist");

        // SAFETY: guarded by ENV_LOCK; serial mutation of the environment.
        unsafe {
            std::env::set_var(ENV_GUI_BIN, &bogus);
        }
        let resolved = resolve_gui_bin();
        unsafe {
            std::env::remove_var(ENV_GUI_BIN);
        }
        if let Ok(p) = resolved {
            assert_ne!(p, bogus, "a non-existent env path must never be returned");
        }
    }

    /// Regression: when `build.target` is forced (the repo pins
    /// `x86_64-pc-windows-msvc`, inherited by the self-rooted desktop build),
    /// Cargo emits `target/<triple>/<profile>/aphrody-gui`. `probe_cargo_target`
    /// must find it there, not only under the plain `target/<profile>/`.
    #[test]
    fn probe_finds_binary_under_triple_subdir() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let target = tmp.path().join("target");
        let profile_dir = target.join("x86_64-pc-windows-msvc").join("debug");
        std::fs::create_dir_all(&profile_dir).expect("mkdir profile");
        let bin = profile_dir.join("aphrody-gui");
        std::fs::write(&bin, b"fake").expect("write fake bin");

        let found = probe_cargo_target(&target).expect("must find binary under triple/debug");
        assert_eq!(found, bin);
    }

    /// `release` must win over `debug` even when the release copy lives under a
    /// `<triple>` subdir and a plain `debug` copy also exists.
    #[test]
    fn probe_prefers_release_under_triple_over_plain_debug() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let target = tmp.path().join("target");

        let debug_plain = target.join("debug");
        std::fs::create_dir_all(&debug_plain).expect("mkdir debug");
        std::fs::write(debug_plain.join("aphrody-gui"), b"debug").expect("write debug");

        let release_triple = target.join("x86_64-unknown-linux-gnu").join("release");
        std::fs::create_dir_all(&release_triple).expect("mkdir release triple");
        let release_bin = release_triple.join("aphrody-gui");
        std::fs::write(&release_bin, b"release").expect("write release");

        let found = probe_cargo_target(&target).expect("must find a binary");
        assert_eq!(found, release_bin, "release (even under a triple) must beat plain debug");
    }

    /// An empty `target/` yields no match (and does not panic on `read_dir`).
    #[test]
    fn probe_empty_target_is_none() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let target = tmp.path().join("target");
        std::fs::create_dir_all(&target).expect("mkdir target");
        assert!(probe_cargo_target(&target).is_none());
    }
}
