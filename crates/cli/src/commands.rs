// SPDX-License-Identifier: Apache-2.0
use std::{net::SocketAddr, path::PathBuf};

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
        BYPASS_ENGINES, CARGO_COMMANDS, SCRIPT_EXTS, STANDARD_ACTIONS, UV_COMMANDS,
    },
    platform,
};

/// A subprocess forwarded by aphrody exited with a non-zero status code.
///
/// This error is produced instead of calling `std::process::exit` directly in
/// library code.  The single authoritative `process::exit` call lives in
/// `main.rs` which downcasts `miette::Report` to this type and propagates the
/// exit code.
#[derive(Debug, miette::Diagnostic)]
pub(crate) struct SubprocessExit(pub(crate) i32);

impl std::fmt::Display for SubprocessExit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "subprocess exited with code {}", self.0)
    }
}

impl std::error::Error for SubprocessExit {}

pub(crate) struct VersionCommand {
    pub json: bool,
}

#[async_trait]
impl TerminalCommand for VersionCommand {
    async fn execute(&self, _ctx: &GoogleContext) -> miette::Result<()> {
        if self.json {
            let info = serde_json::json!({
                "version": env!("CARGO_PKG_VERSION"),
                "commit": env!("APHRODY_GIT_SHA"),
                "built": env!("APHRODY_BUILD_UNIX"),
                "target": env!("APHRODY_TARGET"),
                "profile": env!("APHRODY_PROFILE"),
                "repo": "https://github.com/aphrody-code/aphrody",
                "license": "Apache-2.0",
                "a2a": "ai.json v1 (AGNTCY a2a/v0.4) @ /ai.json + /.well-known/ai.json"
            });
            println!("{}", serde_json::to_string_pretty(&info).unwrap());
            return Ok(());
        }

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
            println!("[ok] mirror started");
        } else {
            println!("[skip] no-op mirror action: {}", self.action);
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

// ---------------------------------------------------------------------------
// `aphrody chromium export-session` — unified Google session JSON
// ---------------------------------------------------------------------------

pub(crate) struct ChromiumExportSessionCommand {
    pub profile: String,
    pub out: Option<PathBuf>,
    pub domain: String,
}

#[derive(serde::Serialize)]
struct GoogleSessionExport {
    schema: &'static str,
    generated_at: String,
    profile: String,
    domain_filter: String,
    #[cfg(target_os = "windows")]
    cookies: Vec<backend::chromium::Cookie>,
    gemini_oauth: Option<serde_json::Value>,
    stats: ExportStats,
}

#[derive(serde::Serialize)]
struct ExportStats {
    total_cookies: usize,
    httponly_count: usize,
    secure_count: usize,
    session_count: usize,
    has_gemini_oauth: bool,
}

/// Read `~/.gemini/oauth_creds.json` and return the parsed JSON, if present.
/// Errors are swallowed — the export is still useful with cookies alone, and
/// the gemini token may simply not exist yet.
fn read_gemini_oauth() -> Option<serde_json::Value> {
    let home = platform::home_dir().ok()?;
    let path = home.join(".gemini").join("oauth_creds.json");
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Read the Antigravity CLI (`agy`) Google OAuth token directly from the
/// Windows Credential Manager.
///
/// gemini-cli was absorbed into the Antigravity CLI; the installer
/// (`antigravity.google/cli/install.ps1`) drops `agy.exe` under
/// `%LOCALAPPDATA%\agy\bin`, and the IDE-side auth stores the Google OAuth
/// token as a **generic credential** named `gemini:antigravity` whose blob is
/// plaintext JSON: `{"token":{"access_token":"ya29…","refresh_token":…}}`.
/// Reading it here lets `aphrody auth` extract the session straight from `agy`
/// with no subprocess and no dependency on the (now-missing) gemini-cli bundle.
#[cfg(target_os = "windows")]
fn read_antigravity_oauth() -> Option<serde_json::Value> {
    use windows_sys::Win32::Security::Credentials::{
        CRED_TYPE_GENERIC, CredFree, CredReadW, CREDENTIALW,
    };

    let target: Vec<u16> =
        "gemini:antigravity".encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: `target` is a NUL-terminated UTF-16 string. On success CredReadW
    // hands back one heap-allocated CREDENTIALW that we read once and free with
    // CredFree; the blob slice lives only for the duration of the read.
    unsafe {
        let mut pcred: *mut CREDENTIALW = std::ptr::null_mut();
        if CredReadW(target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut pcred) == 0 || pcred.is_null() {
            return None;
        }
        let cred = &*pcred;
        let parsed = if cred.CredentialBlob.is_null() || cred.CredentialBlobSize == 0 {
            None
        } else {
            let blob =
                std::slice::from_raw_parts(cred.CredentialBlob, cred.CredentialBlobSize as usize);
            serde_json::from_slice::<serde_json::Value>(blob).ok()
        };
        CredFree(pcred.cast());
        parsed
    }
}

#[cfg(target_os = "windows")]
#[async_trait]
impl TerminalCommand for ChromiumExportSessionCommand {
    async fn execute(&self, _ctx: &GoogleContext) -> miette::Result<()> {
        println!(
            "📦 Export Google session — profil = '{}', filtre domaine = '%{}%'",
            self.profile, self.domain
        );

        let user_data = platform::chrome_user_data()
            .ok_or_else(|| miette::miette!("Chrome user-data path not known on this platform"))?;

        let mut parser = ChromiumParser::new(user_data);
        let profiles = parser.get_profiles();
        if !profiles.iter().any(|p| p == &self.profile) {
            return Err(miette::miette!(
                "Profil '{}' introuvable. Profils disponibles: {:?}",
                self.profile,
                profiles
            ));
        }
        parser.load_master_key().map_err(|e| miette::miette!(e.to_string()))?;
        println!("🔑 Master Key déchiffrée.");

        let cookies = parser
            .get_cookies_full(&self.profile, &self.domain)
            .map_err(|e| miette::miette!("get_cookies_full: {e:#}"))?;
        println!("🍪 Cookies extraits : {} entrées", cookies.len());

        let gemini_oauth = read_gemini_oauth();
        match &gemini_oauth {
            Some(_) => println!("🔓 Token Gemini CLI fusionné depuis ~/.gemini/oauth_creds.json"),
            None => println!(
                "⚠️  ~/.gemini/oauth_creds.json absent ou illisible — section gemini_oauth = null"
            ),
        }

        let stats = ExportStats {
            total_cookies: cookies.len(),
            httponly_count: cookies.iter().filter(|c| c.is_httponly).count(),
            secure_count: cookies.iter().filter(|c| c.is_secure).count(),
            session_count: cookies.iter().filter(|c| c.is_session).count(),
            has_gemini_oauth: gemini_oauth.is_some(),
        };

        let export = GoogleSessionExport {
            schema: "aphrody.google-session/v1",
            generated_at: chrono::Utc::now().to_rfc3339(),
            profile: self.profile.clone(),
            domain_filter: self.domain.clone(),
            cookies,
            gemini_oauth,
            stats,
        };

        let out_path = match self.out.clone() {
            Some(p) => p,
            None => {
                let home = platform::home_dir()
                    .map_err(|_| miette::miette!("home dir unknown"))?;
                let dir = home.join(".aphrody");
                std::fs::create_dir_all(&dir)
                    .map_err(|e| miette::miette!("mkdir {}: {e}", dir.display()))?;
                dir.join("google-session.json")
            },
        };

        let json = serde_json::to_string_pretty(&export)
            .map_err(|e| miette::miette!("serialize: {e}"))?;
        std::fs::write(&out_path, &json)
            .map_err(|e| miette::miette!("write {}: {e}", out_path.display()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&out_path, std::fs::Permissions::from_mode(0o600));
        }

        println!("✅ Session écrite : {} ({} octets)", out_path.display(), json.len());
        println!("   • cookies         : {}", export.stats.total_cookies);
        println!("   • httpOnly        : {}", export.stats.httponly_count);
        println!("   • secure          : {}", export.stats.secure_count);
        println!("   • session-only    : {}", export.stats.session_count);
        println!(
            "   • gemini_oauth    : {}",
            if export.stats.has_gemini_oauth { "yes" } else { "no" }
        );
        Ok(())
    }
}

#[cfg(not(target_os = "windows"))]
#[async_trait]
impl TerminalCommand for ChromiumExportSessionCommand {
    async fn execute(&self, _ctx: &GoogleContext) -> miette::Result<()> {
        Err(miette::miette!(
            "`chromium export-session` is Windows-only (DPAPI master-key path)."
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

        // gemini-cli is deprecated — it was absorbed into the Antigravity CLI
        // (`agy`, installed at %LOCALAPPDATA%\agy\bin per the official
        // install.ps1), and `@google/gemini-cli/bundle/gemini.js` no longer
        // exists. We no longer shell out to it. Instead, extract the Google
        // OAuth token directly from the Windows Credential Manager entry
        // `gemini:antigravity` that agy writes, then fall back to an on-disk
        // token.
        #[cfg(target_os = "windows")]
        if let Some(tok) = read_antigravity_oauth() {
            if let Some(access) = tok.pointer("/token/access_token").and_then(|v| v.as_str()) {
                Self::persist_god_mode_token("antigravity", access)?;
                println!(
                    "🔓 Token Google extrait directement depuis l'Antigravity CLI (agy) — \
                     Credential Manager `gemini:antigravity`."
                );
                return Ok(());
            }
            println!("⚠️ Credential `gemini:antigravity` présent mais sans `token.access_token`.");
        }

        // Last resort: reuse an existing on-disk token (gemini/agy shared store).
        if read_gemini_oauth().is_some() {
            println!("🔓 Token réutilisé depuis ~/.gemini/oauth_creds.json.");
            return Ok(());
        }

        Err(miette::miette!(
            "Aucune source d'authentification disponible. Connectez-vous via l'Antigravity CLI \
             (`agy`, %LOCALAPPDATA%\\agy\\bin\\agy.exe) puis relancez `aphrody auth`. \
             gemini-cli n'est plus utilisé."
        ))
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
                        .expect("Invalid progress bar template"),
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
                // SAFETY: Setting environment variables in a multithreaded process is inherently
                // unsafe. However, this CLI command runs as a short-lived process
                // and this mutation happens before the child process is spawned, so
                // the race window is minimal in practice.
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

pub(crate) struct AutoCommand {
    pub args: Vec<String>,
}

#[async_trait]
impl TerminalCommand for AutoCommand {
    async fn execute(&self, _ctx: &GoogleContext) -> miette::Result<()> {
        // Policy 100% Rust (memory `feedback_aphrody_rust_only`) :
        // `AutoCommand` n'auto-spawn JAMAIS `bun` implicitement. Les
        // utilisateurs qui veulent invoquer bun passent par BYPASS_ENGINES
        // (`aphrody bun add foo`). Le runtime par défaut est `cargo`.
        if self.args.is_empty() {
            eprintln!(
                "⚠️  [Auto] No arguments. Usage: `aphrody <cmd> [args]` or `aphrody \"<NL prompt>\"`.\n   \
                 Default engine is `cargo` (workspace 100% Rust). Use BYPASS engines explicitly: \
                 `aphrody bun add foo`, `aphrody uv sync`, `aphrody npm i`, etc."
            );
            return Err(miette::Report::new(SubprocessExit(2)));
        }

        let first_arg = &self.args[0];

        // All category lists (BYPASS_ENGINES, UV_COMMANDS, CARGO_COMMANDS,
        // STANDARD_ACTIONS, SCRIPT_EXTS) come from `crate::nl_tokens` so
        // the NL detector in `main.rs` and this dispatcher cannot drift.
        //
        // Note (Phase 5, 2026-05-18) : `BUN_COMMANDS` a été retiré du
        // dispatcher implicite — bun reste accessible via BYPASS_ENGINES.

        // Explicit engine bypass (`bun`, `uv`, `cargo`, `npm`, …)
        if BYPASS_ENGINES.contains(&first_arg.as_str()) {
            return Self::run_process(first_arg, &self.args[1..]);
        }
        // Specific commands
        else if UV_COMMANDS.contains(&first_arg.as_str()) {
            return Self::run_process("uv", &self.args[..]);
        } else if CARGO_COMMANDS.contains(&first_arg.as_str()) {
            return Self::run_process("cargo", &self.args[..]);
        }

        let is_known_technical_command = STANDARD_ACTIONS.contains(&first_arg.as_str());
        let is_script_file = SCRIPT_EXTS.iter().any(|ext| first_arg.ends_with(ext));

        // Overlapping Universal Commands (run, install, add, remove, test, build...)
        let is_run_script = first_arg == "run" || first_arg == "exec";

        // Default engine = `cargo` (Rust-first policy). Bun/Node ne sont
        // jamais sélectionnés implicitement par le dispatcher.
        let mut target_engine = "cargo";

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
            // Policy 100% Rust : aucun spawn implicite vers un runtime JS.
            // L'utilisateur doit passer par le BYPASS explicite
            // (`aphrody bun run X.ts`) ou installer un runtime tiers.
            eprintln!(
                "⚠️  [Auto] `{first_arg}` est un script JS/TS — le dispatcher 100% Rust ne \
                 spawn pas de runtime JS implicitement.\n   \
                 Utiliser le bypass explicite : `aphrody bun run {first_arg}` (ou \
                 `aphrody node {first_arg}`, `aphrody deno run {first_arg}`)."
            );
            return Err(miette::Report::new(SubprocessExit(2)));
        } else if first_arg.ends_with(".rs") {
            let mut new_args = vec!["run".to_string()];
            new_args.extend(self.args.clone());
            return Self::run_process("cargo", new_args);
        } else {
            // Contextual ecosystem detection. Note : `package.json` ne
            // sélectionne plus `bun` implicitement (policy 100% Rust) ;
            // l'utilisateur passe par `aphrody bun <cmd>` au besoin.
            let has_cargo = std::path::Path::new("Cargo.toml").exists();
            let has_uv = std::path::Path::new("pyproject.toml").exists()
                || std::path::Path::new("requirements.txt").exists();
            let has_go = std::path::Path::new("go.mod").exists();

            if has_cargo {
                target_engine = "cargo";
            } else if has_uv {
                target_engine = "uv";
            } else if has_go {
                target_engine = "go";
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
            return Err(miette::Report::new(SubprocessExit(status.code().unwrap_or(1))));
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
        // Délègue à la chain unifiée `gemini_runtime::resolve_bin()` :
        // 1. $APHRODY_GEMINI_BIN  2. sibling exe (post-bootstrap)
        // 3. packages/gemini-cli/bundle/gemini[.exe|.js] (fork in-tree)
        // 4. which("gemini") (upstream PATH). Cf. CLAUDE.md §0.4 et memory
        // project_aphrody_owned_tools : on préfère toujours l'outil maison.
        let bin_path = gemini_runtime::resolve_bin().map_err(|e| {
            miette::miette!(
                "Failed to resolve `gemini` binary via aphrody chain: {e}.\n\n\
                 Alternatives :\n\
                 • `aphrody chat --stub --prompt \"…\"` — réponse stub locale, aucun binaire requis.\n\
                 • `aphrody chat --model anthropic/claude-opus-4-7 --prompt \"…\"` \
                   (requiert `ANTHROPIC_API_KEY`).\n\
                 • Build fork in-tree : `cd packages/gemini-cli && bun run bundle` puis re-run.\n\
                 • Installer le CLI Google : `bun add -g @google/gemini-cli`.\n\
                 • Override binaire : `APHRODY_GEMINI_BIN=/abs/path aphrody gemini …`."
            )
        })?;

        let mut cmd = std::process::Command::new(&bin_path);
        cmd.args(&self.args);
        // Gemini CLI refuse de tourner en headless si le cwd n'est pas un
        // "trusted directory". Pour `aphrody a2a` / `aphrody gemini` invoqués
        // par l'autopilot ou tout autre scénario non-interactif, on opt-in au
        // trust par défaut. L'opérateur peut override en exportant
        // GEMINI_CLI_TRUST_WORKSPACE=false avant l'appel.
        if std::env::var_os("GEMINI_CLI_TRUST_WORKSPACE").is_none() {
            cmd.env("GEMINI_CLI_TRUST_WORKSPACE", "true");
        }

        let status = cmd.status().map_err(|e| {
            miette::miette!("Failed to spawn `{}`: {e}", bin_path.display())
        })?;

        if !status.success() {
            return Err(miette::Report::new(SubprocessExit(status.code().unwrap_or(1))));
        }
        Ok(())
    }
}

pub(crate) struct SearchCommand {
    pub query: Vec<String>,
}

#[async_trait]
impl TerminalCommand for SearchCommand {
    async fn execute(&self, _ctx: &GoogleContext) -> miette::Result<()> {
        let q = self.query.join(" ");
        println!("{}", format!("🔍 Recherche : {}", q).bold().blue());

        let client = reqwest::Client::builder()
            .user_agent(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) \
                 Chrome/124.0.0.0 Safari/537.36",
            )
            .build()
            .map_err(|e| miette::miette!("Erreur client HTTP: {}", e))?;

        let res = client
            .get("https://html.duckduckgo.com/html/")
            .query(&[("q", q.as_str())])
            .send()
            .await
            .map_err(|e| miette::miette!("Erreur réseau: {}", e))?;

        let text = res.text().await.map_err(|e| miette::miette!("Erreur lecture: {}", e))?;

        let document = scraper::Html::parse_document(&text);
        let result_selector =
            scraper::Selector::parse("div.result.results_links").expect("static selector");
        let title_selector = scraper::Selector::parse("a.result__a").expect("static selector");
        let snippet_selector =
            scraper::Selector::parse("a.result__snippet").expect("static selector");

        let mut count = 0;
        for result in document.select(&result_selector) {
            let Some(title_elem) = result.select(&title_selector).next() else { continue };
            let title = title_elem.text().collect::<String>().trim().to_string();
            if title.is_empty() {
                continue;
            }
            let raw_href = title_elem.value().attr("href").unwrap_or("");
            let link = unwrap_ddg_redirect(raw_href).unwrap_or_else(|| raw_href.to_string());
            let snippet = result
                .select(&snippet_selector)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            println!("\n{}", title.bold().green());
            println!("{}", link.cyan().underline());
            if !snippet.is_empty() {
                println!("{}", snippet);
            }
            count += 1;
            if count >= 5 {
                break;
            }
        }

        if count == 0 {
            println!(
                "{}",
                "Aucun résultat trouvé (DuckDuckGo a renvoyé une page vide — réessayer plus tard)."
                    .red()
            );
        }

        Ok(())
    }
}

/// Decode the `uddg` query parameter from a DuckDuckGo redirect URL.
/// DDG wraps result links as `//duckduckgo.com/l/?uddg=<percent-encoded>&rut=…`;
/// we want the raw target URL.
fn unwrap_ddg_redirect(href: &str) -> Option<String> {
    let normalised = if let Some(rest) = href.strip_prefix("//") {
        format!("https://{rest}")
    } else {
        href.to_string()
    };
    let parsed = reqwest::Url::parse(&normalised).ok()?;
    if parsed.host_str()? != "duckduckgo.com" {
        return None;
    }
    parsed.query_pairs().find(|(k, _)| k == "uddg").map(|(_, v)| v.into_owned())
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
            return Err(miette::Report::new(SubprocessExit(1)));
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

// ── aphrody notify ───────────────────────────────────────────────────────────
//
// Thin facade over `aphrody-channels` (Slack / Telegram / Matrix). Reads
// credentials + destination from environment variables; returns a clean
// `miette::Report` (exit code 1) when any required variable is missing rather
// than panicking.

pub(crate) struct NotifyCommand {
    pub channel: crate::NotifyChannel,
    pub message: String,
    pub room: Option<String>,
}

#[async_trait]
impl TerminalCommand for NotifyCommand {
    async fn execute(&self, _ctx: &GoogleContext) -> miette::Result<()> {
        use aphrody_messaging::channels::{
            MessagingChannel, matrix::MatrixChannel, slack::SlackChannel, telegram::TelegramChannel,
        };

        use crate::NotifyChannel;

        // Resolve destination room: explicit --room overrides the env-var
        // fallback. The env-var name is channel-specific so error messages
        // tell the operator exactly which variable to set.
        let (room_env_var, ctor_label) = match self.channel {
            NotifyChannel::Slack => ("SLACK_CHANNEL", "slack"),
            NotifyChannel::Telegram => ("TELEGRAM_CHAT_ID", "telegram"),
            NotifyChannel::Matrix => ("MATRIX_ROOM_ID", "matrix"),
        };

        let room = match self.room.clone() {
            Some(r) if !r.is_empty() => r,
            _ => std::env::var(room_env_var).ok().filter(|s| !s.is_empty()).ok_or_else(|| {
                miette::miette!(
                    "missing destination room: pass `--room <id>` or set `{room_env_var}` \
                     environment variable for `aphrody notify --channel {ctor_label}`"
                )
            })?,
        };

        // Dispatch to the matching adapter. Each `from_env()` returns
        // `ChannelError::MissingEnvVar` on missing credentials — we surface
        // that as a clean miette error rather than panicking.
        let result = match self.channel {
            NotifyChannel::Slack => {
                let ch = SlackChannel::from_env()
                    .map_err(|e| miette::miette!("slack credentials error: {e}"))?;
                ch.send_text(&room, &self.message).await
            },
            NotifyChannel::Telegram => {
                let ch = TelegramChannel::from_env()
                    .map_err(|e| miette::miette!("telegram credentials error: {e}"))?;
                ch.send_text(&room, &self.message).await
            },
            NotifyChannel::Matrix => {
                let ch = MatrixChannel::from_env()
                    .map_err(|e| miette::miette!("matrix credentials error: {e}"))?;
                ch.send_text(&room, &self.message).await
            },
        };

        let msg_ref = result.map_err(|e| miette::miette!("send failed: {e}"))?;

        // Stable, machine-friendly success line — first column is the channel,
        // second is the platform-native message ID, third is the room.
        println!("{} {} {}", msg_ref.channel_id, msg_ref.id, msg_ref.room);
        Ok(())
    }
}

// ===========================================================================
// `aphrody chat` — unified turn-loop REPL (one-shot mode in Phase 1).
// ===========================================================================

/// Implementation of `aphrody chat`.
///
/// Composes the building blocks via [`aphrody_chat::ChatLoop`]. In Phase 1
/// we only support `--prompt <text>` one-shot mode; invocation without a
/// prompt returns a structured error instead of opening an interactive REPL
/// (ratatui + slash-commands land in Phase 2).
pub(crate) struct ChatCommand {
    pub prompt: Option<String>,
    pub model: Option<String>,
    pub system: Option<String>,
    pub stub: bool,
}

#[async_trait]
impl TerminalCommand for ChatCommand {
    async fn execute(&self, _ctx: &GoogleContext) -> miette::Result<()> {
        use aphrody_chat::backend::{GeminiBackend, ModelBackend, StubBackend};
        use aphrody_chat::{ChatConfig, ChatLoop};

        let Some(prompt) = self.prompt.clone() else {
            return Err(miette::miette!(
                "interactive REPL not yet wired — Phase 1 requires `aphrody chat \
                 --prompt <text>` for one-shot mode (ratatui REPL + slash-commands \
                 land in Phase 2)"
            ));
        };

        let mut config = ChatConfig::default();
        if let Some(m) = self.model.clone() {
            config.model = m;
        }
        if let Some(sys) = self.system.clone() {
            config.system_prompt = sys;
        }

        // Backend selection: --stub → always StubBackend; otherwise try
        // GeminiBackend first and degrade gracefully to StubBackend with a
        // tracing diagnostic when the `gemini` binary is missing from PATH.
        let backend: Box<dyn ModelBackend> = if self.stub {
            Box::new(StubBackend::with_reply(
                "(stub backend reply — `aphrody chat --stub` mode, no live LLM call)",
            ))
        } else {
            match GeminiBackend::detect().await {
                Ok(gemini) => Box::new(gemini.with_model(Some(config.model.clone()))),
                Err(e) => {
                    eprintln!(
                        "[chat] gemini-runtime detect failed ({e}); falling back to \
                         stub backend. Re-run with `aphrody chat --stub` to \
                         silence this warning, or install `gemini` on PATH."
                    );
                    Box::new(StubBackend::with_reply(
                        "(stub backend reply — gemini binary not found on PATH)",
                    ))
                },
            }
        };

        let mut chat = ChatLoop::builder(config)
            .with_boxed_backend(backend)
            .build()
            .map_err(|e| miette::miette!("ChatLoop build failed: {e}"))?;

        let turn = chat
            .single_turn(&prompt)
            .await
            .map_err(|e| miette::miette!("chat turn failed: {e}"))?;

        // Print the assistant reply on stdout (machine-friendly: a single
        // contiguous blob, no decoration).
        println!("{}", turn.assistant_response);
        Ok(())
    }
}

// ── aphrody notebooklm ──────────────────────────────────────────────────────

/// Façade dispatching every `aphrody notebooklm <sub>` invocation onto the
/// in-tree `notebooklm` crate.  Honours `NOTEBOOKLM_OAUTH_TOKEN` /
/// `NOTEBOOKLM_COOKIES` for credentials and `NOTEBOOKLM_AT_TOKEN` /
/// `NOTEBOOKLM_BL_TOKEN` / `NOTEBOOKLM_FSID_TOKEN` for the per-session
/// XSRF + bootstrap tokens (the upstream web UI exports these on page load;
/// they must be ferried into the CLI environment by the caller).
pub(crate) struct NotebooklmCommand {
    pub action: crate::NotebooklmActions,
}

#[async_trait]
impl TerminalCommand for NotebooklmCommand {
    async fn execute(&self, _ctx: &GoogleContext) -> miette::Result<()> {
        use crate::NotebooklmActions;
        match &self.action {
            NotebooklmActions::List => notebooklm_list().await,
            NotebooklmActions::Create { title } => notebooklm_create(title.as_deref()).await,
            NotebooklmActions::Delete { notebook_id } => notebooklm_delete(notebook_id).await,
            NotebooklmActions::Upload { notebook_id, source } => {
                notebooklm_upload(notebook_id, source).await
            },
            NotebooklmActions::Chat { notebook_id, prompt } => {
                notebooklm_chat(notebook_id, prompt).await
            },
            NotebooklmActions::Generate { notebook_id, kind, prompt } => {
                notebooklm_generate(notebook_id, kind, prompt.as_deref()).await
            },
            NotebooklmActions::Download { notebook_id, artifact_id, output } => {
                notebooklm_download(notebook_id, artifact_id, output).await
            },
        }
    }
}

fn notebooklm_session_tokens() -> miette::Result<::notebooklm::SessionTokens> {
    let at = std::env::var("NOTEBOOKLM_AT_TOKEN").map_err(|_| {
        miette::miette!(
            "NOTEBOOKLM_AT_TOKEN must be set (extract `WIZ_global_data.SNlM0e` from \
             a logged-in notebooklm.google.com page)"
        )
    })?;
    let bl = std::env::var("NOTEBOOKLM_BL_TOKEN").map_err(|_| {
        miette::miette!(
            "NOTEBOOKLM_BL_TOKEN must be set (extract the `cfb2h` / `bl` field \
             from the same bootstrap blob)"
        )
    })?;
    let fsid = std::env::var("NOTEBOOKLM_FSID_TOKEN").ok().filter(|s| !s.is_empty());
    let language = std::env::var("NOTEBOOKLM_HL").ok().filter(|s| !s.is_empty());
    Ok(::notebooklm::SessionTokens { at, bl, fsid, language })
}

fn notebooklm_client() -> miette::Result<::notebooklm::NotebookClient> {
    let tokens = notebooklm_session_tokens()?;
    ::notebooklm::NotebookClient::from_env(tokens)
        .map_err(|e| miette::miette!("notebooklm auth setup failed: {e}"))
}

async fn notebooklm_list() -> miette::Result<()> {
    let client = notebooklm_client()?;
    let notebooks = client
        .list_notebooks()
        .await
        .map_err(|e| miette::miette!("list_notebooks failed: {e}"))?;
    let json = serde_json::to_string_pretty(&notebooks)
        .map_err(|e| miette::miette!("json encode: {e}"))?;
    println!("{json}");
    Ok(())
}

async fn notebooklm_create(title: Option<&str>) -> miette::Result<()> {
    let client = notebooklm_client()?;
    let (notebook_id, thread_id) = client
        .create_notebook()
        .await
        .map_err(|e| miette::miette!("create_notebook failed: {e}"))?;
    if let Some(new_title) = title {
        client
            .rename_notebook(&notebook_id, new_title)
            .await
            .map_err(|e| miette::miette!("rename_notebook failed: {e}"))?;
    }
    let payload = serde_json::json!({
        "notebook_id": notebook_id,
        "thread_id": thread_id,
        "title": title.unwrap_or(""),
    });
    println!("{payload:#}");
    Ok(())
}

async fn notebooklm_delete(notebook_id: &str) -> miette::Result<()> {
    let client = notebooklm_client()?;
    client
        .delete_notebook(notebook_id)
        .await
        .map_err(|e| miette::miette!("delete_notebook failed: {e}"))?;
    println!(r#"{{"deleted":"{notebook_id}"}}"#);
    Ok(())
}

async fn notebooklm_upload(notebook_id: &str, source: &str) -> miette::Result<()> {
    let client = notebooklm_client()?;
    let result = if source.starts_with("http://") || source.starts_with("https://") {
        client.add_url(notebook_id, source).await
    } else {
        client.add_file(notebook_id, source).await
    };
    let (source_id, title) =
        result.map_err(|e| miette::miette!("add source failed: {e}"))?;
    let payload = serde_json::json!({
        "source_id": source_id,
        "title": title,
    });
    println!("{payload:#}");
    Ok(())
}

async fn notebooklm_chat(notebook_id: &str, prompt: &str) -> miette::Result<()> {
    let client = notebooklm_client()?;
    let threads = client
        .list_chat_threads(notebook_id)
        .await
        .map_err(|e| miette::miette!("list_chat_threads failed: {e}"))?;
    let thread = threads.first().map(String::as_str);
    let (_nb, sources) = client
        .get_notebook(notebook_id)
        .await
        .map_err(|e| miette::miette!("get_notebook failed: {e}"))?;
    let source_ids: Vec<String> = sources.into_iter().map(|s| s.id).collect();
    let reply = client
        .send_message(notebook_id, prompt, &source_ids, thread)
        .await
        .map_err(|e| miette::miette!("send_message failed: {e}"))?;
    println!("{}", reply.text);
    Ok(())
}

async fn notebooklm_generate(
    notebook_id: &str,
    kind: &str,
    prompt: Option<&str>,
) -> miette::Result<()> {
    use std::str::FromStr;
    let client = notebooklm_client()?;
    let kind = ::notebooklm::ArtifactKind::from_str(kind)
        .map_err(|e| miette::miette!("invalid artifact kind: {e}"))?;
    let (_nb, sources) = client
        .get_notebook(notebook_id)
        .await
        .map_err(|e| miette::miette!("get_notebook failed: {e}"))?;
    let source_ids: Vec<String> = sources.into_iter().map(|s| s.id).collect();
    let (artifact_id, title) = client
        .generate(notebook_id, &source_ids, kind, prompt)
        .await
        .map_err(|e| miette::miette!("generate failed: {e}"))?;
    let payload = serde_json::json!({
        "artifact_id": artifact_id,
        "title": title,
        "kind": format!("{kind:?}"),
    });
    println!("{payload:#}");
    Ok(())
}

async fn notebooklm_download(
    notebook_id: &str,
    artifact_id: &str,
    output: &std::path::Path,
) -> miette::Result<()> {
    let client = notebooklm_client()?;
    let artifact = client
        .poll(
            notebook_id,
            artifact_id,
            std::time::Duration::from_secs(600),
        )
        .await
        .map_err(|e| miette::miette!("artifact poll failed: {e}"))?;
    let url = artifact
        .download_url
        .ok_or_else(|| miette::miette!("artifact has no download_url after poll"))?;
    let bytes = reqwest::get(&url)
        .await
        .map_err(|e| miette::miette!("download GET {url}: {e}"))?
        .bytes()
        .await
        .map_err(|e| miette::miette!("download read {url}: {e}"))?;
    std::fs::write(output, &bytes)
        .map_err(|e| miette::miette!("write {}: {e}", output.display()))?;
    let payload = serde_json::json!({
        "artifact_id": artifact_id,
        "bytes": bytes.len(),
        "output": output.display().to_string(),
    });
    println!("{payload:#}");
    Ok(())
}
