// SPDX-License-Identifier: Apache-2.0
//! Adobe application directory mapper — forensic inventory of an Adobe
//! application installation (Photoshop, Illustrator, etc.).
//!
//! # Overview
//!
//! [`map_adobe_app`] walks an Adobe application root directory and classifies
//! every file into a typed [`AdobeFileKind`], producing a structured
//! [`AdobeAppMap`] that serialises to JSON via `serde`.
//!
//! ## Design decisions
//!
//! - **Hashing**: SHA-256 via the `sha2` crate (already a workspace dep).
//!   Files larger than [`HASH_SIZE_CAP`] bytes are recorded with
//!   `hash: None` to keep runtime reasonable over hundreds of MB.
//!   Pass `hash = true` to [`MapOptions`] to opt-in; pass `false` to skip
//!   hashing entirely.
//!
//! - **PE version extraction**: [`goblin`] can parse `VS_VERSIONINFO`
//!   resources from the resource section of a PE binary, giving us the
//!   `FileVersion` / `ProductVersion` strings embedded by the Adobe build
//!   system. Extraction is best-effort: files that are not valid PE64 or that
//!   lack a resource section return `version: None` rather than erroring.
//!
//! - **Classification**: driven purely by file extension + parent directory
//!   name + filename stem — no byte-level content analysis at walk time.
//!   This keeps the mapper O(n) over file count without reading every file
//!   twice.
//!
//! ## Hashing cap
//!
//! ```
//! use aphrody_re::adobe::HASH_SIZE_CAP;
//! assert_eq!(HASH_SIZE_CAP, 8 * 1024 * 1024);
//! ```
//!
//! # Examples
//!
//! ```no_run
//! use std::path::Path;
//! use aphrody_re::adobe::{map_adobe_app, MapOptions};
//!
//! let map = map_adobe_app(
//!     Path::new(r"C:\Program Files\Adobe\Adobe Photoshop 2026"),
//!     MapOptions { hash: true },
//! ).expect("walk failed");
//! println!("{} files, {} plugins", map.total_files, map.plugins.len());
//! ```

use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::Serialize;
use sha2::{Digest as _, Sha256};
use walkdir::WalkDir;

// ---------------------------------------------------------------------------
// Public constants
// ---------------------------------------------------------------------------

/// Files larger than this byte count are not hashed; `hash` is set to `None`.
///
/// 8 MiB — chosen to keep total wall time under ~2 s for a typical Photoshop
/// install (~8 000 files, most under this cap). Callers that need complete
/// coverage should hash files in a background thread and merge results.
pub const HASH_SIZE_CAP: u64 = 8 * 1024 * 1024;

// ---------------------------------------------------------------------------
// Public types — AdobeFileKind
// ---------------------------------------------------------------------------

/// Structural classification of a file within an Adobe application install.
///
/// Detection is performed by filename/extension/parent-directory matching in
/// [`classify`]; no file content is read at classification time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AdobeFileKind {
    /// Core runtime DLL at the application root — e.g. `ACE.dll`, `AGM.dll`,
    /// `AdobePDFL.dll`. These implement the fundamental Adobe imaging pipeline.
    CoreDll,
    /// Photoshop/Illustrator native plugin binary (`.8bi`, `.8ba`, `.8bf`,
    /// `.8be`, `.8bx`, `.8ly`, `.8li`) or a modern UXP plugin
    /// (`manifest.json` under a `UXP/` or CEP `extensions/` directory).
    Plugin,
    /// Scripting host library — `ExtendScript.dll`, `ScCore.dll`,
    /// `AdobeXMPScript.dll`, and `.jsx` / `.js` ExtendScript source files
    /// that are part of the required engine surface (not user scripts).
    ScriptingHost,
    /// On-device AI / ML runtime — `CloudAILib.dll`, `DirectML.dll`,
    /// `HalideRuntime.dll`, `AILib.dll`, or any `.onnx` / `.pb` model file,
    /// or any file under the `sensei_models/` or `Mona/` trees.
    AiRuntime,
    /// Localisation data — any file under a `Locales/` subtree, or `.pak`
    /// locale pack files (from the CEP Chromium engine), or `.aff`/`.dic`
    /// spell-check dictionaries.
    Locale,
    /// Executable binary (`.exe`).
    Executable,
    /// Type library or COM interface descriptor (`.tlb`).
    TypeLibrary,
    /// Configuration / markup file — `.xml`, `.json`, `.plist`, `.sif`,
    /// `.atn`, `.abr`, `.grd`, `.csh`, `.pat`, etc.
    Config,
    /// Everything that does not fit the above categories.
    Other,
}

// ---------------------------------------------------------------------------
// Public types — PluginInfo, AdobeEntry, AdobeAppMap
// ---------------------------------------------------------------------------

/// Metadata for a detected plugin (native `.8b?` or UXP/CEP `manifest.json`).
#[derive(Debug, Clone, Serialize)]
pub struct PluginInfo {
    /// Relative path from the application root.
    pub rel_path: String,
    /// Plugin technology: `"legacy"` (native `.8bi`/`.8bf`/…) or `"uxp"`
    /// (modern UXP, identified from a `manifest.json`) or `"cep"` (Common
    /// Extensibility Platform HTML panel).
    pub plugin_type: String,
    /// Plugin identifier read from `manifest.json` (`id` field), if present.
    pub plugin_id: Option<String>,
    /// Plugin version read from `manifest.json` (`version` field), if present.
    pub version: Option<String>,
    /// File size in bytes.
    pub size: u64,
}

