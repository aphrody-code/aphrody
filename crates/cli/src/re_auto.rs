// SPDX-License-Identifier: Apache-2.0
//! Orchestrateur AUTO de reverse engineering.
//!
//! `aphrody re auto <path> [--deep] [--json] [--limit N]`
//!
//! Pour UN binaire : triage, strings, endpoints Google, analyse Go (si détecté),
//! disasm de l'entrypoint, et optionnellement Ghidra headless (`--deep`).
//!
//! Pour UN DOSSIER : batch récursif des exécutables (PE/ELF détectés par magic),
//! concurrence bornée à 4, résultat JSON array.
//!
//! Latence : seules les passes pure-Rust rapides sont exécutées par défaut.
//! `--deep` (Ghidra) est strictement opt-in.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use aphrody_re::{
    extract_strings, golang::analyze_go, google::analyze_google, triage_bounded,
    DisasmInsn, Format, TriageReport, STRINGS_MIN_LEN,
};

// ---------------------------------------------------------------------------
// Constantes
// ---------------------------------------------------------------------------

/// Nombre maximum de strings retournées par défaut dans le rapport auto (valeur
/// transmise à [`run_auto`] depuis l'arm CLI `--limit` default_value_t=2000).
#[allow(dead_code)]
pub(crate) const AUTO_STRINGS_LIMIT: usize = 2_000;

/// Nombre max d'instructions disasm à l'entrypoint.
const AUTO_DISASM_LIMIT: usize = 32;

/// Concurrence max pour le batch dossier.
const BATCH_CONCURRENCY: usize = 4;

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

/// Rapport consolidé produit par `re auto` pour un seul fichier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AutoReport {
    /// Chemin absolu du fichier analysé.
    pub target: String,
    /// Format détecté (`pe32`, `pe64`, `elf32`, `elf64`, `unknown`).
    pub format: String,
    /// Taille en octets.
    pub size: usize,
    /// SHA-256 hex du fichier complet.
    pub sha256: String,
    /// Architecture (`x86_64`, `aarch64`, …) — `null` si inconnu.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arch: Option<String>,
    /// Adresse virtuelle de l'entrypoint — `null` si inconnu.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_point: Option<u64>,
    /// Sections avec entropie.
    pub sections: Vec<SectionSummary>,
    /// Échantillon de strings (borné par `--limit`).
    pub strings_sample: Vec<String>,
    /// Endpoints / OAuth IDs / URLs Google extraits.
    pub endpoints: EndpointsSummary,
    /// Résultat de l'analyse Go (`null` si pas un binaire Go).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub go: Option<GoSummary>,
    /// Instructions disasm de l'entrypoint (`null` si non résolvable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disasm_entry: Option<Vec<DisasmSummary>>,
    /// Résultat de la passe Ghidra (`null` si `--deep` absent ou Ghidra indisponible).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deep: Option<DeepSummary>,
    /// Avertissements non-bloquants (ex. pclntab layout inconnu).
    pub warnings: Vec<String>,
}

/// Résumé d'une section pour le rapport auto (plus léger que `Section` complet).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SectionSummary {
    pub name: String,
    pub vaddr: u64,
    pub size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entropy: Option<f64>,
}

/// Résumé des endpoints extraits par le module `google`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct EndpointsSummary {
    pub family: String,
    pub oauth_client_ids: Vec<String>,
    pub api_endpoints: Vec<String>,
    pub updater_urls: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chromium_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub electron_version: Option<String>,
}

/// Résumé de l'analyse Go.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct GoSummary {
    pub go_version: String,
    pub build_flags: String,
    pub module_path: String,
    pub func_count: usize,
    /// Les N premières fonctions (borné à 50 dans le rapport auto).
    pub funcs_sample: Vec<FuncSummary>,
    pub packages: Vec<String>,
    pub types_sample: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct FuncSummary {
    pub name: String,
    pub addr: u64,
}

/// Résumé d'une instruction disasm (version compacte).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DisasmSummary {
    pub address: u64,
    pub text: String,
}

/// Résultat de la passe Ghidra headless.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DeepSummary {
    /// `true` si Ghidra a été trouvé et invoqué avec succès.
    pub ghidra_available: bool,
    /// Message d'explication si `ghidra_available = false`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ghidra_unavailable_reason: Option<String>,
    /// Sortie brute JSON de Ghidra (si disponible).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ghidra_output: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Point d'entrée principal
