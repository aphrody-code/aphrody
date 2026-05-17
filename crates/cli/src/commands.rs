// SPDX-License-Identifier: Apache-2.0
use std::net::SocketAddr;
use std::path::PathBuf;

use async_trait::async_trait;
// `backend::chromium` is `#[cfg(target_os = "windows")]` because the master-key
// decryption path uses DPAPI / Win32 ACLs. On other OSes the Chromium-based
// commands fall back to a "Windows-only" notice.
#[cfg(target_os = "windows")] use backend::chromium::ChromiumParser;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};

use crate::{
    context::{GoogleContext, TerminalCommand},
    nl_tokens::{
        BUN_COMMANDS, BYPASS_ENGINES, CARGO_COMMANDS, SCRIPT_EXTS, STANDARD_ACTIONS, UV_COMMANDS,
    },
    platform,
    scrape::ScrapeClient,
};

pub(crate) struct VersionCommand;

#[async_trait]
impl TerminalCommand for VersionCommand {
    async fn execute(&self, _ctx: &GoogleContext) -> miette::Result<()> {
        // Format mirrors `gh --version` / `rustc -Vv` — flat key:value, no
        // emoji, no styling (terminals downstream may pipe to grep).
        println!("aphrody {}", env!("CARGO_PKG_VERSION"));
        println!("commit:    {}", env!("APHRODY_GIT_SHA"));
        println!("built:     {} (unix epoch)", env!("APHRODY_BUILD_UNIX"));
        println!("target:    {}", env!("APHRODY_TARGET"));
        println!("profile:   {}", env!("APHRODY_PROFILE"));
        println!();
        println!("repo:      https://github.com/aphrody-code/aphrody");
        println!("license:   Apache-2.0");
        println!("a2a:       ai.json v1 (AGNTCY a2a/v0.4) @ /ai.json + /.well-known/ai.json");
        Ok(())
    }
}

pub(crate) struct MirrorCommand {
    pub action: String,
}

#[async_trait]
impl TerminalCommand for MirrorCommand {
    async fn execute(&self, ctx: &GoogleContext) -> miette::Result<()> {
        if self.action == "start" {
            ctx.mirror.start_mirroring().await.map_err(|e| miette::miette!(e.to_string()))?;
        }
        Ok(())
    }
}

pub(crate) struct ChromiumSyncCommand;

#[cfg(target_os = "windows")]
#[async_trait]
impl TerminalCommand for ChromiumSyncCommand {
    async fn execute(&self, _ctx: &GoogleContext) -> miette::Result<()> {
        println!("🔍 Détection des profils Chromium ({})...", platform::os_short_name());

        let user_data = platform::chrome_user_data()
            .ok_or_else(|| miette::miette!("Chrome user-data path not known on this platform"))?;

        let mut parser = ChromiumParser::new(user_data);
        let profiles = parser.get_profiles();
        println!("✅ Profils trouvés : {:?}", profiles);

        parser.load_master_key().map_err(|e| miette::miette!(e.to_string()))?;
        println!("🔑 Master Key déchiffrée avec succès.");

        Ok(())
    }
}

#[cfg(not(target_os = "windows"))]
#[async_trait]
impl TerminalCommand for ChromiumSyncCommand {
    async fn execute(&self, _ctx: &GoogleContext) -> miette::Result<()> {
        Err(miette::miette!(
            "`chromium sync` is a Windows-only command (DPAPI-backed master-key path). Run on \
             Windows or use the OAuth2 flow via `aphrody auth`."
        ))
    }
}

pub(crate) struct AuthCommand {
    pub force: bool,
}

#[async_trait]
impl TerminalCommand for AuthCommand {
    async fn execute(&self, _ctx: &GoogleContext) -> miette::Result<()> {
        println!("🔐 Tentative d'authentification Google...");

        // Logique God Mode : Extraction via Chromium SxS (Canary) — Windows only
        // because the master-key path relies on DPAPI. Non-Windows hosts skip
        // straight to the OAuth2 PKCE delegation below.
        #[cfg(target_os = "windows")]
        if !self.force {
            println!("🛡️ Mode God Mode activé. Recherche de credentials locaux...");

            let canary_data = platform::chrome_canary_user_data()
                .ok_or_else(|| miette::miette!("Chrome Canary path not known on this platform"))?;

            if canary_data.exists() {
                println!("✅ Chrome Canary détecté : {}", canary_data.display());
                let mut parser = ChromiumParser::new(canary_data);
                if parser.load_master_key().is_ok() {
                    println!("🔑 Master Key récupérée. Injection des tokens en cours...");

                    let profiles = parser.get_profiles();
                    for profile in profiles {
                        if let Ok(cookies) = parser.get_cookies(&profile, "google.com") {
                            let sid = cookies.iter().find(|(n, _)| n == "__Secure-1PSID");
                            if let Some((_, val)) = sid {
                                println!(
                                    "✨ Token __Secure-1PSID trouvé dans le profil '{}'.",
                                    profile
                                );
                                Self::persist_god_mode_token(&profile, val)?;
                                println!("🔓 Authentification God Mode réussie pour {} !", profile);
                                return Ok(());
                            }
                        }
                    }
                    println!("⚠️ Aucun token valide trouvé dans les profils Chrome Canary.");
                }
            } else {
                println!("⚠️ Chrome Canary non trouvé. Passage au mode OAuth2 standard.");
            }
        }

        #[cfg(not(target_os = "windows"))]
        if !self.force {
            println!(
                "ℹ️ God Mode (Chromium Canary master-key extraction) sauté — Windows-only. \
                 Passage direct à OAuth2."
            );
        }

        // OAuth2 PKCE flow not yet ported to native Rust — delegate to the
        // bundled gemini-cli binary which already ships the flow via Bun.
        // Tracking: docs/PLAN.md §"Auth — native OAuth2 (PKCE + callback)".
        println!("🌐 Délégation du flux OAuth2 au binaire gemini-cli embarqué...");
        GeminiCommand { args: vec!["auth".to_string(), "login".to_string()] }.execute(_ctx).await
    }
}

impl AuthCommand {
    /// Persists a God Mode token to the per-user CLI credential store.
    /// File is created with `0600` on Unix; on Windows it inherits the user
    /// profile ACL (DPAPI-equivalent isolation since it sits under `%APPDATA%`).
    ///
    /// Only called from the Windows-gated God Mode branch in
    /// `AuthCommand::execute`; on Linux/macOS the symbol is reachable but
    /// the call site is `cfg`-stripped, so dead-code is allowed.
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    fn persist_god_mode_token(profile: &str, token: &str) -> miette::Result<()> {
        let home = platform::home_dir()
            .map_err(|_| miette::miette!("Impossible de localiser le répertoire utilisateur"))?;
        let dir = home.join(".aphrody").join("credentials");
        std::fs::create_dir_all(&dir)
            .map_err(|e| miette::miette!("Création de {} impossible : {e}", dir.display()))?;
        let file = dir.join(format!("{profile}.gm.token"));
        std::fs::write(&file, token)
            .map_err(|e| miette::miette!("Écriture de {} impossible : {e}", file.display()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600))
                .map_err(|e| miette::miette!("chmod 0600 sur {} : {e}", file.display()))?;
        }
        Ok(())
    }
}

pub(crate) struct DnsCommand {
    pub domain: String,
}

#[async_trait]
impl TerminalCommand for DnsCommand {
    async fn execute(&self, _ctx: &GoogleContext) -> miette::Result<()> {
        println!("[====== WINCLEAN FORENSIC - MAXIMUM DNS RECON ======]");
        println!("[~] Cible: {}", self.domain);
        println!("[~] 1/3 - Lancement de l'OSINT Passif (Agrégation multi-sources)...");

        let recon = backend::dns::DnsRecon::new();
        match recon.run_osint(&self.domain).await {
            Ok(results) => {
                println!(
                    "[+] Découverte OSINT terminée: {} sous-domaines uniques trouvés !",
                    results.len()
                );
                for sub in results.iter().take(10) {
                    println!("  - {}", sub);
                }
                if results.len() > 10 {
                    println!("  ... et {} autres", results.len() - 10);
                }
            },
            Err(e) => {
                println!("[-] Erreur lors de la résolution OSINT: {}", e);
            },
        }

        Ok(())
    }
}

