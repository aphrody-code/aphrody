mod commands;
mod context;
mod platform;
mod scrape;

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use miette::Result;

use crate::{
    commands::{ChromiumSyncCommand, MirrorCommand, ScrapeProfile, VersionCommand},
    context::{GoogleContext, TerminalCommand},
};

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[derive(Parser)]
#[command(name = "aphrody")]
#[command(version = "1.0.0-canary")]
#[command(about = "Aphrody — cross-platform Rust binary (Windows / Linux / macOS / wasm).",
          long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Authentification Google (God Mode / OAuth2)
    Auth {
        #[arg(short, long)]
        force: bool,
    },
    /// Gère le mirroring des assets MD3
    Mirror {
        #[arg(short, long, default_value = "start")]
        action: String,
    },
    /// Résolution DNS OSINT (reconnaissance agressive)
    Dns {
        #[arg(required = true)]
        domain: String,
    },
    /// Affiche la version et l'état du système
    Version,
    /// Forensics Chromium
    Chromium {
        #[command(subcommand)]
        action: ChromiumActions,
    },
    /// Client natif A2A
    A2a {
        #[arg(required = true)]
        prompt: String,
    },
    /// Compilation hyper-optimisée de ChromeOS
    Cros {
        #[command(subcommand)]
        action: CrosActions,
    },
    /// Uutils Coreutils (Rust GNU coreutils)
    Coreutils {
        #[arg(default_value = "build")]
        action: String,
    },
    /// Uutils Util-linux (Rust linux utils)
    UtilLinux {
        #[arg(default_value = "build")]
        action: String,
    },
    /// Recherche Google Native
    Search {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        query: Vec<String>,
    },
    /// Lance le binaire natif Gemini CLI (bundlé via bun --compile)
    Gemini {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Web scraping via BXC daemon (recon ou extraction CSS)
    Scrape {
        /// URL cible
        #[arg(required = true)]
        url: String,
        /// Sélecteur CSS — si absent, bxc recon complet est exécuté
        #[arg(long)]
        selector: Option<String>,
        /// Profil de rendu transmis au daemon BXC
        #[arg(long, value_enum, default_value = "fast")]
        profile: ScrapeProfile,
        /// Fichier de sortie (JSON) — stdout si absent
        #[arg(long, short)]
        output: Option<PathBuf>,
    },
    /// Extraction des design-tokens Material Design 3
    Tokens {
        /// Page source des tokens
        #[arg(long, default_value = "https://m3.material.io/foundations/design-tokens")]
        url: String,
        /// Fichier de destination JSON
        #[arg(long, short, default_value = "packages/ui/tokens/m3.json")]
        output: PathBuf,
        /// Écrase le fichier s'il existe déjà
        #[arg(long)]
        force: bool,
    },
    /// Exécution automatique (Bun, Uv, ou scripts)
    #[command(external_subcommand)]
    Auto(Vec<String>),
}

#[derive(Subcommand)]
enum CrosActions {
    /// Synchronisation ultra-rapide (shallow clone)
    Sync,
    /// Compilation GN/Ninja avec SCCache et optimisation multi-coeur
    Build,
}

#[derive(Subcommand)]
enum ChromiumActions {
    /// Synchronise les profils Chromium
    Sync,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialisation du contexte global Google-Prime
    let ctx = GoogleContext::new().map_err(|e| miette::miette!(e.to_string()))?;
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Auth { force }) => {
            commands::AuthCommand { force }.execute(&ctx).await?;
        },
        Some(Commands::Version) => {
            VersionCommand.execute(&ctx).await?;
        },
        Some(Commands::Mirror { action }) => {
            MirrorCommand { action }.execute(&ctx).await?;
        },
        Some(Commands::Dns { domain }) => {
            commands::DnsCommand { domain }.execute(&ctx).await?;
        },
        Some(Commands::Chromium { action }) => match action {
            ChromiumActions::Sync => {
                ChromiumSyncCommand.execute(&ctx).await?;
            },
        },
        Some(Commands::A2a { prompt }) => {
            commands::A2aCommand { prompt }.execute(&ctx).await?;
        },
        Some(Commands::Cros { action }) => {
            commands::CrosCommand { action }.execute(&ctx).await?;
        },
        Some(Commands::Coreutils { action }) => {
            commands::CoreutilsCommand { action }.execute(&ctx).await?;
        },
        Some(Commands::UtilLinux { action }) => {
            commands::UtilLinuxCommand { action }.execute(&ctx).await?;
        },
        Some(Commands::Search { query }) => {
            commands::SearchCommand { query }.execute(&ctx).await?;
        },
        Some(Commands::Gemini { args }) => {
            commands::GeminiCommand { args }.execute(&ctx).await?;
        },
        Some(Commands::Scrape { url, selector, profile, output }) => {
            commands::ScrapeCommand { url, selector, profile, output }.execute(&ctx).await?;
        },
        Some(Commands::Tokens { url, output, force }) => {
            commands::TokensCommand { url, output, force }.execute(&ctx).await?;
        },
        Some(Commands::Auto(args)) => {
            commands::AutoCommand { args }.execute(&ctx).await?;
        },
        None => {
            commands::AutoCommand { args: vec![] }.execute(&ctx).await?;
        },
    }

    Ok(())
}