// ---------------------------------------------------------------------------

/// Lance l'analyse auto sur un fichier ou un dossier.
///
/// - `path` : chemin vers le binaire ou le dossier.
/// - `deep` : si `true`, invoque Ghidra headless (opt-in, lent).
/// - `json_only` : si `true`, supprime le résumé markdown sur stderr.
/// - `limit` : nombre max de strings par fichier.
///
/// Retourne une chaîne JSON (objet unique ou array).
///
/// # Errors
///
/// Retourne une erreur si la lecture du chemin échoue ou si la sérialisation
/// JSON échoue.
pub(crate) fn run_auto(
    path: &Path,
    deep: bool,
    json_only: bool,
    limit: usize,
) -> Result<String, String> {
    if path.is_dir() {
        run_auto_batch(path, deep, json_only, limit)
    } else {
        let report = analyze_file(path, deep, limit)?;
        if !json_only {
            print_markdown_summary(&report);
        }
        serde_json::to_string_pretty(&report).map_err(|e| e.to_string())
    }
}

// ---------------------------------------------------------------------------
// Analyse d'un fichier unique
// ---------------------------------------------------------------------------

fn analyze_file(path: &Path, deep: bool, strings_limit: usize) -> Result<AutoReport, String> {
    let data = std::fs::read(path)
        .map_err(|e| format!("failed to read {}: {e}", path.display()))?;

    // --- Triage ---
    let triage = triage_bounded(&data, 0) // strings extraites séparément
        .map_err(|e| format!("triage failed: {e}"))?;

    let format_str = format_name(&triage.format);
    let mut warnings = Vec::new();

    // --- Strings (borné) ---
    let strings_sample = extract_strings(&data, STRINGS_MIN_LEN, strings_limit);

    // --- Endpoints Google ---
    let google_report = analyze_google(&data);
    let endpoints = EndpointsSummary {
        family: format!("{:?}", google_report.family),
        oauth_client_ids: google_report.oauth_client_ids,
        api_endpoints: google_report.google_endpoints,
        updater_urls: google_report.updater_urls,
        chromium_version: google_report.chromium_version,
        electron_version: None, // not exposed by GoogleReport
    };

    // --- Analyse Go ---
    let go = match analyze_go(&data) {
        Some(report) => {
            let funcs_sample = report
                .funcs
                .iter()
                .take(50)
                .map(|f| FuncSummary { name: f.name.clone(), addr: f.addr })
                .collect();
            Some(GoSummary {
                go_version: report.go_version,
                build_flags: report.build_flags,
                module_path: report.module_path,
                func_count: report.func_count,
                funcs_sample,
                packages: report.packages,
                types_sample: report.types_sample,
            })
        },
        None => None,
    };

    // --- Disasm entrypoint ---
    let disasm_entry = disasm_entrypoint(&triage, &data, &mut warnings);

    // --- Passe deep (Ghidra) ---
    let deep_result = if deep {
        Some(run_ghidra(path, &mut warnings))
    } else {
        None
    };

    // --- Sections (résumé) ---
    let sections = triage
        .sections
        .iter()
        .map(|s| SectionSummary {
            name: s.name.clone(),
            vaddr: s.vaddr,
            size: s.size,
            entropy: s.entropy,
        })
        .collect();

    Ok(AutoReport {
        target: path.to_string_lossy().into_owned(),
        format: format_str,
        size: triage.size,
        sha256: triage.sha256,
        arch: triage.arch,
        entry_point: triage.entry_point,
        sections,
        strings_sample,
        endpoints,
        go,
        disasm_entry,
        deep: deep_result,
        warnings,
    })
}

// ---------------------------------------------------------------------------
// Disasm de l'entrypoint
// ---------------------------------------------------------------------------