pub(crate) struct A2aCommand {
    pub prompt: String,
}

#[async_trait]
impl TerminalCommand for A2aCommand {
    async fn execute(&self, _ctx: &GoogleContext) -> miette::Result<()> {
        println!("🤖 Initialisation du client natif Rust A2A...");

        println!("🔑 Extraction sécurisée du token Gemini CLI...");
        let home_dir = platform::home_dir().map(|p| p.display().to_string()).unwrap_or_default();
        let creds_path = PathBuf::from(&home_dir).join(".gemini").join("oauth_creds.json");
        let token = if let Ok(content) = std::fs::read_to_string(&creds_path) {
            if let Some(start) = content.find("\"access_token\": \"") {
                let start = start + 17;
                if let Some(end) = content[start..].find("\"") {
                    content[start..start + end].to_string()
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        if token.is_empty() {
            println!(
                "⚠️ Aucun token Gemini CLI trouvé (êtes-vous authentifié via `bun run start -- \
                 login` ?)"
            );
        } else {
            println!("✅ Token Gemini CLI récupéré ({}...)", &token[..10]);
        }

        println!("🌐 Routage de la requête A2A vers le moteur natif Gemini CLI...");
        println!("📤 Envoi du prompt: \"{}\"", self.prompt);

        // Delegate to the bundled single-file gemini-cli binary (built via bun --compile).
        GeminiCommand { args: vec!["--prompt".to_string(), self.prompt.clone()] }
            .execute(_ctx)
            .await
    }
}

pub(crate) struct CrosCommand {
    pub action: crate::CrosActions,
}

#[async_trait]
impl TerminalCommand for CrosCommand {
    async fn execute(&self, _ctx: &GoogleContext) -> miette::Result<()> {
        let cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);

        println!("🚀 Mode Hyper-Performance ChromeOS Activé");
        println!("⚡ Détection matérielle : {} cœurs logiques disponibles", cores);

        match &self.action {
            crate::CrosActions::Sync => {
                println!("🔄 Création de la configuration .gclient pour ChromeOS...");

                let gclient_config = r#"solutions = [
  {
    "url": "https://chromium.googlesource.com/chromium/src.git",
    "name": "src",
  },
]
target_os = ['chromeos']
"#;
                std::fs::write(".gclient", gclient_config)
                    .map_err(|e| miette::miette!("Erreur écriture .gclient: {}", e))?;

                println!("📥 Lancement du shallow fetch multi-threadé (-j {})...", cores);

                let pb = ProgressBar::new_spinner();
                pb.set_style(
                    ProgressStyle::default_spinner()
                        .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
                        .template("{spinner:.green} [{elapsed_precise}] {msg}")
                        .unwrap(),
                );
                pb.set_message("Synchronisation Chromium en cours...");
                pb.enable_steady_tick(std::time::Duration::from_millis(100));

                // Exécution de gclient sync via depot_tools
                let status = std::process::Command::new("cmd")
                    .args([
                        "/C",
                        r"vendor\depot_tools\gclient.bat",
                        "sync",
                        "--no-history",
                        &format!("-j{}", cores),
                    ])
                    .status()
                    .map_err(|e| miette::miette!("Erreur exécution gclient: {}", e))?;

                pb.finish_and_clear();

                if !status.success() {
                    return Err(miette::miette!("Échec de gclient sync"));
                }
                println!("✅ Code source ChromeOS synchronisé avec une vitesse maximale.");
            },
            crate::CrosActions::Build => {
                println!("⚙️ Configuration du moteur GN pour des performances absolues...");

                let out_dir = PathBuf::from("src/out/ChromeOS");
                std::fs::create_dir_all(&out_dir)
                    .map_err(|e| miette::miette!("Erreur création dossier out/ChromeOS: {}", e))?;

                let gn_args = r#"target_os = "chromeos"
is_chromeos_device = false
use_remoteexec = false
is_debug = false
is_component_build = true
symbol_level = 0
blink_symbol_level = 0
v8_symbol_level = 0
use_lld = true
cc_wrapper = "sccache"
"#;
                std::fs::write(out_dir.join("args.gn"), gn_args)
                    .map_err(|e| miette::miette!("Erreur écriture args.gn: {}", e))?;

                println!("🔨 Génération Ninja...");
                let gn_status = std::process::Command::new("cmd")
                    .current_dir("src")
                    .args(["/C", r"..\vendor\depot_tools\gn.bat", "gen", "out/ChromeOS"])
                    .status()
                    .map_err(|e| miette::miette!("Erreur exécution gn: {}", e))?;

                if !gn_status.success() {
                    return Err(miette::miette!("Échec de la génération GN"));
                }

                println!("🔥 Lancement d'Autoninja sur {} threads...", cores);
                unsafe {
                    std::env::set_var("NINJA_SUMMARIZE_BUILD", "1");
                }

                let ninja_status = std::process::Command::new("cmd")
                    .current_dir("src")
                    .args([
                        "/C",
                        r"..\vendor\depot_tools\autoninja.bat",
                        "-C",
                        "out/ChromeOS",
                        &format!("-j{}", cores),
                    ])
                    .status()
                    .map_err(|e| miette::miette!("Erreur exécution autoninja: {}", e))?;

                if !ninja_status.success() {
                    return Err(miette::miette!("Échec de la compilation Ninja"));
                }
                println!("✅ Compilation ChromeOS terminée de manière foudroyante.");
            },
        }

        Ok(())
    }
}

pub(crate) struct CoreutilsCommand {
    pub action: String,
}

#[async_trait]
impl TerminalCommand for CoreutilsCommand {
    async fn execute(&self, _ctx: &GoogleContext) -> miette::Result<()> {
        println!("🧰 Uutils Coreutils Manager");
        match self.action.as_str() {
            "build" => {
                println!("🔨 Compilation du multicall binary Coreutils (Rust)...");
                let status = std::process::Command::new("cargo")
                    .current_dir("crates/coreutils")
                    .env("RUSTFLAGS", "-C target-cpu=native -C opt-level=3")
                    .args(["build", "--release", "--features", "windows"])
                    .status()
                    .map_err(|e| miette::miette!("Erreur d'exécution de cargo: {}", e))?;

                if status.success() {
                    println!(
                        "✅ Coreutils compilé avec succès \
                         (crates/coreutils/target/release/coreutils)."
                    );
                } else {
                    println!("❌ Échec de la compilation de Coreutils.");
                }
            },
            "run" => {
                println!("🚀 Exécution de Coreutils...");
                let status = std::process::Command::new("cargo")
                    .current_dir("crates/coreutils")
                    .args(["run", "--release", "--"])
                    .status()
                    .map_err(|e| miette::miette!("Erreur d'exécution de cargo: {}", e))?;

                if !status.success() {
                    println!("❌ Échec de l'exécution de Coreutils.");
                }
            },
            _ => println!("[-] Action inconnue : {}", self.action),
        }
        Ok(())
    }
}

pub(crate) struct UtilLinuxCommand {
    pub action: String,
}

#[async_trait]
impl TerminalCommand for UtilLinuxCommand {
    async fn execute(&self, _ctx: &GoogleContext) -> miette::Result<()> {
        println!("🐧 Uutils Util-linux Manager");
        match self.action.as_str() {
            "build" => {
                println!("🔨 Compilation des utilitaires Linux (Rust)...");
                let status = std::process::Command::new("cargo")
                    .current_dir("crates/util-linux")
                    .args(["build", "--release"])
                    .status()
                    .map_err(|e| miette::miette!("Erreur d'exécution de cargo: {}", e))?;

                if status.success() {
                    println!(
                        "✅ Util-linux compilé avec succès (crates/util-linux/target/release/)."
                    );
                } else {
                    println!("❌ Échec de la compilation de Util-linux.");
                }
            },
            _ => println!("[-] Action inconnue : {}", self.action),
        }
        Ok(())
    }
}

pub(crate) struct AutoCommand {
    pub args: Vec<String>,
}

#[async_trait]
impl TerminalCommand for AutoCommand {
    async fn execute(&self, _ctx: &GoogleContext) -> miette::Result<()> {
        if self.args.is_empty() {
            return Self::run_process("bun", Vec::<String>::new());
        }

        let first_arg = &self.args[0];

        // All four category lists (BYPASS_ENGINES, BUN_COMMANDS,
        // UV_COMMANDS, CARGO_COMMANDS, STANDARD_ACTIONS, SCRIPT_EXTS)
        // come from `crate::nl_tokens` so the NL detector in `main.rs`
        // and this dispatcher cannot drift.

        // Explicit engine bypass
        if BYPASS_ENGINES.contains(&first_arg.as_str()) {
            return Self::run_process(first_arg, &self.args[1..]);
        }
        // Specific commands
        else if UV_COMMANDS.contains(&first_arg.as_str()) {
            return Self::run_process("uv", &self.args[..]);
        } else if BUN_COMMANDS.contains(&first_arg.as_str()) {
            return Self::run_process("bun", &self.args[..]);
        } else if CARGO_COMMANDS.contains(&first_arg.as_str()) {
            return Self::run_process("cargo", &self.args[..]);
        }

        let is_known_technical_command = STANDARD_ACTIONS.contains(&first_arg.as_str());
        let is_script_file = SCRIPT_EXTS.iter().any(|ext| first_arg.ends_with(ext));

        // Overlapping Universal Commands (run, install, add, remove, test, build...)
        let is_run_script = first_arg == "run" || first_arg == "exec";

        let mut target_engine = "bun"; // Default to bun

        if !is_known_technical_command && !is_script_file {
            // If it's neither a known standard action nor a direct script execution, it's likely a
            // natural language prompt.
            let prompt = self.args.join(" ");
            println!("✨ [Auto] Détection de langage naturel, bascule vers l'agent A2A...");
            return A2aCommand { prompt }.execute(_ctx).await;
        }

        if is_run_script && self.args.len() > 1 && self.args[1].ends_with(".py") {
            target_engine = "uv";
        } else if first_arg.ends_with(".py") {
            let mut new_args = vec!["run".to_string()];
            new_args.extend(self.args.clone());
            return Self::run_process("uv", new_args);
        } else if first_arg.ends_with(".js")
            || first_arg.ends_with(".ts")
            || first_arg.ends_with(".jsx")
            || first_arg.ends_with(".tsx")
        {
            return Self::run_process("bun", &self.args[..]);
        } else if first_arg.ends_with(".rs") {
            let mut new_args = vec!["run".to_string()];
            new_args.extend(self.args.clone());
            return Self::run_process("cargo", new_args);
        } else {
            // Contextual ecosystem detection
            let has_cargo = std::path::Path::new("Cargo.toml").exists();
            let has_uv = std::path::Path::new("pyproject.toml").exists()
                || std::path::Path::new("requirements.txt").exists();
            let has_go = std::path::Path::new("go.mod").exists();
            let has_bun = std::path::Path::new("package.json").exists();

            if has_cargo && !has_bun {
                target_engine = "cargo";
            } else if has_uv && !has_bun {
                target_engine = "uv";
            } else if has_go && !has_bun {
                target_engine = "go";
            } else if has_bun {
                target_engine = "bun";
            } else {
                // No project file. If it's a global install (e.g. `google install git`), fallback
                // to system manager
                let is_install_like =
                    first_arg == "install" || first_arg == "i" || first_arg == "add";
                if is_install_like && self.args.len() > 1 && !self.args[1].starts_with('-') {
                    #[cfg(target_os = "windows")]
                    {
                        target_engine = "winget";
                    }
                    #[cfg(target_os = "linux")]
                    {
                        target_engine = "apt";
                    }
                    #[cfg(target_os = "macos")]
                    {
                        target_engine = "brew";
                    }
                }
            }
        }

        // Command syntax translation for universal compatibility
        let mut final_args = self.args.clone();
        if target_engine == "cargo" {
            if final_args[0] == "install" || final_args[0] == "i" {
                // In Cargo, adding to a project is 'cargo add', global install is 'cargo install'
                if std::path::Path::new("Cargo.toml").exists() {
                    final_args[0] = "add".to_string();
                } else {
                    final_args[0] = "install".to_string();
                }
            } else if final_args[0] == "rm" || final_args[0] == "remove" {
                final_args[0] = "remove".to_string();
            }
        } else if target_engine == "uv" {
            if final_args[0] == "install" || final_args[0] == "i" {
                final_args[0] = "add".to_string();
            }
        } else if target_engine == "winget" && (final_args[0] == "add" || final_args[0] == "i") {
            final_args[0] = "install".to_string();
        }

        Self::run_process(target_engine, final_args)
    }
}

impl AutoCommand {
    fn run_process<I, S>(cmd: &str, args: I) -> miette::Result<()>
    where
        I: IntoIterator<Item = S> + Clone,
        S: AsRef<std::ffi::OsStr>,
    {
        println!(
            "{}",
            format!(
                "⚡ Route native: {} {:?}",
                cmd,
                args.clone()
                    .into_iter()
                    .map(|s| s.as_ref().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
            )
            .bright_black()
        );

        let status = std::process::Command::new(cmd)
            .args(args)
            .status()
            .map_err(|e| miette::miette!("Erreur lors de l'appel au moteur {}: {}", cmd, e))?;

        if !status.success() {
            std::process::exit(status.code().unwrap_or(1));
        }
        Ok(())
    }
}

pub(crate) struct GeminiCommand {
    pub args: Vec<String>,
}

#[async_trait]
impl TerminalCommand for GeminiCommand {
    async fn execute(&self, _ctx: &GoogleContext) -> miette::Result<()> {
        let bin_name = if cfg!(target_os = "windows") { "gemini-cli.exe" } else { "gemini-cli" };
        let candidates = Self::resolve_candidates(bin_name);

        let bin = candidates.iter().find(|p| p.exists()).ok_or_else(|| {
            miette::miette!(
                "Binary `{}` introuvable. Build avec :\n  cd packages/gemini-cli && bun run \
                 build:binary\n\nCandidats vérifiés :\n  {}",
                bin_name,
                candidates.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join("\n  ")
            )
        })?;

        let status = std::process::Command::new(bin)
            .args(&self.args)
            .status()
            .map_err(|e| miette::miette!("Erreur d'exécution de {}: {}", bin.display(), e))?;

        if !status.success() {
            std::process::exit(status.code().unwrap_or(1));
        }
        Ok(())
    }
}

impl GeminiCommand {
    fn resolve_candidates(bin_name: &str) -> Vec<PathBuf> {
        let mut out = Vec::new();
        if let Ok(exe) = std::env::current_exe()
            && let Some(dir) = exe.parent()
        {
            out.push(dir.join(bin_name));
        }
        let triple_bin = if cfg!(target_os = "windows") {
            "gemini-cli-windows-x64.exe"
        } else {
            "gemini-cli-linux-x64"
        };
        out.push(PathBuf::from("target/native").join(triple_bin));
        out.push(PathBuf::from(triple_bin));
        out
    }
}

pub(crate) struct SearchCommand {
    pub query: Vec<String>,
}

#[async_trait]
impl TerminalCommand for SearchCommand {
    async fn execute(&self, _ctx: &GoogleContext) -> miette::Result<()> {
        let q = self.query.join(" ");
        println!("{}", format!("🔍 Recherche Google : {}", q).bold().blue());

        let client = reqwest::Client::builder()
            .user_agent(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) \
                 Chrome/124.0.0.0 Safari/537.36",
            )
            .build()
            .map_err(|e| miette::miette!("Erreur client HTTP: {}", e))?;

