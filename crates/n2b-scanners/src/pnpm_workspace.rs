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

// Scanner de pnpm-workspace.yaml — signale que Bun utilise le champ
// `"workspaces"` du package.json racine + `trustedDependencies` au lieu de
// `onlyBuiltDependencies`. Règles : workspace/pnpm-yaml, workspace/only-built-deps.
use n2b_types::types::{Finding, MakeFindingOpts, Severity};
use n2b_util::make_finding;
use serde_yaml::Value;

pub struct PnpmWorkspaceInfo {
    pub packages: Vec<String>,
    pub only_built: Vec<String>,
}

pub fn parse_pnpm_workspace(content: &str) -> Option<PnpmWorkspaceInfo> {
    let v: Value = serde_yaml::from_str(content).ok()?;
    let map = v.as_mapping()?;
    let packages = map
        .get(Value::String("packages".into()))
        .and_then(|x| x.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let only_built = map
        .get(Value::String("onlyBuiltDependencies".into()))
        .and_then(|x| x.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    Some(PnpmWorkspaceInfo {
        packages,
        only_built,
    })
}

pub fn scan_pnpm_workspace(path: &str, content: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let info = match parse_pnpm_workspace(content) {
        Some(i) => i,
        None => return findings,
    };

    findings.push(make_finding(
        path,
        &[],
        0,
        "workspace/pnpm-yaml",
        format!(
            "pnpm-workspace.yaml présent ({} patterns) — Bun lit \"workspaces\" dans le package.json racine. Migrer et supprimer ce fichier.",
            info.packages.len()
        ),
        "pnpm-workspace.yaml".to_string(),
        Some(format!(
            r#""workspaces": {}"#,
            serde_json::to_string(&info.packages).unwrap_or_default()
        )),
        MakeFindingOpts {
            autofix: Some(false),
            severity: Some(Severity::Warn),
            ..Default::default()
        },
    ));

    if !info.only_built.is_empty() {
        findings.push(make_finding(
            path,
            &[],
            0,
            "workspace/only-built-deps",
            format!(
                "onlyBuiltDependencies ({}) → transférer vers \"trustedDependencies\" dans le package.json racine",
                info.only_built.len()
            ),
            "onlyBuiltDependencies".to_string(),
            Some(format!(
                r#""trustedDependencies": {}"#,
                serde_json::to_string(&info.only_built).unwrap_or_default()
            )),
            MakeFindingOpts {
                autofix: Some(false),
                severity: Some(Severity::Warn),
                ..Default::default()
            },
        ));
    }

    findings
}
