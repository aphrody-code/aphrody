// SPDX-License-Identifier: Apache-2.0
//! Workspace seeding (AH-4).
//!
//! Port of `var/openclaw/src/agents/workspace.ts::ensureAgentWorkspace`, but
//! with the templates embedded via `include_str!` (no runtime template-dir
//! resolution, no `Missing workspace template` failure mode — the binary
//! always carries its own seeds) and with explicit `--skip-bootstrap` /
//! `--force` ergonomics.
//!
//! Seeds the canonical file-map (AGENTS / SOUL / IDENTITY / USER / TOOLS /
//! HEARTBEAT / BOOT and the one-shot BOOTSTRAP), records `bootstrapSeededAt`
//! in the workspace state, and deletes `BOOTSTRAP.md` once first-run setup is
//! marked complete (openclaw `reconcileWorkspaceBootstrapCompletion`).

use std::path::{Path, PathBuf};

use crate::cache::WorkspaceState;
use crate::filenames::{
    BootstrapFile, AGENTS_FILENAME, BOOTSTRAP_FILENAME, BOOT_FILENAME, HEARTBEAT_FILENAME,
    IDENTITY_FILENAME, MEMORY_DIRNAME, SKILLS_DIRNAME, SOUL_FILENAME, TOOLS_FILENAME, USER_FILENAME,
};
use crate::HomeError;

/// Embedded workspace templates (compiled into the binary).
pub const AGENTS_TEMPLATE: &str = include_str!("../templates/AGENTS.md");
/// Embedded `SOUL.md` template.
pub const SOUL_TEMPLATE: &str = include_str!("../templates/SOUL.md");
/// Embedded `IDENTITY.md` template.
pub const IDENTITY_TEMPLATE: &str = include_str!("../templates/IDENTITY.md");
/// Embedded `USER.md` template.
pub const USER_TEMPLATE: &str = include_str!("../templates/USER.md");
/// Embedded `TOOLS.md` template.
pub const TOOLS_TEMPLATE: &str = include_str!("../templates/TOOLS.md");
/// Embedded `HEARTBEAT.md` template.
pub const HEARTBEAT_TEMPLATE: &str = include_str!("../templates/HEARTBEAT.md");
/// Embedded `BOOT.md` template.
pub const BOOT_TEMPLATE: &str = include_str!("../templates/BOOT.md");
/// Embedded `BOOTSTRAP.md` template.
pub const BOOTSTRAP_TEMPLATE: &str = include_str!("../templates/BOOTSTRAP.md");

/// Options controlling [`seed_workspace`].
#[derive(Debug, Clone)]
pub struct OnboardOptions {
    /// Workspace root to seed.
    pub workspace: PathBuf,
    /// Skip the optional persona files (SOUL / IDENTITY / USER / HEARTBEAT /
    /// BOOT). AGENTS.md and TOOLS.md are always written.
    pub skip_bootstrap: bool,
    /// Overwrite files that already exist (otherwise existing files are kept).
    pub force: bool,
}

impl OnboardOptions {
    /// Construct with default flags (no skip, no force).
    #[must_use]
    pub fn new(workspace: impl Into<PathBuf>) -> Self {
        Self {
            workspace: workspace.into(),
            skip_bootstrap: false,
            force: false,
        }
    }
}

/// Result of a seeding pass: which files were written / kept.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SeedReport {
    /// Files newly created this run (workspace-relative names).
    pub created: Vec<String>,
    /// Files that already existed and were kept (no `--force`).
    pub kept: Vec<String>,
    /// Files that existed and were overwritten (`--force`).
    pub overwritten: Vec<String>,
}

/// Strip a leading `---` frontmatter block from a template before writing it,
/// matching openclaw `stripFrontMatter` (templates carry frontmatter for the
/// authoring docs, but the seeded files start at the body).
fn strip_frontmatter(content: &str) -> &str {
    let (_fm, body) = crate::frontmatter::split(content);
    body
}