        // Requête POST vers DuckDuckGo Lite (très fiable pour les CLI)
        let res = client
            .post("https://lite.duckduckgo.com/lite/")
            .form(&[("q", &q)])
            .send()
            .await
            .map_err(|e| miette::miette!("Erreur réseau: {}", e))?;

        let text = res.text().await.map_err(|e| miette::miette!("Erreur lecture: {}", e))?;

        let document = scraper::Html::parse_document(&text);
        let title_selector = scraper::Selector::parse(".result-link").unwrap();
        let snippet_selector = scraper::Selector::parse(".result-snippet").unwrap();

        let titles: Vec<_> = document.select(&title_selector).collect();
        let snippets: Vec<_> = document.select(&snippet_selector).collect();

        let mut count = 0;
        for (title_elem, snippet_elem) in titles.iter().zip(snippets.iter()) {
            let title = title_elem.text().collect::<Vec<_>>().join("").trim().to_string();
            let link = title_elem.value().attr("href").unwrap_or("").to_string();
            let snippet = snippet_elem.text().collect::<Vec<_>>().join("").trim().to_string();

            if !title.is_empty() {
                println!("\n{}", title.bold().green());
                println!("{}", link.cyan().underline());
                println!("{}", snippet);
                count += 1;
            }

            if count >= 5 {
                break;
            }
        }

