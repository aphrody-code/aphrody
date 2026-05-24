// SPDX-License-Identifier: Apache-2.0
//! aphrody desktop shell (Tauri v2) for the Angular 21 + Material 21 frontend.
//!
//! Every aphrody command runs IN-PROCESS via [`aphrody::run_captured`]
//! (Rust -> Rust, no subprocess, no FFI hop), mirroring `crates/aphrody-app`.
//! The captured stdout/stderr + exit code are returned straight to the webview,
//! so a GUI action is the exact same code path the terminal would run.
//!
//! On top of that the shell exposes a small, tightly **path-allowlisted** set of
//! filesystem commands so the control-center UI (soul / identity / agent config /
//! channels) is genuinely functional rather than cosmetic. None of these accept
//! an arbitrary path: every target is resolved here, under `~/.aphrody/`,
//! `~/.gemini/`, `~/.claude/` or `~/.config/aphrody/`, and JSON payloads are
//! validated before any write. Sibling binaries are likewise allowlisted by
//! exact name and resolved next to the running executable.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use base64::Engine as _;

/// [`aphrody::run_captured`] redirects the PROCESS-GLOBAL stdout/stderr handles
/// for the duration of a run (`aphrody-stdio-capture`). Tauri can dispatch
/// commands concurrently, so every capture is serialised behind this lock; two
/// simultaneous redirects would interleave each other's output.
static EXEC_LOCK: Mutex<()> = Mutex::new(());

/// Result of an in-process command run, shaped for the webview (stable IPC
/// surface, owned by the shell rather than re-exporting `aphrody::CapturedRun`).
#[derive(serde::Serialize)]
struct ExecResult {
    /// Process exit code (0 on success).
    code: i32,
    /// Captured standard output (lossy UTF-8).
    stdout: String,
    /// Captured standard error (lossy UTF-8).
    stderr: String,
}

impl From<aphrody::CapturedRun> for ExecResult {
    fn from(run: aphrody::CapturedRun) -> Self {
        Self { code: run.code, stdout: run.stdout, stderr: run.stderr }
    }
}

/// Run an aphrody command in-process and return its captured output.
///
/// `args` is the argument vector WITHOUT the program name (e.g.
/// `["chat", "--prompt", "bonjour"]`); the conventional `argv[0]` is prepended
/// here. The blocking capture runs on Tauri's blocking pool so the UI thread
/// never freezes, and is serialised by [`EXEC_LOCK`].
#[tauri::command]
async fn aphrody_exec(args: Vec<String>) -> Result<ExecResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = EXEC_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut argv = Vec::with_capacity(args.len() + 1);
        argv.push(String::from("aphrody"));
        argv.extend(args);
        ExecResult::from(aphrody::run_captured(argv))
    })
    .await
    .map_err(|err| format!("aphrody_exec failed to join the worker thread: {err}"))
}

/// Static shell + host metadata for the UI header / about / diagnostic views.
#[derive(serde::Serialize)]
struct Meta {
    /// The desktop shell's own version (this crate).
    app_version: &'static str,
    /// `std::env::consts::OS` (e.g. "windows", "linux", "macos").
    target_os: &'static str,
    /// `std::env::consts::ARCH` (e.g. "x86_64", "aarch64").
    target_arch: &'static str,
    /// `std::env::consts::FAMILY` (e.g. "windows", "unix").
    family: &'static str,
}

#[tauri::command]
fn aphrody_meta() -> Meta {
    Meta {
        app_version: env!("CARGO_PKG_VERSION"),
        target_os: std::env::consts::OS,
        target_arch: std::env::consts::ARCH,
        family: std::env::consts::FAMILY,
    }
}

// ---------------------------------------------------------------------------
// Path resolution — mirrors `crates/cli/src/oc_cmd.rs` + `aphrody-agent-home`.
// Every helper returns a path UNDER a known root; callers never see raw input.
// ---------------------------------------------------------------------------