fn template_for(file: BootstrapFile) -> Option<&'static str> {
    Some(match file {
        BootstrapFile::Agents => AGENTS_TEMPLATE,
        BootstrapFile::Soul => SOUL_TEMPLATE,
        BootstrapFile::Identity => IDENTITY_TEMPLATE,
        BootstrapFile::User => USER_TEMPLATE,
        BootstrapFile::Tools => TOOLS_TEMPLATE,
        BootstrapFile::Heartbeat => HEARTBEAT_TEMPLATE,
        BootstrapFile::Boot => BOOT_TEMPLATE,
        BootstrapFile::Bootstrap => BOOTSTRAP_TEMPLATE,
        // MEMORY.md is never seeded; it is user-curated.
        BootstrapFile::Memory => return None,
    })
}

/// The files seeded by default, in order. `BOOTSTRAP.md` is handled separately
/// (only seeded for a brand-new workspace).
const SEED_ORDER: [BootstrapFile; 7] = [
    BootstrapFile::Agents,
    BootstrapFile::Soul,
    BootstrapFile::Identity,
    BootstrapFile::User,
    BootstrapFile::Tools,
    BootstrapFile::Heartbeat,
    BootstrapFile::Boot,
];

/// Whether a freshly-created workspace has no prior user content (so we should
/// seed the one-shot BOOTSTRAP ritual). Mirrors openclaw `isBrandNewWorkspace`.
fn is_brand_new(workspace: &Path) -> bool {
    let candidates = [
        AGENTS_FILENAME,
        SOUL_FILENAME,
        TOOLS_FILENAME,
        IDENTITY_FILENAME,
        USER_FILENAME,
        HEARTBEAT_FILENAME,
        BOOT_FILENAME,
    ];
    for name in candidates {
        if workspace.join(name).exists() {
            return false;
        }
    }
    !workspace.join(MEMORY_DIRNAME).exists()
        && !workspace.join(".git").exists()
        && !workspace.join("MEMORY.md").exists()
}

/// Seed the workspace file-map under `opts.workspace`.
///
/// Creates the workspace directory, the `memory/` and `skills/` subdirectories,
/// writes each template (respecting `skip_bootstrap` / `force`), seeds the
/// one-shot `BOOTSTRAP.md` for a brand-new workspace, and records
/// `bootstrapSeededAt` in the workspace state.
///
/// # Errors
/// [`HomeError::Io`] on any filesystem failure, [`HomeError::Json`] on state
/// serialization failure.
pub fn seed_workspace(opts: &OnboardOptions) -> Result<SeedReport, HomeError> {
    let ws = &opts.workspace;
    std::fs::create_dir_all(ws).map_err(|e| HomeError::io(ws, e))?;

    // Decide "brand new" BEFORE creating the `memory/` indicator dir — that
    // directory is itself one of the brand-new indicators, so creating it
    // first would always mark the workspace as established.
    let brand_new = is_brand_new(ws);

    for sub in [MEMORY_DIRNAME, SKILLS_DIRNAME] {
        let p = ws.join(sub);
        std::fs::create_dir_all(&p).map_err(|e| HomeError::io(&p, e))?;
    }

    let mut report = SeedReport::default();

    for file in SEED_ORDER {
        if opts.skip_bootstrap && file.is_optional() {
            continue;
        }
        let Some(template) = template_for(file) else {
            continue;
        };
        write_seed(ws, file.filename(), strip_frontmatter(template), opts.force, &mut report)?;
    }

    // Load (or default) the workspace state and seed the one-shot ritual only
    // for a brand-new workspace that has not completed setup.
    let mut state = WorkspaceState::load(ws)?;
    let mut state_dirty = false;

    if brand_new && !state.is_setup_completed() && state.bootstrap_seeded_at.is_none() {
        let bootstrap_path = ws.join(BOOTSTRAP_FILENAME);
        let existed = bootstrap_path.exists();
        write_seed(
            ws,
            BOOTSTRAP_FILENAME,
            strip_frontmatter(BOOTSTRAP_TEMPLATE),
            opts.force,
            &mut report,
        )?;
        if !existed || opts.force {
            state.bootstrap_seeded_at = Some(now_iso8601());
            state_dirty = true;
        }
    }

    if state_dirty {
        state.save(ws)?;
    }
    Ok(report)
}

