// SPDX-License-Identifier: Apache-2.0
//! `aphrody ide …` subcommands — Antigravity IDE installation inspector.
//!
//! All read-only sub-commands inspect the Antigravity IDE installation without
//! any modification. Only `launch` has a side-effect (it starts the IDE
//! process with inherited stdio). Output is JSON on stdout; `--json` enables
//! pretty-printing where applicable.
//!
//! # Security
//!
//! The `state` sub-command decodes `state.vscdb` via
//! [`antigravity_sdk::state_sync`] in structural-only mode: it never emits raw
//! token bytes, cookie values, or `secret://` row contents — only lengths,
//! key names, and boolean sign-in flags. This invariant is enforced by the
//! underlying [`antigravity_sdk::state_sync::parse_item_table`] /
//! [`antigravity_sdk::state_sync::decode_unified_state`] APIs.
//!
//! # Platform resolution
//!
//! The IDE installation directory is resolved via:
//! 1. `APHRODY_IDE_DIR` environment variable (highest priority).
//! 2. OS-default install path:
//!    - Windows: `%LOCALAPPDATA%\Programs\Antigravity IDE`
//!    - macOS: `/Applications/Antigravity IDE.app/Contents/Resources`
//!    - Linux: `~/.local/share/antigravity-ide` or `/opt/antigravity-ide`
//! 3. Error if no candidate directory exists on disk.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::json;

// ---------------------------------------------------------------------------
// Clap surface
// ---------------------------------------------------------------------------

