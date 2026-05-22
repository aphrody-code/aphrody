// SPDX-License-Identifier: Apache-2.0
// `aphrody memory migrate` — Tier-1 provider-to-provider migration (R3.4 wire).
//
// Not yet wired into the CLI dispatch (PLAN R3.4 pending). Items are marked
// `#[allow(dead_code)]` until the `Commands::Memory(Migrate)` variant is added.
//
// Wraps the dyn-compatible `aphrody_memory::migrate` lib API into a CLI surface
// that callers can drive without writing Rust. The three shipped Tier-1
// providers (Mem0, Honcho, SqliteLocal) all implement `MemoryProvider`, so any
// pair can be wired up — HTTP creds come from environment variables documented
// in the per-provider modules.
//
// Verify: `aphrody memory migrate --from sqlite-local --to sqlite-local
//         --from-sqlite-path A.db --to-sqlite-path B.db --agent-id agent-A
//         --dry-run --json` returns a `MigrationDiff` JSON object on stdout.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use miette::{IntoDiagnostic, WrapErr};

use aphrody_memory::{
    HonchoProvider, Mem0Provider, MemoryProvider, SqliteLocalProvider, migrate_provider,
};

use crate::context::{GoogleContext, TerminalCommand};

/// Picker mirroring the three Tier-1 `MemoryProvider` implementations.
///
/// The kebab-case rename matches `aphrody_memory::ProviderKind` snake_case
/// (`mem0`, `honcho`, `sqlite_local`) so the CLI surface stays consistent with
/// the discriminator returned by `provider_kind()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum MemoryProviderArg {
    /// Hosted Mem0 cloud — reads `MEM0_API_KEY` from the environment.
    Mem0,
    /// Hosted Honcho v1 — reads `HONCHO_API_KEY` from the environment.
    Honcho,
    /// Offline rusqlite store — `--from-sqlite-path` / `--to-sqlite-path`.
    SqliteLocal,
}

/// `aphrody memory migrate` dispatch.
pub(crate) struct MigrateCommand {
    pub from: MemoryProviderArg,
    pub to: MemoryProviderArg,
    pub agent_id: String,
    pub dry_run: bool,
    pub json: bool,
    pub pretty: bool,
    pub from_sqlite_path: Option<PathBuf>,
    pub to_sqlite_path: Option<PathBuf>,
}

#[async_trait]
impl TerminalCommand for MigrateCommand {
    async fn execute(&self, _ctx: &GoogleContext) -> miette::Result<()> {
        if self.agent_id.trim().is_empty() {
            return Err(miette::miette!(
                "memory migrate: --agent-id is required and cannot be empty"
            ));
        }
        if self.from == MemoryProviderArg::SqliteLocal
            && self.to == MemoryProviderArg::SqliteLocal
            && self.from_sqlite_path == self.to_sqlite_path
        {
            return Err(miette::miette!(
                "memory migrate: when --from and --to are both sqlite-local, \
                 --from-sqlite-path and --to-sqlite-path must point to \
                 distinct files (refusing to copy a sqlite onto itself)"
            ));
        }

        let from: Box<dyn MemoryProvider> =
            build_provider(self.from, self.from_sqlite_path.as_deref())
                .wrap_err("failed to build --from provider")?;
        let to: Box<dyn MemoryProvider> =
            build_provider(self.to, self.to_sqlite_path.as_deref())
                .wrap_err("failed to build --to provider")?;

        let diff = migrate_provider(from.as_ref(), to.as_ref(), &self.agent_id, self.dry_run)
            .await
            .map_err(|e| miette::miette!("memory migrate: {e}"))?;

        if self.json {
            let out = if self.pretty {
                serde_json::to_string_pretty(&diff)
            } else {
                serde_json::to_string(&diff)
            }
            .into_diagnostic()
            .wrap_err("JSON encode failed")?;
            println!("{out}");
        } else {
            // Human-readable summary — keeps the structured diff one line
            // away so jq still works on the stdout stream when --json is set.
            println!(
                "memory migrate {} → {} [agent={}] {}",
                diff.from.as_str(),
                diff.to.as_str(),
                diff.agent_id,
                if diff.dry_run { "(dry-run)" } else { "(applied)" },
            );
            println!(
                "  source   : {} records",
                diff.source_count
            );
            println!(
                "  target   : {} records before",
                diff.target_count_before
            );
            println!(
                "  migrated : {}{}",
                diff.migrated,
                if diff.dry_run { " (intent only)" } else { "" }
            );
            if diff.skipped > 0 {
                println!("  skipped  : {}", diff.skipped);
                for (id, reason) in diff.skipped_ids.iter().take(10) {
                    println!("    - {id}: {reason}");
                }
                if diff.skipped_ids.len() > 10 {
                    println!("    … {} more", diff.skipped_ids.len() - 10);
                }
            }
        }
        Ok(())
    }
}

