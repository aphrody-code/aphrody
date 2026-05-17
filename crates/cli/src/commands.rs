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
    platform,
    scrape::ScrapeClient,
};

pub(crate) struct VersionCommand;

#[async_trait]
impl TerminalCommand for VersionCommand {
    async fn execute(&self, ctx: &GoogleContext) -> miette::Result<()> {
        println!("{}", format!("🚀 Google OS Terminal CLI v{}", ctx.version).bold().cyan());
        // Résolution de la racine pour vérification
        let root = ctx.vfs.resolve("/var/mirror").unwrap_or_default();
        println!("🛡️ VFS /var/mirror: {}", root.display().to_string().yellow());
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

        let bun_commands = [
            "x", "repl", "link", "unlink", "patch", "pm", "info", "why", "create", "c", "feedback",
        ];
        let uv_commands = [
            "auth", "version", "sync", "lock", "export", "tree", "format", "tool", "python", "pip",
            "venv", "cache", "self",
        ];
        let cargo_commands =
            ["check", "clippy", "doc", "fmt", "fetch", "fix", "clean", "metadata", "tree"];

        let bypass_engines = [
            "bun", "uv", "cargo", "winget", "apt", "go", "npm", "yarn", "pnpm", "node", "npx",
            "deno", "python", "pip", "git", "docker", "make", "cmake",
        ];

        // Explicit engine bypass
        if bypass_engines.contains(&first_arg.as_str()) {
            return Self::run_process(first_arg, &self.args[1..]);
        }
        // Specific commands
        else if uv_commands.contains(&first_arg.as_str()) {
            return Self::run_process("uv", &self.args[..]);
        } else if bun_commands.contains(&first_arg.as_str()) {
            return Self::run_process("bun", &self.args[..]);
        } else if cargo_commands.contains(&first_arg.as_str()) {
            return Self::run_process("cargo", &self.args[..]);
        }

        let standard_actions = [
            "run", "exec", "test", "build", "dev", "start", "install", "i", "add", "remove", "rm",
            "fmt", "lint", "check", "clean", "update", "upgrade", "publish", "npm", "yarn", "pnpm",
            "node", "npx", "deno", "python", "pip", "git", "docker", "make", "cmake",
        ];

        let is_known_technical_command = standard_actions.contains(&first_arg.as_str());
        let is_script_file = first_arg.ends_with(".py")
            || first_arg.ends_with(".js")
            || first_arg.ends_with(".ts")
            || first_arg.ends_with(".jsx")
            || first_arg.ends_with(".tsx")
            || first_arg.ends_with(".rs")
            || first_arg.ends_with(".sh");

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
