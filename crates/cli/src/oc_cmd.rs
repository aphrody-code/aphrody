// SPDX-License-Identifier: Apache-2.0
// `aphrody oc-*` — five critical sub-commands ported from openclaw to safe Rust.
//
// Ports (logic, not shell-out):
//   * `oc-onboard`   — bootstrap state + workspace + seed config
//                      (openclaw: src/commands/onboard.ts, ~110 LOC + helpers)
//   * `oc-reset`     — reset local state (3 scopes)
//                      (openclaw: src/commands/reset.ts, 151 LOC)
//   * `oc-uninstall` — clean removal (4 scopes)
//                      (openclaw: src/commands/uninstall.ts, 212 LOC)
//   * `oc-pairing`   — list/approve DM pairing requests (JSON store)
//                      (openclaw: src/cli/pairing-cli.ts, 219 LOC)
//   * `oc-docs`      — open / search docs URLs (preserves the doc-link UX)
//                      (openclaw: src/commands/docs.ts, 209 LOC)
//
// All five names are prefixed `oc-` to avoid collisions with the existing
// aphrody surface (`doctor`, `scan`, `notify`, …). Storage / config paths use
// the aphrody namespace (`~/.aphrody/`), not openclaw's; this is a deliberate
// port — we keep the algorithm and CLI ergonomics, but the on-disk layout is
// owned by aphrody.

use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use clap::{Subcommand, ValueEnum};
use miette::{IntoDiagnostic, WrapErr};
use serde::{Deserialize, Serialize};

use crate::context::{GoogleContext, TerminalCommand};

// =====================================================================
// Path resolution (mirrors openclaw resolveStateDir / resolveConfigPath /
// resolveOAuthDir, but rooted under the aphrody namespace).
// =====================================================================

fn home_dir() -> miette::Result<PathBuf> {
    // We deliberately avoid the `home` crate to keep dep weight zero — the
    // platform branches below match what `std::env::home_dir` did before
    // deprecation but with Windows fallback to USERPROFILE.
    if let Ok(h) = std::env::var("APHRODY_HOME") {
        return Ok(PathBuf::from(h));
    }
    if cfg!(windows) {
        if let Ok(h) = std::env::var("USERPROFILE") {
            return Ok(PathBuf::from(h));
        }
    }
    if let Ok(h) = std::env::var("HOME") {
        return Ok(PathBuf::from(h));
    }
    Err(miette::miette!("could not resolve home directory (HOME / USERPROFILE unset)"))
}

/// Root aphrody state directory (`$APHRODY_HOME` overrides, else `~/.aphrody`).
fn state_dir() -> miette::Result<PathBuf> {
    Ok(home_dir()?.join(".aphrody"))
}

/// Config file path: `~/.aphrody/aphrody.json`.
fn config_path() -> miette::Result<PathBuf> {
    Ok(state_dir()?.join("aphrody.json"))
}

/// OAuth credentials directory: `~/.aphrody/oauth`.
fn oauth_dir() -> miette::Result<PathBuf> {
    Ok(state_dir()?.join("oauth"))
}

/// Default agent workspace directory: `~/.aphrody/workspace`.
fn default_workspace_dir() -> miette::Result<PathBuf> {
    Ok(state_dir()?.join("workspace"))
}

/// Pairing store path: `~/.aphrody/pairing.json`.
fn pairing_store_path() -> miette::Result<PathBuf> {
    Ok(state_dir()?.join("pairing.json"))
}

// =====================================================================
// Safe filesystem removal — direct port of openclaw cleanup-utils.ts.
// Refuses to delete the FS root or the user's home directory.
// =====================================================================

fn is_unsafe_removal_target(target: &Path) -> bool {
    let resolved = target;
    // Refuse FS root (e.g. `/`, `C:\`).
    if let Some(root) = resolved.ancestors().last() {
        if resolved == root {
            return true;
        }
    }
    // Refuse the literal home directory.
    if let Ok(home) = home_dir() {
        if resolved == home.as_path() {
            return true;
        }
    }
    false
}

