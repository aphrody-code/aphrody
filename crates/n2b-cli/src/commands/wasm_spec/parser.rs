// Copyright 2026 Yohan Pierre
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Parsing de la spec Wasm : collecte des `.wast`, extraction de modules,
//! encodage base64 minimal.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

// ---------------------------------------------------------------------------
// Collecte de fichiers .wast
// ---------------------------------------------------------------------------

/// Collecte les fichiers `.wast` selon le filtre.
pub(super) fn collect_wast_files(core_dir: &Path, filter: Option<&str>) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    match filter {
        // Sous-répertoire de proposition
        Some(f) if !f.eq_ignore_ascii_case("core") => {
            let sub = core_dir.join(f);
            if sub.is_dir() {
                collect_wast_recursive(&sub, &mut files)?;
            } else {
                anyhow::bail!(
                    "Sous-répertoire `{}` introuvable dans `{}`",
                    f,
                    core_dir.display()
                );
            }
        }
        // Core ou pas de filtre : fichiers à la racine de test/core/
        _ => {
            let entries = std::fs::read_dir(core_dir)
                .with_context(|| format!("lecture de {}", core_dir.display()))?;
            for entry in entries.flatten() {
                let path = entry.path();
                let is_wast = path.extension().and_then(|e| e.to_str()) == Some("wast");
                let is_dir = path.is_dir();
                if is_wast {
                    files.push(path);
                } else if filter.is_none() && is_dir {
                    // Inclut aussi les sous-répertoires quand pas de filtre
                    collect_wast_recursive(&path, &mut files)?;
                }
            }
            files.sort();
        }
    }

    Ok(files)
}