/// One file entry in the inventory.
#[derive(Debug, Clone, Serialize)]
pub struct AdobeEntry {
    /// Path relative to the application root, using forward slashes.
    pub rel_path: String,
    /// Structural classification.
    pub kind: AdobeFileKind,
    /// File size in bytes.
    pub size: u64,
    /// SHA-256 hex digest, or `null` if the file exceeded [`HASH_SIZE_CAP`]
    /// or if hashing was disabled via [`MapOptions`].
    pub hash: Option<String>,
    /// PE `FileVersion` string (`"M.m.b.p"` form), if the file is a PE binary
    /// and the resource section was readable.
    pub version: Option<String>,
}

/// Complete forensic inventory of an Adobe application install directory.
///
/// Serialises to a self-describing JSON object. All `Vec` fields are always
/// arrays (never `null`).
#[derive(Debug, Clone, Serialize)]
pub struct AdobeAppMap {
    /// Best-effort application name (last component of `root`, e.g.
    /// `"Adobe Photoshop 2026"`).
    pub app_name: String,
    /// Absolute path to the application root that was walked.
    pub root: String,
    /// Total number of files discovered (including dirs-only entries).
    pub total_files: usize,
    /// Total bytes summed over all files.
    pub total_bytes: u64,
    /// Dominant product version string — taken from the main executable's
    /// `FileVersion` PE resource if available, otherwise `None`.
    pub version: Option<String>,
    /// All file entries, one per file found.
    pub entries: Vec<AdobeEntry>,
    /// Subset of entries that are plugins (native + UXP + CEP).
    pub plugins: Vec<PluginInfo>,
    /// Distinct on-device AI/ML runtime names detected (e.g.
    /// `"CloudAILib.dll"`, `"DirectML.dll"`, `"HalideRuntime.dll"`).
    pub ai_runtimes: Vec<String>,
    /// Distinct scripting host names detected (e.g. `"ExtendScript.dll"`,
    /// `"ScCore.dll"`).
    pub scripting_hosts: Vec<String>,
    /// UXP plugin IDs read from `manifest.json` files under `UXP/`.
    pub uxp_plugin_ids: Vec<String>,
    /// CEP panel IDs read from `manifest.json` files under `CEP/extensions/`.
    pub cep_panel_ids: Vec<String>,
    /// ISO-8601 UTC timestamp of when this map was generated.
    pub generated_at: String,
}

// ---------------------------------------------------------------------------
// MapOptions
// ---------------------------------------------------------------------------

/// Options controlling [`map_adobe_app`] behaviour.
#[derive(Debug, Clone, Copy)]
pub struct MapOptions {
    /// Whether to compute SHA-256 hashes.
    ///
    /// When `true`, files up to [`HASH_SIZE_CAP`] bytes are hashed;
    /// larger files get `hash: None`. When `false`, all `hash` fields are
    /// `None` (faster, useful for CI where only structure matters).
    pub hash: bool,
}

// ---------------------------------------------------------------------------
// Main public entry point
// ---------------------------------------------------------------------------

/// Walk `root` and produce a complete [`AdobeAppMap`].
///
/// # Errors
///
/// Returns `Err` only if the root directory cannot be opened (permissions /
/// missing). Individual file-read errors during hashing or PE parsing are
/// silently degraded to `hash: None` / `version: None`.
pub fn map_adobe_app(root: &Path, opts: MapOptions) -> io::Result<AdobeAppMap> {
    let app_name = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| root.to_string_lossy().into_owned());

    let mut entries: Vec<AdobeEntry> = Vec::new();
    let mut plugins: Vec<PluginInfo> = Vec::new();
    let mut ai_runtimes: HashSet<String> = HashSet::new();
    let mut scripting_hosts: HashSet<String> = HashSet::new();
    let mut uxp_plugin_ids: Vec<String> = Vec::new();
    let mut cep_panel_ids: Vec<String> = Vec::new();
    let mut total_bytes: u64 = 0;
    // Try to extract the product version from AMT/application.xml first —
    // this is more reliable than PE resource extraction for very large binaries
    // like Photoshop.exe (267 MB).
    let mut main_version: Option<String> = extract_amt_version(root);

    for entry in WalkDir::new(root).follow_links(false).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }

        let abs_path: PathBuf = entry.path().to_path_buf();
        let rel_path = abs_path
            .strip_prefix(root)
            .unwrap_or(&abs_path)
            .to_string_lossy()
            .replace('\\', "/");

        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let size = metadata.len();
        total_bytes += size;

        let kind = classify(&abs_path, &rel_path);

        // ---- PE version extraction (best-effort) ----
        let version = extract_pe_version_if_small(&abs_path, kind, size);

        // Augment with PE-extracted version for the top-level field when AMT
        // extraction did not produce a version (non-standard installs).
        if main_version.is_none() {
            if let Some(ref v) = version {
                let fname = abs_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_ascii_lowercase())
                    .unwrap_or_default();
                // Photoshop.exe / Illustrator.exe are the main binaries.
                if fname == "photoshop.exe" || fname == "illustrator.exe" {
                    main_version = Some(v.clone());
                }
            }
        }

        // ---- Hashing ----
        let hash = if opts.hash && size <= HASH_SIZE_CAP {
            hash_file(&abs_path)
        } else {
            None
        };

        // ---- Collect AI runtime names ----
        if kind == AdobeFileKind::AiRuntime {
            let name = abs_path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            if !name.is_empty() {
                ai_runtimes.insert(name);
            }
        }

        // ---- Collect scripting host names ----
        if kind == AdobeFileKind::ScriptingHost {
            let name = abs_path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            if !name.is_empty() {
                scripting_hosts.insert(name);
            }
        }

        // ---- Plugin analysis ----
        if kind == AdobeFileKind::Plugin {
            let plugin = analyse_plugin(&abs_path, &rel_path, size, &mut uxp_plugin_ids, &mut cep_panel_ids);
            plugins.push(plugin);
        }

        entries.push(AdobeEntry { rel_path, kind, size, hash, version });
    }

    let total_files = entries.len();

    let mut ai_runtimes_vec: Vec<String> = ai_runtimes.into_iter().collect();
    ai_runtimes_vec.sort();
    let mut scripting_hosts_vec: Vec<String> = scripting_hosts.into_iter().collect();
    scripting_hosts_vec.sort();

    let generated_at = iso8601_now();

    Ok(AdobeAppMap {
        app_name,
        root: root.to_string_lossy().into_owned(),
        total_files,
        total_bytes,
        version: main_version,
        entries,
        plugins,
        ai_runtimes: ai_runtimes_vec,
        scripting_hosts: scripting_hosts_vec,
        uxp_plugin_ids,
        cep_panel_ids,
        generated_at,
    })
}