/// Subcommands of `aphrody ide`.
#[derive(clap::Subcommand, Debug, Clone)]
pub(crate) enum IdeAction {
    /// Detect the Antigravity IDE installation and emit key metadata as JSON.
    ///
    /// Resolves the install directory (env APHRODY_IDE_DIR > OS default path),
    /// reads `resources/app/product.json` + `resources/app/package.json`, and
    /// emits a single JSON object on stdout.
    ///
    /// Example: aphrody ide info --json | jq '.ideVersion'
    Info {
        /// Pretty-print the JSON output (default: compact one-line).
        #[arg(long)]
        json: bool,
    },
    /// List the built-in extensions bundled with the IDE.
    ///
    /// Reads `resources/app/extensions/*/package.json` for each extension.
    /// Extensions in the `antigravity*` namespace are flagged. Output is a
    /// JSON array sorted by extension name.
    ///
    /// Example: aphrody ide extensions --json | jq '.[] | select(.antigravity)'
    Extensions {
        /// Pretty-print the JSON output (default: compact one-line).
        #[arg(long)]
        json: bool,
    },
    /// Reverse-engineer the IDE Electron executable and the sidecar language
    /// server binary using the `aphrody-re` pipeline.
    ///
    /// Analyses both the main `Antigravity IDE.exe` (format triage) and the Go
    /// language-server sidecar (triage + Google-specific extraction: OAuth
    /// client IDs, endpoints, binary family). The sidecar is where all RPC /
    /// auth artefacts live; the Electron binary is mostly devoid of them.
    ///
    /// String extraction is disabled by default because the sidecar is ~133 MB
    /// and a full strings scan adds significant latency. Pass `--strings` to
    /// enable it, and optionally `--strings-limit` to cap the number returned.
    ///
    /// Example: aphrody ide re --json | jq '.language_server.google.oauth_client_ids'
    Re {
        /// Pretty-print the JSON output (default: compact one-line).
        #[arg(long)]
        json: bool,
        /// Also extract ASCII + UTF-16LE strings sample from both binaries.
        ///
        /// Off by default because the sidecar (~133 MB) makes a full strings
        /// scan expensive. When enabled, extraction is capped at `--strings-limit`.
        #[arg(long)]
        strings: bool,
        /// Maximum number of strings to return per binary (when `--strings` is set).
        #[arg(long, default_value_t = aphrody_re::STRINGS_SAMPLE_LIMIT)]
        strings_limit: usize,
    },
    /// Decode the `state.vscdb` global-storage database (structural view only).
    ///
    /// Locates the file under the OS-appropriate app-data directory
    /// (`%APPDATA%\Antigravity IDE\User\globalStorage\state.vscdb` on Windows),
    /// opens it read-only, and decodes it via `antigravity_sdk::state_sync`.
    /// Output: key names, value lengths, and sign-in flags only — no raw token
    /// bytes are ever emitted.
    ///
    /// If the database is not found, a JSON object with `"found": false` is
    /// emitted with exit 0.
    ///
    /// Example: aphrody ide state --json | jq '.entries[] | select(.looks_like_signin_state)'
    State {
        /// Pretty-print the JSON output (default: compact one-line).
        #[arg(long)]
        json: bool,
    },
    /// Launch the Antigravity IDE, forwarding all arguments to the launcher.
    ///
    /// Resolves `bin/antigravity-ide(.cmd)` (preferred) or falls back to the
    /// main `Antigravity IDE.exe`. Inherits stdio; exit code propagated.
    ///
    /// Example: aphrody ide launch -- --new-window /path/to/project
    Launch {
        /// Additional arguments forwarded verbatim to the IDE launcher.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

// ---------------------------------------------------------------------------
// Domain types
// ---------------------------------------------------------------------------

/// Compact inventory of the Antigravity IDE installation.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeInfo {
    name_long: String,
    ide_version: String,
    /// VSCode upstream version (`package.version`).
    vscode_version: String,
    electron: Option<String>,
    commit: Option<String>,
    date: Option<String>,
    application_name: Option<String>,
    data_folder_name: Option<String>,
    url_protocol: Option<String>,
    paths: IdePaths,
}

/// Resolved filesystem paths for the IDE components.
#[derive(Debug, Serialize, Deserialize)]
struct IdePaths {
    /// Installation root (`APHRODY_IDE_DIR` / OS default).
    root: PathBuf,
    /// Main executable (e.g. `Antigravity IDE.exe` on Windows).
    exe: Option<PathBuf>,
    /// Shell launcher (`bin/antigravity-ide` or `bin/antigravity-ide.cmd`).
    launcher: Option<PathBuf>,
    /// `resources/app` directory containing `product.json` / `extensions/`.
    app: PathBuf,
    /// `resources/app/extensions` directory.
    extensions: PathBuf,
}

/// Compact metadata for a single bundled extension.
#[derive(Debug, Serialize)]
struct ExtensionInfo {
    name: String,
    version: String,
    display_name: Option<String>,
    publisher: Option<String>,
    description: Option<String>,
    /// Number of commands declared in `contributes.commands`.
    command_count: usize,
    /// True when the extension name starts with `antigravity`.
    antigravity: bool,
    /// Absolute path to the extension directory.
    path: PathBuf,
}

// ---------------------------------------------------------------------------
// Dispatch entry point
// ---------------------------------------------------------------------------

/// Run the `aphrody ide <action>` dispatch.
///
/// Called from `main.rs` on native (non-wasm32) targets.
pub(crate) async fn run(action: IdeAction) -> miette::Result<()> {
    match action {
        IdeAction::Info { json } => run_info(json),
        IdeAction::Extensions { json } => run_extensions(json),
        IdeAction::Re { json, strings, strings_limit } => {
            run_re(json, strings, strings_limit).await
        },
        IdeAction::State { json } => run_state(json),
        IdeAction::Launch { args } => run_launch(args),
    }
}

// ---------------------------------------------------------------------------
// Installation directory resolution
// ---------------------------------------------------------------------------

/// Locate the Antigravity IDE installation root directory.
///
/// Resolution order:
/// 1. `APHRODY_IDE_DIR` environment variable.
/// 2. Platform-default path (see module-level docs).
///
/// Returns an error if no candidate directory exists on disk.
fn resolve_ide_root() -> miette::Result<PathBuf> {
    // 1. Explicit override.
    if let Ok(v) = std::env::var("APHRODY_IDE_DIR") {
        let p = PathBuf::from(v);
        if p.is_dir() {
            return Ok(p);
        }
        return Err(miette::miette!(
            "APHRODY_IDE_DIR={} does not exist or is not a directory",
            p.display()
        ));
    }

    // 2. OS-default candidates.
    let candidates = default_ide_candidates();
    for c in &candidates {
        if c.is_dir() {
            return Ok(c.clone());
        }
    }

    let listed: Vec<String> = candidates.iter().map(|p| p.display().to_string()).collect();
    Err(miette::miette!(
        "Antigravity IDE not found in any default location.\n\
         Tried:\n  {}\n\
         Set APHRODY_IDE_DIR to override.",
        listed.join("\n  ")
    ))
}

/// Return the ordered list of OS-default IDE installation directories.
fn default_ide_candidates() -> Vec<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let mut candidates = Vec::new();
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            candidates.push(PathBuf::from(&local).join("Programs").join("Antigravity IDE"));
        }
        // Fallback: system-wide install.
        if let Ok(prog) = std::env::var("ProgramFiles") {
            candidates.push(PathBuf::from(&prog).join("Antigravity IDE"));
        }
        candidates
    }

    #[cfg(target_os = "macos")]
    {
        let mut candidates = vec![PathBuf::from(
            "/Applications/Antigravity IDE.app/Contents/Resources",
        )];
        if let Some(home) = dirs::home_dir() {
            candidates.push(
                home.join("Applications/Antigravity IDE.app/Contents/Resources"),
            );
        }
        candidates
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let mut candidates = vec![
            PathBuf::from("/opt/antigravity-ide"),
            PathBuf::from("/usr/lib/antigravity-ide"),
        ];
        if let Some(home) = dirs::home_dir() {
            candidates.push(home.join(".local/share/antigravity-ide"));
        }
        candidates
    }
}