/// Resolve the platform home dir, identical contract to the aphrody CLI:
/// `$APHRODY_HOME` overrides, else `%USERPROFILE%` on Windows, else `$HOME`.
fn home_dir() -> Result<PathBuf, String> {
    if let Ok(h) = std::env::var("APHRODY_HOME") {
        if !h.is_empty() {
            return Ok(PathBuf::from(h));
        }
    }
    #[cfg(windows)]
    if let Ok(h) = std::env::var("USERPROFILE") {
        if !h.is_empty() {
            return Ok(PathBuf::from(h));
        }
    }
    if let Ok(h) = std::env::var("HOME") {
        if !h.is_empty() {
            return Ok(PathBuf::from(h));
        }
    }
    Err("aucun répertoire personnel résolu (APHRODY_HOME / USERPROFILE / HOME)".into())
}

/// `~/.aphrody` — the aphrody state root.
fn state_dir() -> Result<PathBuf, String> {
    Ok(home_dir()?.join(".aphrody"))
}

/// Sanitise a profile / agent token into a filesystem-safe suffix (mirrors
/// `aphrody_agent_home::paths::sanitize_profile`). Guarantees the resolved
/// workspace can never escape `~/.aphrody/`.
fn sanitize_profile(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.trim().chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

/// Resolve the agent workspace dir exactly as `aphrody-agent-home` does:
/// `$APHRODY_WORKSPACE` verbatim, else `~/.aphrody/workspace-<profile>` when
/// `$APHRODY_PROFILE` is set, else `~/.aphrody/workspace`.
fn workspace_dir() -> Result<PathBuf, String> {
    if let Ok(ws) = std::env::var("APHRODY_WORKSPACE") {
        if !ws.is_empty() {
            return Ok(PathBuf::from(ws));
        }
    }
    let base = state_dir()?;
    if let Ok(profile) = std::env::var("APHRODY_PROFILE") {
        let p = sanitize_profile(&profile);
        if !p.is_empty() {
            return Ok(base.join(format!("workspace-{p}")));
        }
    }
    Ok(base.join("workspace"))
}

// ---------------------------------------------------------------------------
// Generic JSON helpers (validate-before-write, reused by every config command).
// ---------------------------------------------------------------------------

/// Read a UTF-8 text file, returning `None` (as an absent marker) when it does
/// not exist. Any other IO error is surfaced verbatim to the UI.
fn read_text_opt(path: &Path) -> Result<Option<String>, String> {
    match std::fs::read_to_string(path) {
        Ok(s) => Ok(Some(s)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("lecture de {} : {e}", path.display())),
    }
}

/// Atomically-ish write: create parent dirs, validate JSON when `as_json`, then
/// write. Validation refuses to clobber a file with malformed JSON.
fn write_text(path: &Path, content: &str, as_json: bool) -> Result<(), String> {
    if as_json {
        serde_json::from_str::<serde_json::Value>(content)
            .map_err(|e| format!("JSON invalide, écriture refusée : {e}"))?;
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("création de {} : {e}", parent.display()))?;
    }
    std::fs::write(path, content).map_err(|e| format!("écriture de {} : {e}", path.display()))
}

// ---------------------------------------------------------------------------
// Agent config files (agy / antigravity / claude / aphrody-mcp).
// EXACT allowlist — no other key is accepted.
// ---------------------------------------------------------------------------

/// Map a logical config name to its on-disk path. The match arm IS the
/// allowlist: an unknown `which` is rejected, so the webview can never coax an
/// arbitrary path out of these commands.
fn agent_config_path(which: &str) -> Result<PathBuf, String> {
    let home = home_dir()?;
    let path = match which {
        // agy CLI and Antigravity 2.0 share the gemini config used by the
        // Antigravity sidecar / agy forwarder.
        "agy" | "antigravity" => home.join(".gemini").join("config").join("config.json"),
        "claude" => home.join(".claude").join("settings.json"),
        "aphrody-mcp" => home.join(".config").join("aphrody").join("mcp.json"),
        other => return Err(format!("config inconnue : {other:?} (autorisées : agy, antigravity, claude, aphrody-mcp)")),
    };
    Ok(path)
}

