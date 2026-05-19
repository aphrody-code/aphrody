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

use n2b_types::types::{Finding, MakeFindingOpts, Severity};
use n2b_util::make_finding;

/// Scanner bunfig.toml : détecte les options dépréciées ou suspectes.
///
/// Références :
///   - https://bun.sh/docs/runtime/bunfig
///   - https://bun.sh/docs/install/bunfig
pub fn scan_bunfig(path: &str, content: &str) -> (Vec<Finding>, String) {
    let mut findings: Vec<Finding> = Vec::new();

    // `install.registry = "https://..."` avec URL npm historique → info
    if let Some(line) = content
        .lines()
        .find(|l| l.trim_start().starts_with("registry"))
    {
        if line.contains("registry.npmjs.org") && !line.contains("//registry") {
            findings.push(make_finding(
                path,
                &[],
                0,
                "bunfig/registry-npmjs",
                "registry pointe sur npmjs.org par défaut — inutile de le déclarer (gain de 0 ligne)"
                    .to_string(),
                line.to_string(),
                None,
                MakeFindingOpts {
                    autofix: Some(false),
                    severity: Some(Severity::Info),
                    ..Default::default()
                },
            ));
        }
    }

    // `telemetry = false` → note info, ok pour CI
    // `install.frozenLockfile = true` → ok, pas de finding

    // `[test].preload` vide alors qu'il y a `reflect-metadata` dans les deps
    // → traité par le scanner package_json, pas ici.

    // `[install.cache].disable = true` ou `.disableManifest` → info
    for (needle, msg) in [
        (
            "install.linker = \"isolated\"",
            "linker isolated : plus lent à l'install mais plus compatible pnpm — vérifier que tu en as besoin",
        ),
        (
            "install.saveTextLockfile = true",
            "saveTextLockfile = true : texte plus volumineux que bun.lock binaire — pertinent uniquement pour diffs lisibles",
        ),
    ] {
        if content.contains(needle) {
            findings.push(make_finding(
                path,
                &[],
                0,
                "bunfig/option-note",
                msg.to_string(),
                needle.to_string(),
                None,
                MakeFindingOpts {
                    autofix: Some(false),
                    severity: Some(Severity::Info),
                    ..Default::default()
                },
            ));
        }
    }

    // `jsxImportSource` présent → info, utile pour frameworks
    if content.contains("jsxImportSource") {
        // pas de warning, c'est legit
    }

    // Détecte les options Node-legacy qui ne s'appliquent pas
    for dead in ["loose =", "babelTargets ="] {
        if let Some(line) = content.lines().find(|l| l.contains(dead)) {
            findings.push(make_finding(
                path,
                &[],
                0,
                "bunfig/unknown-option",
                format!(
                    "option '{dead}' ignorée par Bun — probablement un vestige d'un autre outil"
                ),
                line.to_string(),
                None,
                MakeFindingOpts {
                    autofix: Some(false),
                    severity: Some(Severity::Warn),
                    ..Default::default()
                },
            ));
        }
    }

    (findings, content.to_string())
}