/// Build resolved path metadata from an installation root.
fn build_ide_paths(root: &Path) -> IdePaths {
    let app = root.join("resources").join("app");
    let extensions = app.join("extensions");
    let exe = resolve_main_exe(root);
    let launcher = resolve_launcher(root);
    IdePaths { root: root.to_path_buf(), exe, launcher, app, extensions }
}

/// Locate the main IDE executable in the installation root.
fn resolve_main_exe(root: &Path) -> Option<PathBuf> {
    // Windows: "Antigravity IDE.exe"
    let win = root.join("Antigravity IDE.exe");
    if win.exists() {
        return Some(win);
    }
    // Linux / macOS: "antigravity-ide" binary at root
    let unix = root.join("antigravity-ide");
    if unix.exists() {
        return Some(unix);
    }
    None
}

/// Locate the shell launcher (`bin/antigravity-ide[.cmd]`).
fn resolve_launcher(root: &Path) -> Option<PathBuf> {
    // Windows CMD wrapper preferred for PATH launches.
    let cmd = root.join("bin").join("antigravity-ide.cmd");
    if cmd.exists() {
        return Some(cmd);
    }
    let sh = root.join("bin").join("antigravity-ide");
    if sh.exists() {
        return Some(sh);
    }
    None
}

/// Locate the language server sidecar binary for the current OS/arch.
///
/// The sidecar lives under `resources/app/extensions/antigravity/bin/`.
/// Its name encodes OS + architecture: `language_server_{os}_{arch}.exe` on
/// Windows, `language_server_{os}_{arch}` on Unix.
fn resolve_language_server(root: &Path) -> Option<PathBuf> {
    let bin_dir = root
        .join("resources")
        .join("app")
        .join("extensions")
        .join("antigravity")
        .join("bin");

    let os_part = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else {
        "linux"
    };

    let arch_part = if cfg!(target_arch = "x86_64") {
        "x64"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "x64"
    };

    let stem = format!("language_server_{os_part}_{arch_part}");

    // Try exact name first.
    let suffix = if cfg!(target_os = "windows") { ".exe" } else { "" };
    let exact = bin_dir.join(format!("{stem}{suffix}"));
    if exact.exists() {
        return Some(exact);
    }

    // Fallback: search for any `language_server_*` file in the bin dir.
    if let Ok(rd) = std::fs::read_dir(&bin_dir) {
        for entry in rd.flatten() {
            let name = entry.file_name();
            if name.to_string_lossy().starts_with("language_server_") {
                return Some(entry.path());
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// `ide info`
// ---------------------------------------------------------------------------

fn run_info(pretty: bool) -> miette::Result<()> {
    let root = resolve_ide_root()?;
    let paths = build_ide_paths(&root);

    let product: serde_json::Value = read_json(&paths.app.join("product.json"))?;
    let package: serde_json::Value = read_json(&paths.app.join("package.json"))?;

    let info = IdeInfo {
        name_long: product
            .get("nameLong")
            .and_then(|v| v.as_str())
            .unwrap_or("Antigravity IDE")
            .to_owned(),
        ide_version: product
            .get("ideVersion")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_owned(),
        vscode_version: package
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_owned(),
        electron: product
            .get("electron")
            .or_else(|| product.get("electronVersion"))
            .and_then(|v| v.as_str())
            .map(str::to_owned),
        commit: product
            .get("commit")
            .and_then(|v| v.as_str())
            .map(str::to_owned),
        date: product
            .get("date")
            .and_then(|v| v.as_str())
            .map(str::to_owned),
        application_name: product
            .get("applicationName")
            .and_then(|v| v.as_str())
            .map(str::to_owned),
        data_folder_name: product
            .get("dataFolderName")
            .and_then(|v| v.as_str())
            .map(str::to_owned),
        url_protocol: product
            .get("urlProtocol")
            .and_then(|v| v.as_str())
            .map(str::to_owned),
        paths,
    };

    emit_json(&info, pretty)
}

// ---------------------------------------------------------------------------
// `ide extensions`
// ---------------------------------------------------------------------------

fn run_extensions(pretty: bool) -> miette::Result<()> {
    let root = resolve_ide_root()?;
    let paths = build_ide_paths(&root);

    if !paths.extensions.is_dir() {
        return Err(miette::miette!(
            "extensions directory not found: {}",
            paths.extensions.display()
        ));
    }

    let mut extensions: Vec<ExtensionInfo> = Vec::new();

    let read_dir = std::fs::read_dir(&paths.extensions)
        .map_err(|e| miette::miette!("read_dir {}: {e}", paths.extensions.display()))?;

    for entry in read_dir.flatten() {
        let ext_dir = entry.path();
        if !ext_dir.is_dir() {
            continue;
        }

        let pkg_path = ext_dir.join("package.json");
        if !pkg_path.exists() {
            continue;
        }

        let pkg: serde_json::Value = match read_json(&pkg_path) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let dir_name = ext_dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();

        let name = pkg
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(&dir_name)
            .to_owned();

        let version = pkg
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("0.0.0")
            .to_owned();

        let display_name = pkg
            .get("displayName")
            .and_then(|v| v.as_str())
            .map(str::to_owned);

        let publisher = pkg
            .get("publisher")
            .and_then(|v| v.as_str())
            .map(str::to_owned);

        let description = pkg
            .get("description")
            .and_then(|v| v.as_str())
            .map(str::to_owned);

        let command_count = pkg
            .get("contributes")
            .and_then(|c| c.get("commands"))
            .and_then(|cmds| cmds.as_array())
            .map(|a| a.len())
            .unwrap_or(0);

        let antigravity = name.starts_with("antigravity");

        extensions.push(ExtensionInfo {
            name,
            version,
            display_name,
            publisher,
            description,
            command_count,
            antigravity,
            path: ext_dir,
        });
    }

    extensions.sort_by(|a, b| a.name.cmp(&b.name));

    emit_json(&extensions, pretty)
}

// ---------------------------------------------------------------------------
// `ide re`
// ---------------------------------------------------------------------------

/// Run `aphrody ide re`.
///
/// Performs triage on both the Electron main exe and the Go language-server
/// sidecar, plus Google-specific analysis (OAuth IDs, endpoints, binary
/// family) on the sidecar — this is where all RPC / auth artefacts live.
///
/// String extraction is opt-in (`--strings`) because the sidecar is ~133 MB;
/// the full byte scan dominates wall-clock time. When `--strings` is set,
/// extraction is capped to `--strings-limit` entries.
///
/// JSON is streamed to stdout via `serde_json::to_writer` (no intermediate
/// `String` allocation).
async fn run_re(pretty: bool, include_strings: bool, strings_limit: usize) -> miette::Result<()> {
    use std::io::{self, Write as _};

    let root = resolve_ide_root()?;
    let paths = build_ide_paths(&root);

    // --- Triage the Electron main executable ---
    let electron_analysis = if let Some(exe) = &paths.exe {
        let bytes = std::fs::read(exe)
            .map_err(|e| miette::miette!("failed to read {}: {e}", exe.display()))?;
        let triage = aphrody_re::triage(&bytes)
            .map_err(|e| miette::miette!("triage {} failed: {e}", exe.display()))?;
        let strings_sample: Option<Vec<String>> = if include_strings {
            Some(aphrody_re::extract_strings(
                &bytes,
                aphrody_re::STRINGS_MIN_LEN,
                strings_limit,
            ))
        } else {
            None
        };
        Some(json!({
            "path": exe,
            "triage": triage,
            "strings": strings_sample,
        }))
    } else {
        None
    };

    // --- Triage + Google analysis on the language server sidecar (~133 MB) ---
    // The sidecar is the Go binary carrying all RPC / auth / OAuth artefacts.
    // We always run triage + google analysis; strings extraction is opt-in.
    let ls_path = resolve_language_server(&root);
    let language_server = if let Some(ls) = &ls_path {
        let bytes = std::fs::read(ls).map_err(|e| {
            miette::miette!("failed to read language server {}: {e}", ls.display())
        })?;

        let triage = aphrody_re::triage(&bytes)
            .map_err(|e| miette::miette!("triage language_server failed: {e}"))?;

        // Google analysis always runs; it uses byte-level needle passes for
        // family detection, OAuth IDs, gRPC paths, Chromium version, and
        // code-sign subject — then regex passes for endpoints.
        let google = aphrody_re::google::analyze_google(&bytes);

        let strings_sample: Option<Vec<String>> = if include_strings {
            Some(aphrody_re::extract_strings(
                &bytes,
                aphrody_re::STRINGS_MIN_LEN,
                strings_limit,
            ))
        } else {
            None
        };

        Some(json!({
            "path": ls,
            "triage": triage,
            "google": google,
            "strings": strings_sample,
        }))
    } else {
        None
    };

    let report = json!({
        "electron_exe": electron_analysis,
        "language_server": language_server,
    });

    // Stream JSON directly to stdout — avoids a full-report intermediate String.
    let stdout = io::stdout();
    let mut out = stdout.lock();
    if pretty {
        serde_json::to_writer_pretty(&mut out, &report)
            .map_err(|e| miette::miette!("JSON encode failed: {e}"))?;
    } else {
        serde_json::to_writer(&mut out, &report)
            .map_err(|e| miette::miette!("JSON encode failed: {e}"))?;
    }
    // Trailing newline so the shell prompt appears on its own line.
    out.write_all(b"\n")
        .map_err(|e| miette::miette!("stdout write failed: {e}"))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// `ide state`
// ---------------------------------------------------------------------------

fn run_state(pretty: bool) -> miette::Result<()> {
    let db_path = match resolve_state_vscdb() {
        Some(p) => p,
        None => {
            let report = json!({ "found": false, "message": "state.vscdb path could not be resolved on this platform" });
            return emit_json(&report, pretty);
        },
    };

    if !db_path.exists() {
        let report = json!({
            "found": false,
            "path": db_path,
            "message": "state.vscdb does not exist at expected location",
        });
        return emit_json(&report, pretty);
    }

    // Read the ItemTable from state.vscdb via antigravity-sdk (read-only).
    // SECURITY INVARIANT: raw values are never logged or printed; they flow
    // directly into parse_item_table / decode_unified_state which enforce
    // structural-only output (lengths, keys, boolean flags).
    let items = antigravity_sdk::state_sync::read_state_db(&db_path)
        .map_err(|e| miette::miette!("read state.vscdb: {e}"))?;

    let item_json = serde_json::Value::Object(items);
    let entries = antigravity_sdk::state_sync::parse_item_table(&item_json);

    // For UnifiedStateSync keys, attempt a structural decode (embedded JSON
    // + base32 hint). The decoder never returns token material.
    let decoded_entries: Vec<serde_json::Value> = entries
        .iter()
        .map(|e| {
            let mut obj = json!({
                "key": e.key,
                "value_len": e.value_len,
                "is_unified_state_sync": e.is_unified_state_sync,
                "looks_like_signin_state": e.looks_like_signin_state,
            });
            if e.is_unified_state_sync {
                if let Some(raw_val) = item_json.get(&e.key).and_then(|v| v.as_str()) {
                    if let Ok(decoded) =
                        antigravity_sdk::state_sync::decode_unified_state(raw_val)
                    {
                        obj["decoded"] = decoded;
                    }
                }
            }
            obj
        })
        .collect();

    let report = json!({
        "found": true,
        "path": db_path,
        "entry_count": decoded_entries.len(),
        "entries": decoded_entries,
    });

    emit_json(&report, pretty)
}

/// Locate the `state.vscdb` file for the Antigravity IDE.
///
/// The IDE is a VSCode fork with `dataFolderName = ".antigravity-ide"`.
/// The user-data directory follows VSCode conventions:
/// - Windows: `%APPDATA%\Antigravity IDE\User\globalStorage\state.vscdb`
/// - macOS: `~/Library/Application Support/Antigravity IDE/User/globalStorage/state.vscdb`
/// - Linux: `~/.config/Antigravity IDE/User/globalStorage/state.vscdb`
fn resolve_state_vscdb() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA").ok().map(|appdata| {
            PathBuf::from(appdata)
                .join("Antigravity IDE")
                .join("User")
                .join("globalStorage")
                .join("state.vscdb")
        })
    }

    #[cfg(not(target_os = "windows"))]
    {
        dirs::config_dir().map(|d| {
            d.join("Antigravity IDE")
                .join("User")
                .join("globalStorage")
                .join("state.vscdb")
        })
    }
}

// ---------------------------------------------------------------------------
// `ide launch`
// ---------------------------------------------------------------------------

fn run_launch(args: Vec<String>) -> miette::Result<()> {
    use crate::commands::SubprocessExit;

    let root = resolve_ide_root()?;
    let paths = build_ide_paths(&root);

    // Prefer the shell launcher (picks up PATH shims properly); fall back to
    // the main exe for direct launch.
    let bin = paths
        .launcher
        .as_ref()
        .or(paths.exe.as_ref())
        .ok_or_else(|| {
            miette::miette!(
                "could not find the Antigravity IDE binary in {}",
                root.display()
            )
        })?;

    let mut cmd = std::process::Command::new(bin);
    cmd.args(&args);
    cmd.stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());

    let status = cmd
        .status()
        .map_err(|e| miette::miette!("failed to launch {}: {e}", bin.display()))?;

    let code = status.code().unwrap_or(1);
    if code != 0 {
        return Err(miette::Report::new(SubprocessExit(code)));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Read and parse a JSON file from disk.
fn read_json(path: &Path) -> miette::Result<serde_json::Value> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| miette::miette!("read {}: {e}", path.display()))?;
    serde_json::from_str(&content)
        .map_err(|e| miette::miette!("parse JSON {}: {e}", path.display()))
}