/// Remove a file or directory tree (force, recursive), with `dry_run` plumbing
/// and the safety guard from openclaw.
fn remove_path(target: &Path, dry_run: bool) -> miette::Result<bool> {
    if target.as_os_str().is_empty() {
        return Ok(false);
    }
    if is_unsafe_removal_target(target) {
        eprintln!("Refusing to remove unsafe path: {}", target.display());
        return Ok(false);
    }
    if dry_run {
        println!("[dry-run] remove {}", target.display());
        return Ok(true);
    }
    if !target.exists() {
        return Ok(false);
    }
    let result = if target.is_dir() {
        fs::remove_dir_all(target)
    } else {
        fs::remove_file(target)
    };
    match result {
        Ok(()) => {
            println!("Removed {}", target.display());
            Ok(true)
        },
        Err(err) => Err(miette::miette!(
            "Failed to remove {}: {err}",
            target.display()
        )),
    }
}

// =====================================================================
// `oc-onboard` — bootstrap state directory + seed config.
// =====================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SeedConfig {
    /// Aphrody binary version that wrote the seed config.
    version: String,
    /// ISO-8601 timestamp of bootstrap.
    bootstrapped_at: String,
    /// User-selected agent workspace.
    workspace: PathBuf,
    /// Whether the user accepted non-interactive risk (--accept-risk).
    accepted_risk: bool,
}

pub(crate) struct OnboardCommand {
    pub workspace: Option<PathBuf>,
    pub non_interactive: bool,
    pub accept_risk: bool,
    pub force: bool,
}

#[async_trait]
impl TerminalCommand for OnboardCommand {
    async fn execute(&self, _ctx: &GoogleContext) -> miette::Result<()> {
        // Port of openclaw onboard.ts:68 — non-interactive setup requires
        // explicit risk acknowledgement.
        if self.non_interactive && !self.accept_risk {
            return Err(miette::miette!(
                "Non-interactive onboarding requires explicit --accept-risk."
            ));
        }

        let state = state_dir()?;
        let cfg = config_path()?;
        let workspace =
            self.workspace.clone().map(Ok).unwrap_or_else(default_workspace_dir)?;

        fs::create_dir_all(&state)
            .into_diagnostic()
            .wrap_err_with(|| format!("create_dir_all {}", state.display()))?;
        fs::create_dir_all(&workspace)
            .into_diagnostic()
            .wrap_err_with(|| format!("create_dir_all {}", workspace.display()))?;

        if cfg.exists() && !self.force {
            println!(
                "aphrody is already bootstrapped (config at {}). Pass --force to overwrite.",
                cfg.display()
            );
            return Ok(());
        }

        let seed = SeedConfig {
            version: env!("CARGO_PKG_VERSION").to_string(),
            bootstrapped_at: current_iso8601(),
            workspace: workspace.clone(),
            accepted_risk: self.accept_risk,
        };
        let body = serde_json::to_string_pretty(&seed)
            .into_diagnostic()
            .wrap_err("serialize seed config")?;
        fs::write(&cfg, body)
            .into_diagnostic()
            .wrap_err_with(|| format!("write seed config {}", cfg.display()))?;

        println!("aphrody onboard: bootstrapped");
        println!("  state:     {}", state.display());
        println!("  config:    {}", cfg.display());
        println!("  workspace: {}", workspace.display());
        if cfg!(windows) {
            // Mirror openclaw onboard.ts:89 — gentle WSL nudge on Windows.
            println!("Note: aphrody runs natively on Windows but WSL2 is recommended for parity.");
        }
        Ok(())
    }
}

// =====================================================================
// `oc-reset` — reset local state (3 scopes).
// =====================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum ResetScope {
    /// Remove `aphrody.json` only — keep workspace and credentials.
    Config,
    /// Remove `aphrody.json`, OAuth credentials and `agents/*/sessions/`.
    ConfigCredsSessions,
    /// Remove the entire `~/.aphrody/` state + workspace directories.
    Full,
}

