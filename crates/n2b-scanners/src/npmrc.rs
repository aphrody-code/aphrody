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

/// Scanner `.npmrc` / `.yarnrc` / `.yarnrc.yml` / `.pnpmrc`.
/// Ces fichiers configurent npm/pnpm/yarn. La plupart des directives
/// portent nativement dans `bunfig.toml`, certaines sont superflues.
pub fn scan_npmrc(path: &str, content: &str) -> (Vec<Finding>, String) {
    let mut findings: Vec<Finding> = Vec::new();

    for (idx, raw) in content.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        let lineno = (idx + 1) as u32;

        // registry=...
        if line.starts_with("registry=") || line.starts_with("registry ") {
            findings.push(mk(
                path,
                lineno,
                "npmrc/registry",
                "registry custom détecté — porter dans bunfig.toml : [install].registry = \"...\"",
                raw.to_string(),
                Some("[install]\nregistry = \"...\"".to_string()),
            ));
        }
        // //registry.../:_authToken=...
        else if line.contains(":_authToken=") || line.contains(":_auth=") {
            findings.push(mk(
                path,
                lineno,
                "npmrc/auth-token",
                "auth token détecté — porter dans bunfig.toml : [install.scopes] avec token ou variable d'env",
                raw.to_string(),
                None,
            ));
        }
        // @scope:registry=...
        else if line.starts_with('@') && line.contains(":registry=") {
            findings.push(mk(
                path,
                lineno,
                "npmrc/scoped-registry",
                "registry scopé détecté — porter dans bunfig.toml : [install.scopes]",
                raw.to_string(),
                None,
            ));
        }
        // always-auth=true (propre à npm, pas repris par Bun)
        else if line.starts_with("always-auth=") {
            findings.push(mk(
                path,
                lineno,
                "npmrc/always-auth",
                "'always-auth' est spécifique npm — Bun utilise le token directement quand présent",
                raw.to_string(),
                None,
            ));
        }
        // save-exact / save-prefix
        else if line.starts_with("save-exact=") || line.starts_with("save-prefix=") {
            findings.push(mk(
                path,
                lineno,
                "npmrc/save-prefix",
                "'save-exact' / 'save-prefix' porté par bunfig.toml : [install].exact ou .savePrefix",
                raw.to_string(),
                None,
            ));
        }
        // node-linker=isolated/hoisted (pnpm/yarn berry)
        else if line.starts_with("node-linker=") || line.starts_with("nodeLinker:") {
            findings.push(mk(
                path,
                lineno,
                "npmrc/node-linker",
                "'node-linker' (pnpm/yarn) → bunfig.toml : [install].linker = \"isolated\" | \"hoisted\"",
                raw.to_string(),
                None,
            ));
        }
        // engine-strict
        else if line.starts_with("engine-strict=") {
            findings.push(mk(
                path,
                lineno,
                "npmrc/engine-strict",
                "'engine-strict' : Bun lit engines.bun du package.json et avertit si non matchant",
                raw.to_string(),
                None,
            ));
        }
        // package-lock, shrinkwrap → obsolètes avec Bun
        else if line.starts_with("package-lock=")
            || line.starts_with("shrinkwrap=")
            || line.starts_with("lockfile=")
        {
            findings.push(mk(
                path,
                lineno,
                "npmrc/lockfile-flag",
                "option lockfile obsolète avec Bun (bun.lock est toujours généré sauf --no-save)",
                raw.to_string(),
                None,
            ));
        }
    }

    (findings, content.to_string())
}

pub fn is_rc_file(name: &str) -> bool {
    matches!(
        name,
        ".npmrc" | ".yarnrc" | ".yarnrc.yml" | ".pnpmrc" | "npmrc" | "yarnrc"
    )
}

fn mk(
    path: &str,
    line: u32,
    rule: &str,
    msg: &str,
    original: String,
    replacement: Option<String>,
) -> Finding {
    // make_finding attend offsets + index ; ici on passe un faux couple (le seul
    // besoin côté sortie est la ligne).
    let mut offsets = Vec::new();
    // offsets[i] = position du \n qui termine la ligne i+1.
    // Pour obtenir line = `line`, il faut offsets[line-2] + 1 = 0 quand line==1.
    for i in 1..line {
        offsets.push(i - 1);
    }
    make_finding(
        path,
        &offsets,
        if line == 0 { 0 } else { line as usize - 1 },
        rule,
        msg.to_string(),
        original,
        replacement,
        MakeFindingOpts {
            autofix: Some(false),
            severity: Some(Severity::Info),
            ..Default::default()
        },
    )
}
