// SPDX-License-Identifier: Apache-2.0
//! Canonical workspace file-map.
//!
//! Verbatim port of `var/openclaw/src/agents/workspace.ts:21-40` into the
//! aphrody namespace. These names are the single source of truth for both the
//! seeding (AH-4) and the loading/assembly (AH-5) paths, so the two can never
//! drift. Constants mirror the upstream `MAX_WORKSPACE_BOOTSTRAP_FILE_BYTES`
//! and `WORKSPACE_STATE_*` values exactly.

/// Operating rules / behaviour. Always injected (never optional).
pub const AGENTS_FILENAME: &str = "AGENTS.md";
/// Persona: tone, opinions, brevity, humour, boundaries, bluntness.
pub const SOUL_FILENAME: &str = "SOUL.md";
/// Local tool conventions.
pub const TOOLS_FILENAME: &str = "TOOLS.md";
/// Agent name / vibe / emoji.
pub const IDENTITY_FILENAME: &str = "IDENTITY.md";
/// User identity + how to address them.
pub const USER_FILENAME: &str = "USER.md";
/// Heartbeat-run checklist (optional).
pub const HEARTBEAT_FILENAME: &str = "HEARTBEAT.md";
/// Boot-run checklist (optional).
pub const BOOT_FILENAME: &str = "BOOT.md";
/// One-shot first-run ritual (deleted after setup completes).
pub const BOOTSTRAP_FILENAME: &str = "BOOTSTRAP.md";
/// Long-term curated memory (optional).
pub const MEMORY_FILENAME: &str = "MEMORY.md";

/// Per-file bootstrap byte cap (`workspace.ts:40`: `2 * 1024 * 1024`).
pub const MAX_WORKSPACE_BOOTSTRAP_FILE_BYTES: u64 = 2 * 1024 * 1024;

/// State directory under the workspace (`workspace.ts:29`).
pub const WORKSPACE_STATE_DIRNAME: &str = ".aphrody";
/// State filename (`workspace.ts:30`).
pub const WORKSPACE_STATE_FILENAME: &str = "workspace-state.json";
/// State schema version (`workspace.ts:31`).
pub const WORKSPACE_STATE_VERSION: u32 = 1;

/// The subdirectory that holds daily memory logs (`memory/YYYY-MM-DD.md`).
pub const MEMORY_DIRNAME: &str = "memory";
/// Workspace-local skills directory (consumed by `aphrody-skills`).
pub const SKILLS_DIRNAME: &str = "skills";

/// A recognised bootstrap file. The variant order is the deterministic
/// injection order used by the system-prompt assembler (AH-5): operating
/// rules first, then persona, then identity/user/tools, then the optional
/// run checklists, then the one-shot bootstrap ritual, then curated memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BootstrapFile {
    /// `AGENTS.md`
    Agents,
    /// `SOUL.md`
    Soul,
    /// `IDENTITY.md`
    Identity,
    /// `USER.md`
    User,
    /// `TOOLS.md`
    Tools,
    /// `HEARTBEAT.md`
    Heartbeat,
    /// `BOOT.md`
    Boot,
    /// `BOOTSTRAP.md`
    Bootstrap,
    /// `MEMORY.md`
    Memory,
}

impl BootstrapFile {
    /// Every bootstrap file in deterministic injection order.
    pub const ALL: [BootstrapFile; 9] = [
        BootstrapFile::Agents,
        BootstrapFile::Soul,
        BootstrapFile::Identity,
        BootstrapFile::User,
        BootstrapFile::Tools,
        BootstrapFile::Heartbeat,
        BootstrapFile::Boot,
        BootstrapFile::Bootstrap,
        BootstrapFile::Memory,
    ];

    /// On-disk filename for this bootstrap file.
    #[must_use]
    pub const fn filename(self) -> &'static str {
        match self {
            BootstrapFile::Agents => AGENTS_FILENAME,
            BootstrapFile::Soul => SOUL_FILENAME,
            BootstrapFile::Identity => IDENTITY_FILENAME,
            BootstrapFile::User => USER_FILENAME,
            BootstrapFile::Tools => TOOLS_FILENAME,
            BootstrapFile::Heartbeat => HEARTBEAT_FILENAME,
            BootstrapFile::Boot => BOOT_FILENAME,
            BootstrapFile::Bootstrap => BOOTSTRAP_FILENAME,
            BootstrapFile::Memory => MEMORY_FILENAME,
        }
    }

    /// Section header rendered into the system prompt for this file.
    #[must_use]
    pub const fn section_title(self) -> &'static str {
        match self {
            BootstrapFile::Agents => "Operating rules (AGENTS.md)",
            BootstrapFile::Soul => "Soul (SOUL.md)",
            BootstrapFile::Identity => "Identity (IDENTITY.md)",
            BootstrapFile::User => "User (USER.md)",
            BootstrapFile::Tools => "Tools (TOOLS.md)",
            BootstrapFile::Heartbeat => "Heartbeat checklist (HEARTBEAT.md)",
            BootstrapFile::Boot => "Boot checklist (BOOT.md)",
            BootstrapFile::Bootstrap => "First-run ritual (BOOTSTRAP.md)",
            BootstrapFile::Memory => "Curated memory (MEMORY.md)",
        }
    }

    /// Whether the file is optional in onboarding (skippable, no warning if
    /// missing). Mirrors `workspace.ts` `OPTIONAL_BOOTSTRAP_FILENAMES`
    /// extended with the run checklists.
    #[must_use]
    pub const fn is_optional(self) -> bool {
        matches!(
            self,
            BootstrapFile::Soul
                | BootstrapFile::Identity
                | BootstrapFile::User
                | BootstrapFile::Heartbeat
                | BootstrapFile::Boot
                | BootstrapFile::Memory
        )
    }

    /// Match a basename (case-insensitive) back to a [`BootstrapFile`].
    #[must_use]
    pub fn from_basename(name: &str) -> Option<Self> {
        BootstrapFile::ALL
            .into_iter()
            .find(|f| f.filename().eq_ignore_ascii_case(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_filenames_are_unique_and_round_trip() {
        let mut seen = std::collections::BTreeSet::new();
        for f in BootstrapFile::ALL {
            assert!(seen.insert(f.filename()), "duplicate filename {f:?}");
            assert_eq!(BootstrapFile::from_basename(f.filename()), Some(f));
        }
        assert_eq!(seen.len(), 9);
    }

    #[test]
    fn from_basename_is_case_insensitive() {
        assert_eq!(BootstrapFile::from_basename("soul.md"), Some(BootstrapFile::Soul));
        assert_eq!(BootstrapFile::from_basename("AGENTS.MD"), Some(BootstrapFile::Agents));
        assert_eq!(BootstrapFile::from_basename("nope.md"), None);
    }

    #[test]
    fn cap_matches_openclaw_two_mib() {
        assert_eq!(MAX_WORKSPACE_BOOTSTRAP_FILE_BYTES, 2_097_152);
    }

    #[test]
    fn agents_and_tools_are_required() {
        assert!(!BootstrapFile::Agents.is_optional());
        assert!(!BootstrapFile::Tools.is_optional());
        assert!(BootstrapFile::Soul.is_optional());
    }
}