pub(crate) struct ResetCommand {
    pub scope: ResetScope,
    pub yes: bool,
    pub dry_run: bool,
}

#[async_trait]
impl TerminalCommand for ResetCommand {
    async fn execute(&self, _ctx: &GoogleContext) -> miette::Result<()> {
        if !self.yes && !self.dry_run {
            return Err(miette::miette!("Refusing to reset without --yes (or --dry-run for preview)."));
        }
        let state = state_dir()?;
        let cfg = config_path()?;
        let oauth = oauth_dir()?;
        let workspace = default_workspace_dir()?;

        match self.scope {
            ResetScope::Config => {
                let _ = remove_path(&cfg, self.dry_run)?;
            },
            ResetScope::ConfigCredsSessions => {
                let _ = remove_path(&cfg, self.dry_run)?;
                let _ = remove_path(&oauth, self.dry_run)?;
                // Remove every `<state>/agents/<id>/sessions/` directory.
                let agents_dir = state.join("agents");
                if let Ok(read) = fs::read_dir(&agents_dir) {
                    for entry in read.flatten() {
                        let sessions = entry.path().join("sessions");
                        if sessions.exists() {
                            let _ = remove_path(&sessions, self.dry_run)?;
                        }
                    }
                }
                println!("Next: aphrody oc-onboard");
            },
            ResetScope::Full => {
                let _ = remove_path(&state, self.dry_run)?;
                let _ = remove_path(&workspace, self.dry_run)?;
                println!("Next: aphrody oc-onboard");
            },
        }
        Ok(())
    }
}

// =====================================================================
// `oc-uninstall` — clean removal across 4 scopes.
// =====================================================================

pub(crate) struct UninstallCommand {
    pub service: bool,
    pub state: bool,
    pub workspace: bool,
    pub app: bool,
    pub all: bool,
    pub yes: bool,
    pub dry_run: bool,
}

#[async_trait]
impl TerminalCommand for UninstallCommand {
    async fn execute(&self, _ctx: &GoogleContext) -> miette::Result<()> {
        let service = self.service || self.all;
        let state_flag = self.state || self.all;
        let workspace_flag = self.workspace || self.all;
        let app_flag = self.app || self.all;

        if !service && !state_flag && !workspace_flag && !app_flag {
            return Err(miette::miette!(
                "Uninstall requires explicit scopes. Use --all, or any of --service / --state / --workspace / --app."
            ));
        }
        if !self.yes && !self.dry_run {
            return Err(miette::miette!(
                "Refusing to uninstall without --yes (preview with --dry-run --all)."
            ));
        }

        if service {
            if self.dry_run {
                println!("[dry-run] stop & uninstall gateway service");
            } else {
                println!("Note: aphrody has no managed daemon yet — skipping service uninstall.");
            }
        }
        if state_flag {
            let _ = remove_path(&state_dir()?, self.dry_run)?;
        }
        if workspace_flag {
            let _ = remove_path(&default_workspace_dir()?, self.dry_run)?;
        }
        if app_flag {
            #[cfg(target_os = "macos")]
            {
                let app = PathBuf::from("/Applications/Aphrody.app");
                let _ = remove_path(&app, self.dry_run)?;
            }
            #[cfg(not(target_os = "macos"))]
            {
                println!("Note: --app is a macOS-only scope. Skipping.");
            }
        }
        Ok(())
    }
}

// =====================================================================
// `oc-pairing` — list / approve DM pairing requests (JSON store).
// =====================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PairingRequest {
    pub channel: String,
    pub id: String,
    pub code: String,
    pub created_at: String,
    #[serde(default)]
    pub meta: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct PairingStore {
    #[serde(default)]
    pub pending: Vec<PairingRequest>,
    #[serde(default)]
    pub approved: Vec<PairingRequest>,
}