        if count == 0 {
            println!("{}", "Aucun résultat trouvé ou requête bloquée.".red());
        }

        Ok(())
    }
}

// ── aphrody scrape ────────────────────────────────────────────────────────────

/// Scraping profile hint forwarded verbatim to the BXC daemon.
#[derive(Debug, Clone, clap::ValueEnum)]
pub(crate) enum ScrapeProfile {
    Static,
    Fast,
    Stealth,
    Max,
}

impl ScrapeProfile {
    fn as_str(&self) -> &'static str {
        match self {
            ScrapeProfile::Static => "static",
            ScrapeProfile::Fast => "fast",
            ScrapeProfile::Stealth => "stealth",
            ScrapeProfile::Max => "max",
        }
    }
}

pub(crate) struct ScrapeCommand {
    pub url: String,
    pub selector: Option<String>,
    pub profile: ScrapeProfile,
    pub output: Option<PathBuf>,
}

#[async_trait]
impl TerminalCommand for ScrapeCommand {
    async fn execute(&self, _ctx: &GoogleContext) -> miette::Result<()> {
        let client =
            ScrapeClient::new().map_err(|e| miette::miette!("HTTP client init failed: {e}"))?;

        let json_value: serde_json::Value = match &self.selector {
            Some(sel) => {
                let result =
                    client.scrape(&self.url, sel).await.map_err(|e| miette::miette!("{e}"))?;
                serde_json::to_value(&result)
                    .map_err(|e| miette::miette!("JSON serialization error: {e}"))?
            },
            None => {
                // No selector → full recon. Annotate profile in the JSON output
                // so callers know which rendering mode was active.
                let result = client.recon(&self.url).await.map_err(|e| miette::miette!("{e}"))?;
                let mut value = serde_json::to_value(&result)
                    .map_err(|e| miette::miette!("JSON serialization error: {e}"))?;
                value["profile"] = serde_json::Value::String(self.profile.as_str().to_owned());
                value
            },
        };

        let json_text = serde_json::to_string_pretty(&json_value)
            .map_err(|e| miette::miette!("JSON pretty-print error: {e}"))?;

        match &self.output {
            Some(path) => {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        miette::miette!("Cannot create output directory {}: {e}", parent.display())
                    })?;
                }
                std::fs::write(path, &json_text).map_err(|e| {
                    miette::miette!("Cannot write output file {}: {e}", path.display())
                })?;
                println!("Output written to {}", path.display());
            },
            None => println!("{json_text}"),
        }

        Ok(())
    }
}

// ── aphrody doctor ───────────────────────────────────────────────────────────

/// Canonical TTL (seconds) for the peer heartbeat file, as declared in the
/// `etiquette.ttl_seconds` block of `C:/winclean/.coord/ai.json`.
const HEARTBEAT_TTL_SECS: u64 = 600;

/// Absolute path to the winclean-side inbox watched by aphrody.
const WINCLEAN_INBOX_PATH: &str = "C:/winclean/.coord/inbox-from-winclean.jsonl";

/// Absolute path to the peer heartbeat file written by winclean.
const WINCLEAN_HEARTBEAT_PATH: &str = "C:/winclean/.coord/heartbeat-winclean.txt";

/// Outcome of a single diagnostic check.
enum CheckResult {
    Ok(String),
    Warn(String),
    Err(String),
}

impl CheckResult {
    fn line(&self, key: &str) -> String {
        match self {
            CheckResult::Ok(v) => format!("  {key}: {v}"),
            CheckResult::Warn(v) => format!("  {key}: {v}"),
            CheckResult::Err(v) => format!("  {key}: {v}"),
        }
    }

    fn is_critical_failure(&self) -> bool {
        matches!(self, CheckResult::Err(_))
    }

    fn value(&self) -> &str {
        match self {
            CheckResult::Ok(v) | CheckResult::Warn(v) | CheckResult::Err(v) => v.as_str(),
        }
    }
}

/// Collected diagnostics passed to both text and JSON output paths.
struct DoctorReport {
    binary_version: String,
    rustls_provider: String,
    reqwest_tls: &'static str,
    allocator: &'static str,
    tokio_runtime: &'static str,
    ai_json: CheckResult,
    well_known_ai_json: CheckResult,
    peer_heartbeat: CheckResult,
    inbox: CheckResult,
    http_listener: CheckResult,
    cargo_vet: CheckResult,
    env_vars: Vec<(String, String)>,
}

impl DoctorReport {
    fn verdict(&self) -> &'static str {
        let has_error = self.ai_json.is_critical_failure() || self.cargo_vet.is_critical_failure();
        let has_warn = matches!(self.well_known_ai_json, CheckResult::Warn(_))
            || matches!(self.peer_heartbeat, CheckResult::Warn(_))
            || matches!(self.inbox, CheckResult::Warn(_))
            || matches!(self.http_listener, CheckResult::Warn(_))
            || matches!(self.cargo_vet, CheckResult::Warn(_));
        if has_error {
            "UNHEALTHY"
        } else if has_warn {
            "DEGRADED"
        } else {
            "HEALTHY"
        }
    }
}

pub(crate) struct DoctorCommand {
    pub json_output: bool,
}

#[async_trait]
impl TerminalCommand for DoctorCommand {
    async fn execute(&self, _ctx: &GoogleContext) -> miette::Result<()> {
        let report = collect_diagnostics().await;

        if self.json_output {
            print_json(&report)?;
        } else {
            print_text(&report);
        }

        if report.verdict() == "UNHEALTHY" {
            std::process::exit(1);
        }
        Ok(())
    }
}

