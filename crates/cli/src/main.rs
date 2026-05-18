// SPDX-License-Identifier: Apache-2.0
// `aphrody` — cross-platform CLI entry point.
//
// On native targets (Linux / Windows / macOS) this binary embeds the full
// command surface (auth, mirror, dns, chromium, a2a, search, gemini, scrape,
// tokens, auto, …). On wasm32-* it degrades to a minimal stub that parses
// the same clap surface and prints `--version` / `--help` — the heavy deps
// (tokio "full" runtime, reqwest, rustls/ring, mimalloc, backend forensics,
// a2a transports) cannot be linked on wasm and live behind
// `cfg(not(target_arch = "wasm32"))`.

#[cfg(not(target_arch = "wasm32"))] pub(crate) mod auto_command;
#[cfg(not(target_arch = "wasm32"))] mod commands;
#[cfg(not(target_arch = "wasm32"))] mod context;
#[cfg(not(target_arch = "wasm32"))] pub(crate) mod nl_tokens;
#[cfg(not(target_arch = "wasm32"))] mod platform;
#[cfg(not(target_arch = "wasm32"))] mod scrape;
#[cfg(not(target_arch = "wasm32"))] mod scan_cmd;
#[cfg(not(target_arch = "wasm32"))] mod self_cmd;
#[cfg(not(target_arch = "wasm32"))] mod oc_cmd;

use std::path::PathBuf;

#[cfg(not(target_arch = "wasm32"))] use clap::CommandFactory;
use clap::{Parser, Subcommand};