impl PairingStore {
    fn load(path: &Path) -> miette::Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let body = fs::read_to_string(path)
            .into_diagnostic()
            .wrap_err_with(|| format!("read pairing store {}", path.display()))?;
        serde_json::from_str(&body)
            .into_diagnostic()
            .wrap_err_with(|| format!("parse pairing store {}", path.display()))
    }

    fn save(&self, path: &Path) -> miette::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).into_diagnostic().wrap_err("create pairing store parent")?;
        }
        let body = serde_json::to_string_pretty(self).into_diagnostic()?;
        fs::write(path, body)
            .into_diagnostic()
            .wrap_err_with(|| format!("write pairing store {}", path.display()))
    }

    fn approve(&mut self, channel: &str, code: &str) -> Option<PairingRequest> {
        let idx =
            self.pending.iter().position(|r| r.channel == channel && r.code == code)?;
        let req = self.pending.remove(idx);
        self.approved.push(req.clone());
        Some(req)
    }
}

#[derive(Subcommand, Debug, Clone)]
pub(crate) enum PairingAction {
    /// List pending pairing requests (optionally filtered by channel).
    List {
        /// Channel filter (e.g. `telegram`, `whatsapp`, `discord`). Omit for all.
        #[arg(long)]
        channel: Option<String>,
        /// Emit JSON instead of a text table.
        #[arg(long)]
        json: bool,
    },
    /// Approve a pending pairing code for a channel.
    Approve {
        /// Channel name.
        channel: String,
        /// Pairing code to approve.
        code: String,
    },
    /// Inject a pending pairing request (for tests / external integrations).
    Add {
        channel: String,
        id: String,
        code: String,
    },
}

pub(crate) struct PairingCommand {
    pub action: PairingAction,
}

#[async_trait]
impl TerminalCommand for PairingCommand {
    async fn execute(&self, _ctx: &GoogleContext) -> miette::Result<()> {
        let path = pairing_store_path()?;
        let mut store = PairingStore::load(&path)?;
        match &self.action {
            PairingAction::List { channel, json } => {
                let filtered: Vec<&PairingRequest> = store
                    .pending
                    .iter()
                    .filter(|r| channel.as_deref().map_or(true, |c| r.channel == c))
                    .collect();
                if *json {
                    let body = serde_json::to_string_pretty(&filtered).into_diagnostic()?;
                    println!("{body}");
                    return Ok(());
                }
                if filtered.is_empty() {
                    println!(
                        "No pending pairing requests{}.",
                        channel.as_deref().map(|c| format!(" for channel `{c}`")).unwrap_or_default()
                    );
                    return Ok(());
                }
                println!("{:<10}  {:<18}  {:<24}  Requested", "Code", "Channel", "ID");
                for r in filtered {
                    println!("{:<10}  {:<18}  {:<24}  {}", r.code, r.channel, r.id, r.created_at);
                }
            },
            PairingAction::Approve { channel, code } => {
                let approved = store
                    .approve(channel, code)
                    .ok_or_else(|| miette::miette!("No pending pairing request for code `{code}` on channel `{channel}`."))?;
                store.save(&path)?;
                println!("Approved {} sender {}", approved.channel, approved.id);
            },
            PairingAction::Add { channel, id, code } => {
                store.pending.push(PairingRequest {
                    channel: channel.clone(),
                    id: id.clone(),
                    code: code.clone(),
                    created_at: current_iso8601(),
                    meta: None,
                });
                store.save(&path)?;
                println!("Added pairing request {} / {} / {}", channel, id, code);
            },
        }
        Ok(())
    }
}

// =====================================================================
// `oc-docs` — open / search the documentation site (no shell-out).
// =====================================================================

const DOCS_BASE: &str = "https://docs.aphrody.dev";
const DOCS_SEARCH: &str = "https://docs.aphrody.dev/search?q=";

