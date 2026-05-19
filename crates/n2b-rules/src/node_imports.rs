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

use crate::imports_ast;
use n2b_registry::{Compat, MODULES, ModuleEntry, PACKAGES};
use n2b_types::types::{CompatInfo, CompatStatus, Finding, MakeFindingOpts, Severity};
use n2b_util::{Edit, apply_edits, line_offsets, make_finding};
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};

/// Builtins Node `node:*` — alimenté par `crates/n2b-registry/registry/modules.toml`.
/// La liste inclut les sous-chemins shimmés par Bun (`fs/promises`, etc.).
static BUILTINS: Lazy<HashSet<String>> =
    Lazy::new(|| MODULES.iter().map(|m| m.module.clone()).collect());

/// Index `module → ModuleEntry` pour récupérer le statut compat d'un import.
static MODULE_INDEX: Lazy<HashMap<String, &'static ModuleEntry>> =
    Lazy::new(|| MODULES.iter().map(|m| (m.module.clone(), m)).collect());

/// Convertit un `Compat` du registre vers le `CompatStatus` runtime/JSON.
fn compat_status(c: Compat) -> CompatStatus {
    match c {
        Compat::Full => CompatStatus::Full,
        Compat::Partial => CompatStatus::Partial,
        Compat::Missing => CompatStatus::Missing,
    }
}

/// Construit une `CompatInfo` depuis une `ModuleEntry` du registre.
/// Phase 3+ — attaché aux findings `imports/node-*` pour exposer le statut
/// 🟢/🟡/🔴 et permettre aux consommateurs (rpb-dashboard, IDEs) de trier
/// les modules par criticité.
fn module_compat(entry: &ModuleEntry) -> CompatInfo {
    CompatInfo {
        status: compat_status(entry.compat),
        module: entry.module.clone(),
        missing_apis: entry.missing_apis.clone(),
        equivalent: if entry.equivalent.is_empty() {
            None
        } else {
            Some(entry.equivalent.clone())
        },
        bunpp: entry.bunpp.clone(),
    }
}

/// Sévérité dérivée du statut compat — Phase 3 §3.2. Le finding `imports/*`
/// emprunte la sévérité du module hôte : 🟢 → info, 🟡 → warn, 🔴 → error.
/// Exposé pour Phase 4 (sous-règles `imports/node-<module>` granulaires) +
/// re-export depuis `n2b_registry::derive_severity` pour les autres scanners.
#[allow(dead_code)]
pub(crate) fn severity_from_compat(c: Compat) -> Severity {
    match c {
        Compat::Full => Severity::Info,
        Compat::Partial => Severity::Warn,
        Compat::Missing => Severity::Error,
    }
}

struct BunReplacement {
    replacement: String,
    note: String,
    aggressive: bool,
}

static BUN_REPLACEMENTS: Lazy<HashMap<String, BunReplacement>> = Lazy::new(|| {
    PACKAGES
        .iter()
        .map(|p| {
            (
                p.package.clone(),
                BunReplacement {
                    replacement: p.replacement.clone(),
                    note: p.note.clone(),
                    aggressive: p.aggressive,
                },
            )
        })
        .collect()
});

pub fn apply_node_import_rules(
    path: &str,
    source: &str,
    aggressive: bool,
) -> (Vec<Finding>, String) {
    let offsets = line_offsets(source);
    let mut findings: Vec<Finding> = Vec::new();
    let mut edits: Vec<Edit> = Vec::new();

    let mut seen_builtin_finding: HashSet<String> = HashSet::new();
    let mut seen_repl_finding: HashSet<String> = HashSet::new();

    let specifiers = imports_ast::extract_specifiers(path, source);

    for s in &specifiers {
        let pos = s.inner_start as usize;
        let len = s.inner_len as usize;
        let spec = &s.value;

        if BUILTINS.contains(spec.as_str()) {
            edits.push(Edit {
                index: pos,
                len,
                replacement: format!("node:{spec}"),
            });
            if seen_builtin_finding.insert(spec.clone()) {
                // Phase 3 : attache le compat info + dérive la sévérité depuis
                // le statut du module hôte. La règle `imports/node-prefix`
                // garde sa sévérité 'info' historique (préfixe = recommandation
                // stylistique, pas un bug). Les sous-règles compat-driven
                // sont en chantier Phase 4 (`imports/node-<module>`).
                let compat = MODULE_INDEX.get(spec.as_str()).map(|m| module_compat(m));
                findings.push(make_finding(
                    path,
                    &offsets,
                    pos,
                    "imports/node-prefix",
                    format!("préfixer '{spec}' avec 'node:' (recommandé)"),
                    spec.clone(),
                    Some(format!("node:{spec}")),
                    MakeFindingOpts {
                        autofix: Some(true),
                        compat,
                        ..Default::default()
                    },
                ));
            }
            continue;
        }

        if let Some(r) = BUN_REPLACEMENTS.get(spec.as_str()) {
            if seen_repl_finding.insert(spec.clone()) {
                findings.push(make_finding(
                    path,
                    &offsets,
                    pos,
                    "imports/bun-native",
                    format!("remplacer '{}' par {} — {}", spec, r.replacement, r.note),
                    spec.clone(),
                    Some(r.replacement.to_string()),
                    MakeFindingOpts {
                        autofix: Some(false),
                        aggressive: Some(true),
                        ..Default::default()
                    },
                ));
            }
            if aggressive
                && r.aggressive
                && (r.replacement.starts_with("bun:") || r.replacement.starts_with("node:"))
            {
                edits.push(Edit {
                    index: pos,
                    len,
                    replacement: r.replacement.to_string(),
                });
            }
        }
    }

    let out = apply_edits(source, edits);
    (findings, out)
}