async fn collect_diagnostics() -> DoctorReport {
    let version = env!("CARGO_PKG_VERSION");
    let sha = env!("APHRODY_GIT_SHA");
    let built_unix: u64 = env!("APHRODY_BUILD_UNIX").parse().unwrap_or(0);
    let target = env!("APHRODY_TARGET");
    let built_date = if built_unix > 0 { epoch_to_date(built_unix) } else { "unknown".to_string() };
    let binary_version = format!("{version} (commit {sha}, built {built_date}, target {target})");

    let rustls_label = match rustls::crypto::ring::default_provider().install_default() {
        Ok(()) => "installed (ring)".to_string(),
        Err(_) => "already installed (ring)".to_string(),
    };

    let env_vars = ["RUST_BACKTRACE", "RUST_LOG", "TERM", "TZ"]
        .iter()
        .map(|v| (v.to_string(), std::env::var(v).unwrap_or_else(|_| "not set".to_string())))
        .collect();

    DoctorReport {
        binary_version,
        rustls_provider: rustls_label,
        reqwest_tls: "rustls",
        allocator: "active (native)",
        tokio_runtime: "multi-thread, full",
        ai_json: check_ai_json(),
        well_known_ai_json: check_well_known_ai_json(),
        peer_heartbeat: check_peer_heartbeat(),
        inbox: check_inbox(),
        http_listener: check_http_listener().await,
        cargo_vet: check_vet_audits(),
        env_vars,
    }
}

fn print_text(r: &DoctorReport) {
    println!("aphrody doctor — environment + integration diagnostics");
    println!();
    println!("[runtime]");
    println!("  binary version: {}", r.binary_version);
    println!("  rustls CryptoProvider: {}", r.rustls_provider);
    println!("  reqwest TLS backend: {}", r.reqwest_tls);
    println!("  mimalloc allocator: {}", r.allocator);
    println!("  tokio runtime: {}", r.tokio_runtime);
    println!("{}", r.ai_json.line("ai.json"));
    println!("{}", r.well_known_ai_json.line(".well-known/ai.json"));

    println!();
    println!("[peer A2A coord]");
    println!("{}", r.peer_heartbeat.line("peer winclean"));
    println!("{}", r.inbox.line("inbox-from-winclean.jsonl"));
    println!("{}", r.http_listener.line("HTTP listener"));

    println!();
    println!("[supply chain]");
    println!("  cargo-deny: not present at runtime — use the dev tool for CI");
    println!("{}", r.cargo_vet.line("cargo-vet audits"));

    println!();
    println!("[environment]");
    for (var, val) in &r.env_vars {
        println!("  {var}: {val}");
    }

    println!();
    let verdict = r.verdict();
    let detail = match verdict {
        "UNHEALTHY" => "(one or more critical checks failed)",
        "DEGRADED" => "(peer offline or non-critical resource missing)",
        _ => "(all critical checks pass)",
    };
    println!("Verdict: {verdict} {detail}");
}

fn print_json(r: &DoctorReport) -> miette::Result<()> {
    // Build structured JSON without deriving Serialize on internal types, to
    // keep the data model self-contained in this function.
    let env_obj: serde_json::Map<String, serde_json::Value> =
        r.env_vars.iter().map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone()))).collect();

    // Extract heartbeat age_s and is_stale from the peer_heartbeat check value.
    // The text carries "N s ago" — parse it back for the JSON field.
    let (heartbeat_age_s, heartbeat_is_stale) = parse_age_from_result(&r.peer_heartbeat);

    // Extract HTTP listener status code if present.
    let http_status_code = r
        .http_listener
        .value()
        .split("-> ")
        .nth(1)
        .and_then(|s| s.trim_end_matches(')').parse::<u16>().ok());

    let doc = serde_json::json!({
        "runtime": {
            "binary_version": r.binary_version,
            "rustls_provider": r.rustls_provider,
            "reqwest_tls": r.reqwest_tls,
            "allocator": r.allocator,
            "tokio_runtime": r.tokio_runtime,
            "ai_json": r.ai_json.value(),
            "well_known_ai_json": r.well_known_ai_json.value(),
        },
        "peer_a2a": {
            "heartbeat_age_s": heartbeat_age_s,
            "is_stale": heartbeat_is_stale,
            "heartbeat_detail": r.peer_heartbeat.value(),
            "http_listener_status": http_status_code,
            "http_listener_detail": r.http_listener.value(),
            "inbox_detail": r.inbox.value(),
        },
        "supply_chain": {
            "cargo_deny": "not present at runtime — use the dev tool for CI",
            "cargo_vet_audits": r.cargo_vet.value(),
        },
        "environment": serde_json::Value::Object(env_obj),
        "verdict": r.verdict(),
    });

    let text = serde_json::to_string_pretty(&doc)
        .map_err(|e| miette::miette!("JSON serialization error: {e}"))?;
    println!("{text}");
    Ok(())
}

/// Extract age in seconds from a `CheckResult` value string of the form
/// "... (N s ago — ...)". Returns `(None, false)` when the age cannot be
/// parsed (e.g. peer offline).
fn parse_age_from_result(r: &CheckResult) -> (Option<u64>, bool) {
    let text = r.value();
    // Look for pattern "(NNN s ago"
    if let Some(start) = text.find('(') {
        let inner = &text[start + 1..];
        if let Some(end) = inner.find(" s ago") {
            if let Ok(age) = inner[..end].trim().parse::<u64>() {
                let is_stale = age > HEARTBEAT_TTL_SECS;
                return (Some(age), is_stale);
            }
        }
    }
    (None, false)
}

// ── doctor helpers ────────────────────────────────────────────────────────────

/// Convert a Unix epoch (seconds) to a YYYY-MM-DD string without external deps.
fn epoch_to_date(unix: u64) -> String {
    // Days since 1970-01-01.  Zeller / proleptic Gregorian.
    let days = unix / 86400;
    // Algorithm from https://www.researchgate.net/publication/316558298
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

/// Locate repo root by walking up from the current exe directory.
fn repo_root() -> Option<std::path::PathBuf> {
    // Primary heuristic: walk up from cwd until we find a Cargo.toml with
    // `[workspace]`, which is reliable in both dev and release layouts.
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let candidate = dir.join("Cargo.toml");
        if candidate.exists() {
            if let Ok(content) = std::fs::read_to_string(&candidate) {
                if content.contains("[workspace]") {
                    return Some(dir);
                }
            }
        }
        if !dir.pop() {
            break;
        }
    }

    // Fallback: exe-relative (release layout: target/<triple>/release/aphrody)
    let exe = std::env::current_exe().ok()?;
    // Walk up exe path until Cargo.toml with [workspace] found.
    let mut dir = exe.parent()?.to_path_buf();
    loop {
        let candidate = dir.join("Cargo.toml");
        if candidate.exists() {
            if let Ok(content) = std::fs::read_to_string(&candidate) {
                if content.contains("[workspace]") {
                    return Some(dir);
                }
            }
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

fn check_ai_json() -> CheckResult {
    let root = match repo_root() {
        Some(r) => r,
        None => {
            return CheckResult::Warn("repo root not found — skip".to_string());
        },
    };
    let path = root.join("ai.json");
    if !path.exists() {
        return CheckResult::Err(format!("absent (expected {})", path.display()));
    }
    // Try to read schema version and A2A version from the JSON.
    let schema_v = match std::fs::read_to_string(&path) {
        Ok(text) => {
            let schema_ver = if text.contains("\"version\": \"1.0.0\"") {
                "schema v1.0.0"
            } else {
                "schema unknown"
            };
            let a2a_ver = if text.contains("a2a/v0.4") || text.contains("a2a_protocol_version") {
                "A2A v0.4"
            } else {
                "A2A unknown"
            };
            format!("{schema_ver}, {a2a_ver}")
        },
        Err(e) => return CheckResult::Warn(format!("unreadable ({e})")),
    };
    CheckResult::Ok(format!("present at {} ({schema_v})", path.display()))
}

fn check_well_known_ai_json() -> CheckResult {
    let root = match repo_root() {
        Some(r) => r,
        None => return CheckResult::Warn("repo root not found — skip".to_string()),
    };
    let path = root.join(".well-known").join("ai.json");
    if !path.exists() {
        return CheckResult::Warn("absent (non-critical for local dev)".to_string());
    }
    // Look for canonical_manifest field.
    let detail = match std::fs::read_to_string(&path) {
        Ok(text) => {
            if text.contains("canonical_manifest") || text.contains("../ai.json") {
                "present (canonical_manifest -> ../ai.json)".to_string()
            } else {
                "present".to_string()
            }
        },
        Err(e) => return CheckResult::Warn(format!("unreadable ({e})")),
    };
    CheckResult::Ok(detail)
}

fn check_peer_heartbeat() -> CheckResult {
    // The peer heartbeat file is written by the winclean side.  The canonical
    // path and TTL are declared in C:/winclean/.coord/ai.json under
    // `channels[type=heartbeat_file]`.  We use the hard-coded constants so
    // the check is self-contained and does not require parsing the coord ai.json
    // at runtime.
    let candidates = if cfg!(target_os = "windows") {
        vec![std::path::PathBuf::from(WINCLEAN_HEARTBEAT_PATH)]
    } else {
        // WSL / Linux: translate C:/ prefix.
        vec![
            std::path::PathBuf::from(WINCLEAN_HEARTBEAT_PATH.replace("C:/", "/c/")),
            std::path::PathBuf::from(WINCLEAN_HEARTBEAT_PATH.replace("C:/", "/mnt/c/")),
        ]
    };

    let path = match candidates.iter().find(|p| p.exists()) {
        Some(p) => p.clone(),
        None => {
            return CheckResult::Warn(format!(
                "absent at {WINCLEAN_HEARTBEAT_PATH} — peer offline"
            ));
        },
    };

    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => return CheckResult::Warn(format!("unreadable ({e})")),
    };

    let ts_str = text.trim().to_string();
    if ts_str.is_empty() {
        return CheckResult::Warn("heartbeat file is empty — peer not yet active".to_string());
    }

    match parse_iso8601_approx(&ts_str) {
        None => CheckResult::Ok(format!("heartbeat {ts_str} (age unknown)")),
        Some(ts_unix) => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs());
            let age_secs = now.saturating_sub(ts_unix);
            // Bug fix: use the declared TTL (600 s) as the staleness boundary.
            let (freshness, is_stale) = if age_secs > HEARTBEAT_TTL_SECS {
                (format!("stale (over {HEARTBEAT_TTL_SECS}s TTL)"), true)
            } else {
                ("fresh".to_string(), false)
            };
            let result = format!("heartbeat {ts_str} ({age_secs} s ago — {freshness})");
            if is_stale { CheckResult::Warn(result) } else { CheckResult::Ok(result) }
        },
    }
}