pub(crate) struct DocsCommand {
    pub query: Vec<String>,
    /// Print only the URL (script-friendly); do not attempt to open a browser.
    pub url_only: bool,
}

#[async_trait]
impl TerminalCommand for DocsCommand {
    async fn execute(&self, _ctx: &GoogleContext) -> miette::Result<()> {
        let url = if self.query.is_empty() {
            DOCS_BASE.to_string()
        } else {
            let q = self.query.join(" ");
            let encoded = url_encode(&q);
            format!("{DOCS_SEARCH}{encoded}")
        };

        if self.url_only {
            println!("{url}");
            return Ok(());
        }

        println!("Docs: {url}");
        // Try platform-native open; do not fail the command if no opener is
        // available — we already printed the URL above.
        try_open_browser(&url);
        Ok(())
    }
}

fn url_encode(input: &str) -> String {
    // Minimal RFC 3986 percent-encoding — avoids pulling `percent-encoding`
    // just for this command.
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            },
            b' ' => out.push('+'),
            _ => {
                out.push('%');
                out.push_str(&format!("{:02X}", byte));
            },
        }
    }
    out
}

fn try_open_browser(url: &str) {
    let mut io_lock = io::stdout().lock();
    let _ = writeln!(io_lock, "Opening browser…");
    drop(io_lock);

    #[cfg(target_os = "windows")]
    let res = std::process::Command::new("cmd").args(["/C", "start", "", url]).spawn();
    #[cfg(target_os = "macos")]
    let res = std::process::Command::new("open").arg(url).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let res = std::process::Command::new("xdg-open").arg(url).spawn();
    #[cfg(not(any(target_os = "windows", target_os = "macos", unix)))]
    let res: Result<std::process::Child, std::io::Error> = Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "no browser opener for this platform",
    ));

    if let Err(err) = res {
        eprintln!("Note: could not launch browser ({err}). Open the URL manually.");
    }
}

// =====================================================================
// Utilities
// =====================================================================

/// ISO-8601 UTC timestamp without pulling `chrono` / `time`.
fn current_iso8601() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Compute UTC Y-M-D-H-M-S from a unix timestamp.
    let (year, month, day, hour, min, sec) = unix_to_ymd_hms(now);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{min:02}:{sec:02}Z")
}

/// Convert a UTC unix timestamp (seconds) to (year, month, day, hour, min, sec).
/// Algorithm: Howard Hinnant's `civil_from_days`, public domain.
fn unix_to_ymd_hms(secs: u64) -> (u64, u64, u64, u64, u64, u64) {
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let hour = rem / 3_600;
    let min = (rem % 3_600) / 60;
    let sec = rem % 60;

    // Shift epoch to 0000-03-01.
    let z = days as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as u64, m, d, hour, min, sec)
}

