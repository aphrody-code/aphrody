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

//! Scanner `jest.config.*` / `vitest.config.*` / `webpack.config.*` /
//! `babel.config.*` / `.mocharc.*` — détecte les configs d'outils qui
//! ont un équivalent Bun natif.
//!
//! - jest/vitest → `bun test` (Bun.test API compatible Jest)
//! - mocha → `bun test`
//! - webpack/rollup → `bun build`
//! - babel → `bun` (transpile TS/JSX natif, pas de babel nécessaire)
//!
//! Phase 4.

use n2b_types::types::{Finding, MakeFindingOpts, Severity};
use n2b_util::{line_offsets, make_finding};

pub fn is_js_config(name: &str) -> bool {
    classify(name).is_some()
}

fn classify(name: &str) -> Option<JsConfigKind> {
    // Strip extension to normalize jest.config.{js,mjs,ts,cjs,json}.
    let base = name.rsplit_once('.').map(|(a, _)| a).unwrap_or(name);
    if name == ".mocharc" || name.starts_with(".mocharc.") {
        return Some(JsConfigKind::Mocha);
    }
    match base {
        "jest.config" => Some(JsConfigKind::Jest),
        "vitest.config" => Some(JsConfigKind::Vitest),
        "webpack.config" => Some(JsConfigKind::Webpack),
        "rollup.config" => Some(JsConfigKind::Rollup),
        "babel.config" => Some(JsConfigKind::Babel),
        ".babelrc" => Some(JsConfigKind::Babel),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy)]
enum JsConfigKind {
    Jest,
    Vitest,
    Webpack,
    Rollup,
    Babel,
    Mocha,
}

impl JsConfigKind {
    fn rule_id(&self) -> &'static str {
        match self {
            Self::Jest => "js-config/jest",
            Self::Vitest => "js-config/vitest",
            Self::Webpack => "js-config/webpack",
            Self::Rollup => "js-config/rollup",
            Self::Babel => "js-config/babel",
            Self::Mocha => "js-config/mocha",
        }
    }
    fn message(&self) -> &'static str {
        match self {
            Self::Jest => {
                "Bun.test API est compatible Jest (`describe`/`it`/`expect`) — supprimer jest.config + `jest` et utiliser `bun test`"
            }
            Self::Vitest => {
                "Bun.test couvre les usages courants Vitest — supprimer vitest.config + `vitest` et utiliser `bun test` (mocks via `mock.module()`)"
            }
            Self::Webpack => {
                "Bun.build remplace webpack pour la plupart des cas (esbuild-like, plugins n2b-compatibles)"
            }
            Self::Rollup => "Bun.build remplace rollup (bundling ESM + tree-shaking natif)",
            Self::Babel => {
                "Bun transpile TS/JSX/decorators nativement — supprimer babel.config / .babelrc"
            }
            Self::Mocha => "Bun.test remplace mocha — supprimer .mocharc",
        }
    }
}

pub fn scan_js_config(path: &str, content: &str) -> (Vec<Finding>, String) {
    let name = path.rsplit('/').next().unwrap_or(path);
    let Some(kind) = classify(name) else {
        return (Vec::new(), content.to_string());
    };
    let offsets = line_offsets(content);
    let findings = vec![make_finding(
        path,
        &offsets,
        0,
        kind.rule_id(),
        kind.message().to_string(),
        name.to_string(),
        None,
        MakeFindingOpts {
            severity: Some(Severity::Info),
            autofix: Some(false),
            ..Default::default()
        },
    )];
    (findings, content.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_jest_config_variants() {
        assert!(is_js_config("jest.config.js"));
        assert!(is_js_config("jest.config.ts"));
        assert!(is_js_config("jest.config.mjs"));
        assert!(is_js_config("jest.config.json"));
    }

    #[test]
    fn classifies_vitest_config() {
        assert!(is_js_config("vitest.config.ts"));
    }

    #[test]
    fn classifies_babel() {
        assert!(is_js_config("babel.config.js"));
        assert!(is_js_config(".babelrc.js"));
    }

    #[test]
    fn classifies_mocharc() {
        assert!(is_js_config(".mocharc.js"));
        assert!(is_js_config(".mocharc.yml"));
        assert!(is_js_config(".mocharc.json"));
    }

    #[test]
    fn rejects_unrelated() {
        assert!(!is_js_config("tsconfig.json"));
        assert!(!is_js_config("package.json"));
        assert!(!is_js_config("server.js"));
    }

    #[test]
    fn emits_jest_finding() {
        let (findings, _) = scan_js_config("jest.config.js", "module.exports = {};");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "js-config/jest");
    }

    #[test]
    fn emits_vitest_finding() {
        let (findings, _) = scan_js_config("vitest.config.ts", "");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "js-config/vitest");
    }
}