fn check_inbox() -> CheckResult {
    // The canonical inbox path is declared in the coord ai.json channels block
    // (channels[type=file_jsonl].outbox).  We resolve the platform-specific
    // equivalent of the C:/ path rather than looking in the aphrody repo root.
    let path = if cfg!(target_os = "windows") {
        std::path::PathBuf::from(WINCLEAN_INBOX_PATH)
    } else {
        // WSL / Linux: try both mount prefixes.
        let p1 = std::path::PathBuf::from(WINCLEAN_INBOX_PATH.replace("C:/", "/c/"));
        let p2 = std::path::PathBuf::from(WINCLEAN_INBOX_PATH.replace("C:/", "/mnt/c/"));
        if p1.exists() { p1 } else { p2 }
    };

    if !path.exists() {
        return CheckResult::Warn(format!("absent at {WINCLEAN_INBOX_PATH}"));
    }
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => return CheckResult::Warn(format!("unreadable ({e})")),
    };
    let envelope_count = text.lines().filter(|l| !l.trim().is_empty()).count();
    // Extract the id field from the last non-empty line.
    let last_id = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .next_back()
        .and_then(|line| {
            extract_json_str_field(line, "id").or_else(|| extract_json_str_field(line, "from"))
        })
        .unwrap_or_else(|| "unknown".to_string());
    CheckResult::Ok(format!("{envelope_count} envelopes, last id={last_id}"))
}

async fn check_http_listener() -> CheckResult {
    let url = "http://localhost:8788/ping";
    // Build a reqwest client with a short timeout so we don't block the
    // user for multi-second hangs when the peer listener is down.
    let client = match reqwest::Client::builder().timeout(std::time::Duration::from_secs(2)).build()
    {
        Ok(c) => c,
        Err(e) => return CheckResult::Warn(format!("could not build HTTP client: {e}")),
    };

    match client.get(url).send().await {
        Ok(resp) if resp.status().is_success() => {
            CheckResult::Ok(format!("localhost:8788 (alive, /ping -> {})", resp.status().as_u16()))
        },
        Ok(resp) => {
            CheckResult::Warn(format!("localhost:8788 (non-200: {})", resp.status().as_u16()))
        },
        Err(_) => {
            CheckResult::Warn("localhost:8788 (no response — listener not running)".to_string())
        },
    }
}

fn check_vet_audits() -> CheckResult {
    let root = match repo_root() {
        Some(r) => r,
        None => return CheckResult::Warn("repo root not found — skip".to_string()),
    };
    let path = root.join("supply-chain").join("audits.toml");
    if !path.exists() {
        return CheckResult::Warn(format!("absent (expected {})", path.display()));
    }
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => return CheckResult::Warn(format!("unreadable ({e})")),
    };
    // Count `[[audits.<crate>]]` style entries.
    let entries = text
        .lines()
        .filter(|l| {
            let t = l.trim();
            t.starts_with("[[audits.") || (t.starts_with("[audits.") && !t.starts_with("[audits]"))
        })
        .count();
    CheckResult::Ok(format!("present at {} ({entries} entries)", path.display()))
}

/// Extract the string value for a JSON key using simple substring search.
/// Works for simple flat fields like `"key": "value"`.
fn extract_json_str_field(text: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let start = text.find(&needle)?;
    let after_key = &text[start + needle.len()..];
    // Skip whitespace and colon.
    let colon = after_key.find(':')?;
    let after_colon = after_key[colon + 1..].trim_start();
    if after_colon.starts_with('"') {
        let inner = &after_colon[1..];
        let end = inner.find('"')?;
        Some(inner[..end].to_string())
    } else {
        None
    }
}

/// Parse an ISO-8601 datetime string to a Unix timestamp (seconds, UTC approx).
/// Handles `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS+HH:MM` forms.
fn parse_iso8601_approx(s: &str) -> Option<u64> {
    // Expect at least YYYY-MM-DD (10 chars).
    if s.len() < 10 {
        return None;
    }
    let year: u64 = s[0..4].parse().ok()?;
    let month: u64 = s[5..7].parse().ok()?;
    let day: u64 = s[8..10].parse().ok()?;

    let (hour, min, sec) = if s.len() >= 19 {
        let h: u64 = s[11..13].parse().ok()?;
        let m: u64 = s[14..16].parse().ok()?;
        let sc: u64 = s[17..19].parse().ok()?;
        (h, m, sc)
    } else {
        (0, 0, 0)
    };

    // Parse optional timezone offset (+HH:MM or -HH:MM).
    let tz_offset_secs: i64 = if s.len() >= 25 {
        let tz = &s[19..];
        let tz = tz.trim_start_matches('Z');
        if tz.len() >= 6 && (tz.starts_with('+') || tz.starts_with('-')) {
            let sign: i64 = if tz.starts_with('+') { 1 } else { -1 };
            let tz_h: i64 = tz[1..3].parse().ok()?;
            let tz_m: i64 = tz[4..6].parse().ok()?;
            sign * (tz_h * 3600 + tz_m * 60)
        } else {
            0
        }
    } else {
        0
    };

    // Days since epoch using the same Gregorian formula as `epoch_to_date`.
    // Convert calendar date to JDN then to Unix days.
    let a = (14u64.wrapping_sub(month)) / 12;
    let y = year + 4800 - a;
    let m_adj = month + 12 * a - 3;
    let jdn = day + (153 * m_adj + 2) / 5 + 365 * y + y / 4 - y / 100 + y / 400 - 32045;
    // JDN of 1970-01-01 is 2440588.
    let unix_days = jdn.saturating_sub(2440588);
    let unix_secs = unix_days * 86400 + hour * 3600 + min * 60 + sec;
    let adjusted = unix_secs as i64 - tz_offset_secs;
    Some(adjusted.max(0) as u64)
}