/// Write a single seed file, honouring `force`. Records the action in `report`.
fn write_seed(
    workspace: &Path,
    name: &str,
    body: &str,
    force: bool,
    report: &mut SeedReport,
) -> Result<(), HomeError> {
    let path = workspace.join(name);
    let exists = path.exists();
    if exists && !force {
        report.kept.push(name.to_string());
        return Ok(());
    }
    let content = ensure_trailing_newline(body);
    std::fs::write(&path, content.as_bytes()).map_err(|e| HomeError::io(&path, e))?;
    if exists {
        report.overwritten.push(name.to_string());
    } else {
        report.created.push(name.to_string());
    }
    Ok(())
}

fn ensure_trailing_newline(s: &str) -> std::borrow::Cow<'_, str> {
    if s.ends_with('\n') {
        std::borrow::Cow::Borrowed(s)
    } else {
        std::borrow::Cow::Owned(format!("{s}\n"))
    }
}

/// Mark first-run setup complete and delete the one-shot `BOOTSTRAP.md`
/// (openclaw `reconcileWorkspaceBootstrapCompletion`). Idempotent.
///
/// # Errors
/// [`HomeError::Io`] / [`HomeError::Json`] on filesystem / state failure.
pub fn complete_bootstrap(workspace: &Path) -> Result<bool, HomeError> {
    let mut state = WorkspaceState::load(workspace)?;
    if state.is_setup_completed() {
        // Already complete; ensure the ritual file is gone but do not error.
        let bootstrap_path = workspace.join(BOOTSTRAP_FILENAME);
        let _ = std::fs::remove_file(bootstrap_path);
        return Ok(false);
    }
    state.setup_completed_at = Some(now_iso8601());
    if state.bootstrap_seeded_at.is_none() {
        state.bootstrap_seeded_at = Some(now_iso8601());
    }
    state.save(workspace)?;
    let bootstrap_path = workspace.join(BOOTSTRAP_FILENAME);
    match std::fs::remove_file(&bootstrap_path) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Err(e) => Err(HomeError::io(bootstrap_path, e)),
    }
}

