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

//! `n2b wasm spec` — référence WebAssembly spec officielle.
//!
//! Trois sous-commandes :
//!   - `testsuite` : scanne les fichiers `.wast` du repo WebAssembly/spec
//!     et extrait/compte les modules inline, les passe à `wat2wasm` si dispo.
//!   - `features`  : analyse un binaire `.wasm` et liste les propositions utilisées.
//!   - `opcodes`   : affiche la table d'opcodes statique, filtrée par proposition.
//!
//! Références :
//!   - <https://webassembly.github.io/spec/>
//!   - <https://github.com/WebAssembly/spec/tree/main/interpreter/binary/decode.ml>
//!   - <https://github.com/WebAssembly/spec/tree/main/document/core/appendix/index-instructions.rst>

use anyhow::Result;
use colored::Colorize;
use std::path::PathBuf;

mod codegen;
mod parser;
mod validator;

// ---------------------------------------------------------------------------
// Options publiques
// ---------------------------------------------------------------------------

/// Options pour `n2b wasm spec testsuite`.
pub struct TestsuiteOpts {
    /// Chemin vers la racine du clone WebAssembly/spec (contient `test/core/`).
    pub path: PathBuf,
    /// Filtre facultatif : `core`, `simd`, `gc`, `threads`, `bulk-memory`, etc.
    pub filter: Option<String>,
    /// Runtime : `bun` ou `wasmtime` — réservé V2 (actuellement ignored).
    #[allow(dead_code)]
    pub runtime: String,
    /// Timeout par `.wast` en secondes — réservé V2 (actuellement ignored).
    #[allow(dead_code)]
    pub timeout_secs: u64,
    pub quiet: bool,
}

/// Options pour `n2b wasm spec features`.
pub struct FeaturesOpts {
    /// Chemin du binaire `.wasm` à analyser.
    pub path: PathBuf,
    pub quiet: bool,
}

/// Options pour `n2b wasm spec opcodes`.
pub struct OpcodesOpts {
    /// Filtre par proposition (ex. `mvp`, `bulk-memory`, `simd`, …).
    /// `None` = affiche tous.
    pub proposal: Option<String>,
    /// Format de sortie : `text` | `md` | `json`.
    pub report: String,
    pub quiet: bool,
}

// ---------------------------------------------------------------------------
// Point d'entrée principal
// ---------------------------------------------------------------------------

/// Point d'entrée principal appelé depuis `wasm_cmd::run`.
pub fn run_spec(cmd: crate::wasm_cmd::WasmSpecCmd, quiet: bool) -> anyhow::Result<()> {
    match cmd {
        crate::wasm_cmd::WasmSpecCmd::Testsuite {
            path,
            filter,
            runtime,
            timeout_secs,
        } => run_testsuite(&TestsuiteOpts {
            path,
            filter,
            runtime,
            timeout_secs,
            quiet,
        }),
        crate::wasm_cmd::WasmSpecCmd::Features { path } => {
            validator::run_features(&FeaturesOpts { path, quiet })
        }
        crate::wasm_cmd::WasmSpecCmd::Opcodes { proposal, report } => {
            codegen::run_opcodes(&OpcodesOpts {
                proposal,
                report,
                quiet,
            })
        }
    }
}

// ---------------------------------------------------------------------------
// Testsuite — orchestration (délègue le parsing à parser.rs)
// ---------------------------------------------------------------------------

/// Lance la testsuite WebAssembly.
///
/// Stratégie :
/// 1. Collecte les fichiers `.wast` selon le filtre demandé.
/// 2. Pour chaque `.wast`, extrait les blocs `(module …)` top-level avec un
///    parseur de parenthèses minimal (pas de WAST complet).
/// 3. Si `wat2wasm` est disponible, compile chaque module extrait et le valide
///    via `new WebAssembly.Module(bytes)` sous Bun.
/// 4. Sinon, compte seulement les modules (mode "count-only").
pub fn run_testsuite(opts: &TestsuiteOpts) -> Result<()> {
    let spec_root = &opts.path;
    let core_dir = spec_root.join("test/core");

    if !core_dir.exists() {
        anyhow::bail!(
            "Répertoire `test/core` introuvable dans `{}`.\n\
             Cloner le repo WebAssembly/spec :\n\
             \n  git clone --depth 1 https://github.com/WebAssembly/spec {}\n",
            spec_root.display(),
            spec_root.display()
        );
    }

    let wast_files = parser::collect_wast_files(&core_dir, opts.filter.as_deref())?;
    if wast_files.is_empty() {
        if !opts.quiet {
            eprintln!(
                "[wasm spec testsuite] Aucun fichier .wast trouvé (filtre: {:?})",
                opts.filter
            );
        }
        return Ok(());
    }

    let has_wat2wasm = parser::which("wat2wasm");
    let has_bun = parser::which("bun");

    if !opts.quiet {
        let mode = match (has_wat2wasm.is_some(), has_bun.is_some()) {
            (true, true) => "validation Bun (wat2wasm + WebAssembly.Module)",
            (true, false) => "compile-only (wat2wasm, bun absent)",
            _ => "count-only (wat2wasm absent — installer wabt)",
        };
        eprintln!(
            "[wasm spec testsuite] {} fichiers .wast — mode: {}",
            wast_files.len(),
            mode
        );
    }

    let mut total_modules = 0usize;
    let mut total_passed = 0usize;
    let mut total_failed = 0usize;
    let mut total_skipped = 0usize;

    for wast_path in &wast_files {
        let result =
            parser::process_wast_file(wast_path, has_wat2wasm.as_deref(), has_bun.as_deref())?;
        total_modules += result.modules_found;
        total_passed += result.modules_passed;
        total_failed += result.modules_failed;
        if result.skipped {
            total_skipped += 1;
        }

        if !opts.quiet {
            let rel = wast_path
                .strip_prefix(&core_dir)
                .unwrap_or(wast_path.as_path());
            if result.skipped {
                println!("  {} {}", "SKIP".yellow(), rel.display());
            } else {
                let status = if result.modules_failed == 0 {
                    "OK".green()
                } else {
                    "FAIL".red()
                };
                println!(
                    "  {} {} ({} modules, {} passed, {} failed)",
                    status,
                    rel.display(),
                    result.modules_found,
                    result.modules_passed,
                    result.modules_failed
                );
            }
        }
    }

    // Résumé final
    if !opts.quiet {
        println!();
        println!(
            "Testsuite WebAssembly — {} fichiers .wast",
            wast_files.len()
        );
        println!("  modules trouvés  : {}", total_modules);
        if has_wat2wasm.is_some() {
            println!("  passed           : {}", total_passed.to_string().green());
            println!(
                "  failed           : {}",
                if total_failed == 0 {
                    total_failed.to_string().green()
                } else {
                    total_failed.to_string().red()
                }
            );
        }
        println!("  skipped (assert) : {}", total_skipped);
        if has_wat2wasm.is_none() {
            println!();
            println!(
                "  {} Pour l'exécution complète, installer wabt : apt install wabt",
                "TODO:".yellow()
            );
            println!(
                "  {} Support runtime Bun natif (sans wat2wasm) prévu en V2.",
                "TODO:".yellow()
            );
        }
    }

    Ok(())
}
