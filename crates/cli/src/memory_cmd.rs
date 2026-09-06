// SPDX-License-Identifier: Apache-2.0
// `aphrody memory migrate` — Tier-1 provider-to-provider migration (R3.4 wire).
//
// Wraps the dyn-compatible `aphrody_memory::migrate` lib API into a CLI surface
// that callers can drive without writing Rust. The four shipped Tier-1
// providers (Mem0, Honcho, SqliteLocal, LanceDb) all implement `MemoryProvider`,
// so any pair can be wired up — HTTP creds come from environment variables
// documented in the per-provider modules.
//
// Verify: `aphrody memory migrate --from lancedb --to honcho
//         --from-lancedb-path ./var/memory --lancedb-ndims 768
//         --agent-id agent-A --dry-run` returns a MigrationDiff JSON.

use std::path::{Path, PathBuf};

#[cfg(feature = "memory-lancedb")] use aphrody_memory::LanceDbAdapter;
use aphrody_memory::{
    HonchoProvider, Mem0Provider, MemoryProvider, SqliteLocalProvider, migrate_provider,
};
use async_trait::async_trait;
use miette::{IntoDiagnostic, WrapErr};

use crate::context::{GoogleContext, TerminalCommand};

/// Picker mirroring the four Tier-1 `MemoryProvider` implementations.
///
/// The kebab-case rename matches `aphrody_memory::ProviderKind` snake_case
/// (`mem0`, `honcho`, `sqlite_local`, `lancedb`) so the CLI surface stays
/// consistent with the discriminator returned by `provider_kind()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum MemoryProviderArg {
    /// Hosted Mem0 cloud — reads `MEM0_API_KEY` from the environment.
    Mem0,
    /// Hosted Honcho v1 — reads `HONCHO_API_KEY` from the environment.
    Honcho,
    /// Offline rusqlite store — `--from-sqlite-path` / `--to-sqlite-path`.
    SqliteLocal,
    /// Embedded LanceDB vector store — `--from-lancedb-path` / `--to-lancedb-path`
    /// with `--lancedb-ndims`. Records are stored with `ns = agent_id`;
    /// embeddings are preserved on copies within LanceDB and dropped (set to
    /// `None`) when writing to HTTP-backed providers that have no vector column.
    LanceDb,
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
    /// Filesystem path for the source LanceDB store (lancedb only).
    pub from_lancedb_path: Option<PathBuf>,
    /// Filesystem path for the target LanceDB store (lancedb only).
    pub to_lancedb_path: Option<PathBuf>,
    /// Embedding dimensionality for LanceDB backends (required when either end
    /// is `lancedb`; defaults to 768 which matches standard embedding models).
    pub lancedb_ndims: usize,
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
                "memory migrate: when --from and --to are both sqlite-local, --from-sqlite-path \
                 and --to-sqlite-path must point to distinct files (refusing to copy a sqlite \
                 onto itself)"
            ));
        }
        if self.from == MemoryProviderArg::LanceDb
            && self.to == MemoryProviderArg::LanceDb
            && self.from_lancedb_path == self.to_lancedb_path
        {
            return Err(miette::miette!(
                "memory migrate: when --from and --to are both lancedb, --from-lancedb-path and \
                 --to-lancedb-path must point to distinct directories (refusing to copy a lancedb \
                 store onto itself)"
            ));
        }

        let from: Box<dyn MemoryProvider> = build_provider(
            self.from,
            self.from_sqlite_path.as_deref(),
            self.from_lancedb_path.as_deref(),
            self.lancedb_ndims,
        )
        .await
        .wrap_err("failed to build --from provider")?;
        let to: Box<dyn MemoryProvider> = build_provider(
            self.to,
            self.to_sqlite_path.as_deref(),
            self.to_lancedb_path.as_deref(),
            self.lancedb_ndims,
        )
        .await
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
            println!("  source   : {} records", diff.source_count);
            println!("  target   : {} records before", diff.target_count_before);
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