/// A config file's content + presence + resolved path, for the UI editor.
#[derive(serde::Serialize)]
struct ConfigDoc {
    /// Logical name echoed back.
    which: String,
    /// Absolute resolved path (shown read-only in the UI).
    path: String,
    /// `true` when the file currently exists on disk.
    exists: bool,
    /// File content (empty string when absent).
    content: String,
}

/// Read one allowlisted agent config file.
#[tauri::command]
fn read_agent_config(which: String) -> Result<ConfigDoc, String> {
    let path = agent_config_path(&which)?;
    let content = read_text_opt(&path)?;
    Ok(ConfigDoc {
        which,
        path: path.display().to_string(),
        exists: content.is_some(),
        content: content.unwrap_or_default(),
    })
}

/// Write one allowlisted agent config file. The content MUST be valid JSON;
/// otherwise the write is refused and the existing file is left untouched.
#[tauri::command]
fn write_agent_config(which: String, content: String) -> Result<ConfigDoc, String> {
    let path = agent_config_path(&which)?;
    write_text(&path, &content, true)?;
    Ok(ConfigDoc {
        which,
        path: path.display().to_string(),
        exists: true,
        content,
    })
}

// ---------------------------------------------------------------------------
// Agent-home soul / identity (markdown + optional YAML frontmatter).
// Files live at `<workspace>/SOUL.md` and `<workspace>/IDENTITY.md`, resolved
// exactly like the `aphrody-agent-home` crate.
// ---------------------------------------------------------------------------

/// Allowlist for the two persona files. Returns the absolute path under the
/// resolved workspace; an unknown `which` is rejected.
fn agent_home_path(which: &str) -> Result<PathBuf, String> {
    let ws = workspace_dir()?;
    let name = match which {
        "soul" => "SOUL.md",
        "identity" => "IDENTITY.md",
        other => return Err(format!("fichier persona inconnu : {other:?} (autorisés : soul, identity)")),
    };
    Ok(ws.join(name))
}

/// A persona document (soul / identity) for the UI editor.
#[derive(serde::Serialize)]
struct PersonaDoc {
    /// Logical name echoed back ("soul" | "identity").
    which: String,
    /// Absolute resolved path.
    path: String,
    /// Resolved workspace root (shown for context).
    workspace: String,
    /// `true` when the file currently exists.
    exists: bool,
    /// Markdown content (empty when absent).
    content: String,
}

/// Read the soul or identity markdown file.
#[tauri::command]
fn read_agent_home(which: String) -> Result<PersonaDoc, String> {
    let path = agent_home_path(&which)?;
    let ws = workspace_dir()?;
    let content = read_text_opt(&path)?;
    Ok(PersonaDoc {
        which,
        path: path.display().to_string(),
        workspace: ws.display().to_string(),
        exists: content.is_some(),
        content: content.unwrap_or_default(),
    })
}

/// Write the soul or identity markdown file (plain markdown — NOT JSON, so no
/// JSON validation; the parent workspace dir is created if missing).
#[tauri::command]
fn write_agent_home(which: String, content: String) -> Result<PersonaDoc, String> {
    let path = agent_home_path(&which)?;
    let ws = workspace_dir()?;
    write_text(&path, &content, false)?;
    Ok(PersonaDoc {
        which,
        path: path.display().to_string(),
        workspace: ws.display().to_string(),
        exists: true,
        content,
    })
}

// ---------------------------------------------------------------------------
// Channels — messaging credentials persisted to ~/.aphrody/channels.json.
// This file is OUTSIDE the repo. We never echo secrets back: read returns only
// which keys are SET (booleans), never their values.
// ---------------------------------------------------------------------------

/// `~/.aphrody/channels.json`.
fn channels_path() -> Result<PathBuf, String> {
    Ok(state_dir()?.join("channels.json"))
}

