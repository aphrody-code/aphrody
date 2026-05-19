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

//! Chargement et validation des `registry/*.toml` au premier accès via
//! `once_cell::Lazy`. Toute erreur de chargement → panic → échec immédiat
//! de `cargo test --workspace`.

use crate::schema::{
    ApiEntry, ApisFile, CliEntry, CliFile, GlobalEntry, GlobalsFile, ModuleEntry, ModulesFile,
    PackageEntry, PackagesFile,
};
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashSet;

const APIS_TOML: &str = include_str!("../registry/apis.toml");
const PACKAGES_TOML: &str = include_str!("../registry/packages.toml");
const MODULES_TOML: &str = include_str!("../registry/modules.toml");
const CLI_TOML: &str = include_str!("../registry/cli.toml");
const GLOBALS_TOML: &str = include_str!("../registry/globals.toml");

pub static APIS: Lazy<Vec<ApiEntry>> = Lazy::new(|| {
    let parsed: ApisFile = toml::from_str(APIS_TOML).expect("registry/apis.toml: TOML invalide");
    validate_unique_ids_and_patterns(parsed.apis.iter().map(|e| (&e.id, &e.pattern)), "apis.toml");
    for e in &parsed.apis {
        Regex::new(&e.pattern).unwrap_or_else(|err| {
            panic!("registry/apis.toml: regex invalide pour '{}': {err}", e.id)
        });
    }
    parsed.apis
});

pub static PACKAGES: Lazy<Vec<PackageEntry>> = Lazy::new(|| {
    let parsed: PackagesFile =
        toml::from_str(PACKAGES_TOML).expect("registry/packages.toml: TOML invalide");
    let mut seen = HashSet::new();
    for e in &parsed.packages {
        if !seen.insert(e.package.clone()) {
            panic!("registry/packages.toml: package en doublon: {}", e.package);
        }
    }
    parsed.packages
});

pub static MODULES: Lazy<Vec<ModuleEntry>> = Lazy::new(|| {
    let parsed: ModulesFile =
        toml::from_str(MODULES_TOML).expect("registry/modules.toml: TOML invalide");
    let mut seen = HashSet::new();
    for e in &parsed.modules {
        if !seen.insert(e.module.clone()) {
            panic!("registry/modules.toml: module en doublon: {}", e.module);
        }
    }
    parsed.modules
});

pub static CLI: Lazy<Vec<CliEntry>> = Lazy::new(|| {
    let parsed: CliFile = toml::from_str(CLI_TOML).expect("registry/cli.toml: TOML invalide");
    validate_unique_ids_and_patterns(parsed.cli.iter().map(|e| (&e.id, &e.pattern)), "cli.toml");
    for e in &parsed.cli {
        Regex::new(&e.pattern).unwrap_or_else(|err| {
            panic!("registry/cli.toml: regex invalide pour '{}': {err}", e.id)
        });
    }
    parsed.cli
});

pub static GLOBALS: Lazy<Vec<GlobalEntry>> = Lazy::new(|| {
    let parsed: GlobalsFile =
        toml::from_str(GLOBALS_TOML).expect("registry/globals.toml: TOML invalide");
    let mut seen = HashSet::new();
    for e in &parsed.globals {
        if !seen.insert(e.id.clone()) {
            panic!("registry/globals.toml: id en doublon: {}", e.id);
        }
    }
    parsed.globals
});

/// IDs uniques par couple (id, pattern) — autorise plusieurs entrées partageant
/// un Rule ID si elles correspondent à des patterns distincts (cas
/// `imports/bun-native` partagé par toutes les deps de `packages.toml`).
fn validate_unique_ids_and_patterns<'a, I>(entries: I, file: &str)
where
    I: Iterator<Item = (&'a String, &'a String)>,
{
    let mut seen = HashSet::new();
    for (id, pat) in entries {
        let key = format!("{id}|{pat}");
        if !seen.insert(key) {
            panic!("registry/{file}: doublon (id, pattern) pour: {id}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_loads_without_panic() {
        // Force chaque Lazy à se charger — toute erreur de validation panique ici.
        assert!(!APIS.is_empty(), "apis.toml ne doit pas être vide");
        assert!(!PACKAGES.is_empty(), "packages.toml ne doit pas être vide");
        assert!(!MODULES.is_empty(), "modules.toml ne doit pas être vide");
        assert!(!CLI.is_empty(), "cli.toml ne doit pas être vide");
        // globals.toml est intentionnellement vide en Phase 1 (rempli en Phase 4).
        let _ = &*GLOBALS;
    }

    #[test]
    fn apis_count_matches_baseline() {
        // Garde-fou Phase 1 : la transcription préserve le nombre d'entrées.
        // Source : crates/n2b-rules/src/bun_apis.rs RULES — 73 entrées
        // (incluant 2 next/* + 71 api/*).
        assert_eq!(APIS.len(), 73, "apis.toml a divergé de bun_apis.rs RULES");
    }

    #[test]
    fn packages_count_matches_baseline() {
        // Source : node_imports.rs BUN_REPLACEMENTS — 94 entrées.
        assert_eq!(
            PACKAGES.len(),
            94,
            "packages.toml a divergé de node_imports.rs BUN_REPLACEMENTS"
        );
    }

    #[test]
    fn modules_count_matches_baseline() {
        // Phase 4 : 56 modules = 53 historiques + 3 nouveaux Node v24
        // (sqlite, quic, sea). 42 top-level + 11 sub-paths + 3 v24.
        assert_eq!(
            MODULES.len(),
            56,
            "modules.toml a divergé du baseline (Phase 4 = 56 modules)"
        );
    }

    #[test]
    fn globals_phase4_populated() {
        // Phase 4 : globals.toml passe de 0 à 9 entrées (cf. plan/phase-4-couverture.md §4.5).
        assert_eq!(
            GLOBALS.len(),
            9,
            "globals.toml doit avoir 9 entrées Phase 4 — trouvé: {}",
            GLOBALS.len()
        );
    }

    #[test]
    fn cli_count_matches_baseline() {
        // Source : cli_commands.rs MAPPINGS — 47 entrées.
        assert_eq!(
            CLI.len(),
            47,
            "cli.toml a divergé de cli_commands.rs MAPPINGS"
        );
    }
}