fn disasm_entrypoint(
    triage: &TriageReport,
    data: &[u8],
    warnings: &mut Vec<String>,
) -> Option<Vec<DisasmSummary>> {
    // On ne tente la disasm que pour les formats PE/ELF x86_64.
    let is_x64 = triage.arch.as_deref() == Some("x86_64");
    if !is_x64 {
        return None;
    }

    let ep_va = triage.entry_point?;

    // Pour PE : l'entrypoint est une RVA. On doit trouver la section qui la
    // contient et calculer l'offset dans le fichier.
    // Pour ELF : ep_va est une VA absolue — il faut retrouver le segment.
    //
    // Approche conservatrice : on cherche la section dont la VA encadre ep_va.
    let ep_file_offset = resolve_va_to_file_offset(triage, ep_va);

    let ep_offset = match ep_file_offset {
        Some(off) => off,
        None => {
            warnings.push(format!(
                "entrypoint VA {ep_va:#x} not mapped to a file offset — disasm skipped"
            ));
            return None;
        },
    };

    let code_slice = match data.get(ep_offset..) {
        Some(s) if !s.is_empty() => s,
        _ => {
            warnings.push("entrypoint offset out of bounds".to_owned());
            return None;
        },
    };

    let insns: Vec<DisasmInsn> = aphrody_re::disasm(code_slice, ep_va, AUTO_DISASM_LIMIT);
    if insns.is_empty() {
        warnings.push(format!(
            "disasm at entrypoint {ep_va:#x} yielded no valid instructions"
        ));
        return None;
    }

    Some(
        insns
            .into_iter()
            .map(|i| DisasmSummary { address: i.address, text: i.text })
            .collect(),
    )
}