// ---------------------------------------------------------------------------
// Classification logic
// ---------------------------------------------------------------------------

/// Classify a file by its path within the Adobe application tree.
///
/// All decisions are made purely on path components and extension — no file
/// content is read. The function is intentionally pure (no I/O) so it can be
/// tested on synthetic paths.
///
/// # Classification order (highest priority first)
///
/// 1. Under `sensei_models/` or `Mona/` tree → `AiRuntime`
/// 2. Under `Locales/` tree or `.pak` / `.pak.info` file → `Locale`
/// 3. Known scripting-host filenames → `ScriptingHost`
/// 4. Known AI/ML filenames or extensions (`.onnx`) → `AiRuntime`
/// 5. Native plugin extensions (`.8bi`, `.8ba`, `.8bf`, `.8be`, `.8bx`,
///    `.8ly`, `.8li`) → `Plugin`
/// 6. `manifest.json` inside UXP or CEP extension tree → `Plugin`
/// 7. `.exe` → `Executable`
/// 8. `.tlb` → `TypeLibrary`
/// 9. `.dll` → `CoreDll` (if at root level) or `Other` (if inside a subdir)
/// 10. Config/data extensions (`.xml`, `.json`, `.atn`, `.abr`, …) → `Config`
/// 11. Everything else → `Other`
///
/// ```
/// use std::path::Path;
/// use aphrody_re::adobe::{classify, AdobeFileKind};
///
/// let p = Path::new(r"C:\PS\CloudAILib.dll");
/// assert_eq!(classify(p, "CloudAILib.dll"), AdobeFileKind::AiRuntime);
///
/// let p2 = Path::new(r"C:\PS\Required\Plug-ins\File Formats\Cineon.8bi");
/// assert_eq!(classify(p2, "Required/Plug-ins/File Formats/Cineon.8bi"), AdobeFileKind::Plugin);
/// ```
#[must_use]
pub fn classify(abs_path: &Path, rel_path: &str) -> AdobeFileKind {
    let rel_lower = rel_path.to_ascii_lowercase();
    let fname = abs_path
        .file_name()
        .map(|n| n.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();

    // ---- 1. Subtree: sensei_models/ and Mona/ → AiRuntime ----
    if rel_lower.contains("sensei_models/") || rel_lower.starts_with("mona/") {
        return AdobeFileKind::AiRuntime;
    }

    // ---- 2. Subtree: Locales/ or locale pack files ----
    if rel_lower.contains("locales/")
        || fname.ends_with(".pak")
        || fname.ends_with(".pak.info")
        || fname.ends_with(".aff")    // hunspell affix (spell-check)
        || fname.ends_with(".dic")    // hunspell dictionary
    {
        return AdobeFileKind::Locale;
    }

    // ---- 3. Known scripting host filenames ----
    // ExtendScript engine, XMP scripting, Script Core, JSX runner.
    let scripting_stems: &[&str] = &[
        "extendscript",
        "sccore",
        "adobexmpscript",
        "adobexmpfiles",
        "adobexmp",
        "plugplugrexternalobject", // deprecated CEP JS bridge
        "plugplugowl",
        "PlugPlugExternalObject",
    ];
    if fname.ends_with(".dll") && scripting_stems.iter().any(|s| fname.starts_with(s)) {
        return AdobeFileKind::ScriptingHost;
    }
    // .jsx / .js files that ship with the application (not user scripts) —
    // they live under Required/ and represent the ExtendScript surface.
    if rel_lower.starts_with("required/") && (fname.ends_with(".jsx") || fname.ends_with(".jsxbin")) {
        return AdobeFileKind::ScriptingHost;
    }

    // ---- 4. Known AI/ML runtime filenames and model file types ----
    let ai_dll_stems: &[&str] = &[
        "cloudailib",
        "directml",
        "halideruntime",
        "ailib",       // AILib.dll / AIRes.dll
        "aide",        // AIDE.dll
        "aiid",        // potential AI id library
        "libxpuinfo",  // XPU info for DirectML
    ];
    if fname.ends_with(".dll") && ai_dll_stems.iter().any(|s| fname.starts_with(s)) {
        return AdobeFileKind::AiRuntime;
    }
    // ONNX and protocol-buffer model formats.
    if fname.ends_with(".onnx")
        || fname.ends_with(".pb")
        || fname.ends_with(".tflite")
        || fname.ends_with(".wnnx")   // Windows ML native format
        || fname.ends_with(".onnb")   // ONNB (Windows ML export)
    {
        return AdobeFileKind::AiRuntime;
    }

    // ---- 5. Native plugin file extensions (.8bi etc.) ----
    let ext = abs_path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    let native_plugin_exts: &[&str] = &["8bi", "8ba", "8bf", "8be", "8bx", "8by", "8bz", "8ly", "8li"];
    if native_plugin_exts.contains(&ext.as_str()) {
        return AdobeFileKind::Plugin;
    }

    // ---- 6. UXP / CEP manifest.json ----
    // A manifest.json under a UXP/ or CEP/extensions/ directory identifies a
    // modern plugin or panel.
    if fname == "manifest.json"
        && (rel_lower.contains("/uxp/") || rel_lower.contains("/cep/extensions/"))
    {
        return AdobeFileKind::Plugin;
    }

    // ---- 7. Executable ----
    if ext == "exe" {
        return AdobeFileKind::Executable;
    }

    // ---- 8. Type library ----
    if ext == "tlb" {
        return AdobeFileKind::TypeLibrary;
    }

    // ---- 9. DLL classification by depth ----
    if ext == "dll" {
        // Depth 0 = root-level DLL → CoreDll.
        // Count forward-slash separators in rel_path to determine depth.
        let depth = rel_path.matches('/').count();
        if depth == 0 {
            return AdobeFileKind::CoreDll;
        }
        // DLLs inside subdirectories (Required/, System/, etc.) are Other.
        return AdobeFileKind::Other;
    }

    // ---- 10. Config / data extensions ----
    let config_exts: &[&str] = &[
        "xml", "json", "plist", "ini", "sif", "txt",
        // Photoshop preset formats
        "atn",  // Actions
        "abr",  // Brushes
        "grd",  // Gradients
        "csh",  // Custom Shapes
        "pat",  // Patterns
        "asl",  // Styles
        "aco",  // Color Swatches
        "ase",  // Adobe Swatch Exchange
        "p3e",  // 3D Extrusion Presets
        "p3f",  // Face Expression Presets
        "shc",  // Contours
        "mnu",  // Menus
        "tpl",  // Tool Presets
        "psp",  // Type Styles
        "pem",  // PEM certificate (cacert.pem)
        "css",  // CEP panel stylesheets
        "html", // CEP panel pages
        "htm",
        "js",   // CEP/Generator JS scripts
        "jsx",  // ExtendScript (outside Required/)
        "jsxbin",
    ];
    if config_exts.contains(&ext.as_str()) {
        return AdobeFileKind::Config;
    }

    AdobeFileKind::Other
}

// ---------------------------------------------------------------------------
// AMT version extraction (from AMT/application.xml)
// ---------------------------------------------------------------------------

/// Extract the `ProductVersion` from the Adobe Licensing Toolkit (AMT)
/// `application.xml` file located at `<root>/AMT/application.xml`.
///
/// This is the authoritative product version for Adobe CC applications and
/// avoids having to read a multi-hundred-MB main executable for PE resource
/// parsing. The XML contains a `<Data key="ProductVersion">27.7</Data>` element.
///
/// Returns `None` if the file is absent, unreadable, or doesn't contain the
/// expected key.
fn extract_amt_version(root: &Path) -> Option<String> {
    let xml_path = root.join("AMT").join("application.xml");
    let contents = std::fs::read_to_string(&xml_path).ok()?;
    // Quick memmem scan rather than a full XML parser — the AMT XML is small
    // and the key pattern is unambiguous.
    let needle = r#"key="ProductVersion">"#;
    let start = contents.find(needle)? + needle.len();
    let end = contents[start..].find('<')? + start;
    let version = contents[start..end].trim().to_owned();
    if version.is_empty() { None } else { Some(version) }
}

// ---------------------------------------------------------------------------
// PE version extraction
// ---------------------------------------------------------------------------

/// Extract `FileVersion` from the PE VS_VERSIONINFO resource of a file.
///
/// Returns `None` silently on any error: not a PE, no resource section,
/// resource parse failure, no FileVersion string. Reads up to 4 MiB of the
/// file into memory.
///
/// Only attempted for `.dll` and `.exe` files; returns `None` for everything
/// else immediately.
fn extract_pe_version_if_small(path: &Path, kind: AdobeFileKind, size: u64) -> Option<String> {
    // Only attempt on binary types.
    let ext = path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    if ext != "dll" && ext != "exe" {
        return None;
    }

    // Heuristic: skip PE version extraction for large binaries (> 32 MiB) to
    // avoid spending 100 ms per file on a multi-GB walk.
    const PE_VERSION_MAX: u64 = 32 * 1024 * 1024;
    if size > PE_VERSION_MAX {
        return None;
    }

    // Suppress the warning – we only use these internally.
    let _ = kind;

    extract_pe_version_from_path(path)
}

/// Read the file and extract `FileVersion` from its PE resource section.
///
/// Returns `None` on any error.
pub(crate) fn extract_pe_version_from_path(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    extract_pe_version_from_bytes(&bytes)
}

/// Extract `FileVersion` from a raw PE byte slice using goblin.
///
/// Returns `None` on any parse failure.
pub(crate) fn extract_pe_version_from_bytes(bytes: &[u8]) -> Option<String> {
    // Must start with MZ and have at least the DOS stub.
    if bytes.len() < 64 || &bytes[..2] != b"MZ" {
        return None;
    }

    let pe = goblin::pe::PE::parse(bytes).ok()?;

    // goblin 0.10 exposes resource data on `pe.resource_data`.
    let resource_data = pe.resource_data?;

    // Retrieve the VersionInfo table.
    let ver = resource_data.version_info?;

    // `StringFileInfo::file_version()` decodes the UTF-16LE "FileVersion"
    // value from the VS_VERSIONINFO string table.
    let raw = ver.string_info.file_version()?;
    // Normalize: Adobe sometimes uses comma-separated form ("26, 0, 0, 100").
    let v = raw.trim().replace(", ", ".").replace(',', ".");
    if v.is_empty() { None } else { Some(v) }
}

// ---------------------------------------------------------------------------
// Plugin analysis
// ---------------------------------------------------------------------------

/// Build a [`PluginInfo`] record for a detected plugin file.
///
/// For `manifest.json` files, attempts to parse the JSON to extract `id` and
/// `version`. On parse failure, those fields are `None`. Identifies whether
/// this is a UXP or CEP panel based on path.
fn analyse_plugin(
    abs_path: &Path,
    rel_path: &str,
    size: u64,
    uxp_plugin_ids: &mut Vec<String>,
    cep_panel_ids: &mut Vec<String>,
) -> PluginInfo {
    let fname = abs_path
        .file_name()
        .map(|n| n.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();

    if fname == "manifest.json" {
        let rel_lower = rel_path.to_ascii_lowercase();
        let is_uxp = rel_lower.contains("/uxp/");
        let plugin_type = if is_uxp { "uxp" } else { "cep" }.to_owned();

        let (plugin_id, version) = parse_manifest_json(abs_path);

        if let Some(ref id) = plugin_id {
            if is_uxp {
                uxp_plugin_ids.push(id.clone());
            } else {
                cep_panel_ids.push(id.clone());
            }
        }

        PluginInfo { rel_path: rel_path.to_owned(), plugin_type, plugin_id, version, size }
    } else {
        // Native .8b? plugin
        PluginInfo {
            rel_path: rel_path.to_owned(),
            plugin_type: "legacy".to_owned(),
            plugin_id: None,
            version: None,
            size,
        }
    }
}

/// Parse a UXP/CEP `manifest.json` to extract `id` and `version` fields.
///
/// Returns `(None, None)` on any error (missing file, invalid JSON, missing
/// fields). The manifest format is not formally versioned here — we read the
/// top-level `id` and `version` string fields which are present in both UXP
/// `manifestVersion 4/5` and the older CEP `ExtensionManifest` JSON form.
fn parse_manifest_json(path: &Path) -> (Option<String>, Option<String>) {
    let contents = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return (None, None),
    };
    let v: serde_json::Value = match serde_json::from_str(&contents) {
        Ok(v) => v,
        Err(_) => return (None, None),
    };
    let id = v.get("id").and_then(|x| x.as_str()).map(ToOwned::to_owned);
    let version = v.get("version").and_then(|x| x.as_str()).map(ToOwned::to_owned);
    (id, version)
}

// ---------------------------------------------------------------------------
// SHA-256 file hashing
// ---------------------------------------------------------------------------

/// Compute the SHA-256 hex digest of the file at `path`.
///
/// Returns `None` on any I/O error (permission denied, file disappeared, etc.)
/// rather than propagating — the caller records `hash: None` gracefully.
fn hash_file(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let digest = Sha256::digest(&bytes);
    Some(hex::encode(digest))
}

// ---------------------------------------------------------------------------
// Timestamp
// ---------------------------------------------------------------------------

fn iso8601_now() -> String {
    // Use SystemTime to produce an ISO-8601 UTC string without pulling in
    // `chrono` as a direct dep of this module (though chrono is in the
    // workspace, we already have `sha2` / `sha2 workspace dep`).
    // Format manually from UNIX epoch: simple and zero-alloc.
    let dur = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    // Convert to Y-M-D H:M:S UTC using the proleptic Gregorian calendar.
    // (Good enough for a timestamp; we do not need sub-second precision.)
    let (y, mo, d, h, mi, s) = epoch_to_ymd_hms(secs);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

/// Minimal proleptic-Gregorian epoch decomposition.
///
/// Input: seconds since 1970-01-01 00:00:00 UTC.
/// Output: `(year, month, day, hour, minute, second)`.
///
/// Algorithm: Neri-Schneider / `date::civil_from_days` variant — branchless,
/// correct for years 1970–9999.
fn epoch_to_ymd_hms(epoch_secs: u64) -> (u32, u32, u32, u32, u32, u32) {
    let h = ((epoch_secs % 86400) / 3600) as u32;
    let mi = ((epoch_secs % 3600) / 60) as u32;
    let s = (epoch_secs % 60) as u32;
    let days = (epoch_secs / 86400) as u32; // days since 1970-01-01

    // Shift epoch from 1970-01-01 to 0000-03-01 for easier arithmetic.
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;           // day of era [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // year of era
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // day of year [0, 365]
    let mp = (5 * doy + 2) / 153;          // month index [0, 11] from March
    let d = doy - (153 * mp + 2) / 5 + 1; // day [1, 31]
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };
    (y, mo, d, h, mi, s)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // classify — pure path logic, no I/O
    // -----------------------------------------------------------------------

    fn p(s: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(s)
    }

    #[test]
    fn classify_core_dll_at_root() {
        assert_eq!(classify(&p("ACE.dll"), "ACE.dll"), AdobeFileKind::CoreDll);
        assert_eq!(classify(&p("AGM.dll"), "AGM.dll"), AdobeFileKind::CoreDll);
        assert_eq!(classify(&p("BIB.dll"), "BIB.dll"), AdobeFileKind::CoreDll);
        assert_eq!(classify(&p("AdobePDFL.dll"), "AdobePDFL.dll"), AdobeFileKind::CoreDll);
    }

    #[test]
    fn classify_ai_runtime_known_dlls() {
        assert_eq!(
            classify(&p("CloudAILib.dll"), "CloudAILib.dll"),
            AdobeFileKind::AiRuntime,
            "CloudAILib must be AiRuntime"
        );
        assert_eq!(
            classify(&p("DirectML.dll"), "DirectML.dll"),
            AdobeFileKind::AiRuntime,
            "DirectML must be AiRuntime"
        );
        assert_eq!(
            classify(&p("HalideRuntime.dll"), "HalideRuntime.dll"),
            AdobeFileKind::AiRuntime,
            "HalideRuntime must be AiRuntime"
        );
        assert_eq!(
            classify(&p("AILib.dll"), "AILib.dll"),
            AdobeFileKind::AiRuntime,
            "AILib must be AiRuntime"
        );
    }

    #[test]
    fn classify_ai_runtime_under_sensei_models_tree() {
        let rel = "Required/sensei_models/Select_Subject/WinML/manifest.json";
        assert_eq!(
            classify(&p(rel), rel),
            AdobeFileKind::AiRuntime,
            "sensei_models/ subtree must be AiRuntime"
        );
    }

    #[test]
    fn classify_ai_runtime_under_mona_tree() {
        let rel = "Mona/model/x24dnm/weights.bin";
        assert_eq!(classify(&p(rel), rel), AdobeFileKind::AiRuntime);
    }

    #[test]
    fn classify_ai_runtime_onnx_model() {
        let rel = "Required/DynamicLinkMediaServer/DVAMLTestAssets/model.onnx";
        assert_eq!(classify(&p(rel), rel), AdobeFileKind::AiRuntime);
    }

    #[test]
    fn classify_scripting_host_extendscript() {
        assert_eq!(
            classify(&p("ExtendScript.dll"), "ExtendScript.dll"),
            AdobeFileKind::ScriptingHost
        );
        assert_eq!(classify(&p("ScCore.dll"), "ScCore.dll"), AdobeFileKind::ScriptingHost);
        assert_eq!(
            classify(&p("AdobeXMPScript.dll"), "AdobeXMPScript.dll"),
            AdobeFileKind::ScriptingHost
        );
    }

    #[test]
    fn classify_scripting_host_jsx_in_required() {
        let rel = "Required/CAFcropCorners.jsx";
        assert_eq!(classify(&p(rel), rel), AdobeFileKind::ScriptingHost);
    }

    #[test]
    fn classify_native_plugin_extensions() {
        for ext in ["8bi", "8ba", "8bf", "8be", "8bx", "8ly", "8li"] {
            let name = format!("Required/Plug-ins/TestPlugin.{ext}");
            assert_eq!(
                classify(&p(&name), &name),
                AdobeFileKind::Plugin,
                ".{ext} must be Plugin"
            );
        }
    }

    #[test]
    fn classify_uxp_manifest_is_plugin() {
        let rel = "Required/UXP/com.adobe.photoshop.adjustments-panel/manifest.json";
        assert_eq!(classify(&p(rel), rel), AdobeFileKind::Plugin);
    }

    #[test]
    fn classify_cep_manifest_is_plugin() {
        let rel = "Required/CEP/extensions/com.adobe.DesignLibraryPanel.html/manifest.json";
        assert_eq!(classify(&p(rel), rel), AdobeFileKind::Plugin);
    }

    #[test]
    fn classify_locale_by_directory() {
        let rel = "Locales/fr_FR/strings.dat";
        assert_eq!(classify(&p(rel), rel), AdobeFileKind::Locale);
    }

    #[test]
    fn classify_locale_pak_file() {
        let rel = "Required/CEP/CEPHtmlEngine/locales/fr.pak";
        assert_eq!(classify(&p(rel), rel), AdobeFileKind::Locale);
    }

    #[test]
    fn classify_executable() {
        assert_eq!(classify(&p("Photoshop.exe"), "Photoshop.exe"), AdobeFileKind::Executable);
        assert_eq!(
            classify(&p("Required/CEP/CEPHtmlEngine/CEPHtmlEngine.exe"), "Required/CEP/CEPHtmlEngine/CEPHtmlEngine.exe"),
            AdobeFileKind::Executable
        );
    }

    #[test]
    fn classify_type_library() {
        assert_eq!(classify(&p("TypeLibrary.tlb"), "TypeLibrary.tlb"), AdobeFileKind::TypeLibrary);
    }

    #[test]
    fn classify_config_xml() {
        assert_eq!(
            classify(&p("AMT/application.xml"), "AMT/application.xml"),
            AdobeFileKind::Config
        );
    }

    #[test]
    fn classify_dll_in_subdir_is_other() {
        // A DLL deep inside a subdirectory is not a "core" DLL.
        let rel = "Required/CEP/CEPHtmlEngine/libcef.dll";
        assert_eq!(classify(&p(rel), rel), AdobeFileKind::Other);
    }

    // -----------------------------------------------------------------------
    // JSON round-trip for AdobeFileKind
    // -----------------------------------------------------------------------

    #[test]
    fn adobe_file_kind_serializes_snake_case() {
        let cases: &[(AdobeFileKind, &str)] = &[
            (AdobeFileKind::CoreDll, "\"core_dll\""),
            (AdobeFileKind::Plugin, "\"plugin\""),
            (AdobeFileKind::ScriptingHost, "\"scripting_host\""),
            (AdobeFileKind::AiRuntime, "\"ai_runtime\""),
            (AdobeFileKind::Locale, "\"locale\""),
            (AdobeFileKind::Executable, "\"executable\""),
            (AdobeFileKind::TypeLibrary, "\"type_library\""),
            (AdobeFileKind::Config, "\"config\""),
            (AdobeFileKind::Other, "\"other\""),
        ];
        for (kind, expected_json) in cases {
            let got = serde_json::to_string(kind).expect("serialize");
            assert_eq!(&got, expected_json, "kind {:?} must serialize to {expected_json}", kind);
        }
    }

    // -----------------------------------------------------------------------
    // AdobeEntry JSON round-trip
    // -----------------------------------------------------------------------

    #[test]
    fn adobe_entry_json_shape() {
        let entry = AdobeEntry {
            rel_path: "ACE.dll".to_owned(),
            kind: AdobeFileKind::CoreDll,
            size: 1024,
            hash: Some("deadbeef".repeat(8)),
            version: Some("26.0.0.100".to_owned()),
        };
        let json = serde_json::to_value(&entry).expect("serialize");
        assert_eq!(json["kind"], "core_dll");
        assert_eq!(json["size"], 1024u64);
        assert!(json["hash"].is_string());
        assert_eq!(json["version"], "26.0.0.100");
    }

    #[test]
    fn adobe_entry_null_hash_and_version() {
        let entry = AdobeEntry {
            rel_path: "bigfile.dat".to_owned(),
            kind: AdobeFileKind::Other,
            size: 1_000_000_000,
            hash: None,
            version: None,
        };
        let json = serde_json::to_value(&entry).expect("serialize");
        assert!(json["hash"].is_null());
        assert!(json["version"].is_null());
    }

    // -----------------------------------------------------------------------
    // AdobeAppMap on a synthetic temp directory
    // -----------------------------------------------------------------------

    #[test]
    fn map_adobe_app_on_synthetic_tree() {
        use std::fs;
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path();

        // Create a minimal synthetic Adobe-like tree.
        // Root-level DLLs
        fs::write(root.join("ACE.dll"), b"MZ\x00\x00 synthetic").unwrap();
        fs::write(root.join("CloudAILib.dll"), b"MZ\x00\x00 ai").unwrap();
        fs::write(root.join("ExtendScript.dll"), b"MZ\x00\x00 jsx").unwrap();
        fs::write(root.join("Photoshop.exe"), b"MZ\x00\x00 main").unwrap();

        // Locales subtree
        let loc = root.join("Locales").join("fr_FR");
        fs::create_dir_all(&loc).unwrap();
        fs::write(loc.join("UI.strings"), b"bonjour").unwrap();

        // Plugin: native .8bi
        let plugdir = root.join("Required").join("Plug-ins").join("File Formats");
        fs::create_dir_all(&plugdir).unwrap();
        fs::write(plugdir.join("Cineon.8bi"), b"plugin").unwrap();

        // Plugin: UXP manifest.json
        let uxpdir = root.join("Required").join("UXP").join("com.adobe.test");
        fs::create_dir_all(&uxpdir).unwrap();
        let manifest_content = r#"{"id":"com.adobe.test","version":"1.2.3","manifestVersion":5}"#;
        fs::write(uxpdir.join("manifest.json"), manifest_content.as_bytes()).unwrap();

        // AI model
        let modeldir = root.join("Required").join("sensei_models").join("SomeModel");
        fs::create_dir_all(&modeldir).unwrap();
        fs::write(modeldir.join("model.onnx"), b"\x00ONNX").unwrap();

        let map = map_adobe_app(root, MapOptions { hash: false }).expect("map");

        // Basic counts
        assert!(map.total_files >= 7, "expected >= 7 files, got {}", map.total_files);
        assert!(map.total_bytes > 0);

        // AI runtime detection
        assert!(
            map.ai_runtimes.iter().any(|r| r.to_ascii_lowercase().contains("cloudailib")),
            "CloudAILib.dll must be in ai_runtimes: {:?}",
            map.ai_runtimes
        );

        // Scripting host detection
        assert!(
            map.scripting_hosts.iter().any(|s| s.to_ascii_lowercase().contains("extendscript")),
            "ExtendScript.dll must be in scripting_hosts: {:?}",
            map.scripting_hosts
        );

        // Plugin detection: native + UXP
        assert!(!map.plugins.is_empty(), "plugins must not be empty");
        assert!(
            map.plugins.iter().any(|p| p.plugin_type == "legacy"),
            "must detect legacy (.8bi) plugin"
        );
        assert!(
            map.plugins.iter().any(|p| p.plugin_type == "uxp"),
            "must detect UXP (manifest.json) plugin"
        );

        // UXP plugin ID extracted from manifest.json
        assert!(
            map.uxp_plugin_ids.iter().any(|id| id == "com.adobe.test"),
            "uxp_plugin_ids must contain com.adobe.test: {:?}",
            map.uxp_plugin_ids
        );

        // JSON round-trip
        let json = serde_json::to_value(&map).expect("serialize");
        assert!(json["entries"].is_array());
        assert!(json["plugins"].is_array());
        assert!(json["ai_runtimes"].is_array());
        assert!(json["scripting_hosts"].is_array());
        assert!(json["uxp_plugin_ids"].is_array());
        assert!(json["generated_at"].is_string());
        let ga = json["generated_at"].as_str().unwrap();
        assert!(ga.ends_with('Z'), "generated_at must be UTC ISO-8601: {ga}");
    }

    // -----------------------------------------------------------------------
    // epoch_to_ymd_hms sanity checks
    // -----------------------------------------------------------------------

    #[test]
    fn epoch_zero_is_unix_epoch() {
        let (y, mo, d, h, mi, s) = epoch_to_ymd_hms(0);
        assert_eq!((y, mo, d, h, mi, s), (1970, 1, 1, 0, 0, 0));
    }

    #[test]
    fn epoch_known_date() {
        // 2026-05-21 00:00:00 UTC
        // Verified: node -e "console.log(Date.UTC(2026,4,21)/1000)" → 1_779_321_600
        let (y, mo, d, h, mi, s) = epoch_to_ymd_hms(1_779_321_600);
        assert_eq!((y, mo, d), (2026, 5, 21));
        assert_eq!((h, mi, s), (0, 0, 0));
    }

    // -----------------------------------------------------------------------
    // Live smoke test — real Photoshop 2026 install
    // Gated behind env var: ADOBE_PS_2026_ROOT or simply ADOBE_SMOKE_TEST=1
    // Run with: ADOBE_SMOKE_TEST=1 cargo test -p aphrody-re -- adobe::tests::live_photoshop_2026_smoke_test --ignored --nocapture
    // -----------------------------------------------------------------------

    #[test]
    #[ignore = "live smoke test — requires ADOBE_SMOKE_TEST=1 and Photoshop 2026 installed"]
    fn live_photoshop_2026_smoke_test() {
        if std::env::var("ADOBE_SMOKE_TEST").as_deref() != Ok("1") {
            eprintln!("Skipped: set ADOBE_SMOKE_TEST=1 to enable.");
            return;
        }
        let root_str = std::env::var("ADOBE_PS_2026_ROOT")
            .unwrap_or_else(|_| r"C:\Program Files\Adobe\Adobe Photoshop 2026".to_owned());
        let root = std::path::Path::new(&root_str);
        assert!(root.exists(), "Photoshop root not found: {root_str}");

        let map = map_adobe_app(root, MapOptions { hash: true }).expect("walk");
        assert!(map.total_files > 100, "expected many files, got {}", map.total_files);
        assert!(!map.ai_runtimes.is_empty(), "expected AI runtimes, got none");
        assert!(!map.scripting_hosts.is_empty(), "expected scripting hosts, got none");
        assert!(!map.plugins.is_empty(), "expected plugins, got none");
        eprintln!(
            "Photoshop 2026 map: {} files, {} bytes, {} plugins, AI runtimes: {:?}",
            map.total_files, map.total_bytes, map.plugins.len(), map.ai_runtimes
        );
    }

    // -----------------------------------------------------------------------
    // Artifact generator — writes var/data/adobe-map/photoshop-2026.json
    // Run with: ADOBE_GEN_ARTIFACT=1 cargo test -p aphrody-re -- adobe::tests::generate_photoshop_2026_json --ignored --nocapture
    // Hashing is enabled; files > 8 MiB get hash: null.
    // -----------------------------------------------------------------------

    #[test]
    #[ignore = "artifact generator — requires ADOBE_GEN_ARTIFACT=1 and Photoshop 2026 installed"]
    fn generate_photoshop_2026_json() {
        if std::env::var("ADOBE_GEN_ARTIFACT").as_deref() != Ok("1") {
            eprintln!("Skipped: set ADOBE_GEN_ARTIFACT=1 to enable.");
            return;
        }
        let root_str = std::env::var("ADOBE_PS_2026_ROOT")
            .unwrap_or_else(|_| r"C:\Program Files\Adobe\Adobe Photoshop 2026".to_owned());
        let root = std::path::Path::new(&root_str);
        assert!(root.exists(), "Photoshop root not found: {root_str}");

        let map = map_adobe_app(root, MapOptions { hash: true }).expect("walk failed");

        // Determine output path: workspace root / var / data / adobe-map / photoshop-2026.json
        let out_path = {
            // Walk up from the test binary to find the workspace root (contains Cargo.toml).
            let mut candidate = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .unwrap_or_else(|| std::path::PathBuf::from("."));
            // Go up until we find Cargo.toml (workspace root).
            loop {
                if candidate.join("Cargo.toml").exists() {
                    break;
                }
                match candidate.parent() {
                    Some(p) => candidate = p.to_path_buf(),
                    None => break,
                }
            }
            candidate.join("var").join("data").join("adobe-map").join("photoshop-2026.json")
        };

        std::fs::create_dir_all(out_path.parent().unwrap()).expect("create dirs");
        let json = serde_json::to_string_pretty(&map).expect("serialize");
        std::fs::write(&out_path, json.as_bytes()).expect("write artifact");

        eprintln!("Artifact written to: {}", out_path.display());
        eprintln!(
            "Summary: {} files, {} bytes total, {} plugins, {} UXP plugin IDs, {} CEP panel IDs",
            map.total_files, map.total_bytes, map.plugins.len(),
            map.uxp_plugin_ids.len(), map.cep_panel_ids.len()
        );
        eprintln!("AI runtimes ({}):", map.ai_runtimes.len());
        for r in &map.ai_runtimes {
            eprintln!("  - {r}");
        }
        eprintln!("Scripting hosts ({}):", map.scripting_hosts.len());
        for h in &map.scripting_hosts {
            eprintln!("  - {h}");
        }
        eprintln!("Detected app version: {:?}", map.version);
    }
}
