// `aphrody` — cross-platform CLI entry point.
//
// On native targets (Linux / Windows / macOS) this binary embeds the full
// command surface (auth, mirror, dns, chromium, a2a, search, gemini, scrape,
// tokens, auto, …). On wasm32-* it degrades to a minimal stub that parses
// the same clap surface and prints `--version` / `--help` — the heavy deps
// (tokio "full" runtime, reqwest, rustls/ring, mimalloc, backend forensics,
// a2a transports) cannot be linked on wasm and live behind
// `cfg(not(target_arch = "wasm32"))`.

#[cfg(not(target_arch = "wasm32"))] mod commands;
#[cfg(not(target_arch = "wasm32"))] mod context;
#[cfg(not(target_arch = "wasm32"))] mod platform;
#[cfg(not(target_arch = "wasm32"))] mod scrape;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[cfg(not(target_arch = "wasm32"))]
use crate::{
    commands::{ChromiumSyncCommand, DoctorCommand, MirrorCommand, ScrapeProfile, VersionCommand},
    context::{GoogleContext, TerminalCommand},
};

// On wasm we still need the `ScrapeProfile` enum for clap; provide a stub
// that mirrors the native enum shape so the CLI surface is identical.
#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub(crate) enum ScrapeProfile {
    Fast,
    Full,
    Stealth,
}

#[cfg(not(target_arch = "wasm32"))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[derive(Parser)]
#[command(name = "aphrody")]
#[command(version = "1.0.0-canary")]
#[command(about = "Aphrody — cross-platform Rust binary (Linux / Windows / macOS / wasm).",
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
    /// Diagnostic env + intégration A2A + supply-chain (first-impression)
    Doctor,
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
    // On wasm the inner Vec is consumed only at the type level by clap; the
    // dispatch arm uses `Commands::Auto(_)`. Native dispatch reads it.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
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

// ===========================================================================
// Native entry point — full command dispatch.
// ===========================================================================
#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() -> miette::Result<()> {
    // rustls 0.23 requires an explicit CryptoProvider install before any
    // reqwest::Client::new() call (otherwise reqwest panics at runtime in
    // async_impl/client.rs:2461). `GoogleContext::new()` builds a reqwest
    // client immediately, so this must come first.
    let _ = rustls::crypto::ring::default_provider().install_default();

    let ctx = GoogleContext::new().map_err(|e| miette::miette!(e.to_string()))?;
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Auth { force }) => {
            commands::AuthCommand { force }.execute(&ctx).await?;
        },
        Some(Commands::Version) => {
            VersionCommand.execute(&ctx).await?;
        },
        Some(Commands::Doctor) => {
            DoctorCommand.execute(&ctx).await?;
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

// ===========================================================================
// wasm entry point — parses the clap surface so `--version` / `--help`
// behave identically; every command short-circuits to a "not available on
// wasm" notice so users discover what the native binary offers.
// ===========================================================================
#[cfg(target_arch = "wasm32")]
fn main() {
    let cli = Cli::parse();
    match cli.command {
        None | Some(Commands::Version) => {
            println!(
                "aphrody {} — wasm stub. Use the native binary for full command surface.",
                env!("CARGO_PKG_VERSION"),
            );
        },
        Some(other) => {
            let name = match other {
                Commands::Auth { .. } => "auth",
                Commands::Mirror { .. } => "mirror",
                Commands::Dns { .. } => "dns",
                Commands::Chromium { .. } => "chromium",
                Commands::A2a { .. } => "a2a",
                Commands::Cros { .. } => "cros",
                Commands::Coreutils { .. } => "coreutils",
                Commands::UtilLinux { .. } => "util-linux",
                Commands::Search { .. } => "search",
                Commands::Gemini { .. } => "gemini",
                Commands::Scrape { .. } => "scrape",
                Commands::Tokens { .. } => "tokens",
                Commands::Doctor => "doctor",
                Commands::Auto(_) => "auto",
                Commands::Version => unreachable!(),
            };
            eprintln!(
                "command `{name}` is not available on the wasm target — use the native aphrody \
                 binary (Linux / Windows / macOS) instead."
            );
        },
    }
}