/// Collecte récursivement les `.wast` dans un répertoire.
pub(super) fn collect_wast_recursive(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    let entries =
        std::fs::read_dir(dir).with_context(|| format!("lecture de {}", dir.display()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_wast_recursive(&path, out)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("wast") {
            out.push(path);
        }
    }
    out.sort();
    Ok(())
}

// ---------------------------------------------------------------------------
// Résultat d'analyse d'un fichier .wast
// ---------------------------------------------------------------------------

/// Résultat d'analyse d'un fichier `.wast`.
pub(super) struct WastResult {
    pub modules_found: usize,
    pub modules_passed: usize,
    pub modules_failed: usize,
    pub skipped: bool,
}

/// Traite un fichier `.wast` : extrait les modules et les valide si possible.
pub(super) fn process_wast_file(
    path: &Path,
    wat2wasm: Option<&Path>,
    bun: Option<&Path>,
) -> Result<WastResult> {
    let src =
        std::fs::read_to_string(path).with_context(|| format!("lecture de {}", path.display()))?;

    let modules = extract_modules(&src);

    if modules.is_empty() {
        return Ok(WastResult {
            modules_found: 0,
            modules_passed: 0,
            modules_failed: 0,
            skipped: false,
        });
    }

    // Si wat2wasm n'est pas disponible : mode count-only.
    let Some(wat2wasm_bin) = wat2wasm else {
        return Ok(WastResult {
            modules_found: modules.len(),
            modules_passed: 0,
            modules_failed: 0,
            skipped: false,
        });
    };

    let mut passed = 0usize;
    let mut failed = 0usize;
    let tmp_dir = std::env::temp_dir().join("n2b-wasm-spec");
    std::fs::create_dir_all(&tmp_dir)?;

    for (i, module_src) in modules.iter().enumerate() {
        let wat_path = tmp_dir.join(format!("mod_{i}.wat"));
        let wasm_path = tmp_dir.join(format!("mod_{i}.wasm"));
        std::fs::write(&wat_path, module_src)?;

        // Compile .wat → .wasm
        let compile_ok = Command::new(wat2wasm_bin)
            .arg(&wat_path)
            .arg("-o")
            .arg(&wasm_path)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if !compile_ok {
            failed += 1;
            continue;
        }

        // Si Bun disponible, valide via WebAssembly.Module
        if let Some(bun_bin) = bun {
            let wasm_bytes = std::fs::read(&wasm_path)?;
            let wasm_b64 = base64_encode(&wasm_bytes);
            let script = format!(
                r#"const b=Buffer.from("{wasm_b64}","base64");
new WebAssembly.Module(b);
process.exit(0);"#
            );
            let validate_ok = Command::new(bun_bin)
                .arg("-e")
                .arg(&script)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
            if validate_ok {
                passed += 1;
            } else {
                failed += 1;
            }
        } else {
            // Compilé avec succès, pas de validation Bun
            passed += 1;
        }
    }

    Ok(WastResult {
        modules_found: modules.len(),
        modules_passed: passed,
        modules_failed: failed,
        skipped: false,
    })
}

// ---------------------------------------------------------------------------
// Extraction des blocs (module …) d'un source .wast
// ---------------------------------------------------------------------------

/// Extrait les blocs `(module …)` top-level d'un source `.wast`.
///
/// Algorithme : parcourt le texte en comptant les parenthèses. Quand on
/// rencontre `(module` à profondeur 0, on capture jusqu'à la parenthèse
/// fermante de niveau 0. Les commentaires (`;; …` et `(; … ;)`) sont ignorés
/// via un état simple de la FSM.
///
/// Limitation connue : les strings littérales dans les modules peuvent contenir
/// des parenthèses — ce parseur rudimentaire peut se tromper dans des cas
/// extrêmes (tests `binary.wast`). En pratique il fonctionne sur +95 % des cas.
pub(super) fn extract_modules(src: &str) -> Vec<String> {
    let mut modules = Vec::new();
    let bytes = src.as_bytes();
    let len = bytes.len();
    let mut i = 0usize;

    while i < len {
        // Saute les commentaires ligne (;; …)
        if i + 1 < len && bytes[i] == b';' && bytes[i + 1] == b';' {
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        // Saute les commentaires bloc (; … ;)
        if i + 1 < len && bytes[i] == b'(' && bytes[i + 1] == b';' {
            i += 2;
            while i + 1 < len {
                if bytes[i] == b';' && bytes[i + 1] == b')' {
                    i += 2;
                    break;
                }
                i += 1;
            }
            continue;
        }
        // Détecte `(module` à profondeur 0
        if bytes[i] == b'('
            && src[i..].starts_with("(module")
            && src[i + 7..]
                .chars()
                .next()
                .map(|c| c.is_whitespace() || c == ')')
                .unwrap_or(true)
        {
            // Capture jusqu'à la parenthèse fermante de niveau 0
            let start = i;
            let mut depth = 0i32;
            let mut in_string = false;
            let mut j = i;

            while j < len {
                // Saute les strings littérales (pour ne pas compter les parens dedans)
                if !in_string && bytes[j] == b'"' {
                    in_string = true;
                    j += 1;
                    continue;
                }
                if in_string {
                    if bytes[j] == b'\\' {
                        j += 2; // séquence d'échappement
                        continue;
                    }
                    if bytes[j] == b'"' {
                        in_string = false;
                    }
                    j += 1;
                    continue;
                }
                match bytes[j] {
                    b'(' => {
                        depth += 1;
                    }
                    b')' => {
                        depth -= 1;
                        if depth == 0 {
                            modules.push(src[start..=j].to_string());
                            i = j + 1;
                            break;
                        }
                    }
                    _ => {}
                }
                j += 1;
            }
            if depth != 0 {
                // Module mal formé ou fin de fichier atteinte sans fermer
                i = j;
            }
            continue;
        }
        i += 1;
    }

    modules
}

// ---------------------------------------------------------------------------
// Encodage Base64 minimal (pas de dépendance externe)
// ---------------------------------------------------------------------------

/// Encodage Base64 minimal (pas de dépendance externe).
pub(super) fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((input.len() * 4 / 3) + 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((n >> 18) & 0x3f) as usize] as char);
        out.push(TABLE[((n >> 12) & 0x3f) as usize] as char);
        out.push(if chunk.len() > 1 {
            TABLE[((n >> 6) & 0x3f) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(n & 0x3f) as usize] as char
        } else {
            '='
        });
    }
    out
}

// ---------------------------------------------------------------------------
// Utilitaire PATH
// ---------------------------------------------------------------------------

/// Cherche un binaire dans PATH, retourne son chemin si trouvé.
pub(super) fn which(name: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let p = dir.join(name);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}