/// The set of channel credential keys the UI manages. Keeping the list here (and
/// validating against it on write) means the file can never accumulate unknown
/// keys from a tampered payload.
const CHANNEL_KEYS: &[&str] = &[
    "DISCORD_BOT_TOKEN",
    "X_HANDLE",
    "OPENAI_API_KEY",
    "ELEVENLABS_API_KEY",
    "SLACK_BOT_TOKEN",
    "SLACK_CHANNEL",
    "TELEGRAM_BOT_TOKEN",
    "TELEGRAM_CHAT_ID",
    "MATRIX_HOMESERVER",
    "MATRIX_ACCESS_TOKEN",
    "MATRIX_USER_ID",
    "MATRIX_ROOM_ID",
];

/// Which keys are non-secret handles (echoed back in full) vs. secrets (only a
/// "set" boolean is returned).
fn is_secret_key(key: &str) -> bool {
    !matches!(
        key,
        "X_HANDLE" | "SLACK_CHANNEL" | "TELEGRAM_CHAT_ID" | "MATRIX_HOMESERVER" | "MATRIX_USER_ID" | "MATRIX_ROOM_ID"
    )
}

/// Channels state for the UI: every managed key, its "configured" flag, and —
/// for non-secret handles only — the current value.
#[derive(serde::Serialize)]
struct ChannelsState {
    /// Absolute path of the store.
    path: String,
    /// `true` when channels.json exists.
    exists: bool,
    /// For each key: is a non-empty value stored?
    configured: std::collections::BTreeMap<String, bool>,
    /// For NON-secret keys only: the plain value (secrets are omitted).
    values: std::collections::BTreeMap<String, String>,
}

/// Load the on-disk channels object (a flat string map), tolerating absence.
fn load_channels() -> Result<serde_json::Map<String, serde_json::Value>, String> {
    let path = channels_path()?;
    match read_text_opt(&path)? {
        None => Ok(serde_json::Map::new()),
        Some(s) if s.trim().is_empty() => Ok(serde_json::Map::new()),
        Some(s) => {
            let v: serde_json::Value =
                serde_json::from_str(&s).map_err(|e| format!("channels.json corrompu : {e}"))?;
            Ok(v.as_object().cloned().unwrap_or_default())
        }
    }
}

/// Read the channels configuration. Secrets are NEVER returned — only a boolean
/// telling the UI whether each secret is set, plus the plain non-secret handles.
#[tauri::command]
fn read_channels() -> Result<ChannelsState, String> {
    let path = channels_path()?;
    let exists = path.exists();
    let obj = load_channels()?;
    let mut configured = std::collections::BTreeMap::new();
    let mut values = std::collections::BTreeMap::new();
    for &key in CHANNEL_KEYS {
        let val = obj.get(key).and_then(serde_json::Value::as_str).unwrap_or("");
        configured.insert(key.to_string(), !val.is_empty());
        if !is_secret_key(key) {
            values.insert(key.to_string(), val.to_string());
        }
    }
    Ok(ChannelsState { path: path.display().to_string(), exists, configured, values })
}

/// A single channel-field update from the UI.
#[derive(serde::Deserialize)]
struct ChannelUpdate {
    /// Key — must be one of [`CHANNEL_KEYS`].
    key: String,
    /// New value. Empty string clears the key.
    value: String,
}

/// Persist channel updates. Only allowlisted keys are accepted; empty values
/// delete the key. Secrets that are NOT in the update set are preserved as-is
/// (so the UI can save a partial form without wiping previously-set secrets).
/// Returns the refreshed [`ChannelsState`] (still secret-free).
#[tauri::command]
fn write_channels(updates: Vec<ChannelUpdate>) -> Result<ChannelsState, String> {
    let mut obj = load_channels()?;
    for up in updates {
        if !CHANNEL_KEYS.contains(&up.key.as_str()) {
            return Err(format!("clé de canal inconnue : {:?}", up.key));
        }
        let trimmed = up.value.trim();
        if trimmed.is_empty() {
            obj.remove(&up.key);
        } else {
            obj.insert(up.key, serde_json::Value::String(trimmed.to_string()));
        }
    }
    // Drop any stray keys that crept in previously (defence in depth).
    obj.retain(|k, _| CHANNEL_KEYS.contains(&k.as_str()));
    let path = channels_path()?;
    let serialized =
        serde_json::to_string_pretty(&serde_json::Value::Object(obj)).map_err(|e| e.to_string())?;
    write_text(&path, &serialized, true)?;
    read_channels()
}

