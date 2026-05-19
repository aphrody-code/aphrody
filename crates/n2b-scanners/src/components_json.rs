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

//! Scanner components.json — config shadcn/ui (path aliases, style, tailwind).
//!
//! Refs :
//!   - https://ui.shadcn.com/docs/components-json
//!   - https://ui.shadcn.com/docs/registry/getting-started

use n2b_types::types::{Finding, MakeFindingOpts, Severity};
use n2b_util::make_finding;
use serde_json::Value;

pub fn scan_components_json(path: &str, content: &str) -> (Vec<Finding>, String) {
    let mut findings: Vec<Finding> = Vec::new();

    let parsed: Value = match serde_json::from_str(content) {
        Ok(v) => v,
        Err(_) => return (findings, content.to_string()),
    };

    // Détecte la présence, en émettant un info ecosystem
    findings.push(make_finding(
        path,
        &[],
        0,
        "ecosystem/shadcn",
        "components.json présent — shadcn/ui configuré (copy-paste components + CLI `bunx shadcn@latest add <component>`)",
        "components.json".to_string(),
        Some("https://ui.shadcn.com/".to_string()),
        MakeFindingOpts {
            autofix: Some(false),
            severity: Some(Severity::Info),
            ..Default::default()
        },
    ));

    // Vérifie que le tailwind config est renseigné
    if let Some(tw) = parsed.get("tailwind").and_then(|v| v.as_object()) {
        if let Some(css) = tw.get("css").and_then(|v| v.as_str()) {
            if !css.ends_with(".css") {
                findings.push(make_finding(
                    path,
                    &[],
                    0,
                    "shadcn/tailwind-css",
                    format!("tailwind.css='{css}' — doit pointer sur un .css avec les layers @tailwind base/components/utilities (ou @import 'tailwindcss' en v4)"),
                    css.to_string(),
                    None,
                    MakeFindingOpts {
                        autofix: Some(false),
                        severity: Some(Severity::Warn),
                        ..Default::default()
                    },
                ));
            }
        }
    }

    // Style : new-york ou default
    if let Some(style) = parsed.get("style").and_then(|v| v.as_str()) {
        if !matches!(style, "default" | "new-york") {
            findings.push(make_finding(
                path,
                &[],
                0,
                "shadcn/style-unknown",
                format!("style='{style}' — valeurs supportées : 'default' ou 'new-york'"),
                style.to_string(),
                None,
                MakeFindingOpts {
                    autofix: Some(false),
                    severity: Some(Severity::Info),
                    ..Default::default()
                },
            ));
        }
    }

    // Registry custom (v2 feature)
    if parsed.get("registries").is_some() {
        findings.push(make_finding(
            path,
            &[],
            0,
            "shadcn/custom-registry",
            "registries custom détecté (shadcn registry API) — voir https://ui.shadcn.com/docs/registry/getting-started",
            "registries".to_string(),
            None,
            MakeFindingOpts {
                autofix: Some(false),
                severity: Some(Severity::Info),
                ..Default::default()
            },
        ));
    }

    (findings, content.to_string())
}

pub fn is_components_json(name: &str) -> bool {
    name == "components.json"
}
