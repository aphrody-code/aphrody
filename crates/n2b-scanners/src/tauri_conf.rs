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

//! Scanner tauri.conf.json (v2). Détecte les patterns d'intégration
//! frontend (Next.js, SvelteKit, etc.) et signale les frontendDist /
//! devUrl qui doivent être adaptés pour Bun.
//!
//! Refs :
//!   - https://v2.tauri.app/
//!   - https://v2.tauri.app/start/frontend/nextjs/

use n2b_types::types::{Finding, MakeFindingOpts, Severity};
use n2b_util::make_finding;
use serde_json::Value;

pub fn scan_tauri_conf(path: &str, content: &str) -> (Vec<Finding>, String) {
    let mut findings: Vec<Finding> = Vec::new();

    let parsed: Value = match serde_json::from_str(content) {
        Ok(v) => v,
        Err(_) => return (findings, content.to_string()),
    };

    // Détecte build.beforeDevCommand / beforeBuildCommand pour voir si bun est utilisé
    if let Some(build) = parsed.get("build").and_then(|v| v.as_object()) {
        for key in ["beforeDevCommand", "beforeBuildCommand"] {
            if let Some(cmd) = build.get(key).and_then(|v| v.as_str()) {
                // Si ça contient npm/pnpm/yarn alors qu'on migre vers Bun → info
                let uses_other_pm = cmd.contains("npm ")
                    || cmd.contains("pnpm ")
                    || cmd.contains("yarn ")
                    || cmd.starts_with("npx ");
                if uses_other_pm {
                    findings.push(make_finding(
                        path,
                        &[],
                        0,
                        "tauri/before-cmd-pm",
                        format!(
                            "build.{key}='{cmd}' utilise npm/pnpm/yarn/npx — porter vers 'bun run' ou 'bunx' pour le workflow Bun"
                        ),
                        cmd.to_string(),
                        None,
                        MakeFindingOpts {
                            autofix: Some(false),
                            severity: Some(Severity::Info),
                            ..Default::default()
                        },
                    ));
                }
            }
        }

        // frontendDist : Next.js static export produit 'out/', SvelteKit produit '.svelte-kit/'
        if let Some(dist) = build.get("frontendDist").and_then(|v| v.as_str()) {
            if dist == "../out" || dist == "./out" {
                findings.push(make_finding(
                    path,
                    &[],
                    0,
                    "tauri/frontend-dist-next-export",
                    "frontendDist pointe vers 'out' — compatible avec Next.js static export. Vérifier que next.config utilise output: 'export'",
                    dist.to_string(),
                    None,
                    MakeFindingOpts {
                        autofix: Some(false),
                        severity: Some(Severity::Info),
                        ..Default::default()
                    },
                ));
            }
        }
    }

    (findings, content.to_string())
}

pub fn is_tauri_conf(name: &str) -> bool {
    matches!(name, "tauri.conf.json" | "tauri.conf.json5")
}