#[cfg(not(target_arch = "wasm32"))]
use crate::{
    commands::{
        ChromiumSyncCommand, DoctorCommand, MirrorCommand, ScrapeProfile, SubprocessExit,
        VersionCommand,
    },
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

/// Messaging-channel selector for `aphrody notify`.
///
/// Mirrors the production adapters in `aphrody-channels` (Slack, Telegram,
/// Matrix). Declared at the top level (not gated) so the clap surface is
/// identical on native and wasm32 builds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum NotifyChannel {
    /// Slack Bot API (chat.postMessage). Reads `SLACK_BOT_TOKEN` + `SLACK_CHANNEL`.
    Slack,
    /// Telegram Bot API (sendMessage). Reads `TELEGRAM_BOT_TOKEN` + `TELEGRAM_CHAT_ID`.
    Telegram,
    /// Matrix Client-Server API v3. Reads `MATRIX_HOMESERVER`, `MATRIX_ACCESS_TOKEN`,
    /// `MATRIX_USER_ID` + `MATRIX_ROOM_ID`.
    Matrix,
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
    Doctor {
        /// Emit diagnostics as a single JSON object instead of human-readable text
        #[arg(long)]
        json: bool,
    },
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
    /// Pont WebSocket-PTY pour le frontend WASM (localhost uniquement)
    Term {
        /// Adresse d'écoute du serveur WebSocket (host:port)
        #[arg(long, default_value = "127.0.0.1:8788")]
        addr: String,
        /// Shell à lancer (défaut : autodetect selon la plateforme)
        #[arg(long)]
        shell: Option<String>,
        /// Répertoire de travail initial du shell
        #[arg(long)]
        cwd: Option<std::path::PathBuf>,
    },
    /// Node-to-Bun migration tool (facade around packages/n2b/src/cli.ts via bun).
    N2b {
        /// Subcommand / arguments forwarded verbatim to the n2b CLI.
        ///
        /// Examples:
        ///   aphrody n2b scan .
        ///   aphrody n2b fix src/
        ///   aphrody n2b rules --report=json
        ///   aphrody n2b watch --interval 60 --path src/
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Browser-in-process engine (bxc) — recon / scrape / detect / tokens / daemon.
    Bxc {
        #[command(subcommand)]
        action: BxcAction,
    },
    /// Envoie un message via Slack / Telegram / Matrix.
    ///
    /// Credentials lus depuis l'environnement (voir `aphrody notify --help`).
    Notify {
        /// Channel cible : slack, telegram ou matrix.
        #[arg(long, value_enum)]
        channel: NotifyChannel,
        /// Texte du message (plain-text).
        #[arg(long, short)]
        message: String,
        /// Destinataire (chat ID Telegram, channel ID Slack, room ID Matrix).
        ///
        /// Si absent, lu depuis `SLACK_CHANNEL`, `TELEGRAM_CHAT_ID`,
        /// ou `MATRIX_ROOM_ID` selon le `--channel` choisi.
        #[arg(long, short)]
        room: Option<String>,
    },
    /// Génère des completions shell pour bash / zsh / fish / pwsh / elvish
    Completions {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
    /// Installer / bootstrap natif (remplace les .ps1/.sh scripts/).
    #[command(name = "self")]
    SelfCmd {
        #[command(subcommand)]
        action: SelfAction,
    },
    /// Repo analytics : scan tree (size/file-count) + scan manifests (Cargo/JSON/TOML).
    Scan {
        #[command(subcommand)]
        action: ScanAction,
    },
    /// (ported from openclaw) Bootstrap aphrody local state + seed config.
    #[cfg(not(target_arch = "wasm32"))]
    OcOnboard {
        /// Override the default agent workspace directory.
        #[arg(long)]
        workspace: Option<PathBuf>,
        /// Run without interactive prompts (CI / scripts).
        #[arg(long)]
        non_interactive: bool,
        /// Required with `--non-interactive`: acknowledge security defaults.
        #[arg(long)]
        accept_risk: bool,
        /// Overwrite an existing aphrody.json instead of bailing out.
        #[arg(long, short)]
        force: bool,
    },
    /// (ported from openclaw) Reset local state. Choose a scope:
    /// `config`, `config-creds-sessions`, or `full`.
    #[cfg(not(target_arch = "wasm32"))]
    OcReset {
        /// Reset scope.
        #[arg(long, value_enum)]
        scope: oc_cmd::ResetScope,
        /// Confirm the destructive operation (required unless --dry-run).
        #[arg(long)]
        yes: bool,
        /// Preview deletions without touching disk.
        #[arg(long)]
        dry_run: bool,
    },
    /// (ported from openclaw) Uninstall aphrody scopes (service / state /
    /// workspace / app, or `--all`).
    #[cfg(not(target_arch = "wasm32"))]
    OcUninstall {
        #[arg(long)]
        service: bool,
        #[arg(long)]
        state: bool,
        #[arg(long)]
        workspace: bool,
        #[arg(long)]
        app: bool,
        #[arg(long)]
        all: bool,
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        dry_run: bool,
    },
    /// (ported from openclaw) Secure DM pairing — list / approve / inject
    /// requests against the local pairing store (~/.aphrody/pairing.json).
    #[cfg(not(target_arch = "wasm32"))]
    OcPairing {
        #[command(subcommand)]
        action: oc_cmd::PairingAction,
    },
    /// (ported from openclaw) Open or search the documentation site.
    #[cfg(not(target_arch = "wasm32"))]
    OcDocs {
        /// Free-form search query (joined with spaces).
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        query: Vec<String>,
        /// Print only the URL; do not attempt to open a browser.
        #[arg(long)]
        url_only: bool,
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

/// Actions for the `scan` kernel subcommand (repo analytics).
#[derive(Subcommand, Debug, Clone)]
pub(crate) enum ScanAction {
    /// Size + file-count breakdown for top-level groups (vendor/packages/
    /// crates by default).
    Tree {
        /// Root of the project; defaults to the current working directory.
        #[arg(long)]
        root: Option<PathBuf>,
        /// Comma-separated list of top-level group directories to scan.
        #[arg(long, value_delimiter = ',')]
        groups: Vec<String>,
        /// Output JSON file path; `-` writes to stdout. Omitted = prints
        /// only the console summary.
        #[arg(long, short)]
        output: Option<PathBuf>,
        /// Number of top extensions to include per directory.
        #[arg(long, default_value = "5")]
        top_ext: usize,
    },
    /// Walk the repo for Cargo.toml / package.json / pyproject.toml / etc.
    /// and emit metadata (name, version, dependency count, workspace members).
    Manifests {
        #[arg(long)]
        root: Option<PathBuf>,
        #[arg(long, short)]
        output: Option<PathBuf>,
    },
}

/// Actions for the `self` kernel subcommand (installer + bootstrap).
#[derive(Subcommand, Debug, Clone)]
pub(crate) enum SelfAction {
    /// Register the release binary on the user PATH (HKCU on Windows,
    /// symlink into $HOME/.local/bin on Unix).
    InstallPath {
        /// Absolute path to the aphrody binary; defaults to
        /// `<cwd>/target/release/aphrody[.exe]`.
        #[arg(long)]
        bin: Option<PathBuf>,
        /// Run `cargo build -p aphrody --release --locked` first when the
        /// binary is missing.
        #[arg(long)]
        build: bool,
        /// Plan-only: print the intended actions, never mutate PATH/symlinks.
        #[arg(long)]
        dry_run: bool,
    },
    /// Inventory dev toolchain (rustup, cargo, git, zigbuild, wasm targets).
    /// With `--check`, prints the inventory and exits non-zero on missing
    /// required tools; without it, attempts `rustup target add` to fill gaps.
    Bootstrap {
        /// Print inventory only; do not install anything.
        #[arg(long)]
        check: bool,
    },
}

/// Actions for the `bxc` kernel subcommand.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Subcommand, Debug, Clone)]
pub(crate) enum BxcAction {
    /// Start (or supervise) the bxc-engine daemon in the background.
    ///
    /// Spawns `bxc-engine serve --port <port>` and persists its PID at
    /// `var/run/bxc.pid` under the repository root so subsequent commands
    /// can check whether the daemon is alive.
    Daemon {
        /// TCP port the daemon listens on.
        #[arg(long, default_value = "8765")]
        port: u16,
    },
    /// Full-page reconnaissance (passthrough to /recon).
    Recon {
        /// Target URL.
        url: String,
    },
    /// CSS-selector scrape (passthrough to /scrape).
    Scrape {
        /// Target URL.
        url: String,
        /// CSS selector to extract.
        #[arg(long)]
        selector: Option<String>,
    },
    /// Framework / runtime detection (passthrough to /detect).
    Detect {
        /// Target URL.
        url: String,
    },
    /// Material Design 3 token extraction (passthrough to /tokens/m3).
    Tokens {
        /// Target URL.
        url: String,
    },
}

/// Wasm stub — same shape so the clap surface compiles on wasm32 too.
#[cfg(target_arch = "wasm32")]
#[derive(Subcommand, Debug, Clone)]
pub(crate) enum BxcAction {
    Daemon {
        #[arg(long, default_value = "8765")]
        port: u16,
    },
    Recon {
        url: String,
    },
    Scrape {
        url: String,
        #[arg(long)]
        selector: Option<String>,
    },
    Detect {
        url: String,
    },
    Tokens {
        url: String,
    },
}

// Natural-language prompt detection lives in `crate::nl_tokens`. The
// canonical token inventory and the detector are shared with
// `commands::AutoCommand::execute` to guarantee both call sites agree.
#[cfg(not(target_arch = "wasm32"))]
use crate::nl_tokens::is_natural_language_prompt;

// ===========================================================================
// Native entry point — full command dispatch.
// ===========================================================================

/// Run the full command dispatch and return a `miette::Result<()>`.
///
/// Extracted from `main` so the tokio entry-point can intercept
/// `SubprocessExit` errors and issue the single authoritative
/// `process::exit` call there, keeping all library code free of direct
/// exit calls.
#[cfg(not(target_arch = "wasm32"))]
async fn dispatch(ctx: &GoogleContext, cli: Cli) -> miette::Result<()> {
    match cli.command {
        Some(Commands::Auth { force }) => {
            commands::AuthCommand { force }.execute(ctx).await?;
        },
        Some(Commands::Version) => {
            VersionCommand.execute(ctx).await?;
        },
        Some(Commands::Doctor { json }) => {
            DoctorCommand { json_output: json }.execute(ctx).await?;
        },
        Some(Commands::Mirror { action }) => {
            MirrorCommand { action }.execute(ctx).await?;
        },
        Some(Commands::Dns { domain }) => {
            commands::DnsCommand { domain }.execute(ctx).await?;
        },
        Some(Commands::Chromium { action }) => match action {
            ChromiumActions::Sync => {
                ChromiumSyncCommand.execute(ctx).await?;
            },
        },
        Some(Commands::A2a { prompt }) => {
            commands::A2aCommand { prompt }.execute(ctx).await?;
        },
        Some(Commands::Cros { action }) => {
            commands::CrosCommand { action }.execute(ctx).await?;
        },
        Some(Commands::Coreutils { action }) => {
            commands::CoreutilsCommand { action }.execute(ctx).await?;
        },
        Some(Commands::UtilLinux { action }) => {
            commands::UtilLinuxCommand { action }.execute(ctx).await?;
        },
        Some(Commands::Search { query }) => {
            commands::SearchCommand { query }.execute(ctx).await?;
        },
        Some(Commands::Gemini { args }) => {
            commands::GeminiCommand { args }.execute(ctx).await?;
        },
        Some(Commands::Scrape { url, selector, profile, output }) => {
            commands::ScrapeCommand { url, selector, profile, output }.execute(ctx).await?;
        },
        Some(Commands::Tokens { url, output, force }) => {
            commands::TokensCommand { url, output, force }.execute(ctx).await?;
        },
        Some(Commands::Term { addr, shell, cwd }) => {
            commands::TermCommand { addr, shell, cwd }.execute(ctx).await?;
        },
        Some(Commands::N2b { args }) => {
            commands::N2bCommand { args }.execute(ctx).await?;
        },
        Some(Commands::Bxc { action }) => {
            commands::BxcCommand { action }.execute(ctx).await?;
        },
        Some(Commands::Notify { channel, message, room }) => {
            commands::NotifyCommand { channel, message, room }.execute(ctx).await?;
        },
        Some(Commands::Completions { shell }) => {
            let mut cmd = Cli::command();
            clap_complete::generate(shell, &mut cmd, "aphrody", &mut std::io::stdout());
        },
        Some(Commands::SelfCmd { action }) => match action {
            SelfAction::InstallPath { bin, build, dry_run } => {
                self_cmd::InstallPathCommand { bin, build_if_missing: build, dry_run }
                    .execute(ctx)
                    .await?;
            },
            SelfAction::Bootstrap { check } => {
                self_cmd::BootstrapCommand { check }.execute(ctx).await?;
            },
        },
        Some(Commands::Scan { action }) => match action {
            ScanAction::Tree { root, groups, output, top_ext } => {
                scan_cmd::TreeCommand {
                    root,
                    groups,
                    output,
                    top_ext_count: top_ext,
                }
                .execute(ctx)
                .await?;
            },
            ScanAction::Manifests { root, output } => {
                scan_cmd::ManifestsCommand { root, output }.execute(ctx).await?;
            },
        },
        Some(Commands::OcOnboard { workspace, non_interactive, accept_risk, force }) => {
            oc_cmd::OnboardCommand { workspace, non_interactive, accept_risk, force }
                .execute(ctx)
                .await?;
        },
        Some(Commands::OcReset { scope, yes, dry_run }) => {
            oc_cmd::ResetCommand { scope, yes, dry_run }.execute(ctx).await?;
        },
        Some(Commands::OcUninstall { service, state, workspace, app, all, yes, dry_run }) => {
            oc_cmd::UninstallCommand {
                service,
                state,
                workspace,
                app,
                all,
                yes,
                dry_run,
            }
            .execute(ctx)
            .await?;
        },
        Some(Commands::OcPairing { action }) => {
            oc_cmd::PairingCommand { action }.execute(ctx).await?;
        },
        Some(Commands::OcDocs { query, url_only }) => {
            oc_cmd::DocsCommand { query, url_only }.execute(ctx).await?;
        },
        Some(Commands::Auto(args)) => {
            // Route NL prompts to the native A2A JSON-RPC client; defer to
            // the legacy bun/uv/cargo engine dispatcher only for tokens
            // that clearly look like a CLI command (known engine name,
            // standard subcommand, or a script file).
            if is_natural_language_prompt(&args) {
                let opts = auto_command::AutoCommand::new(args.join(" "));
                match auto_command::run(opts).await {
                    Ok(_) => {},
                    Err(err) => {
                        eprintln!("aphrody: {err}");
                        return Err(miette::miette!(
                            "auto_command failed (exit {})",
                            err.exit_code()
                        ));
                    },
                }
            } else {
                commands::AutoCommand { args }.execute(ctx).await?;
            }
        },
        None => {
            commands::AutoCommand { args: vec![] }.execute(ctx).await?;
        },
    }

    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() {
    // rustls 0.23 requires an explicit CryptoProvider install before any
    // reqwest::Client::new() call (otherwise reqwest panics at runtime in
    // async_impl/client.rs:2461). `GoogleContext::new()` builds a reqwest
    // client immediately, so this must come first.
    let _ = rustls::crypto::ring::default_provider().install_default();

    let ctx = match GoogleContext::new() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("aphrody: {e}");
            std::process::exit(1);
        },
    };
    let cli = Cli::parse();

    match dispatch(&ctx, cli).await {
        Ok(()) => {},
        Err(report) => {
            // `SubprocessExit` is produced by commands that forward a child
            // process and want to propagate its exit code verbatim.  Extract
            // the code and exit here — this is the single authorised call to
            // `process::exit` in the entire binary.
            if let Some(se) = report.downcast_ref::<SubprocessExit>() {
                let code = se.0;
                std::process::exit(code);
            }
            // All other errors: pretty-print via miette and exit 1.
            eprintln!("{report:?}");
            std::process::exit(1);
        },
    }
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
                Commands::Doctor { .. } => "doctor",
                Commands::Term { .. } => "term",
                Commands::N2b { .. } => "n2b",
                Commands::Bxc { .. } => "bxc",
                Commands::Notify { .. } => "notify",
                Commands::Completions { .. } => "completions",
                Commands::SelfCmd { .. } => "self",
                Commands::Scan { .. } => "scan",
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
