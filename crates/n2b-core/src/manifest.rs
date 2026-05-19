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

//! Manifeste `n2b.json` — Phase 4 §4.7.
//!
//! Lecture + résolution + merge config. n2b cherche `n2b.json` en remontant
//! l'arbre depuis le `root` du scan, le valide contre le schéma, et applique
//! `mode`/`include`/`ignore`/`targets`/`rules` par dessus les défauts. La
//! précédence est : flags CLI > n2b.json > défauts.
//!
//! Volet **écriture** (`.n2b/state.json` après `--migrate`) : Phase 5 §5.6.

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Entrée déserialisée depuis `n2b.json` à la racine du projet.
///
/// Tous les champs sont optionnels — un manifeste minimal est `{}`. Le merge
/// avec les défauts vit dans `apply_to_run_options`.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct N2bManifest {
    #[serde(rename = "$schema", default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<ManifestMode>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub include: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ignore: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub targets: Vec<String>,

    /// Override par Rule ID. Valeurs : `"off"|"info"|"warn"|"error"` (string)
    /// ou `{ severity, autofix }` (object). Désactive ou re-priorise une règle.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub rules: HashMap<String, RuleOverride>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry: Option<LocalRegistry>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ManifestMode {
    Check,
    Fix,
    Aggressive,
    Migrate,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum RuleOverride {
    /// Forme courte : `"my-rule": "off"` ou `"warn"`.
    Severity(RuleSeverity),
    /// Forme longue : `"my-rule": { severity: "warn", autofix: true }`.
    Detailed(DetailedRuleOverride),
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RuleSeverity {
    Off,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DetailedRuleOverride {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub severity: Option<RuleSeverity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub autofix: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct LocalRegistry {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub packages: Vec<LocalPackageEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub apis: Vec<LocalApiEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct LocalPackageEntry {
    pub id: String,
    pub package: String,
    pub replacement: String,
    pub note: String,
    #[serde(default)]
    pub aggressive: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct LocalApiEntry {
    pub id: String,
    pub pattern: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub import_from: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub severity: Option<RuleSeverity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replace: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replacement: Option<String>,
    #[serde(default)]
    pub aggressive: bool,
}

/// Résultat de la résolution + chargement d'un manifeste.
#[derive(Debug, Clone)]
pub struct ResolvedManifest {
    pub path: PathBuf,
    pub manifest: N2bManifest,
}

/// Cherche `n2b.json` en remontant l'arbre depuis `start` (typiquement la
/// racine du scan). Retourne `Ok(None)` si rien trouvé jusqu'à la racine du FS.
///
/// Phase 4 — résolution conservatrice, ne traverse pas les boundaries de
/// container (s'arrête à `/`). N'examine PAS les sous-dossiers — c'est une
/// recherche *parent-first* comme `package.json` ou `tsconfig.json`.
pub fn find_manifest(start: &Path) -> Option<PathBuf> {
    let mut cur: Option<&Path> = Some(start);
    while let Some(dir) = cur {
        let candidate = dir.join("n2b.json");
        if candidate.is_file() {
            return Some(candidate);
        }
        cur = dir.parent();
    }
    None
}

/// Charge et désérialise un manifeste depuis `path`. Erreur explicite avec
/// numéro de ligne si JSON invalide.
pub fn load_manifest(path: &Path) -> Result<N2bManifest> {
    let bytes =
        std::fs::read(path).with_context(|| format!("lire le manifeste: {}", path.display()))?;
    let manifest: N2bManifest = serde_json::from_slice(&bytes).map_err(|e| {
        anyhow!(
            "manifeste {} invalide: {} (ligne {}, col {})",
            path.display(),
            e,
            e.line(),
            e.column()
        )
    })?;
    Ok(manifest)
}

/// Helper : résout + charge en une étape. Retourne `Ok(None)` si pas de
/// manifeste, `Err` si présent mais invalide.
pub fn resolve_and_load(start: &Path) -> Result<Option<ResolvedManifest>> {
    let Some(path) = find_manifest(start) else {
        return Ok(None);
    };
    let manifest = load_manifest(&path)?;
    Ok(Some(ResolvedManifest { path, manifest }))
}

/// Type alias pour la table d'overrides — exposé pour Arc<…> dans run.rs.
pub type RuleOverrideMap = HashMap<String, RuleOverride>;

/// Applique les overrides de règles à un set de FileFix. Phase 4 §4.7 :
/// `"off"` → drop le finding ; sévérité différente → ré-ajuste ; autofix
/// override → ré-ajuste. Ne touche pas aux findings sans override.
pub fn apply_rule_overrides(fixes: &mut [n2b_types::types::FileFix], overrides: &RuleOverrideMap) {
    use n2b_types::types::Severity;

    for fix in fixes.iter_mut() {
        fix.findings.retain_mut(|f| {
            let Some(over) = overrides.get(&f.rule_id) else {
                return true;
            };
            if over.is_off() {
                return false;
            }
            if let Some(sev) = over.effective_severity() {
                f.severity = match sev {
                    RuleSeverity::Info => Severity::Info,
                    RuleSeverity::Warn => Severity::Warn,
                    RuleSeverity::Error => Severity::Error,
                    RuleSeverity::Off => unreachable!("filtré ci-dessus"),
                };
            }
            if let Some(af) = over.autofix_override() {
                f.autofix = af;
                if !af {
                    f.replacement = None;
                }
            }
            true
        });
    }
}

impl RuleOverride {
    /// Sévérité effective de l'override. `Off` → la règle ne doit plus émettre
    /// de finding ; les autres re-priorise.
    pub fn effective_severity(&self) -> Option<RuleSeverity> {
        match self {
            RuleOverride::Severity(s) => Some(*s),
            RuleOverride::Detailed(d) => d.severity,
        }
    }

    pub fn autofix_override(&self) -> Option<bool> {
        match self {
            RuleOverride::Severity(_) => None,
            RuleOverride::Detailed(d) => d.autofix,
        }
    }

    /// True si la règle doit être supprimée (pas émettre de finding).
    pub fn is_off(&self) -> bool {
        self.effective_severity() == Some(RuleSeverity::Off)
    }
}

impl ManifestMode {
    /// Conversion vers le `Mode` runtime. `Migrate` n'a pas d'équivalent —
    /// c'est un raccourci pour `Aggressive` + side-effects (rollback Phase 5).
    pub fn to_run_mode(self) -> n2b_types::types::Mode {
        use n2b_types::types::Mode;
        match self {
            ManifestMode::Check => Mode::Check,
            ManifestMode::Fix => Mode::Fix,
            ManifestMode::Aggressive | ManifestMode::Migrate => Mode::Aggressive,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_manifest() {
        let m: N2bManifest = serde_json::from_str("{}").unwrap();
        assert!(m.mode.is_none());
        assert!(m.rules.is_empty());
    }

    #[test]
    fn parses_full_manifest() {
        let src = r#"{
            "$schema": "https://example.com/schema.json",
            "mode": "fix",
            "include": ["src/**"],
            "ignore": ["test/fixtures/**"],
            "rules": {
                "api/marked-call": "off",
                "imports/node-prefix": { "severity": "warn", "autofix": false }
            },
            "registry": {
                "packages": [
                    {
                        "id": "imports/internal-foo",
                        "package": "@my-org/foo",
                        "replacement": "@my-org/foo-bun",
                        "note": "polyfill interne"
                    }
                ]
            }
        }"#;
        let m: N2bManifest = serde_json::from_str(src).unwrap();
        assert_eq!(m.mode, Some(ManifestMode::Fix));
        assert_eq!(m.include, vec!["src/**".to_string()]);
        assert_eq!(m.rules.len(), 2);
        assert!(m.rules.get("api/marked-call").unwrap().is_off());
        assert_eq!(
            m.rules
                .get("imports/node-prefix")
                .unwrap()
                .effective_severity(),
            Some(RuleSeverity::Warn)
        );
        assert_eq!(
            m.rules
                .get("imports/node-prefix")
                .unwrap()
                .autofix_override(),
            Some(false)
        );
        assert_eq!(m.registry.unwrap().packages.len(), 1);
    }

    #[test]
    fn rejects_unknown_field() {
        let src = r#"{ "this_field_does_not_exist": true }"#;
        assert!(serde_json::from_str::<N2bManifest>(src).is_err());
    }

    #[test]
    fn manifest_mode_to_run_mode() {
        assert_eq!(
            ManifestMode::Check.to_run_mode(),
            n2b_types::types::Mode::Check
        );
        assert_eq!(
            ManifestMode::Migrate.to_run_mode(),
            n2b_types::types::Mode::Aggressive
        );
    }

    #[test]
    fn find_manifest_walks_up() {
        let tmp = std::env::temp_dir().join(format!("n2b-test-{}", std::process::id()));
        let nested = tmp.join("a/b/c");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(tmp.join("n2b.json"), "{}").unwrap();

        let found = find_manifest(&nested);
        assert_eq!(found.as_deref(), Some(tmp.join("n2b.json").as_path()));

        std::fs::remove_dir_all(&tmp).ok();
    }
}
