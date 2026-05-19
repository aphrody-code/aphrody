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

//! Scanner `docker-compose.yml` / `compose.yaml` — détecte les images
//! `node:*` (suggère `oven/bun:*`) et les commandes `npm`/`yarn`/`pnpm`
//! en clé `command:` ou `entrypoint:`. Phase 4.

use n2b_rules::cli_commands::apply_cli_rules;
use n2b_types::types::{Finding, MakeFindingOpts, Severity};
use n2b_util::{line_offsets, make_finding};
use once_cell::sync::Lazy;
use regex::Regex;

/// `image: node:20-alpine` → suggère `oven/bun:1-alpine`.
static IMAGE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?m)^\s*image:\s*['"]?(?P<img>node:[^\s'"]+)"#)
        .expect("invariant: docker-compose IMAGE_RE valid")
});

pub fn is_docker_compose(name: &str) -> bool {
    matches!(
        name,
        "docker-compose.yml"
            | "docker-compose.yaml"
            | "compose.yml"
            | "compose.yaml"
            | "docker-compose.override.yml"
            | "docker-compose.override.yaml"
    )
}

pub fn scan_docker_compose(path: &str, content: &str) -> (Vec<Finding>, String) {
    let offsets = line_offsets(content);
    let mut findings: Vec<Finding> = Vec::new();

    for cap in IMAGE_RE.captures_iter(content) {
        if let Some(m) = cap.name("img") {
            let img = m.as_str();
            // Map node:<tag> → oven/bun:<short-tag>. Tag « 20-alpine » →
            // « 1-alpine » (Bun majeur 1.x). Tag pur version → « 1 ».
            let tag = img.strip_prefix("node:").unwrap_or("");
            let bun_tag = if tag.contains("-alpine") {
                "1-alpine"
            } else if tag.contains("-slim") {
                "1-slim"
            } else if tag.contains("-debian") {
                "1-debian"
            } else {
                "1"
            };
            let suggestion = format!("oven/bun:{bun_tag}");
            findings.push(make_finding(
                path,
                &offsets,
                m.start(),
                "docker-compose/node-image",
                format!(
                    "image '{img}' — passer à '{suggestion}' (Bun a son image officielle Alpine/Debian)"
                ),
                img.to_string(),
                Some(suggestion),
                MakeFindingOpts {
                    severity: Some(Severity::Warn),
                    autofix: Some(false),
                    aggressive: Some(true),
                    ..Default::default()
                },
            ));
        }
    }

    // Délègue aussi à apply_cli_rules pour `command:` / `entrypoint:`
    // contenant `npm install`, `yarn dev`, etc. Le scanner shell capture
    // les patterns inline.
    let (cli_findings, _) = apply_cli_rules(path, content);
    findings.extend(cli_findings);

    (findings, content.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_node_image_alpine() {
        let src = "services:\n  app:\n    image: node:20-alpine\n";
        let (findings, _) = scan_docker_compose("docker-compose.yml", src);
        assert!(
            findings
                .iter()
                .any(|f| f.rule_id == "docker-compose/node-image")
        );
        let f = findings
            .iter()
            .find(|f| f.rule_id == "docker-compose/node-image")
            .unwrap();
        assert_eq!(f.replacement.as_deref(), Some("oven/bun:1-alpine"));
    }

    #[test]
    fn detects_node_image_default_to_1() {
        let src = "services:\n  app:\n    image: 'node:20'\n";
        let (findings, _) = scan_docker_compose("compose.yaml", src);
        let f = findings
            .iter()
            .find(|f| f.rule_id == "docker-compose/node-image")
            .unwrap();
        assert_eq!(f.replacement.as_deref(), Some("oven/bun:1"));
    }

    #[test]
    fn ignores_non_node_image() {
        let src = "services:\n  db:\n    image: postgres:16\n";
        let (findings, _) = scan_docker_compose("docker-compose.yml", src);
        assert!(
            !findings
                .iter()
                .any(|f| f.rule_id == "docker-compose/node-image")
        );
    }

    #[test]
    fn is_docker_compose_predicate() {
        assert!(is_docker_compose("docker-compose.yml"));
        assert!(is_docker_compose("docker-compose.yaml"));
        assert!(is_docker_compose("compose.yml"));
        assert!(is_docker_compose("compose.yaml"));
        assert!(!is_docker_compose("Dockerfile"));
        assert!(!is_docker_compose("docker-compose.local.yml"));
    }
}
