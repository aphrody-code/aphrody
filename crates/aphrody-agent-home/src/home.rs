// SPDX-License-Identifier: Apache-2.0
//! [`AgentHome`] — the composition root (AH-5 / AH-10 wiring).
//!
//! `AgentHome` is `Send + Sync` and cheap to clone (every heavy field is
//! behind an `Arc`). It owns the resolved workspace root, the parsed typed
//! docs, and a shared content-addressed [`FileCache`]. It is the single entry
//! point the CLI and runtime use:
//!
//! * [`AgentHome::open`] resolves the workspace (profile / agent-aware),
//!   loads + parses the bootstrap files through the boundary guard and the
//!   zero-copy cache, and seeds the cache from the persisted state.
//! * [`AgentHome::onboard`] seeds the file-map then opens the result.
//! * [`AgentHome::system_prompt`] assembles a borrowed [`SystemPromptView`].
//! * [`AgentHome::git_backup`] (feature `git`) commits the workspace.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::assemble::{LoadedFile, SystemPromptView};
use crate::budget::{BootstrapBudget, WarningMode};
use crate::cache::{FileCache, WorkspaceState};
use crate::filenames::BootstrapFile;
use crate::guard::WorkspaceGuard;
use crate::heartbeat::SessionKind;
use crate::identity::Identity;
use crate::onboard::{seed_workspace, OnboardOptions};
use crate::soul::Soul;
use crate::tools::ToolsDoc;
use crate::user::UserProfile;
use crate::HomeError;

/// Options for [`AgentHome::open`].
#[derive(Debug, Clone, Default)]
pub struct HomeOptions {
    /// Explicit workspace root. When `None`, resolved from
    /// `$APHRODY_WORKSPACE` / `$APHRODY_PROFILE` / agent id / default.
    pub workspace: Option<PathBuf>,
    /// Named profile (`$APHRODY_PROFILE` override).
    pub profile: Option<String>,
    /// Agent id for multi-agent workspaces.
    pub agent_id: Option<String>,
    /// Whether to enforce SOUL.md anti-pattern lints on open. When `false`,
    /// a soul that trips a lint is loaded leniently (the operator accepted it).
    pub strict_soul: bool,
    /// Session kind, gating which files are injected (subagent / cron).
    pub session_kind: SessionKind,
}

/// The parsed typed docs of a workspace.
#[derive(Debug, Clone, Default)]
struct Docs {
    soul: Option<Soul>,
    identity: Identity,
    user: Option<UserProfile>,
    tools: Option<ToolsDoc>,
}

/// Shared inner state behind an `Arc` so `AgentHome` is cheap to clone and
/// `Send + Sync`.
#[derive(Debug)]
struct Inner {
    root: PathBuf,
    guard: WorkspaceGuard,
    cache: FileCache,
    docs: Docs,
    /// Raw (untruncated) content of each present file, keyed by kind. Owned so
    /// the assembler can borrow it for the lifetime of the `AgentHome`.
    raw: Vec<(BootstrapFile, String)>,
    session_kind: SessionKind,
}

/// The agent's persistent home. Clone is cheap (`Arc` bump).
#[derive(Debug, Clone)]
pub struct AgentHome {
    inner: Arc<Inner>,
}

// `Inner` holds only `Send + Sync` fields (PathBuf, WorkspaceGuard, FileCache
// with a Mutex, plain data), so `AgentHome` is `Send + Sync` by construction.
// We assert it at compile time in the tests.

