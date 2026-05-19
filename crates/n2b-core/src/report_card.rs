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

//! Migration report card — Phase 5 §5.4.
//!
//! Produit l'objet `report_card` exposé par `n2b --migrate --report=json` :
//! `auto_migratable_pct`, `total_findings`, `auto_migrated`, `manual_residue`.
//! Cible : rendre le pilier 2 *mesurable* — un trou de couverture devient
//! un chiffre, pas un silence.
//!
//! Persistance entre runs : `.n2b/state.json` (cf. §5.6).

use n2b_types::types::{FileFix, Severity};
use serde::{Deserialize, Serialize};

/// Carte synthétique de fin de migration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportCard {
    /// Ratio findings auto-applicables sur findings non-info.
    pub auto_migratable_pct: f64,
    pub total_findings: usize,
    /// Findings non-info (warn + error). Dénominateur du pct.
    pub blocking_findings: usize,
    /// Findings effectivement réécrits (autofix appliqué et fichier touché).
    pub auto_migrated: usize,
    /// Liste des findings manuels — chaque résidu est explicité.
    pub manual_residue: Vec<ManualResidueEntry>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualResidueEntry {
    pub rule_id: String,
    pub file: String,
    pub line: u32,
    pub reason: String,
    /// Suggestion concrète : recette codemod, polyfill bunpp, ou message.
    pub suggestion: String,
}

/// Calcule la carte depuis les findings finaux.
///
/// Heuristique :
/// - `auto_migrated` = findings dont `replacement` est `Some` ET le fichier
///   a effectivement changé (`fix.before != fix.after`).
/// - `manual_residue` = findings warn/error sans replacement OU avec un
///   replacement non appliqué (mode != aggressive). Le `reason` est
///   construit depuis le `compat` (si 🔴 → polyfill bunpp).
pub fn build(fixes: &[FileFix]) -> ReportCard {
    let mut total = 0usize;
    let mut blocking = 0usize;
    let mut migrated = 0usize;
    let mut residue: Vec<ManualResidueEntry> = Vec::new();

    for fix in fixes {
        let file_changed = fix.before != fix.after;
        for f in &fix.findings {
            total += 1;
            if f.severity == Severity::Info {
                continue;
            }
            blocking += 1;
            // Considéré "auto-migrated" si le replacement existe ET que le
            // fichier global a effectivement changé. Approximation : pas de
            // tracking par-finding du fait que telle édition est appliquée.
            if f.replacement.is_some() && f.autofix && file_changed {
                migrated += 1;
                continue;
            }
            // Sinon → résidu manuel.
            let (reason, suggestion) = derive_residue_explanation(f);
            residue.push(ManualResidueEntry {
                rule_id: f.rule_id.clone(),
                file: fix.file.clone(),
                line: f.line,
                reason,
                suggestion,
            });
        }
    }

    let auto_migratable_pct = if blocking == 0 {
        1.0
    } else {
        migrated as f64 / blocking as f64
    };

    ReportCard {
        auto_migratable_pct,
        total_findings: total,
        blocking_findings: blocking,
        auto_migrated: migrated,
        manual_residue: residue,
        timestamp: now_iso8601(),
    }
}

fn derive_residue_explanation(f: &n2b_types::types::Finding) -> (String, String) {
    // Si `compat` indique 🔴 et `bunpp` est défini → suggérer le polyfill.
    if let Some(c) = &f.compat {
        use n2b_types::types::CompatStatus;
        if c.status == CompatStatus::Missing {
            if let Some(bunpp) = &c.bunpp {
                return (
                    format!("module {} (compat: missing)", c.module),
                    format!(
                        "scaffold polyfill : `bunx n2b bunpp scaffold {}` (puis `import` depuis le polyfill)",
                        bunpp.trim_start_matches("@bun++/")
                    ),
                );
            }
            return (
                format!("module {} (compat: missing)", c.module),
                "pas de polyfill disponible — réécriture manuelle requise".to_string(),
            );
        }
        if c.status == CompatStatus::Partial && !c.missing_apis.is_empty() {
            return (
                format!(
                    "module {} (compat: partial — manque {})",
                    c.module,
                    c.missing_apis.join(", ")
                ),
                "vérifier que les sous-APIs utilisées sont dans la liste supportée".to_string(),
            );
        }
    }
    // Cas par défaut : pas mécanisable, suggérer la lecture du message.
    (
        if f.replacement.is_none() {
            "pas de réécriture mécanique disponible".to_string()
        } else {
            "réécriture non appliquée (mode != aggressive)".to_string()
        },
        f.message.clone(),
    )
}