// ---------------------------------------------------------------------------
// Filesystem stat — read-only size/mtime probe for known aphrody-home files.
// Allowlisted by logical name, never an arbitrary path.
// ---------------------------------------------------------------------------

/// Lightweight stat result for a known file.
#[derive(serde::Serialize)]
struct StatResult {
    /// Logical name echoed back.
    which: String,
    /// Absolute resolved path.
    path: String,
    /// `true` when the file exists.
    exists: bool,
    /// File size in bytes (0 when absent).
    size: u64,
    /// Last-modified time as a Unix epoch in seconds (0 when unknown).
    modified_secs: u64,
}

/// Resolve an allowlisted stat target.
fn stat_path(which: &str) -> Result<PathBuf, String> {
    let path = match which {
        "memory-sqlite" => state_dir()?.join("aphrody-memory.sqlite"),
        "agent-config" => state_dir()?.join("aphrody.json"),
        "channels" => state_dir()?.join("channels.json"),
        "soul" => workspace_dir()?.join("SOUL.md"),
        "identity" => workspace_dir()?.join("IDENTITY.md"),
        other => return Err(format!("cible stat inconnue : {other:?}")),
    };
    Ok(path)
}

/// Stat a known aphrody-home file (size + mtime). Used by the Mémoire / Soul /
/// Identity settings tabs to show real on-disk facts.
#[tauri::command]
fn aphrody_stat(which: String) -> Result<StatResult, String> {
    let path = stat_path(&which)?;
    match std::fs::metadata(&path) {
        Ok(meta) => {
            let modified_secs = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_secs());
            Ok(StatResult {
                which,
                path: path.display().to_string(),
                exists: true,
                size: meta.len(),
                modified_secs,
            })
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(StatResult {
            which,
            path: path.display().to_string(),
            exists: false,
            size: 0,
            modified_secs: 0,
        }),
        Err(e) => Err(format!("stat de {} : {e}", path.display())),
    }
}

// ---------------------------------------------------------------------------
// Sibling binary exec — aphrody-skills / aphrody-mcp next to the running exe.
// Allowlisted by EXACT binary name. Resolved next to current_exe(), then PATH.
// ---------------------------------------------------------------------------