impl AgentHome {
    /// Open an existing workspace, loading + parsing its bootstrap files.
    ///
    /// Missing files are tolerated (the corresponding doc is `None` / default).
    /// The workspace directory itself does not need to exist yet — an empty
    /// home is returned, which [`AgentHome::system_prompt`] renders as the
    /// minimal default.
    ///
    /// # Errors
    /// [`HomeError`] on path resolution, an oversized file, a SOUL lint
    /// failure (when `strict_soul`), or a state JSON error.
    pub fn open(opts: HomeOptions) -> Result<Self, HomeError> {
        let root = match opts.workspace {
            Some(w) => w,
            None => crate::paths::resolve_workspace(opts.profile.as_deref(), opts.agent_id.as_deref())?,
        };
        let guard = WorkspaceGuard::new(&root);
        let cache = FileCache::new();

        // Seed the cache from any persisted state (skip re-hashing unchanged
        // files across restart).
        if let Ok(state) = WorkspaceState::load(guard.root()) {
            cache.seed_from_state(&state, guard.root());
        }

        // Load every present bootstrap file through the guard + cache.
        let mut raw: Vec<(BootstrapFile, String)> = Vec::new();
        for file in BootstrapFile::ALL {
            // A known basename never escapes the boundary; skip defensively.
            let Ok(abs) = guard.resolve(file.filename()) else {
                continue;
            };
            if !abs.is_file() {
                continue;
            }
            // Cap check via the guard (re-reads metadata cheaply).
            let meta = std::fs::metadata(&abs).map_err(|e| HomeError::io(&abs, e))?;
            if meta.len() > guard.max_bytes() {
                return Err(HomeError::FileTooLarge {
                    path: abs,
                    size: meta.len(),
                    cap: guard.max_bytes(),
                });
            }
            let hit = cache.load(file.filename(), &abs)?;
            // Bootstrap files are UTF-8 markdown; lossy view tolerates strays.
            let content = hit.bytes.to_string_lossy().into_owned();
            raw.push((file, content));
        }

        // Parse the typed docs.
        let mut docs = Docs::default();
        let mut has_identity_file = false;
        for (file, content) in &raw {
            match file {
                BootstrapFile::Soul => {
                    let soul = if opts.strict_soul {
                        Soul::parse(content)?
                    } else {
                        Soul::parse_lenient(content)
                    };
                    docs.soul = Some(soul);
                }
                BootstrapFile::Identity => {
                    docs.identity = Identity::parse(content);
                    has_identity_file = true;
                }
                BootstrapFile::User => docs.user = Some(UserProfile::parse(content)),
                BootstrapFile::Tools => docs.tools = Some(ToolsDoc::parse(content)),
                _ => {}
            }
        }

        // The agent always has an identity: when `IDENTITY.md` is absent,
        // synthesize a section from the default (neutral placeholder) so the
        // assembled prompt still tells the model who it is. Stored owned so the
        // borrowed `SystemPromptView` can reference it.
        if !has_identity_file {
            raw.push((BootstrapFile::Identity, docs.identity.render_directives()));
            // Keep canonical injection order (Identity sits right after Soul).
            raw.sort_by_key(|(file, _)| BootstrapFile::ALL.iter().position(|f| f == file));
        }

        // Persist the (possibly updated) cache table so the next open is fast.
        if let Ok(mut state) = WorkspaceState::load(guard.root()) {
            state.cache = cache.export_records();
            // best-effort: a read-only workspace may reject the write.
            let _ = state.save(guard.root());
        }

        Ok(Self {
            inner: Arc::new(Inner {
                root,
                guard,
                cache,
                docs,
                raw,
                session_kind: opts.session_kind,
            }),
        })
    }

    /// Seed the workspace file-map, then open the result.
    ///
    /// # Errors
    /// [`HomeError`] on seeding or opening failure.
    pub fn onboard(opts: &OnboardOptions) -> Result<Self, HomeError> {
        seed_workspace(opts)?;
        Self::open(HomeOptions {
            workspace: Some(opts.workspace.clone()),
            strict_soul: false,
            ..HomeOptions::default()
        })
    }

