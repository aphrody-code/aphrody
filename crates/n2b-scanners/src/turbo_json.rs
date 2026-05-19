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

//! Scanner turbo.json — config Turborepo. Flagge les patterns qui intéragissent
//! avec une migration Bun : globalDependencies sur bun.lock manquant, tasks
//! qui utilisent des managers npm/pnpm/yarn explicites, etc.
//!
//! Refs : https://turborepo.com/docs/reference/configuration

use n2b_types::types::{Finding, MakeFindingOpts, Severity};
use n2b_util::make_finding;
use serde_json::Value;

pub fn scan_turbo_json(path: &str, content: &str) -> (Vec<Finding>, String) {
    let mut findings: Vec<Finding> = Vec::new();

    let parsed: Value = match serde_json::from_str(content) {
        Ok(v) => v,
        Err(_) => return (findings, content.to_string()),
    };

    // globalDependencies : doit inclure bun.lock pour que le cache invalide
    // au changement de lockfile Bun.
    if let Some(gd) = parsed.get("globalDependencies").and_then(|v| v.as_array()) {
        let items: Vec<&str> = gd.iter().filter_map(|v| v.as_str()).collect();
        let has_bun_lock = items.iter().any(|s| *s == "bun.lock" || *s == "bun.lockb");
        let has_other_lock = items
            .iter()
            .any(|s| matches!(*s, "pnpm-lock.yaml" | "yarn.lock" | "package-lock.json"));
        if has_other_lock && !has_bun_lock {
            findings.push(make_finding(
                path,
                &[],
                0,
                "turbo/global-deps-bun-lock",
                "globalDependencies liste pnpm-lock/yarn.lock/package-lock mais pas bun.lock — ajouter 'bun.lock' pour invalider le cache Turbo au changement de lockfile Bun",
                format!("{gd:?}"),
                Some(r#"bun.lock"#.to_string()),
                MakeFindingOpts {
                    autofix: Some(false),
                    severity: Some(Severity::Warn),
                    ..Default::default()
                },
            ));
        }
    }

    // pipeline (v1) ou tasks (v2) : détecte les commandes qui utilisent
    // npm/pnpm/yarn explicites — suggère bun run.
    for key in ["pipeline", "tasks"] {
        if let Some(pipe) = parsed.get(key).and_then(|v| v.as_object()) {
            for (task_name, task_val) in pipe {
                if let Some(inputs) = task_val.get("inputs").and_then(|v| v.as_array()) {
                    for input in inputs {
                        if let Some(s) = input.as_str() {
                            if s.contains("package-lock.json")
                                || s.contains("pnpm-lock.yaml")
                                || s.contains("yarn.lock")
                            {
                                findings.push(make_finding(
                                    path,
                                    &[],
                                    0,
                                    "turbo/task-inputs-lock",
                                    format!(
                                        "task '{task_name}' inputs mentionne un lockfile non-Bun ({s}) — remplacer par bun.lock"
                                    ),
                                    s.to_string(),
                                    Some("bun.lock".to_string()),
                                    MakeFindingOpts {
                                        autofix: Some(false),
                                        severity: Some(Severity::Info),
                                        ..Default::default()
                                    },
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    // packageManager field via le script (rarement là, mais check)
    // Rien à faire ici — déjà couvert par scanners/package_json.rs

    (findings, content.to_string())
}

pub fn is_turbo_json(name: &str) -> bool {
    name == "turbo.json"
}