/// ISO-8601 UTC timestamp (no `chrono` dependency; same public-domain
/// `civil_from_days` algorithm as `oc_cmd.rs::current_iso8601`). The integer
/// casts are part of that algorithm and operate on bounded day/era counts that
/// never overflow or change sign for any realistic timestamp.
#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
fn now_iso8601() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (hour, min, sec) = (rem / 3_600, (rem % 3_600) / 60, rem % 60);
    let z = days as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}T{hour:02}:{min:02}:{sec:02}Z")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn seeds_full_file_map() {
        let td = tempdir().unwrap();
        let ws = td.path().join("workspace");
        let report = seed_workspace(&OnboardOptions::new(&ws)).unwrap();
        for name in [
            AGENTS_FILENAME,
            SOUL_FILENAME,
            IDENTITY_FILENAME,
            USER_FILENAME,
            TOOLS_FILENAME,
            HEARTBEAT_FILENAME,
            BOOT_FILENAME,
            BOOTSTRAP_FILENAME,
        ] {
            assert!(ws.join(name).exists(), "missing seeded file {name}");
        }
        assert!(ws.join(MEMORY_DIRNAME).is_dir());
        assert!(ws.join(SKILLS_DIRNAME).is_dir());
        assert!(report.created.contains(&AGENTS_FILENAME.to_string()));
        // State recorded the seed.
        let state = WorkspaceState::load(&ws).unwrap();
        assert!(state.bootstrap_seeded_at.is_some());
    }

    #[test]
    fn seeded_files_have_no_frontmatter() {
        let td = tempdir().unwrap();
        let ws = td.path().join("workspace");
        seed_workspace(&OnboardOptions::new(&ws)).unwrap();
        let soul = std::fs::read_to_string(ws.join(SOUL_FILENAME)).unwrap();
        // The body should NOT start with the YAML frontmatter delimiter; the
        // frontmatter is stripped at seed time.
        assert!(!soul.trim_start().starts_with("---"), "frontmatter not stripped:\n{soul}");
        assert!(soul.contains("focused engineering companion"));
    }

    #[test]
    fn skip_bootstrap_omits_optional_files() {
        let td = tempdir().unwrap();
        let ws = td.path().join("workspace");
        let opts = OnboardOptions {
            workspace: ws.clone(),
            skip_bootstrap: true,
            force: false,
        };
        seed_workspace(&opts).unwrap();
        // Required files present.
        assert!(ws.join(AGENTS_FILENAME).exists());
        assert!(ws.join(TOOLS_FILENAME).exists());
        // Optional persona files skipped.
        assert!(!ws.join(SOUL_FILENAME).exists());
        assert!(!ws.join(IDENTITY_FILENAME).exists());
        assert!(!ws.join(USER_FILENAME).exists());
    }

    #[test]
    fn does_not_overwrite_without_force() {
        let td = tempdir().unwrap();
        let ws = td.path().join("workspace");
        std::fs::create_dir_all(&ws).unwrap();
        std::fs::write(ws.join(SOUL_FILENAME), "MY CUSTOM SOUL").unwrap();
        let report = seed_workspace(&OnboardOptions::new(&ws)).unwrap();
        assert_eq!(std::fs::read_to_string(ws.join(SOUL_FILENAME)).unwrap(), "MY CUSTOM SOUL");
        assert!(report.kept.contains(&SOUL_FILENAME.to_string()));
    }

    #[test]
    fn force_overwrites_existing() {
        let td = tempdir().unwrap();
        let ws = td.path().join("workspace");
        std::fs::create_dir_all(&ws).unwrap();
        std::fs::write(ws.join(SOUL_FILENAME), "OLD").unwrap();
        let opts = OnboardOptions {
            workspace: ws.clone(),
            skip_bootstrap: false,
            force: true,
        };
        let report = seed_workspace(&opts).unwrap();
        assert_ne!(std::fs::read_to_string(ws.join(SOUL_FILENAME)).unwrap(), "OLD");
        assert!(report.overwritten.contains(&SOUL_FILENAME.to_string()));
    }

    #[test]
    fn complete_bootstrap_removes_ritual_and_marks_state() {
        let td = tempdir().unwrap();
        let ws = td.path().join("workspace");
        seed_workspace(&OnboardOptions::new(&ws)).unwrap();
        assert!(ws.join(BOOTSTRAP_FILENAME).exists());
        let removed = complete_bootstrap(&ws).unwrap();
        assert!(removed);
        assert!(!ws.join(BOOTSTRAP_FILENAME).exists());
        let state = WorkspaceState::load(&ws).unwrap();
        assert!(state.is_setup_completed());
        // Idempotent second call.
        assert!(!complete_bootstrap(&ws).unwrap());
    }

    #[test]
    fn no_bootstrap_ritual_for_non_brand_new_workspace() {
        let td = tempdir().unwrap();
        let ws = td.path().join("workspace");
        std::fs::create_dir_all(ws.join(MEMORY_DIRNAME)).unwrap();
        // memory/ present => not brand new => no BOOTSTRAP.md.
        seed_workspace(&OnboardOptions::new(&ws)).unwrap();
        assert!(!ws.join(BOOTSTRAP_FILENAME).exists());
    }
}