fn now_iso8601() -> String {
    use std::time::SystemTime;
    let secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Format YYYY-MM-DDTHH:MM:SSZ — minimaliste, sans dépendance chrono.
    format_unix_secs(secs)
}

fn format_unix_secs(mut secs: u64) -> String {
    // Algorithme Howard Hinnant (civil_from_days) — tronqué pour rester
    // simple. Précis post-1970.
    let day = (secs / 86400) as i64;
    secs %= 86400;
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    let (y, mo, d) = civil_from_days(day);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (y + (m <= 2) as i64, m, d)
}

// ---------------------------------------------------------------------------
// Persistance .n2b/state.json (Phase 5 §5.6)
// ---------------------------------------------------------------------------

/// État persisté entre runs `--migrate`. Réutilise le report card +
/// quelques champs de tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct N2bState {
    pub status: StateStatus,
    pub last_run: String,
    pub n2b_version: String,
    pub auto_migratable_pct: f64,
    pub manual_residue: Vec<ManualResidueEntry>,
    pub migrated_files: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StateStatus {
    /// Au moins un finding manuel reste — la migration n'est pas terminée.
    InProgress,
    /// `manual_residue` est vide — la cible est complètement migrée.
    Complete,
}

impl N2bState {
    pub fn from_card(card: &ReportCard, fixes: &[FileFix]) -> Self {
        let migrated_files: Vec<String> = fixes
            .iter()
            .filter(|f| f.before != f.after)
            .map(|f| f.file.clone())
            .collect();
        let status = if card.manual_residue.is_empty() {
            StateStatus::Complete
        } else {
            StateStatus::InProgress
        };
        Self {
            status,
            last_run: card.timestamp.clone(),
            n2b_version: env!("CARGO_PKG_VERSION").to_string(),
            auto_migratable_pct: card.auto_migratable_pct,
            manual_residue: card.manual_residue.clone(),
            migrated_files,
        }
    }

    /// Persiste sur disque sous `<root>/.n2b/state.json`. Crée le dossier
    /// si absent. Suggère d'ajouter `.n2b/` au `.gitignore` (Phase 6).
    pub fn write_to(&self, root: &std::path::Path) -> std::io::Result<()> {
        let dir = root.join(".n2b");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("state.json");
        let json = serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string());
        std::fs::write(path, json)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use n2b_types::types::{Finding, Severity};

    fn finding(rule: &str, sev: Severity, repl: Option<&str>, autofix: bool) -> Finding {
        Finding {
            file: "x.ts".into(),
            line: 1,
            col: 1,
            rule_id: rule.into(),
            severity: sev,
            message: "msg".into(),
            original: "orig".into(),
            replacement: repl.map(String::from),
            autofix,
            aggressive: None,
            compat: None,
        }
    }

    fn fix(findings: Vec<Finding>, changed: bool) -> FileFix {
        FileFix {
            file: "x.ts".into(),
            before: "before".into(),
            after: if changed {
                "after".into()
            } else {
                "before".into()
            },
            findings,
        }
    }

    #[test]
    fn pct_is_one_when_no_blocking_findings() {
        let card = build(&[fix(
            vec![finding("api/x", Severity::Info, None, false)],
            false,
        )]);
        assert_eq!(card.auto_migratable_pct, 1.0);
        assert_eq!(card.blocking_findings, 0);
        assert_eq!(card.total_findings, 1);
    }

    #[test]
    fn pct_counts_only_warn_and_error() {
        let card = build(&[fix(
            vec![
                finding("api/a", Severity::Warn, Some("BunX"), true),
                finding("api/b", Severity::Error, None, false),
                finding("api/c", Severity::Info, None, false),
            ],
            true,
        )]);
        // 1 migrated / 2 blocking = 0.5
        assert_eq!(card.blocking_findings, 2);
        assert_eq!(card.auto_migrated, 1);
        assert!((card.auto_migratable_pct - 0.5).abs() < 1e-9);
        assert_eq!(card.manual_residue.len(), 1);
        assert_eq!(card.manual_residue[0].rule_id, "api/b");
    }

    #[test]
    fn state_complete_when_no_residue() {
        let card = build(&[fix(
            vec![finding("api/a", Severity::Warn, Some("BunX"), true)],
            true,
        )]);
        let state = N2bState::from_card(
            &card,
            &[fix(
                vec![finding("api/a", Severity::Warn, Some("BunX"), true)],
                true,
            )],
        );
        assert_eq!(state.status, StateStatus::Complete);
        assert_eq!(state.migrated_files, vec!["x.ts".to_string()]);
    }
}