/// Build a `Box<dyn MemoryProvider>` from a CLI selector + optional backend paths.
///
/// HTTP providers read their credentials from env vars (documented per module
/// in `aphrody-memory`); failure to find a key surfaces as a structured
/// `MemoryError::MissingConfig` that bubbles up here as a `miette::Report`.
///
/// `lancedb_ndims` is only used when `kind == LanceDb`.
async fn build_provider(
    kind: MemoryProviderArg,
    sqlite_path: Option<&Path>,
    lancedb_path: Option<&Path>,
    lancedb_ndims: usize,
) -> miette::Result<Box<dyn MemoryProvider>> {
    match kind {
        MemoryProviderArg::Mem0 => {
            let p = Mem0Provider::from_env().map_err(|e| miette::miette!("mem0: {e}"))?;
            Ok(Box::new(p))
        },
        MemoryProviderArg::Honcho => {
            let p = HonchoProvider::from_env().map_err(|e| miette::miette!("honcho: {e}"))?;
            Ok(Box::new(p))
        },
        MemoryProviderArg::SqliteLocal => {
            let path = sqlite_path.map(Path::to_path_buf).unwrap_or_else(default_sqlite_path);
            // Ensure parent dir exists so `~/.aphrody/memory.sqlite` works
            // on a fresh install without `mkdir -p` first.
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent).into_diagnostic().wrap_err_with(|| {
                        format!("failed to create parent dir {}", parent.display())
                    })?;
                }
            }
            let p = SqliteLocalProvider::open(&path)
                .map_err(|e| miette::miette!("sqlite_local: {e} (path={})", path.display()))?;
            Ok(Box::new(p))
        },
        MemoryProviderArg::LanceDb => {
            #[cfg(not(feature = "memory-lancedb"))]
            {
                let _ = (lancedb_path, lancedb_ndims);
                return Err(miette::miette!(
                    "LanceDB is optional; rebuild Aphrody with --features memory-lancedb"
                ));
            }
            #[cfg(feature = "memory-lancedb")]
            {
                let path = lancedb_path.map(Path::to_path_buf).unwrap_or_else(default_lancedb_path);
                let ndims = if lancedb_ndims == 0 { 768 } else { lancedb_ndims };
                // Ensure directory exists before LanceDB tries to create its layout.
                if let Some(parent) = path.parent() {
                    if !parent.as_os_str().is_empty() {
                        std::fs::create_dir_all(parent).into_diagnostic().wrap_err_with(|| {
                            format!("failed to create lancedb parent dir {}", parent.display())
                        })?;
                    }
                }
                let p = LanceDbAdapter::open(&path, ndims)
                    .await
                    .map_err(|e| miette::miette!("lancedb: {e} (path={})", path.display()))?;
                Ok(Box::new(p))
            }
        },
    }
}

/// Default offline SQLite store location — `$HOME/.aphrody/memory.sqlite`.
/// Falls back to `./memory.sqlite` if the home directory cannot be resolved.
fn default_sqlite_path() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".aphrody").join("memory.sqlite"))
        .unwrap_or_else(|| PathBuf::from("memory.sqlite"))
}

/// Default LanceDB store location — `$HOME/.aphrody/memory-lancedb/`.
/// Falls back to `./memory-lancedb/` if the home directory cannot be resolved.
#[cfg(any(feature = "memory-lancedb", test))]
fn default_lancedb_path() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".aphrody").join("memory-lancedb"))
        .unwrap_or_else(|| PathBuf::from("memory-lancedb"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_sqlite_path_under_home_when_available() {
        let p = default_sqlite_path();
        assert_eq!(p.file_name().and_then(|s| s.to_str()), Some("memory.sqlite"));
    }

    #[test]
    fn default_lancedb_path_under_home_when_available() {
        let p = default_lancedb_path();
        assert_eq!(p.file_name().and_then(|s| s.to_str()), Some("memory-lancedb"));
    }

    fn make_cmd(
        from: MemoryProviderArg,
        to: MemoryProviderArg,
        agent_id: &str,
        from_sqlite_path: Option<PathBuf>,
        to_sqlite_path: Option<PathBuf>,
        from_lancedb_path: Option<PathBuf>,
        to_lancedb_path: Option<PathBuf>,
    ) -> MigrateCommand {
        MigrateCommand {
            from,
            to,
            agent_id: agent_id.into(),
            dry_run: true,
            json: true,
            pretty: false,
            from_sqlite_path,
            to_sqlite_path,
            from_lancedb_path,
            to_lancedb_path,
            lancedb_ndims: 768,
        }
    }

    #[tokio::test]
    async fn migrate_rejects_empty_agent_id() {
        let cmd = make_cmd(
            MemoryProviderArg::SqliteLocal,
            MemoryProviderArg::SqliteLocal,
            "   ",
            Some(PathBuf::from("a.db")),
            Some(PathBuf::from("b.db")),
            None,
            None,
        );
        assert!(cmd.agent_id.trim().is_empty());
    }

    #[test]
    fn migrate_rejects_same_sqlite_path_both_ends() {
        let same = PathBuf::from("only.db");
        let cmd = make_cmd(
            MemoryProviderArg::SqliteLocal,
            MemoryProviderArg::SqliteLocal,
            "agent-A",
            Some(same.clone()),
            Some(same),
            None,
            None,
        );
        let both_sqlite =
            cmd.from == MemoryProviderArg::SqliteLocal && cmd.to == MemoryProviderArg::SqliteLocal;
        assert!(both_sqlite);
        assert_eq!(cmd.from_sqlite_path, cmd.to_sqlite_path);
    }

    #[test]
    fn migrate_rejects_same_lancedb_path_both_ends() {
        let same = PathBuf::from("./data/lancedb");
        let cmd = make_cmd(
            MemoryProviderArg::LanceDb,
            MemoryProviderArg::LanceDb,
            "agent-A",
            None,
            None,
            Some(same.clone()),
            Some(same),
        );
        let both_lance =
            cmd.from == MemoryProviderArg::LanceDb && cmd.to == MemoryProviderArg::LanceDb;
        assert!(both_lance);
        assert_eq!(cmd.from_lancedb_path, cmd.to_lancedb_path);
    }
}
