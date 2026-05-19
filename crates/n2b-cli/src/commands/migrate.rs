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

/// Applique les side-effects du mode `--migrate` :
///  1. Migre pnpm-workspace.yaml → `workspaces` + `trustedDependencies` dans package.json
///  2. Retire pnpm-lock.yaml / yarn.lock / package-lock.json
///  3. Exécute `bun install` pour reconstruire bun.lock
///  4. Ajoute `@types/bun` en devDep si source utilise `Bun.*`
///
/// Utilise `BackupGuard` pour un rollback transactionnel : si `bun install`
/// échoue, les fichiers modifiés sont restaurés depuis leurs `.n2b-bak`.
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::subprocess::bun::{self, BackupGuard};
use n2b_core::types::FileFix;

/// Wrapper rétrocompatible (sans options Phase 6) — conservé pour les
/// appelants externes qui ne consomment pas encore `MigrateOpts`.
#[allow(dead_code)]
pub fn run_migrate_side_effects(root: &Path, fixes: &[FileFix], quiet: bool) -> Result<()> {
    run_migrate_side_effects_with_opts(root, fixes, quiet, MigrateOpts::default())
}

#[derive(Debug, Clone, Default)]
pub struct MigrateOpts {
    /// Phase 6 §6.3 : appelle `bunpp scaffold <module>` pour les modules 🔴
    /// trouvés dans les findings. Crée des fichiers visibles — opt-in.
    pub scaffold_polyfills: bool,
}

pub fn run_migrate_side_effects_with_opts(
    root: &Path,
    fixes: &[FileFix],
    quiet: bool,
    opts: MigrateOpts,
) -> Result<()> {
    let log = |msg: &str| {
        if !quiet {
            eprintln!("[migrate] {msg}");
        }
    };

    let pnpm_ws = root.join("pnpm-workspace.yaml");
    let root_pkg = root.join("package.json");

    // Collect rival lockfiles that exist.
    let rivals: Vec<PathBuf> = n2b_core::scanners::lockfile::RIVAL_LOCKFILES
        .iter()
        .map(|name| root.join(name))
        .filter(|p| p.exists())
        .collect();

    // --- BackupGuard setup ---
    let mut guard = BackupGuard::new();
    guard.backup(&root_pkg)?;
    guard.backup(&pnpm_ws)?;
    for rival in &rivals {
        guard.backup(rival)?;
    }

    // 1. pnpm-workspace.yaml → package.json
    if pnpm_ws.exists() && root_pkg.exists() {
        let pnpm_content = std::fs::read_to_string(&pnpm_ws)?;
        if let Some(info) = n2b_core::scanners::pnpm_workspace::parse_pnpm_workspace(&pnpm_content)
        {
            let pkg_content = std::fs::read_to_string(&root_pkg)?;
            let mut pkg: serde_json::Value = serde_json::from_str(&pkg_content)?;
            let mut mutated = false;
            if pkg.get("workspaces").is_none() && !info.packages.is_empty() {
                pkg["workspaces"] = serde_json::json!(info.packages);
                mutated = true;
                log("  + workspaces ajouté dans package.json");
            }
            if pkg.get("trustedDependencies").is_none() && !info.only_built.is_empty() {
                pkg["trustedDependencies"] = serde_json::json!(info.only_built);
                mutated = true;
                log("  + trustedDependencies ajouté dans package.json");
            }
            if mutated {
                let mut out = serde_json::to_string_pretty(&pkg)?;
                if pkg_content.ends_with('\n') && !out.ends_with('\n') {
                    out.push('\n');
                }
                std::fs::write(&root_pkg, out)?;
            }
            std::fs::remove_file(&pnpm_ws)?;
            log("  - pnpm-workspace.yaml supprimé");
        }
    }

    // 2. Retire les lockfiles concurrents
    for rival in &rivals {
        let name = rival.file_name().unwrap_or_default().to_string_lossy();
        std::fs::remove_file(rival)?;
        log(&format!("  - {name} supprimé"));
    }

    // 3. bun install (reconstruit bun.lock)
    log("  → bun install");
    match bun::install(root) {
        Ok(()) => {
            log("  ✓ bun install OK");
        }
        Err(e) => {
            guard.restore_all();
            return Err(e).context("bun install a échoué ; fichiers restaurés depuis .n2b-bak");
        }
    }

    // 4. @types/bun si usage Bun.* détecté et absent des deps
    let uses_bun_api = fixes
        .iter()
        .any(|f| f.file.ends_with(".ts") || f.file.ends_with(".tsx") || f.file.ends_with(".mts"))
        && fixes.iter().any(|f| {
            f.after.contains("Bun.") || f.findings.iter().any(|x| x.rule_id.starts_with("api/"))
        });
    if uses_bun_api {
        if let Ok(pkg_content) = std::fs::read_to_string(&root_pkg) {
            if let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&pkg_content) {
                let has_types = pkg
                    .get("devDependencies")
                    .and_then(|d| d.get("@types/bun"))
                    .is_some()
                    || pkg
                        .get("dependencies")
                        .and_then(|d| d.get("@types/bun"))
                        .is_some();
                if !has_types {
                    log("  → bun add -d @types/bun");
                    if let Err(e) = bun::add_dev(root, "@types/bun") {
                        log(&format!("  ✗ bun add -d @types/bun: {e}"));
                    } else {
                        log("  ✓ @types/bun ajouté");
                    }
                }
            }
        }
    }

    // 5. Phase 6 §6.3 : scaffold polyfills bunpp pour les modules 🔴
    //    Opt-in (--scaffold-polyfills). Sous BackupGuard global.
    if opts.scaffold_polyfills {
        let mut scaffolded: std::collections::HashSet<String> = std::collections::HashSet::new();
        for fix in fixes {
            for f in &fix.findings {
                if let Some(c) = &f.compat {
                    use n2b_core::types::CompatStatus;
                    if c.status == CompatStatus::Missing {
                        if let Some(bunpp) = &c.bunpp {
                            let module = bunpp.trim_start_matches("@bun++/node-").to_string();
                            if scaffolded.insert(module.clone()) {
                                log(&format!("  → bunpp scaffold node-{module}"));
                                if let Err(e) = bun::bunpp_scaffold(root, &module) {
                                    log(&format!("  ✗ bunpp scaffold node-{module}: {e}"));
                                } else {
                                    log(&format!("  ✓ polyfill {bunpp} scaffolded"));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    guard.commit()?;
    Ok(())
}