/// Resolve an allowlisted sibling binary. Looks next to the running executable
/// first (the deployed `~/.local/bin/` layout puts the siblings there), then
/// falls back to the binary name on `PATH` (dev convenience).
fn resolve_sibling(bin: &str) -> Result<PathBuf, String> {
    if !matches!(bin, "aphrody-skills" | "aphrody-mcp") {
        return Err(format!("binaire voisin non autorisé : {bin:?} (autorisés : aphrody-skills, aphrody-mcp)"));
    }
    let exe_name = if cfg!(windows) { format!("{bin}.exe") } else { bin.to_string() };
    if let Ok(cur) = std::env::current_exe() {
        if let Some(dir) = cur.parent() {
            let candidate = dir.join(&exe_name);
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
    // Fall back to PATH: return the bare name and let the OS resolve it.
    Ok(PathBuf::from(exe_name))
}

/// Run an allowlisted sibling binary (`aphrody-skills` / `aphrody-mcp`) as a
/// real subprocess and capture its output. Unlike [`aphrody_exec`] these are
/// SEPARATE binaries, so they cannot run in-process.
#[tauri::command]
async fn aphrody_sibling_exec(bin: String, args: Vec<String>) -> Result<ExecResult, String> {
    let program = resolve_sibling(&bin)?;
    tauri::async_runtime::spawn_blocking(move || {
        let output = std::process::Command::new(&program).args(&args).output();
        match output {
            Ok(out) => ExecResult {
                code: out.status.code().unwrap_or(-1),
                stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
            },
            Err(e) => ExecResult {
                code: -1,
                stdout: String::new(),
                stderr: format!("échec du lancement de {} : {e}", program.display()),
            },
        }
    })
    .await
    .map_err(|err| format!("aphrody_sibling_exec failed to join the worker thread: {err}"))
}

// ---------------------------------------------------------------------------
// Voice — TTS (ElevenLabs) + STT (Whisper API / ElevenLabs STT).
// Keys are resolved: process env first, then ~/.aphrody/channels.json.
// Secrets are NEVER echoed back. Audio bytes are base64-encoded for transport.
// ---------------------------------------------------------------------------

/// Resolve a voice API key: process env variable first, then channels.json.
///
/// Returns `None` (-> "no key") when neither source has the key set.
fn resolve_voice_key(env_var: &str) -> Option<String> {
    // 1. Process environment (highest priority).
    if let Ok(v) = std::env::var(env_var) {
        let trimmed = v.trim().to_owned();
        if !trimmed.is_empty() {
            return Some(trimmed);
        }
    }
    // 2. Fallback: ~/.aphrody/channels.json (same file the Channels tab writes).
    let obj = load_channels().ok()?;
    let val = obj.get(env_var)?.as_str()?;
    let trimmed = val.trim();
    if trimmed.is_empty() { None } else { Some(trimmed.to_owned()) }
}

/// Synthesise `text` via ElevenLabs and return the audio as a base64 string.
///
/// The frontend plays it via `new Audio("data:audio/mpeg;base64,<result>")`.
/// Returns `Err("no-tts-key")` when neither ELEVENLABS_API_KEY nor channels.json
/// contains a key — the frontend falls back to `speechSynthesis.speak()`.
#[tauri::command]
async fn voice_synthesize(text: String, voice_id: Option<String>) -> Result<String, String> {
    let key = resolve_voice_key("ELEVENLABS_API_KEY").ok_or_else(|| "no-tts-key".to_owned())?;
    let vid = voice_id.as_deref().unwrap_or("EXAVITQu4vr4xnSDxMaL");
    let provider = aphrody_voice::elevenlabs::ElevenLabsProvider::new(key);
    use aphrody_voice::VoiceProvider as _;
    let audio = provider
        .synthesize(&text, vid, aphrody_voice::SynthOptions::default())
        .await
        .map_err(|e| format!("tts: {e}"))?;
    Ok(base64::engine::general_purpose::STANDARD.encode(&audio))
}

/// Transcribe base64-encoded audio via OpenAI Whisper or ElevenLabs STT.
///
/// Key resolution order:
/// - OPENAI_API_KEY (env or channels.json) -> WhisperApiProvider
/// - ELEVENLABS_API_KEY (env or channels.json) -> ElevenLabsSttProvider
/// - Neither -> `Err("no-stt-key")`; the frontend falls back to Web Speech.
///
/// `mime` is informational (from the MediaRecorder) and not forwarded to the
/// provider (both APIs infer codec from file magic bytes).
#[tauri::command]
async fn voice_transcribe(audio_b64: String, mime: String) -> Result<String, String> {
    let _ = mime; // informational only
    let audio = base64::engine::general_purpose::STANDARD
        .decode(audio_b64.trim())
        .map_err(|e| format!("base64 decode: {e}"))?;

    use aphrody_voice::stt::SttProvider as _;
    let opts = aphrody_voice::stt::SttOptions::default();

    if let Some(key) = resolve_voice_key("OPENAI_API_KEY") {
        let provider = aphrody_voice::stt::whisper_api::WhisperApiProvider::new(key);
        let transcript = provider
            .transcribe(&audio, opts)
            .await
            .map_err(|e| format!("stt-whisper: {e}"))?;
        return Ok(transcript.text);
    }

    if let Some(key) = resolve_voice_key("ELEVENLABS_API_KEY") {
        let provider = aphrody_voice::stt::elevenlabs_stt::ElevenLabsSttProvider::new(key);
        let transcript = provider
            .transcribe(&audio, opts)
            .await
            .map_err(|e| format!("stt-elevenlabs: {e}"))?;
        return Ok(transcript.text);
    }

    Err("no-stt-key".to_owned())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // rustls 0.23 requires an explicit CryptoProvider before the first
    // `reqwest::Client::new()` (otherwise reqwest panics in async_impl/client.rs).
    // `aphrody_exec` installs it lazily via `run_async`, but the native voice
    // commands (`voice_synthesize` / `voice_transcribe`) build their reqwest
    // client OUTSIDE that path — a mic click before any `aphrody_exec` would
    // panic. Install once here, at startup, so the order is guaranteed.
    // Idempotent: the later `run_async` install is a no-op (CLAUDE.md §7).
    let _ = rustls::crypto::ring::default_provider().install_default();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            aphrody_exec,
            aphrody_meta,
            aphrody_sibling_exec,
            read_agent_config,
            write_agent_config,
            read_agent_home,
            write_agent_home,
            read_channels,
            write_channels,
            aphrody_stat,
            voice_synthesize,
            voice_transcribe,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_config_allowlist_rejects_unknown() {
        assert!(agent_config_path("../etc/passwd").is_err());
        assert!(agent_config_path("random").is_err());
        // Known keys resolve (home may or may not be set in CI; just check the
        // arm is reached without the "inconnue" error when home exists).
        if home_dir().is_ok() {
            assert!(agent_config_path("claude").is_ok());
            assert!(agent_config_path("agy").is_ok());
        }
    }

    #[test]
    fn agent_home_allowlist_rejects_unknown() {
        assert!(agent_home_path("../escape").is_err());
        assert!(agent_home_path("memory").is_err());
    }

    #[test]
    fn sibling_allowlist_rejects_unknown() {
        assert!(resolve_sibling("rm").is_err());
        assert!(resolve_sibling("aphrody").is_err());
        assert!(resolve_sibling("aphrody-skills").is_ok());
        assert!(resolve_sibling("aphrody-mcp").is_ok());
    }

    #[test]
    fn sanitize_profile_contains_no_separators() {
        for raw in ["../../etc", "a/../b", "..\\win", "/abs"] {
            let s = sanitize_profile(raw);
            assert!(!s.contains('/') && !s.contains('\\') && !s.contains(".."), "leaked: {s}");
        }
    }

    #[test]
    fn secret_classification() {
        assert!(is_secret_key("DISCORD_BOT_TOKEN"));
        assert!(is_secret_key("OPENAI_API_KEY"));
        assert!(!is_secret_key("X_HANDLE"));
        assert!(!is_secret_key("SLACK_CHANNEL"));
    }

    #[test]
    fn write_text_refuses_invalid_json() {
        let dir = std::env::temp_dir().join("aphrody-desktop-test");
        let _ = std::fs::create_dir_all(&dir);
        let p = dir.join("invalid.json");
        let err = write_text(&p, "{ not json", true).unwrap_err();
        assert!(err.contains("JSON invalide"));
    }

    // ── Voice: base64 round-trip ──────────────────────────────────────────────

    #[test]
    fn base64_round_trip() {
        let original = b"audio bytes sample";
        let encoded = base64::engine::general_purpose::STANDARD.encode(original);
        let decoded = base64::engine::general_purpose::STANDARD.decode(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    // ── Voice: no-key sentinel from resolve_voice_key ─────────────────────────

    #[test]
    fn resolve_voice_key_absent_returns_none() {
        // Use a key name that cannot possibly be set in any realistic environment.
        let absent = resolve_voice_key("APHRODY_DESKTOP_TEST_NONEXISTENT_VOICE_KEY_12345");
        assert!(absent.is_none(), "expected None for absent key, got {absent:?}");
    }
}