// ── aphrody tokens ───────────────────────────────────────────────────────────

pub(crate) struct TokensCommand {
    pub url: String,
    pub output: PathBuf,
    pub force: bool,
}

#[async_trait]
impl TerminalCommand for TokensCommand {
    async fn execute(&self, _ctx: &GoogleContext) -> miette::Result<()> {
        if self.output.exists() && !self.force {
            return Err(miette::miette!(
                "Output file {} already exists. Use --force to overwrite.",
                self.output.display()
            ));
        }

        let client =
            ScrapeClient::new().map_err(|e| miette::miette!("HTTP client init failed: {e}"))?;

        let token_map =
            client.extract_m3_tokens(&self.url).await.map_err(|e| miette::miette!("{e}"))?;

        let json_text = serde_json::to_string_pretty(&token_map)
            .map_err(|e| miette::miette!("JSON serialization error: {e}"))?;

        if let Some(parent) = self.output.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                miette::miette!("Cannot create output directory {}: {e}", parent.display())
            })?;
        }

        std::fs::write(&self.output, &json_text)
            .map_err(|e| miette::miette!("Cannot write {}: {e}", self.output.display()))?;

        println!(
            "M3 tokens written to {} ({} entries)",
            self.output.display(),
            token_map.tokens.len()
        );

        Ok(())
    }
}

// ── aphrody n2b ──────────────────────────────────────────────────────────────

/// Locate the `packages/n2b/src/cli.ts` entry-point relative to the repository
/// root.  Falls back to `APHRODY_N2B_CLI` env override for packaged installs.
fn resolve_n2b_cli() -> anyhow::Result<std::path::PathBuf> {
    // 1. Explicit override (packaged install or CI).
    if let Ok(v) = std::env::var("APHRODY_N2B_CLI") {
        let p = std::path::PathBuf::from(v);
        if p.exists() {
            return Ok(p);
        }
    }

    // 2. Walk up from cwd to find the workspace root, then resolve the
    //    well-known path.
    if let Some(root) = repo_root() {
        let candidate = root.join("packages").join("n2b").join("src").join("cli.ts");
        if candidate.exists() {
            return Ok(candidate);
        }
        return Err(anyhow::anyhow!(
            "n2b CLI not found at {}. Run `bun install` inside packages/n2b/ first.",
            candidate.display()
        ));
    }

    Err(anyhow::anyhow!(
        "Cannot locate repository root. Set APHRODY_N2B_CLI to the absolute path of \
         packages/n2b/src/cli.ts."
    ))
}

/// Spawn `bun run <n2b-cli> -- <args>` and stream its stdout/stderr to the
/// terminal.  The caller's exit code is propagated via `std::process::exit`.
async fn spawn_n2b(cli_path: &std::path::Path, args: &[String]) -> anyhow::Result<()> {
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tokio::process::Command;

    let mut cmd = Command::new("bun");
    cmd.arg("run").arg(cli_path).arg("--").args(args);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| anyhow::anyhow!("Failed to spawn `bun`: {e}. Is bun installed and on PATH?"))?;

    // Stream stdout.
    let stdout = child.stdout.take().expect("stdout piped");
    let stderr = child.stderr.take().expect("stderr piped");

    let stdout_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            println!("{line}");
        }
    });
    let stderr_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            eprintln!("{line}");
        }
    });

    let status = child.wait().await.map_err(|e| anyhow::anyhow!("n2b process wait failed: {e}"))?;
    let _ = tokio::join!(stdout_task, stderr_task);

    if !status.success() {
        let code = status.code().unwrap_or(1);
        anyhow::bail!("n2b exited with code {code}");
    }
    Ok(())
}

pub(crate) struct N2bCommand {
    pub args: Vec<String>,
}

#[async_trait]
impl TerminalCommand for N2bCommand {
    async fn execute(&self, _ctx: &GoogleContext) -> miette::Result<()> {
        // Intercept `watch` as a Rust-side loop so the "kernel" owns the
        // supervision loop rather than delegating it to the TS process.
        // Syntax: aphrody n2b watch [--interval <secs>] [--path <glob>] [-- <extra-n2b-args>]
        if self.args.first().map(|s| s.as_str()) == Some("watch") {
            return self.execute_watch().await;
        }

        let cli_path =
            resolve_n2b_cli().map_err(|e| miette::miette!("n2b CLI resolution failed: {e}"))?;
        spawn_n2b(&cli_path, &self.args).await.map_err(|e| miette::miette!("{e}"))
    }
}

impl N2bCommand {
    /// `aphrody n2b watch [--interval <secs>] [--path <glob>] [-- <extra>]`
    ///
    /// Runs `n2b scan [--path] [extra]` in a Rust `loop`, sleeping `interval_s`
    /// seconds between iterations.  Exits cleanly on Ctrl-C (SIGINT) via
    /// `tokio::signal::ctrl_c()`.
    async fn execute_watch(&self) -> miette::Result<()> {
        // Parse the watch-specific flags from self.args (skip "watch" itself).
        let tail = &self.args[1..];
        let mut interval_s: u64 = 60;
        let mut scan_path: Option<String> = None;
        let mut passthrough: Vec<String> = Vec::new();
        let mut iter = tail.iter().peekable();
        while let Some(arg) = iter.next() {
            if arg == "--" {
                passthrough.extend(iter.cloned());
                break;
            } else if arg == "--interval" {
                if let Some(v) = iter.next() {
                    interval_s = v.parse::<u64>().map_err(|_| {
                        miette::miette!("--interval must be a positive integer (got '{v}')")
                    })?;
                }
            } else if let Some(v) = arg.strip_prefix("--interval=") {
                interval_s = v.parse::<u64>().map_err(|_| {
                    miette::miette!("--interval must be a positive integer (got '{v}')")
                })?;
            } else if arg == "--path" {
                scan_path = iter.next().cloned();
            } else if let Some(v) = arg.strip_prefix("--path=") {
                scan_path = Some(v.to_owned());
            } else {
                passthrough.push(arg.clone());
            }
        }

        let cli_path = resolve_n2b_cli()
            .map_err(|e| miette::miette!("n2b CLI resolution failed: {e}"))?;

        eprintln!(
            "[n2b watch] interval={}s path={} — press Ctrl-C to stop",
            interval_s,
            scan_path.as_deref().unwrap_or(".")
        );

        loop {
            let mut scan_args = Vec::new();
            if let Some(ref p) = scan_path {
                scan_args.push(p.clone());
            }
            scan_args.extend(passthrough.clone());

            tokio::select! {
                biased;
                _ = tokio::signal::ctrl_c() => {
                    eprintln!("[n2b watch] Ctrl-C received — exiting");
                    break;
                }
                result = spawn_n2b(&cli_path, &scan_args) => {
                    if let Err(e) = result {
                        eprintln!("[n2b watch] scan error: {e}");
                    }
                }
            }

            tokio::select! {
                biased;
                _ = tokio::signal::ctrl_c() => {
                    eprintln!("[n2b watch] Ctrl-C received — exiting");
                    break;
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(interval_s)) => {}
            }
        }

        Ok(())
    }
}

// ── aphrody bxc ──────────────────────────────────────────────────────────────

pub(crate) struct BxcCommand {
    pub action: crate::BxcAction,
}

