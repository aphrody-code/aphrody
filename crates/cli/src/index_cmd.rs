// SPDX-License-Identifier: Apache-2.0
//! `aphrody index …` — persistent local-filesystem search (SQLite FTS5).
//!
//! Backed by the `aphrody-fsindex` crate (mirrors the Google desktop app's
//! `local_files.db`): `build` walks a root and persists a metadata-only index;
//! `search` runs an FTS5 prefix query. Opt-in (`--features index`) because it
//! links `rusqlite` with the bundled libsqlite3 C source (host-only, not wasm).

use std::path::PathBuf;

use aphrody_fsindex::FsIndex;

/// Actions for the `index` subcommand.
#[derive(clap::Subcommand, Debug, Clone)]
pub(crate) enum IndexAction {
    /// Walk a directory and (re)build the local-file index. Metadata only —
    /// file contents are never read.
    ///
    /// Example: aphrody index build --root ~/projects
    Build {
        /// Directory to index recursively.
        #[arg(long)]
        root: PathBuf,
        /// Index database path (default: `~/.aphrody/fsindex.db`).
        #[arg(long)]
        db: Option<PathBuf>,
    },
    /// Full-text search the index by file name / path (FTS5 prefix match).
    ///
    /// Example: aphrody index search "readme" --limit 20
    Search {
        /// Query terms (quoted + prefix-matched).
        query: String,
        /// Index database path (default: `~/.aphrody/fsindex.db`).
        #[arg(long)]
        db: Option<PathBuf>,
        /// Max results (default 25).
        #[arg(long, default_value_t = 25)]
        limit: usize,
        /// Emit JSON instead of a plain path list.
        #[arg(long)]
        json: bool,
    },
}

/// Default index location: `~/.aphrody/fsindex.db`.
fn default_db() -> miette::Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| miette::miette!("cannot resolve home directory for the index db"))?;
    Ok(home.join(".aphrody").join("fsindex.db"))
}

/// Run an `index` action.
///
/// # Errors
///
/// Returns a `miette` report on IO / SQLite / index failures.
pub(crate) async fn run(action: IndexAction) -> miette::Result<()> {
    match action {
        IndexAction::Build { root, db } => {
            let db_path = match db {
                Some(p) => p,
                None => default_db()?,
            };
            if let Some(parent) = db_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| miette::miette!("create index dir: {e}"))?;
            }
            let mut index =
                FsIndex::open(&db_path).map_err(|e| miette::miette!("open index: {e}"))?;
            let count = index.build(&root).map_err(|e| miette::miette!("build index: {e}"))?;
            println!("indexed {count} entries from {} into {}", root.display(), db_path.display());
            Ok(())
        },
        IndexAction::Search { query, db, limit, json } => {
            let db_path = match db {
                Some(p) => p,
                None => default_db()?,
            };
            let index = FsIndex::open(&db_path).map_err(|e| miette::miette!("open index: {e}"))?;
            let hits =
                index.search(&query, limit).map_err(|e| miette::miette!("search index: {e}"))?;
            if json {
                let rendered = serde_json::to_string_pretty(&hits)
                    .map_err(|e| miette::miette!("encode results: {e}"))?;
                println!("{rendered}");
            } else if hits.is_empty() {
                println!("(no matches for {query:?})");
            } else {
                for hit in &hits {
                    println!("{}", hit.path);
                }
            }
            Ok(())
        },
    }
}