/// Résolution VA → offset fichier pour PE et ELF (best-effort).
fn resolve_va_to_file_offset(triage: &TriageReport, va: u64) -> Option<usize> {
    for section in &triage.sections {
        let sec_va_end = section.vaddr.saturating_add(section.size);
        if va >= section.vaddr && va < sec_va_end {
            // L'offset fichier d'une section est souvent la même chose que sa
            // VA pour les ELF chargés à l'adresse 0. Pour les PE, le raw offset
            // est différent de la VA mais nous n'avons pas cette info dans le
            // `TriageReport`. On retourne la VA comme offset direct — cela
            // fonctionne pour les ELF non-PIE et dégrade proprement sinon.
            //
            // Pour une résolution parfaite il faudrait exposer `raw_offset`
            // dans `Section`, ce qui est un futur travail.
            let delta = va.saturating_sub(section.vaddr) as usize;
            // Sanity check : l'offset calculé doit être < usize::MAX et
            // "raisonnable" (< 2 GiB).
            if delta < 2 * 1024 * 1024 * 1024 {
                return Some(delta + section.vaddr as usize);
            }
        }
    }

    // Si aucune section ne contient la VA, on essaie l'offset direct (cas ELF
    // position-dependant chargé à 0x400000 etc.).
    if va < 256 * 1024 * 1024 {
        // Heuristique : VA < 256 MiB → probablement un offset direct.
        Some(va as usize)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Passe Ghidra headless
// ---------------------------------------------------------------------------

/// Tente d'invoquer `analyzeHeadless` de Ghidra sur le binaire.
///
/// Résolution de Ghidra : `$GHIDRA_INSTALL_DIR` > `$GHIDRA_HOME` > PATH.
/// Si non trouvé, retourne `DeepSummary { ghidra_available: false, … }` sans
/// erreur fatale.
fn run_ghidra(path: &Path, warnings: &mut Vec<String>) -> DeepSummary {
    let ghidra_dir = resolve_ghidra_dir();

    let Some(ghidra_dir) = ghidra_dir else {
        let reason = "$GHIDRA_INSTALL_DIR / $GHIDRA_HOME not set and `analyzeHeadless` not in PATH";
        warnings.push(format!("ghidra_unavailable: {reason}"));
        return DeepSummary {
            ghidra_available: false,
            ghidra_unavailable_reason: Some(reason.to_owned()),
            ghidra_output: None,
        };
    };

    // Construire la commande analyzeHeadless.
    #[cfg(target_os = "windows")]
    let script_name = "analyzeHeadless.bat";
    #[cfg(not(target_os = "windows"))]
    let script_name = "analyzeHeadless";

    let headless_bin = ghidra_dir.join("support").join(script_name);
    if !headless_bin.exists() {
        let reason = format!("analyzeHeadless not found at {}", headless_bin.display());
        warnings.push(format!("ghidra_unavailable: {reason}"));
        return DeepSummary {
            ghidra_available: false,
            ghidra_unavailable_reason: Some(reason),
            ghidra_output: None,
        };
    }

    // Dossier projet temporaire.
    let tmp_proj = std::env::temp_dir().join(format!(
        "aphrody_ghidra_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    ));

    let status = std::process::Command::new(&headless_bin)
        .arg(&tmp_proj)
        .arg("aphrody_auto_re")
        .args(["-import", &path.to_string_lossy()])
        .arg("-deleteProject")
        .arg("-analysisTimeoutPerFile")
        .arg("120")
        .output();

    match status {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
            // Tenter de parser la sortie JSON si le script l'émet.
            let json_out = serde_json::from_str::<serde_json::Value>(&stdout).ok();
            DeepSummary {
                ghidra_available: true,
                ghidra_unavailable_reason: None,
                ghidra_output: json_out.or_else(|| {
                    Some(serde_json::Value::String(
                        stdout.chars().take(4096).collect(),
                    ))
                }),
            }
        },
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            let reason = format!(
                "analyzeHeadless exited with status {}: {}",
                output.status,
                stderr.chars().take(512).collect::<String>()
            );
            warnings.push(format!("ghidra error: {reason}"));
            DeepSummary {
                ghidra_available: true,
                ghidra_unavailable_reason: Some(reason),
                ghidra_output: None,
            }
        },
        Err(e) => {
            let reason = format!("failed to spawn analyzeHeadless: {e}");
            warnings.push(format!("ghidra spawn error: {reason}"));
            DeepSummary {
                ghidra_available: false,
                ghidra_unavailable_reason: Some(reason),
                ghidra_output: None,
            }
        },
    }
}

/// Résout le répertoire d'installation de Ghidra.
/// Ordre : `$GHIDRA_INSTALL_DIR` > `$GHIDRA_HOME` > PATH.
fn resolve_ghidra_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("GHIDRA_INSTALL_DIR") {
        let p = PathBuf::from(dir);
        if p.is_dir() {
            return Some(p);
        }
    }
    if let Ok(dir) = std::env::var("GHIDRA_HOME") {
        let p = PathBuf::from(dir);
        if p.is_dir() {
            return Some(p);
        }
    }
    // Tenter de trouver analyzeHeadless dans PATH.
    #[cfg(target_os = "windows")]
    let script = "analyzeHeadless.bat";
    #[cfg(not(target_os = "windows"))]
    let script = "analyzeHeadless";

    which_in_path(script).and_then(|p| {
        // analyzeHeadless est dans <ghidra>/support/
        p.parent()?.parent().map(PathBuf::from)
    })
}

/// Cherche un exécutable dans `PATH` — implémentation minimale sans nouvelle dep.
fn which_in_path(name: &str) -> Option<PathBuf> {
    let path_var = std::env::var("PATH").ok()?;
    #[cfg(target_os = "windows")]
    let sep = ';';
    #[cfg(not(target_os = "windows"))]
    let sep = ':';

    for dir in path_var.split(sep) {
        let candidate = PathBuf::from(dir).join(name);
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Batch (dossier)
// ---------------------------------------------------------------------------

fn run_auto_batch(
    dir: &Path,
    deep: bool,
    json_only: bool,
    limit: usize,
) -> Result<String, String> {
    use std::sync::{Arc, Mutex};

    let executables = collect_executables(dir);

    if !json_only {
        eprintln!(
            "[auto-re] batch scan: {} executable(s) found in {}",
            executables.len(),
            dir.display()
        );
    }

    let results: Arc<Mutex<Vec<AutoReport>>> = Arc::new(Mutex::new(Vec::new()));
    let errors: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    // Concurrence bornée : on utilise des threads natifs avec un sémaphore
    // manuel (pas de tokio ici — la fonction est synchrone et appelée depuis
    // le dispatch tokio via `spawn_blocking`).
    let semaphore = Arc::new(Semaphore::new(BATCH_CONCURRENCY));
    let mut handles = Vec::new();

    for path in executables {
        let results = Arc::clone(&results);
        let errors = Arc::clone(&errors);
        let sem = Arc::clone(&semaphore);

        let handle = std::thread::spawn(move || {
            let _permit = sem.acquire();
            match analyze_file(&path, deep, limit) {
                Ok(r) => {
                    results.lock().unwrap().push(r);
                },
                Err(e) => {
                    errors.lock().unwrap().push(e);
                },
            }
        });
        handles.push(handle);
    }

    for h in handles {
        let _ = h.join();
    }

    let reports = Arc::try_unwrap(results)
        .map_err(|_| "arc unwrap failed".to_owned())?
        .into_inner()
        .map_err(|e| e.to_string())?;

    let err_list = Arc::try_unwrap(errors)
        .map_err(|_| "arc unwrap failed".to_owned())?
        .into_inner()
        .map_err(|e| e.to_string())?;

    if !json_only && !err_list.is_empty() {
        for e in &err_list {
            eprintln!("[auto-re] error: {e}");
        }
    }

    serde_json::to_string_pretty(&reports).map_err(|e| e.to_string())
}

/// Collecte récursivement les fichiers qui sont des exécutables PE ou ELF.
fn collect_executables(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(walker) = walkdir_collect(dir) else { return out };
    for entry in walker {
        if is_executable_by_magic(&entry) {
            out.push(entry);
        }
    }
    out
}

fn walkdir_collect(dir: &Path) -> Result<Vec<PathBuf>, ()> {
    let mut paths = Vec::new();
    walk_dir_recursive(dir, &mut paths, 0, 8);
    Ok(paths)
}

fn walk_dir_recursive(dir: &Path, out: &mut Vec<PathBuf>, depth: usize, max_depth: usize) {
    if depth > max_depth {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_dir_recursive(&path, out, depth + 1, max_depth);
        } else if path.is_file() {
            out.push(path);
        }
    }
}

/// Détecte les exécutables par magic bytes (PE: `MZ`, ELF: `\x7FELF`).
fn is_executable_by_magic(path: &Path) -> bool {
    let Ok(mut f) = std::fs::File::open(path) else { return false };
    use std::io::Read;
    let mut magic = [0u8; 4];
    if f.read_exact(&mut magic).is_err() {
        return false;
    }
    &magic[..4] == b"\x7FELF" || &magic[..2] == b"MZ"
}

// ---------------------------------------------------------------------------
// Sémaphore minimal (sans tokio)
// ---------------------------------------------------------------------------

/// Sémaphore de comptage basique pour la concurrence bornée en threads natifs.
struct Semaphore {
    count: std::sync::Mutex<usize>,
    cond: std::sync::Condvar,
}

impl Semaphore {
    fn new(n: usize) -> Self {
        Self { count: std::sync::Mutex::new(n), cond: std::sync::Condvar::new() }
    }

    fn acquire(&self) -> SemaphorePermit<'_> {
        let mut count = self.count.lock().unwrap();
        while *count == 0 {
            count = self.cond.wait(count).unwrap();
        }
        *count -= 1;
        SemaphorePermit { sem: self }
    }
}

struct SemaphorePermit<'a> {
    sem: &'a Semaphore,
}