#[async_trait]
impl TerminalCommand for BxcCommand {
    async fn execute(&self, _ctx: &GoogleContext) -> miette::Result<()> {
        use crate::BxcAction;
        match &self.action {
            BxcAction::Daemon { port } => bxc_daemon(*port).await,
            BxcAction::Recon { url } => bxc_recon(url).await,
            BxcAction::Scrape { url, selector } => bxc_scrape(url, selector.as_deref()).await,
            BxcAction::Detect { url } => bxc_detect(url).await,
            BxcAction::Tokens { url } => bxc_tokens(url).await,
        }
    }
}

/// Resolve `var/run/` directory under the repo root, creating it if needed.
fn var_run_dir() -> anyhow::Result<std::path::PathBuf> {
    let root = repo_root()
        .ok_or_else(|| anyhow::anyhow!("Cannot locate repository root for var/run/bxc.pid"))?;
    let dir = root.join("var").join("run");
    std::fs::create_dir_all(&dir)
        .map_err(|e| anyhow::anyhow!("Cannot create {}: {e}", dir.display()))?;
    Ok(dir)
}

/// Start the bxc-engine daemon as a detached background process.
///
/// The daemon PID is written to `var/run/bxc.pid` so subsequent commands
/// (and the user) can check whether the engine is alive.
///
/// If `bxc-engine` is not on PATH, falls back to `bun run` against the
/// worktree location `/c/worktree/bxc/` (Windows) or `/worktree/bxc/`
/// (Linux).
async fn bxc_daemon(port: u16) -> miette::Result<()> {
    let pid_dir = var_run_dir().map_err(|e| miette::miette!("{e}"))?;
    let pid_file = pid_dir.join("bxc.pid");

    // Check if a daemon is already running at the stored PID.
    if let Ok(text) = std::fs::read_to_string(&pid_file) {
        if let Ok(existing_pid) = text.trim().parse::<u32>() {
            // Portable liveness check: on Unix use `kill -0 <pid>` via a shell
            // command; on Windows probe via `tasklist /FI "PID eq <pid>"`.
            let alive = is_pid_alive(existing_pid);
            if alive {
                eprintln!(
                    "[bxc daemon] already running (pid={existing_pid}). \
                     Stop it first or remove {}.",
                    pid_file.display()
                );
                return Ok(());
            }
        }
    }

    // Resolve the bxc-engine executable.
    let (program, args): (&str, Vec<String>) =
        if which_in_path("bxc-engine") {
            ("bxc-engine", vec!["serve".into(), format!("--port={port}")])
        } else {
            // Fallback: run via bun inside the worktree.
            let bxc_worktree = if cfg!(target_os = "windows") {
                "/c/worktree/bxc"
            } else {
                "/worktree/bxc"
            };
            let entry = format!("{bxc_worktree}/src/serve.ts");
            ("bun", vec!["run".into(), entry, format!("--port={port}")])
        };

    let mut cmd = std::process::Command::new(program);
    cmd.args(&args);

    // Detach from the terminal on Unix; on Windows use DETACHED_PROCESS.
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x00000008;
        cmd.creation_flags(DETACHED_PROCESS);
    }

    let child = cmd
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| {
            miette::miette!(
                "Failed to spawn bxc-engine: {e}. \
                 Ensure `bxc-engine` is on PATH or /c/worktree/bxc/ is checked out."
            )
        })?;

    let pid = child.id();
    std::fs::write(&pid_file, pid.to_string()).map_err(|e| {
        miette::miette!("Cannot write PID file {}: {e}", pid_file.display())
    })?;

    println!("[bxc daemon] started pid={pid} port={port}. PID file: {}", pid_file.display());
    Ok(())
}

/// Check whether a process identified by `pid` is currently alive.
///
/// Uses platform-native probes without requiring the `libc` crate:
/// - Unix: `kill -0 <pid>` (signal 0 = existence check, no signal delivered).
/// - Windows: `tasklist /FI "PID eq <pid>"` and grep the output.
fn is_pid_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        let out = std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .output();
        matches!(out, Ok(o) if o.status.success())
    }
    #[cfg(windows)]
    {
        let out = std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .output();
        out.map(|o| String::from_utf8_lossy(&o.stdout).contains(&pid.to_string()))
            .unwrap_or(false)
    }
    #[cfg(not(any(unix, windows)))]
    {
        // On other targets (e.g. wasm32 — unreachable at runtime) assume dead.
        let _ = pid;
        false
    }
}

/// Returns true if `name` resolves to an executable on PATH.
fn which_in_path(name: &str) -> bool {
    std::env::var_os("PATH")
        .map(|path_var| {
            std::env::split_paths(&path_var).any(|dir| {
                let candidate = dir.join(name);
                candidate.exists()
                    || dir
                        .join(format!("{name}.exe"))
                        .exists()
            })
        })
        .unwrap_or(false)
}

async fn bxc_recon(url: &str) -> miette::Result<()> {
    let client =
        ScrapeClient::new().map_err(|e| miette::miette!("HTTP client init failed: {e}"))?;
    let result = client.recon(url).await.map_err(|e| miette::miette!("{e}"))?;
    let text = serde_json::to_string_pretty(&result)
        .map_err(|e| miette::miette!("JSON serialization error: {e}"))?;
    println!("{text}");
    Ok(())
}

async fn bxc_scrape(url: &str, selector: Option<&str>) -> miette::Result<()> {
    let client =
        ScrapeClient::new().map_err(|e| miette::miette!("HTTP client init failed: {e}"))?;
    let sel = selector.unwrap_or("body");
    let result = client.scrape(url, sel).await.map_err(|e| miette::miette!("{e}"))?;
    let text = serde_json::to_string_pretty(&result)
        .map_err(|e| miette::miette!("JSON serialization error: {e}"))?;
    println!("{text}");
    Ok(())
}

async fn bxc_detect(url: &str) -> miette::Result<()> {
    let client =
        ScrapeClient::new().map_err(|e| miette::miette!("HTTP client init failed: {e}"))?;
    let result = client.detect(url).await.map_err(|e| miette::miette!("{e}"))?;
    let text = serde_json::to_string_pretty(&result)
        .map_err(|e| miette::miette!("JSON serialization error: {e}"))?;
    println!("{text}");
    Ok(())
}

async fn bxc_tokens(url: &str) -> miette::Result<()> {
    let client =
        ScrapeClient::new().map_err(|e| miette::miette!("HTTP client init failed: {e}"))?;
    let result = client.extract_m3_tokens(url).await.map_err(|e| miette::miette!("{e}"))?;
    let text = serde_json::to_string_pretty(&result)
        .map_err(|e| miette::miette!("JSON serialization error: {e}"))?;
    println!("{text}");
    Ok(())
}

// ── aphrody term ─────────────────────────────────────────────────────────────

/// Arguments for the `term` subcommand, mirroring the clap variant fields.
pub(crate) struct TermCommand {
    /// WebSocket bind address as a raw string (parsed at runtime).
    pub addr: String,
    /// Optional shell override forwarded to the backend.
    pub shell: Option<String>,
    /// Optional working directory forwarded to the backend.
    pub cwd: Option<PathBuf>,
}

#[async_trait]
impl TerminalCommand for TermCommand {
    async fn execute(&self, _ctx: &GoogleContext) -> miette::Result<()> {
        let addr: SocketAddr = self
            .addr
            .parse()
            .map_err(|e| miette::miette!("invalid --addr '{}': {e}", self.addr))?;

        let cfg = aphrody_terminal_backend::PtyConfig {
            cols: 80,
            rows: 24,
            shell: self.shell.clone(),
            cwd: self.cwd.clone(),
        };

        eprintln!("aphrody term: ws://{addr} (open the WASM UI to connect)");

        tokio::select! {
            res = aphrody_terminal_backend::serve(addr, cfg) => {
                res.map_err(|e| miette::miette!("terminal server error: {e:#}"))?;
            }
            _ = tokio::signal::ctrl_c() => {
                eprintln!("\naphrody term: received Ctrl-C, shutting down");
            }
        }

        Ok(())
    }
}
