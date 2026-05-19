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

/// Helpers pour invoquer Bun depuis un process enfant.
///
/// Toutes les fonctions utilisent `std::process::Command` et capturent
/// stderr pour construire des messages d'erreur riches.
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

// ---------------------------------------------------------------------------
// Commandes Bun
// ---------------------------------------------------------------------------

/// Retourne la version de `bun` installé (ex. `"1.1.20"`).
/// Renvoie une erreur si `bun` est absent du PATH.
#[allow(dead_code)] // preexisting: planned for future version-check command
pub fn version() -> Result<String> {
    let out = std::process::Command::new("bun")
        .arg("--version")
        .output()
        .context("impossible de lancer `bun` — est-il installé et dans PATH ?")?;
    if !out.status.success() {
        bail!(
            "`bun --version` a échoué (exit {}):\n{}",
            out.status,
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

/// Lance `bun install` dans `cwd`.
/// Capture stdout/stderr ; renvoie une erreur riche si le processus échoue.
pub fn install(cwd: &Path) -> Result<()> {
    let out = std::process::Command::new("bun")
        .arg("install")
        .current_dir(cwd)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .context("impossible de lancer `bun install`")?;
    if !out.status.success() {
        bail!(
            "`bun install` a échoué (exit {}) dans {}:\n{}",
            out.status,
            cwd.display(),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(())
}

/// Phase 6 §6.3 : appelle `n2b bunpp scaffold node-<module>` dans `cwd`.
/// Crée des fichiers visibles (`@bun++/node-<module>.ts` dans le projet).
/// Sous BackupGuard côté appelant.
pub fn bunpp_scaffold(cwd: &Path, module: &str) -> Result<()> {
    // n2b se ré-invoque lui-même — chemin du binaire courant.
    let exe = std::env::current_exe().context("current_exe")?;
    let out = std::process::Command::new(&exe)
        .args(["bunpp", "scaffold", &format!("node-{module}")])
        .current_dir(cwd)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .with_context(|| format!("impossible de lancer `n2b bunpp scaffold node-{module}`"))?;
    if !out.status.success() {
        bail!(
            "`n2b bunpp scaffold node-{module}` a échoué (exit {}) dans {}:\n{}",
            out.status,
            cwd.display(),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(())
}

/// Lance `bun add -d <pkg>` dans `cwd`.
/// Capture stdout/stderr ; renvoie une erreur riche si le processus échoue.
pub fn add_dev(cwd: &Path, pkg: &str) -> Result<()> {
    let out = std::process::Command::new("bun")
        .args(["add", "-d", pkg])
        .current_dir(cwd)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .with_context(|| format!("impossible de lancer `bun add -d {pkg}`"))?;
    if !out.status.success() {
        bail!(
            "`bun add -d {pkg}` a échoué (exit {}) dans {}:\n{}",
            out.status,
            cwd.display(),
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// BackupGuard — rollback transactionnel
// ---------------------------------------------------------------------------

/// Garde transactionnel : sauvegarde des fichiers sous `<path>.n2b-bak` et
/// permet de les restaurer en cas d'échec ou de supprimer les backups en
/// cas de succès.
///
/// L'implémentation de `Drop` assure une restauration automatique si
/// `commit()` n'a pas été appelé (protection contre panics / early-return).
pub struct BackupGuard {
    /// (original, backup) — backup = `<original>.n2b-bak`.
    backups: Vec<(PathBuf, PathBuf)>,
    /// Passe à `true` après `commit()` ; évite la restauration dans `Drop`.
    committed: bool,
}

impl BackupGuard {
    /// Crée un nouveau garde vide.
    pub fn new() -> Self {
        Self {
            backups: Vec::new(),
            committed: false,
        }
    }

    /// Copie `path` vers `<path>.n2b-bak` et enregistre la paire pour
    /// restauration potentielle.
    ///
    /// Si `path` n'existe pas, l'appel est un no-op (on ne peut pas
    /// sauvegarder ce qui n'existe pas, donc pas de restauration prévue).
    pub fn backup(&mut self, path: &Path) -> Result<()> {
        if !path.exists() {
            return Ok(());
        }
        let bak = path.with_extension(match path.extension() {
            Some(ext) => format!("{}.n2b-bak", ext.to_string_lossy()),
            None => "n2b-bak".to_owned(),
        });
        std::fs::copy(path, &bak).with_context(|| {
            format!(
                "impossible de sauvegarder {} → {}",
                path.display(),
                bak.display()
            )
        })?;
        self.backups.push((path.to_owned(), bak));
        Ok(())
    }

    /// Restaure tous les fichiers sauvegardés depuis leurs `.n2b-bak`.
    /// Consume self (utilisé en cas d'erreur explicite).
    pub fn restore_all(mut self) {
        self.do_restore();
        self.committed = true; // empêche Drop de restaurer une seconde fois
    }

    /// Supprime tous les fichiers `.n2b-bak` (succès confirmé).
    /// Consume self.
    pub fn commit(mut self) -> Result<()> {
        for (_, bak) in &self.backups {
            if bak.exists() {
                std::fs::remove_file(bak).with_context(|| {
                    format!("impossible de supprimer le backup {}", bak.display())
                })?;
            }
        }
        self.committed = true;
        Ok(())
    }

    fn do_restore(&self) {
        for (orig, bak) in &self.backups {
            if bak.exists() {
                if let Err(e) = std::fs::copy(bak, orig) {
                    eprintln!(
                        "[migrate] impossible de restaurer {} depuis {}: {e}",
                        orig.display(),
                        bak.display()
                    );
                } else if let Err(e) = std::fs::remove_file(bak) {
                    eprintln!(
                        "[migrate] impossible de supprimer le backup {}: {e}",
                        bak.display()
                    );
                }
            }
        }
    }
}

impl Default for BackupGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for BackupGuard {
    fn drop(&mut self) {
        if !self.committed {
            // panic ou early-return sans commit → restaurer.
            self.do_restore();
        }
    }
}
