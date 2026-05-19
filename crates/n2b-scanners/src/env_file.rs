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

//! Scanner `.env` / `.env.*` — détecte les vars Node-spécifiques qui
//! n'ont pas de sens sous Bun (NODE_ENV, NODE_OPTIONS).
//!
//! Bun lit nativement `.env` (autoload) — pas besoin de `dotenv`.
//! NODE_ENV reste lu par compat (production/development), mais Bun ne
//! consulte PAS NODE_OPTIONS (--max-old-space-size, --inspect, etc.).
//!
//! Phase 4 — `n2b-scanners`.

use n2b_types::types::{Finding, MakeFindingOpts, Severity};
use n2b_util::{line_offsets, make_finding};

/// Vars connues comme Node-only (pas d'effet sous Bun).
const NODE_ONLY_VARS: &[(&str, &str)] = &[
    (
        "NODE_OPTIONS",
        "Bun n'interprète pas NODE_OPTIONS (--max-old-space-size, --inspect, etc.) — passer les flags équivalents à `bun --` (ex: --inspect=URL, --smol, --max-http-header-size)",
    ),
    (
        "NODE_PATH",
        "Bun ignore NODE_PATH (utilise sa propre résolution module) — supprimer ou utiliser un alias dans bunfig.toml",
    ),
    (
        "NODE_NO_WARNINGS",
        "Bun n'émet pas les warnings que ce flag silence — la variable est inerte",
    ),
    (
        "NODE_TLS_REJECT_UNAUTHORIZED",
        "Bun lit cette variable (compat Node) mais préférer `tls.rejectUnauthorized` au cas-par-cas",
    ),
    (
        "NPM_TOKEN",
        "Bun lit `BUN_AUTH_TOKEN` ou `.npmrc` — NPM_TOKEN reste fonctionnel pour compat",
    ),
];

pub fn is_env_file(name: &str) -> bool {
    name == ".env" || name == ".env.local" || name.starts_with(".env.")
}

pub fn scan_env_file(path: &str, content: &str) -> (Vec<Finding>, String) {
    let offsets = line_offsets(content);
    let mut findings: Vec<Finding> = Vec::new();

    for (line_idx, line) in content.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some(eq) = trimmed.find('=') else {
            continue;
        };
        let key = trimmed[..eq].trim();
        for (var, msg) in NODE_ONLY_VARS {
            if key == *var {
                // Compute byte offset of the line + key inside the source.
                let line_start = offsets
                    .iter()
                    .position(|&o| o as usize >= byte_pos(content, line_idx))
                    .map(|i| {
                        if i == 0 {
                            0
                        } else {
                            offsets[i - 1] as usize + 1
                        }
                    })
                    .unwrap_or_else(|| byte_pos(content, line_idx));
                let key_pos = line_start + line.find(key).unwrap_or(0);
                findings.push(make_finding(
                    path,
                    &offsets,
                    key_pos,
                    "env/node-only-var",
                    msg.to_string(),
                    key.to_string(),
                    None,
                    MakeFindingOpts {
                        severity: Some(Severity::Warn),
                        autofix: Some(false),
                        ..Default::default()
                    },
                ));
                break;
            }
        }
    }

    (findings, content.to_string())
}

/// Renvoie le byte-offset du début de la ligne `line_idx` (0-based) — sans
/// reconstruire toute la table d'offsets. Utilisé en interne par le scanner.
fn byte_pos(content: &str, line_idx: usize) -> usize {
    let mut pos = 0usize;
    for (i, l) in content.split_inclusive('\n').enumerate() {
        if i == line_idx {
            return pos;
        }
        pos += l.len();
    }
    pos
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_node_options() {
        let src = "NODE_OPTIONS=--max-old-space-size=4096\nFOO=bar\n";
        let (findings, _) = scan_env_file(".env", src);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "env/node-only-var");
        assert!(
            findings[0]
                .message
                .contains("Bun n'interprète pas NODE_OPTIONS")
        );
    }

    #[test]
    fn ignores_comments_and_other_vars() {
        let src = "# NODE_OPTIONS=ignored\nNODE_ENV=production\nDB_URL=postgres://...\n";
        let (findings, _) = scan_env_file(".env", src);
        assert_eq!(findings.len(), 0);
    }

    #[test]
    fn detects_node_path() {
        let src = "NODE_PATH=/usr/local/lib/node_modules\n";
        let (findings, _) = scan_env_file(".env.production", src);
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn is_env_file_predicate() {
        assert!(is_env_file(".env"));
        assert!(is_env_file(".env.local"));
        assert!(is_env_file(".env.production"));
        assert!(is_env_file(".env.test"));
        assert!(!is_env_file(".env-backup"));
        assert!(!is_env_file("env"));
        assert!(!is_env_file("envrc"));
    }
}