impl Drop for SemaphorePermit<'_> {
    fn drop(&mut self) {
        let mut count = self.sem.count.lock().unwrap();
        *count += 1;
        self.sem.cond.notify_one();
    }
}

// ---------------------------------------------------------------------------
// Résumé markdown (stderr)
// ---------------------------------------------------------------------------

fn print_markdown_summary(r: &AutoReport) {
    eprintln!("## RE Auto: {}", r.target);
    eprintln!("- Format: {} | Arch: {} | Size: {} bytes",
        r.format,
        r.arch.as_deref().unwrap_or("unknown"),
        r.size,
    );
    eprintln!("- SHA-256: {}", r.sha256);
    eprintln!("- Sections: {}", r.sections.len());
    eprintln!("- Strings: {} (sample)", r.strings_sample.len());
    eprintln!("- Endpoints family: {}", r.endpoints.family);
    if !r.endpoints.oauth_client_ids.is_empty() {
        eprintln!("- OAuth IDs: {}", r.endpoints.oauth_client_ids.join(", "));
    }
    if let Some(go) = &r.go {
        eprintln!("- Go: {} | funcs: {} | packages: {}",
            go.go_version, go.func_count, go.packages.len()
        );
    }
    if let Some(ep) = &r.entry_point {
        eprintln!("- Entrypoint: {ep:#x}");
        if let Some(disasm) = &r.disasm_entry {
            eprintln!("- Disasm ({} insns):", disasm.len());
            for insn in disasm.iter().take(5) {
                eprintln!("    {:#010x}  {}", insn.address, insn.text);
            }
        }
    }
    if !r.warnings.is_empty() {
        eprintln!("- Warnings: {}", r.warnings.join("; "));
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn format_name(f: &Format) -> String {
    match f {
        Format::Pe32 => "pe32".to_owned(),
        Format::Pe64 => "pe64".to_owned(),
        Format::Elf32 => "elf32".to_owned(),
        Format::Elf64 => "elf64".to_owned(),
        Format::Unknown => "unknown".to_owned(),
    }
}