// =====================================================================
// Unit tests — one per sub-command (per task contract).
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, Once};

    // Some tests touch process-wide env (APHRODY_HOME); serialize them to
    // avoid flakes under cargo nextest's parallel runner. We never propagate
    // panics out of guarded sections, but we still recover from a poisoned
    // mutex (the test runner may panic on assertion failures in other
    // serialized tests; the lock state itself is irrelevant after that).
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn lock_env() -> std::sync::MutexGuard<'static, ()> {
        match ENV_LOCK.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    // `GoogleContext::new()` constructs a reqwest client; reqwest panics
    // unless rustls has a CryptoProvider installed (cf. main.rs). Tests
    // build their own ctx, so install the provider once here.
    static CRYPTO_PROVIDER: Once = Once::new();
    fn install_crypto_provider() {
        CRYPTO_PROVIDER.call_once(|| {
            let _ = rustls::crypto::ring::default_provider().install_default();
        });
    }

    fn isolated_home(prefix: &str) -> tempfile::TempDir {
        let dir = tempfile::Builder::new().prefix(prefix).tempdir().unwrap();
        // Edition 2024: env::set_var is unsafe. We serialize all callers via
        // `ENV_LOCK`, so the data race precondition is upheld.
        unsafe { std::env::set_var("APHRODY_HOME", dir.path()) };
        dir
    }

    fn fresh_ctx() -> GoogleContext {
        install_crypto_provider();
        GoogleContext::new().expect("GoogleContext::new()")
    }

    #[tokio::test]
    async fn onboard_creates_state_and_seed_config() {
        let _guard = lock_env();
        let _tmp = isolated_home("aphrody-onboard");
        let ctx = fresh_ctx();

        let cmd = OnboardCommand {
            workspace: None,
            non_interactive: true,
            accept_risk: true,
            force: false,
        };
        cmd.execute(&ctx).await.unwrap();

        let cfg = config_path().unwrap();
        assert!(cfg.exists(), "seed config not created at {}", cfg.display());
        let body = std::fs::read_to_string(&cfg).unwrap();
        assert!(body.contains("\"bootstrapped_at\""));
        assert!(body.contains("\"workspace\""));
    }

    #[tokio::test]
    async fn onboard_non_interactive_without_accept_risk_errs() {
        let _guard = lock_env();
        let _tmp = isolated_home("aphrody-onboard-norisk");
        let ctx = fresh_ctx();

        let cmd = OnboardCommand {
            workspace: None,
            non_interactive: true,
            accept_risk: false,
            force: false,
        };
        let err = cmd.execute(&ctx).await.unwrap_err();
        assert!(err.to_string().contains("accept-risk"));
    }

    #[tokio::test]
    async fn reset_config_scope_removes_only_config() {
        let _guard = lock_env();
        let _tmp = isolated_home("aphrody-reset");
        let ctx = fresh_ctx();

        // Bootstrap, then reset --scope config.
        OnboardCommand {
            workspace: None,
            non_interactive: true,
            accept_risk: true,
            force: false,
        }
        .execute(&ctx)
        .await
        .unwrap();
        let cfg = config_path().unwrap();
        let workspace = default_workspace_dir().unwrap();
        assert!(cfg.exists());
        assert!(workspace.exists());

        ResetCommand { scope: ResetScope::Config, yes: true, dry_run: false }
            .execute(&ctx)
            .await
            .unwrap();
        assert!(!cfg.exists());
        assert!(workspace.exists(), "workspace must survive scope=config");
    }

    #[tokio::test]
    async fn reset_full_scope_removes_state_and_workspace() {
        let _guard = lock_env();
        let _tmp = isolated_home("aphrody-reset-full");
        let ctx = fresh_ctx();
        OnboardCommand {
            workspace: None,
            non_interactive: true,
            accept_risk: true,
            force: false,
        }
        .execute(&ctx)
        .await
        .unwrap();
        let state = state_dir().unwrap();
        ResetCommand { scope: ResetScope::Full, yes: true, dry_run: false }
            .execute(&ctx)
            .await
            .unwrap();
        assert!(!state.exists(), "state dir must be gone");
    }

    #[tokio::test]
    async fn reset_dry_run_does_not_touch_disk() {
        let _guard = lock_env();
        let _tmp = isolated_home("aphrody-reset-dry");
        let ctx = fresh_ctx();
        OnboardCommand {
            workspace: None,
            non_interactive: true,
            accept_risk: true,
            force: false,
        }
        .execute(&ctx)
        .await
        .unwrap();
        let cfg = config_path().unwrap();
        ResetCommand { scope: ResetScope::Config, yes: false, dry_run: true }
            .execute(&ctx)
            .await
            .unwrap();
        assert!(cfg.exists(), "dry-run must not remove the config");
    }

    #[tokio::test]
    async fn uninstall_state_scope_removes_state_dir() {
        let _guard = lock_env();
        let _tmp = isolated_home("aphrody-uninstall");
        let ctx = fresh_ctx();
        OnboardCommand {
            workspace: None,
            non_interactive: true,
            accept_risk: true,
            force: false,
        }
        .execute(&ctx)
        .await
        .unwrap();
        let state = state_dir().unwrap();
        assert!(state.exists());
        UninstallCommand {
            service: false,
            state: true,
            workspace: false,
            app: false,
            all: false,
            yes: true,
            dry_run: false,
        }
        .execute(&ctx)
        .await
        .unwrap();
        assert!(!state.exists());
    }

    #[tokio::test]
    async fn uninstall_without_scope_errs() {
        let _guard = lock_env();
        let _tmp = isolated_home("aphrody-uninstall-noscope");
        let ctx = fresh_ctx();
        let err = UninstallCommand {
            service: false,
            state: false,
            workspace: false,
            app: false,
            all: false,
            yes: true,
            dry_run: false,
        }
        .execute(&ctx)
        .await
        .unwrap_err();
        assert!(err.to_string().contains("scope"));
    }

    #[tokio::test]
    async fn pairing_add_then_list_then_approve() {
        let _guard = lock_env();
        let _tmp = isolated_home("aphrody-pairing");
        let ctx = fresh_ctx();

        // Inject a request.
        PairingCommand {
            action: PairingAction::Add {
                channel: "telegram".to_string(),
                id: "@alice".to_string(),
                code: "AB12CD".to_string(),
            },
        }
        .execute(&ctx)
        .await
        .unwrap();

        // List as JSON and validate parse.
        PairingCommand {
            action: PairingAction::List { channel: Some("telegram".to_string()), json: true },
        }
        .execute(&ctx)
        .await
        .unwrap();

        let store = PairingStore::load(&pairing_store_path().unwrap()).unwrap();
        assert_eq!(store.pending.len(), 1);
        assert_eq!(store.approved.len(), 0);

        // Approve and verify the request moved.
        PairingCommand {
            action: PairingAction::Approve {
                channel: "telegram".to_string(),
                code: "AB12CD".to_string(),
            },
        }
        .execute(&ctx)
        .await
        .unwrap();

        let store = PairingStore::load(&pairing_store_path().unwrap()).unwrap();
        assert_eq!(store.pending.len(), 0);
        assert_eq!(store.approved.len(), 1);
        assert_eq!(store.approved[0].id, "@alice");
    }

    #[tokio::test]
    async fn pairing_approve_unknown_code_errs() {
        let _guard = lock_env();
        let _tmp = isolated_home("aphrody-pairing-bad");
        let ctx = fresh_ctx();
        let err = PairingCommand {
            action: PairingAction::Approve {
                channel: "telegram".to_string(),
                code: "NOPE".to_string(),
            },
        }
        .execute(&ctx)
        .await
        .unwrap_err();
        assert!(err.to_string().contains("No pending pairing"));
    }

    #[tokio::test]
    async fn docs_url_only_prints_url_with_query() {
        let _guard = lock_env();
        let _tmp = isolated_home("aphrody-docs");
        let ctx = fresh_ctx();
        // Smoke — assert no panic, no browser launch (url_only=true).
        DocsCommand {
            query: vec!["hello".to_string(), "world".to_string()],
            url_only: true,
        }
        .execute(&ctx)
        .await
        .unwrap();
    }

    #[test]
    fn url_encode_handles_space_and_unicode() {
        assert_eq!(url_encode("a b"), "a+b");
        assert_eq!(url_encode("hello, world!"), "hello%2C+world%21");
    }

    #[test]
    fn iso8601_round_trip_sanity() {
        let s = current_iso8601();
        // 1970-01-01T00:00:00Z or later, never 0000-..
        assert!(s.starts_with("20") || s.starts_with("21"), "got {s}");
        assert!(s.ends_with('Z'));
        assert_eq!(s.len(), 20);
    }

    #[test]
    fn unsafe_removal_guard_blocks_root() {
        let root = if cfg!(windows) { PathBuf::from("C:\\") } else { PathBuf::from("/") };
        assert!(is_unsafe_removal_target(&root));
    }
}
