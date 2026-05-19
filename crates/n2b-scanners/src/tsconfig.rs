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
use serde_json::Value;

/// Vérifie les options tsconfig qui gagnent à être ajustées pour Bun :
///   - `types` doit inclure `"bun"` pour que `Bun.*` soit typé
///   - `moduleResolution: "bundler"` recommandé (match Bun / Next 16)
pub fn scan_tsconfig(path: &str, content: &str) -> (Vec<Finding>, String) {
    let mut findings: Vec<Finding> = Vec::new();

    // tsconfig accepte les commentaires JSON5 — parse best-effort.
    let parsed: Value = match serde_json::from_str(content) {
        Ok(v) => v,
        Err(_) => return (findings, content.to_string()),
    };
    let co = match parsed.get("compilerOptions").and_then(|v| v.as_object()) {
        Some(c) => c,
        None => return (findings, content.to_string()),
    };

    // types : ajouter "bun" si @types/bun est attendu
    if let Some(types) = co.get("types").and_then(|v| v.as_array()) {
        let has_bun = types.iter().any(|t| t.as_str() == Some("bun"));
        let has_node = types.iter().any(|t| t.as_str() == Some("node"));
        if has_node && !has_bun {
            findings.push(make_finding(
                path,
                &[],
                0,
                "tsconfig/bun-types",
                "compilerOptions.types inclut 'node' mais pas 'bun' — ajouter 'bun' pour typer Bun.*",
                format!("{:?}", types),
                None,
                MakeFindingOpts {
                    autofix: Some(false),
                    severity: Some(Severity::Info),
                    ..Default::default()
                },
            ));
        }
    }

    // moduleResolution : flag si classic/node (pas bundler)
    if let Some(mr) = co.get("moduleResolution").and_then(|v| v.as_str()) {
        let lower = mr.to_ascii_lowercase();
        if lower == "node" || lower == "classic" {
            findings.push(make_finding(
                path,
                &[],
                0,
                "tsconfig/module-resolution",
                format!(
                    "moduleResolution='{mr}' — 'bundler' ou 'nodenext' offre une meilleure compat Bun/Next (ESM first)"
                ),
                mr.to_string(),
                None,
                MakeFindingOpts {
                    autofix: Some(false),
                    severity: Some(Severity::Info),
                    ..Default::default()
                },
            ));
        }
    }

    // module : Bun préfère ESNext / Preserve (avec moduleResolution=bundler)
    if let Some(module) = co.get("module").and_then(|v| v.as_str()) {
        let lower = module.to_ascii_lowercase();
        if matches!(
            lower.as_str(),
            "commonjs" | "amd" | "umd" | "system" | "none" | "es6" | "es2015"
        ) {
            findings.push(make_finding(
                path,
                &[],
                0,
                "tsconfig/module-legacy",
                format!(
                    "module='{module}' — 'ESNext' ou 'Preserve' est recommandé pour Bun (ESM natif)"
                ),
                module.to_string(),
                None,
                MakeFindingOpts {
                    autofix: Some(false),
                    severity: Some(Severity::Info),
                    ..Default::default()
                },
            ));
        }
    }

    // target : Bun supporte ESNext, cibles < ES2022 introduisent des down-compilations inutiles
    if let Some(target) = co.get("target").and_then(|v| v.as_str()) {
        let lower = target.to_ascii_lowercase();
        let legacy = matches!(
            lower.as_str(),
            "es3"
                | "es5"
                | "es6"
                | "es2015"
                | "es2016"
                | "es2017"
                | "es2018"
                | "es2019"
                | "es2020"
                | "es2021"
        );
        if legacy {
            findings.push(make_finding(
                path,
                &[],
                0,
                "tsconfig/target-legacy",
                format!("target='{target}' — Bun supporte ESNext/ES2022+, downlevel inutile"),
                target.to_string(),
                None,
                MakeFindingOpts {
                    autofix: Some(false),
                    severity: Some(Severity::Info),
                    ..Default::default()
                },
            ));
        }
    }

    // lib : si dom présent mais pas dom.iterable → note (côté Bun runtime DOM n'est pas utile)
    // Pas une règle n2b prioritaire — skip.

    // moduleDetection : 'force' recommandé quand tous les fichiers sont ESM
    if co.get("moduleDetection").is_none() {
        findings.push(make_finding(
            path,
            &[],
            0,
            "tsconfig/module-detection",
            "compilerOptions.moduleDetection absent — 'force' garantit que chaque fichier est ESM (évite les .js traités comme CJS)"
                .to_string(),
            "moduleDetection".to_string(),
            Some("\"force\"".to_string()),
            MakeFindingOpts {
                autofix: Some(false),
                severity: Some(Severity::Info),
                ..Default::default()
            },
        ));
    }

    // verbatimModuleSyntax : recommandé avec moduleResolution=bundler + isolatedModules
    if co
        .get("moduleResolution")
        .and_then(|v| v.as_str())
        .map(|s| s.eq_ignore_ascii_case("bundler"))
        .unwrap_or(false)
        && co.get("verbatimModuleSyntax").and_then(|v| v.as_bool()) != Some(true)
    {
        findings.push(make_finding(
            path,
            &[],
            0,
            "tsconfig/verbatim-module-syntax",
            "moduleResolution=bundler + verbatimModuleSyntax=true est le combo recommandé Bun (force `import type` explicite)"
                .to_string(),
            "verbatimModuleSyntax".to_string(),
            Some("true".to_string()),
            MakeFindingOpts {
                autofix: Some(false),
                severity: Some(Severity::Info),
                ..Default::default()
            },
        ));
    }

    // allowImportingTsExtensions : si moduleResolution=bundler, on peut l'activer
    // pour importer directement des .ts (Bun resolve les .ts sans transform step).
    if co
        .get("moduleResolution")
        .and_then(|v| v.as_str())
        .map(|s| s.eq_ignore_ascii_case("bundler"))
        .unwrap_or(false)
        && co
            .get("allowImportingTsExtensions")
            .and_then(|v| v.as_bool())
            != Some(true)
    {
        findings.push(make_finding(
            path,
            &[],
            0,
            "tsconfig/allow-ts-extensions",
            "Bun résout les extensions .ts nativement — allowImportingTsExtensions=true permet `import './x.ts'`"
                .to_string(),
            "allowImportingTsExtensions".to_string(),
            Some("true".to_string()),
            MakeFindingOpts {
                autofix: Some(false),
                severity: Some(Severity::Info),
                ..Default::default()
            },
        ));
    }

    // noEmit : avec moduleResolution=bundler, noEmit=true est la config "types only"
    // recommandée (Bun fait le build, tsc valide juste les types).
    if co
        .get("moduleResolution")
        .and_then(|v| v.as_str())
        .map(|s| s.eq_ignore_ascii_case("bundler"))
        .unwrap_or(false)
        && co.get("noEmit").and_then(|v| v.as_bool()) != Some(true)
    {
        findings.push(make_finding(
            path,
            &[],
            0,
            "tsconfig/no-emit",
            "moduleResolution=bundler typiquement couplé à noEmit=true (Bun émet le JS, tsc ne fait que le type-check)"
                .to_string(),
            "noEmit".to_string(),
            Some("true".to_string()),
            MakeFindingOpts {
                autofix: Some(false),
                severity: Some(Severity::Info),
                ..Default::default()
            },
        ));
    }

    // @types/node présent et @types/bun présent : @types/bun suffit (Bun fournit aussi les types Node)
    if let Some(types) = co.get("types").and_then(|v| v.as_array()) {
        let has_node = types.iter().any(|t| t.as_str() == Some("node"));
        let has_bun = types.iter().any(|t| t.as_str() == Some("bun"));
        if has_bun && has_node {
            findings.push(make_finding(
                path,
                &[],
                0,
                "tsconfig/duplicate-node-types",
                "compilerOptions.types = ['bun', 'node'] — '@types/bun' inclut déjà les types Node, 'node' est redondant"
                    .to_string(),
                "types".to_string(),
                Some("[\"bun\"]".to_string()),
                MakeFindingOpts {
                    autofix: Some(false),
                    severity: Some(Severity::Info),
                    ..Default::default()
                },
            ));
        }
    }

    (findings, content.to_string())
}
