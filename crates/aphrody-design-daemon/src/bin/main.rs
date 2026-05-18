// SPDX-License-Identifier: Apache-2.0
//! `aphrody-design-daemon` — CLI front-end for the SQLite project store and
//! the design-systems registry.
//!
//! Every subcommand prints JSON on stdout so the daemon can be driven from
//! shells, agents, or other Rust binaries without ad-hoc parsing.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::Serialize;

use aphrody_design_daemon::{
    ConversationId, ProjectId, ProjectStore, Role,
    registry::DesignSystemRegistry,
};

#[derive(Parser, Debug)]
#[command(
    name = "aphrody-design-daemon",
    about = "SQLite project store + design-systems registry daemon",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: TopCmd,
}

#[derive(Subcommand, Debug)]
enum TopCmd {
    /// SQLite project-store operations.
    Db {
        #[command(subcommand)]
        action: DbCmd,
    },
    /// Design-systems registry operations.
    Registry {
        #[command(subcommand)]
        action: RegistryCmd,
    },
}

#[derive(Subcommand, Debug)]
enum DbCmd {
    /// Initialise (or re-open) the SQLite database. Idempotent.
    Init {
        /// Path to the SQLite file. Defaults to `$XDG_DATA_HOME/aphrody/design-daemon/app.sqlite`.
        #[arg(long, env = "APHRODY_DESIGN_DB")]
        path: Option<PathBuf>,
    },
    Project {
        #[command(subcommand)]
        action: ProjectCmd,
    },
    Conversation {
        #[command(subcommand)]
        action: ConversationCmd,
    },
    Message {
        #[command(subcommand)]
        action: MessageCmd,
    },
}

#[derive(Subcommand, Debug)]
enum ProjectCmd {
    /// Create a new project row.
    Create {
        name: String,
        /// Working directory the project belongs to. Defaults to the
        /// current cwd if omitted.
        #[arg(long)]
        cwd: Option<PathBuf>,
        #[arg(long, env = "APHRODY_DESIGN_DB")]
        path: Option<PathBuf>,
    },
    /// List every project.
    List {
        #[arg(long, env = "APHRODY_DESIGN_DB")]
        path: Option<PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
enum ConversationCmd {
    /// Create a new conversation under a project.
    Create {
        project_id: String,
        title: String,
        #[arg(long, env = "APHRODY_DESIGN_DB")]
        path: Option<PathBuf>,
    },
    /// List every conversation for a project.
    List {
        project_id: String,
        #[arg(long, env = "APHRODY_DESIGN_DB")]
        path: Option<PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
enum MessageCmd {
    /// Append a message to a conversation.
    Append {
        conversation_id: String,
        /// `user` / `assistant` / `system` / `tool`.
        role: String,
        content: String,
        #[arg(long, env = "APHRODY_DESIGN_DB")]
        path: Option<PathBuf>,
    },
    /// List messages in a conversation.
    List {
        conversation_id: String,
        #[arg(long, default_value_t = 100)]
        limit: usize,
        #[arg(long, env = "APHRODY_DESIGN_DB")]
        path: Option<PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
enum RegistryCmd {
    /// Load and dump the registry from a directory.
    Load {
        dir: PathBuf,
    },
    /// Same as `load`, kept for symmetry — prints a flat list.
    List {
        dir: PathBuf,
    },
}

fn default_db_path() -> Result<PathBuf> {
    if let Some(base) = dirs::data_dir() {
        Ok(base.join("aphrody").join("design-daemon").join("app.sqlite"))
    } else {
        Ok(PathBuf::from("./aphrody-design-daemon.sqlite"))
    }
}

fn resolve_path(opt: Option<PathBuf>) -> Result<PathBuf> {
    match opt {
        Some(p) => Ok(p),
        None => default_db_path(),
    }
}

fn open_store(path: Option<PathBuf>) -> Result<(PathBuf, ProjectStore)> {
    let p = resolve_path(path)?;
    let store = ProjectStore::open(&p)
        .with_context(|| format!("open project store at {}", p.display()))?;
    Ok((p, store))
}

#[derive(Serialize)]
struct InitResult {
    path: String,
    initialized: bool,
}

fn print_json<T: Serialize>(value: &T) -> Result<()> {
    let s = serde_json::to_string_pretty(value)?;
    println!("{s}");
    Ok(())
}

fn main() -> Result<()> {
    // Lightweight tracing — INFO+ on stderr so JSON-on-stdout stays clean.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    match cli.command {
        TopCmd::Db { action } => handle_db(action),
        TopCmd::Registry { action } => handle_registry(action),
    }
}

fn handle_db(action: DbCmd) -> Result<()> {
    match action {
        DbCmd::Init { path } => {
            let (p, _store) = open_store(path)?;
            print_json(&InitResult {
                path: p.display().to_string(),
                initialized: true,
            })
        }
        DbCmd::Project { action } => handle_project(action),
        DbCmd::Conversation { action } => handle_conversation(action),
        DbCmd::Message { action } => handle_message(action),
    }
}

fn handle_project(action: ProjectCmd) -> Result<()> {
    match action {
        ProjectCmd::Create { name, cwd, path } => {
            let (_p, store) = open_store(path)?;
            let cwd = match cwd {
                Some(c) => c,
                None => std::env::current_dir().context("read current_dir")?,
            };
            let id = store.create_project(&name, &cwd)?;
            let project = store
                .get_project(&id)?
                .context("just-created project must exist")?;
            print_json(&project)
        }
        ProjectCmd::List { path } => {
            let (_p, store) = open_store(path)?;
            let projects = store.list_projects()?;
            print_json(&projects)
        }
    }
}

fn handle_conversation(action: ConversationCmd) -> Result<()> {
    match action {
        ConversationCmd::Create {
            project_id,
            title,
            path,
        } => {
            let (_p, store) = open_store(path)?;
            let id = store.create_conversation(&ProjectId::from(project_id), &title)?;
            print_json(&id)
        }
        ConversationCmd::List { project_id, path } => {
            let (_p, store) = open_store(path)?;
            let list = store.list_conversations(&ProjectId::from(project_id))?;
            print_json(&list)
        }
    }
}

fn handle_message(action: MessageCmd) -> Result<()> {
    match action {
        MessageCmd::Append {
            conversation_id,
            role,
            content,
            path,
        } => {
            let (_p, store) = open_store(path)?;
            let role = Role::parse(&role)?;
            let id = store.append_message(
                &ConversationId::from(conversation_id),
                role,
                &content,
            )?;
            print_json(&id)
        }
        MessageCmd::List {
            conversation_id,
            limit,
            path,
        } => {
            let (_p, store) = open_store(path)?;
            let list = store
                .list_messages(&ConversationId::from(conversation_id), limit)?;
            print_json(&list)
        }
    }
}

fn handle_registry(action: RegistryCmd) -> Result<()> {
    match action {
        RegistryCmd::Load { dir } | RegistryCmd::List { dir } => {
            let reg = DesignSystemRegistry::load_from_dir(&dir)?;
            print_json(&reg.to_vec())
        }
    }
}