    /// Resolved workspace root.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.inner.root
    }

    /// Parsed soul, if `SOUL.md` is present.
    #[must_use]
    pub fn soul(&self) -> Option<&Soul> {
        self.inner.docs.soul.as_ref()
    }

    /// Parsed identity (defaults to the neutral placeholder when absent).
    #[must_use]
    pub fn identity(&self) -> &Identity {
        &self.inner.docs.identity
    }

    /// Parsed user profile, if `USER.md` is present.
    #[must_use]
    pub fn user(&self) -> Option<&UserProfile> {
        self.inner.docs.user.as_ref()
    }

    /// Parsed tools doc, if `TOOLS.md` is present.
    #[must_use]
    pub fn tools(&self) -> Option<&ToolsDoc> {
        self.inner.docs.tools.as_ref()
    }

    /// Shared content-addressed cache.
    #[must_use]
    pub fn cache(&self) -> &FileCache {
        &self.inner.cache
    }

    /// The workspace boundary guard, for callers that need to read additional
    /// (non-bootstrap) workspace files with the same containment + size cap.
    #[must_use]
    pub fn guard(&self) -> &WorkspaceGuard {
        &self.inner.guard
    }

    /// Assemble the system prompt as a borrowed view (AH-5).
    ///
    /// The view borrows directly from the home's owned raw content for
    /// untruncated sections (zero-copy), filtered by the session kind: a
    /// subagent gets only operating rules + tools; a cron session also gets the
    /// heartbeat; an interactive session gets everything including the boot
    /// checklist.
    #[must_use]
    pub fn system_prompt(&self, budget: &BootstrapBudget) -> SystemPromptView<'_> {
        self.system_prompt_with_warning(budget, WarningMode::Once, &[])
    }

    /// Assemble with explicit warning dedup state.
    #[must_use]
    pub fn system_prompt_with_warning(
        &self,
        budget: &BootstrapBudget,
        mode: WarningMode,
        seen_signatures: &[String],
    ) -> SystemPromptView<'_> {
        let kind = self.inner.session_kind;
        let loaded: Vec<LoadedFile<'_>> = self
            .inner
            .raw
            .iter()
            .filter(|(file, _)| Self::file_allowed(*file, kind))
            .map(|(file, content)| LoadedFile {
                file: *file,
                path: self.inner.root.join(file.filename()).display().to_string(),
                content: Some(content.as_str()),
            })
            .collect();
        SystemPromptView::assemble_with_warning(&loaded, budget, mode, seen_signatures)
    }

    /// Session-kind allowlist (openclaw `filterBootstrapFilesForSession` +
    /// the heartbeat/boot gates).
    fn file_allowed(file: BootstrapFile, kind: SessionKind) -> bool {
        match file {
            // Always available.
            BootstrapFile::Agents | BootstrapFile::Tools => true,
            // Persona files: not for subagents.
            BootstrapFile::Soul
            | BootstrapFile::Identity
            | BootstrapFile::User
            | BootstrapFile::Memory => kind.wants_persona(),
            // Heartbeat: cron only.
            BootstrapFile::Heartbeat => kind.wants_heartbeat(),
            // Boot: interactive only.
            BootstrapFile::Boot => kind.wants_boot(),
            // The one-shot ritual is never auto-injected into the prompt.
            BootstrapFile::Bootstrap => false,
        }
    }

    /// Mark first-run setup complete and delete `BOOTSTRAP.md` (AH-4).
    ///
    /// # Errors
    /// [`HomeError`] on filesystem / state failure.
    pub fn complete_bootstrap(&self) -> Result<bool, HomeError> {
        crate::onboard::complete_bootstrap(&self.inner.root)
    }

    /// Git-backup the workspace via `gix` (feature `git`, host-only).
    ///
    /// # Errors
    /// [`HomeError::Git`] on any git failure.
    #[cfg(all(not(target_arch = "wasm32"), feature = "git"))]
    pub fn git_backup(&self, message: &str) -> Result<crate::git::BackupOutcome, HomeError> {
        crate::git::backup(&self.inner.root, message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn agent_home_is_send_sync() {
        assert_send_sync::<AgentHome>();
    }

    #[test]
    fn onboard_then_open_round_trips() {
        let td = tempdir().unwrap();
        let ws = td.path().join("workspace");
        let home = AgentHome::onboard(&OnboardOptions::new(&ws)).unwrap();
        // Seeded identity parses to the neutral placeholder name.
        assert_eq!(home.identity().name, "aphrody");
        // Soul present + parsed.
        assert!(home.soul().is_some());
        assert!(home.tools().is_some());
        assert!(home.user().is_some());
    }

    #[test]
    fn system_prompt_contains_persona() {
        let td = tempdir().unwrap();
        let ws = td.path().join("workspace");
        let home = AgentHome::onboard(&OnboardOptions::new(&ws)).unwrap();
        let view = home.system_prompt(&BootstrapBudget::default());
        let prompt = view.render();
        // The seeded IDENTITY.md body identifies the agent as aphrody.
        assert!(prompt.contains("Identité (IDENTITY.md)"), "identity section missing:\n{prompt}");
        assert!(prompt.contains("aphrody"), "agent name missing");
        assert!(prompt.contains("Règles d'opération"), "agents missing");
        assert!(prompt.contains("agent d'ingénierie souverain"), "soul body missing");
    }

    #[test]
    fn subagent_session_drops_persona() {
        let td = tempdir().unwrap();
        let ws = td.path().join("workspace");
        AgentHome::onboard(&OnboardOptions::new(&ws)).unwrap();
        let home = AgentHome::open(HomeOptions {
            workspace: Some(ws),
            session_kind: SessionKind::Subagent,
            ..HomeOptions::default()
        })
        .unwrap();
        let prompt = home.system_prompt(&BootstrapBudget::default()).render();
        // Subagent gets rules + tools, never the identity/soul persona.
        assert!(prompt.contains("Règles d'opération"));
        assert!(!prompt.contains("Identité (IDENTITY.md)"), "subagent must not see identity");
    }

    #[test]
    fn cron_session_includes_heartbeat() {
        let td = tempdir().unwrap();
        let ws = td.path().join("workspace");
        AgentHome::onboard(&OnboardOptions::new(&ws)).unwrap();
        let home = AgentHome::open(HomeOptions {
            workspace: Some(ws),
            session_kind: SessionKind::Cron,
            ..HomeOptions::default()
        })
        .unwrap();
        let prompt = home.system_prompt(&BootstrapBudget::default()).render();
        assert!(prompt.contains("Checklist de heartbeat"));
        // Cron does not get the boot checklist.
        assert!(!prompt.contains("Checklist de démarrage"));
    }

    #[test]
    fn open_missing_workspace_yields_minimal_default() {
        let td = tempdir().unwrap();
        let ws = td.path().join("does-not-exist");
        let home = AgentHome::open(HomeOptions {
            workspace: Some(ws),
            ..HomeOptions::default()
        })
        .unwrap();
        assert_eq!(home.identity().name, "aphrody");
        assert!(home.soul().is_none());
        let prompt = home.system_prompt(&BootstrapBudget::default()).render();
        // Default identity always renders.
        assert!(prompt.contains("Tu es aphrody"));
    }

    #[test]
    fn strict_soul_rejects_bad_persona() {
        let td = tempdir().unwrap();
        let ws = td.path().join("workspace");
        std::fs::create_dir_all(&ws).unwrap();
        std::fs::write(ws.join("SOUL.md"), "---\ntone: x\n---\n## v1.0.0\n- shipped\n").unwrap();
        let err = AgentHome::open(HomeOptions {
            workspace: Some(ws.clone()),
            strict_soul: true,
            ..HomeOptions::default()
        })
        .unwrap_err();
        assert!(matches!(err, HomeError::SoulValidation(_)));
        // Lenient open accepts it.
        let home = AgentHome::open(HomeOptions {
            workspace: Some(ws),
            strict_soul: false,
            ..HomeOptions::default()
        })
        .unwrap();
        assert!(home.soul().is_some());
    }
}