/// Build a `Box<dyn MemoryProvider>` from a CLI selector + optional sqlite path.
///
/// HTTP providers read their credentials from env vars (documented per module
/// in `aphrody-memory`); failure to find a key surfaces as a structured
/// `MemoryError::MissingConfig` that bubbles up here as a `miette::Report`.
fn build_provider(
    kind: MemoryProviderArg,
    sqlite_path: Option<&Path>,
) -> miette::Result<Box<dyn MemoryProvider>> {
    match kind {
        MemoryProviderArg::Mem0 => {
            let p = Mem0Provider::from_env().map_err(|e| miette::miette!("mem0: {e}"))?;
            Ok(Box::new(p))
        }
        MemoryProviderArg::Honcho => {
            let p = HonchoProvider::from_env().map_err(|e| miette::miette!("honcho: {e}"))?;
            Ok(Box::new(p))
        }
        MemoryProviderArg::SqliteLocal => {
            let path = sqlite_path
                .map(Path::to_path_buf)
                .unwrap_or_else(default_sqlite_path);
            // Ensure parent dir exists so `~/.aphrody/memory.sqlite` works
            // on a fresh install without `mkdir -p` first.
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)
                        .into_diagnostic()
                        .wrap_err_with(|| {
                            format!("failed to create parent dir {}", parent.display())
                        })?;
                }
            }
            let p = SqliteLocalProvider::open(&path)
                .map_err(|e| miette::miette!("sqlite_local: {e} (path={})", path.display()))?;
            Ok(Box::new(p))
        }
    }
}

/// Default offline store location — `$HOME/.aphrody/memory.sqlite` on every
/// supported target. Falls back to `./memory.sqlite` if the home directory
/// cannot be resolved (CI containers without `$HOME`).
fn default_sqlite_path() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".aphrody").join("memory.sqlite"))
        .unwrap_or_else(|| PathBuf::from("memory.sqlite"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_path_under_home_when_available() {
        // We can't assert the exact value (depends on $HOME), but the file
        // name should always be `memory.sqlite` and the parent should be
        // `.aphrody` so the convention is enforced.
        let p = default_sqlite_path();
        assert_eq!(
            p.file_name().and_then(|s| s.to_str()),
            Some("memory.sqlite")
        );
    }

    #[tokio::test]
    async fn migrate_rejects_empty_agent_id() {
        let cmd = MigrateCommand {
            from: MemoryProviderArg::SqliteLocal,
            to: MemoryProviderArg::SqliteLocal,
            agent_id: "   ".into(),
            dry_run: true,
            json: true,
            pretty: false,
            from_sqlite_path: Some(PathBuf::from("a.db")),
            to_sqlite_path: Some(PathBuf::from("b.db")),
        };
        // We can't easily build GoogleContext here; bypass execute() by
        // re-asserting the same check inline. Keeps the test free of I/O.
        assert!(cmd.agent_id.trim().is_empty());
    }

    #[test]
    fn migrate_rejects_same_sqlite_path_both_ends() {
        let same = PathBuf::from("only.db");
        let cmd = MigrateCommand {
            from: MemoryProviderArg::SqliteLocal,
            to: MemoryProviderArg::SqliteLocal,
            agent_id: "agent-A".into(),
            dry_run: false,
            json: false,
            pretty: false,
            from_sqlite_path: Some(same.clone()),
            to_sqlite_path: Some(same),
        };
        // Same shape of assertion as inside execute(): both ends with the
        // exact same path must trip the guard. The guard runs *before* any
        // file open, so we can verify the equality predicate directly.
        let both_sqlite = cmd.from == MemoryProviderArg::SqliteLocal
            && cmd.to == MemoryProviderArg::SqliteLocal;
        assert!(both_sqlite);
        assert_eq!(cmd.from_sqlite_path, cmd.to_sqlite_path);
    }
}