/// Serialize `value` as compact or pretty JSON and print it on stdout.
fn emit_json<T: serde::Serialize>(value: &T, pretty: bool) -> miette::Result<()> {
    let out = if pretty {
        serde_json::to_string_pretty(value)
    } else {
        serde_json::to_string(value)
    }
    .map_err(|e| miette::miette!("JSON encode failed: {e}"))?;
    println!("{out}");
    Ok(())
}

// ---------------------------------------------------------------------------
// Unit tests (pure, no real install required)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_ide_candidates_is_non_empty() {
        let candidates = default_ide_candidates();
        assert!(!candidates.is_empty(), "must produce at least one candidate path");
    }

    #[test]
    fn resolve_launcher_missing_dir() {
        // Non-existent root must return None without panicking.
        let root = PathBuf::from("/nonexistent/antigravity-ide");
        assert!(resolve_launcher(&root).is_none());
        assert!(resolve_main_exe(&root).is_none());
    }

    #[test]
    fn resolve_language_server_missing_dir() {
        let root = PathBuf::from("/nonexistent/antigravity-ide");
        assert!(resolve_language_server(&root).is_none());
    }

    #[test]
    fn parse_product_json_from_string() {
        let raw = r#"{
            "nameLong": "Antigravity IDE",
            "ideVersion": "2.0.2",
            "applicationName": "antigravity-ide",
            "dataFolderName": ".antigravity-ide",
            "urlProtocol": "antigravity-ide"
        }"#;
        let v: serde_json::Value = serde_json::from_str(raw).unwrap();
        assert_eq!(v["ideVersion"].as_str(), Some("2.0.2"));
        assert_eq!(v["applicationName"].as_str(), Some("antigravity-ide"));
        assert_eq!(v["urlProtocol"].as_str(), Some("antigravity-ide"));
    }

    #[test]
    fn parse_package_json_version() {
        let raw = r#"{"name": "Antigravity IDE", "version": "1.107.0"}"#;
        let v: serde_json::Value = serde_json::from_str(raw).unwrap();
        assert_eq!(v["version"].as_str(), Some("1.107.0"));
    }

    #[test]
    fn state_vscdb_path_is_absolute() {
        if let Some(p) = resolve_state_vscdb() {
            assert!(p.is_absolute(), "state.vscdb path must be absolute");
        }
    }

    #[test]
    fn extension_antigravity_flag() {
        let names = ["antigravity", "antigravity-code-executor", "bat", "cpp"];
        let flags: Vec<bool> = names.iter().map(|n| n.starts_with("antigravity")).collect();
        assert_eq!(flags, [true, true, false, false]);
    }

    #[test]
    fn build_ide_paths_non_existent_root() {
        let root = PathBuf::from("/tmp/nonexistent-antigravity-test");
        let paths = build_ide_paths(&root);
        assert_eq!(paths.app, root.join("resources").join("app"));
        assert!(paths.exe.is_none());
        assert!(paths.launcher.is_none());
    }
}
