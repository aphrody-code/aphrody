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

use n2b_registry::CLI;
use n2b_types::types::{Finding, MakeFindingOpts};
use n2b_util::{Edit, apply_edits, line_offsets, make_finding};
use once_cell::sync::Lazy;
use regex::Regex;

/// Vue runtime d'une `CliEntry` du registre — la regex est compilée une
/// fois au premier accès via `Lazy`. Champ `respect_comments` honoré dans
/// `apply_cli_rules` (déjà couvert par PS4 — détection ET édition partagent
/// le même filtre `COMMENT_PREFIX`).
struct Mapping {
    re: Regex,
    replace: String,
    rule_id: String,
    message: String,
}

static MAPPINGS: Lazy<Vec<Mapping>> = Lazy::new(|| {
    CLI.iter()
        .map(|e| Mapping {
            re: Regex::new(&e.pattern).unwrap_or_else(|err| {
                panic!(
                    "invariant: cli_commands mapping pattern '{}' is invalid: {}",
                    e.pattern, err
                )
            }),
            replace: e.replace.clone(),
            rule_id: e.id.clone(),
            message: e.message.clone(),
        })
        .collect()
});

static COMMENT_PREFIX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\s*(#|//)").expect("invariant: COMMENT_PREFIX regex literal is valid")
});

/// Applique chaque mapping en séquence, en ignorant les lignes commentées.
/// **Détection et édition partagent le même filtre `COMMENT_PREFIX`** —
/// avant ce fix, l'édition utilisait `re.replace_all` global et réécrivait
/// les lignes commentées que la détection ignorait (PS4).
pub fn apply_cli_rules(path: &str, source: &str) -> (Vec<Finding>, String) {
    let mut out = source.to_string();
    let mut findings: Vec<Finding> = Vec::new();
    let mut offsets = line_offsets(&out);
    let mut offsets_stale = false;

    for rule in MAPPINGS.iter() {
        if offsets_stale {
            offsets = line_offsets(&out);
            offsets_stale = false;
        }

        let mut edits: Vec<Edit> = Vec::new();
        for mat in rule.re.find_iter(&out) {
            let line_start = out[..mat.start()].rfind('\n').map(|p| p + 1).unwrap_or(0);
            let line_end = out[mat.start()..]
                .find('\n')
                .map(|p| mat.start() + p)
                .unwrap_or(out.len());
            let line = &out[line_start..line_end];
            if COMMENT_PREFIX.is_match(line) {
                continue;
            }

            let text = mat.as_str().to_string();
            // Rejoue la regex sur le hit pour conserver les back-refs ($1, $2).
            let rewritten = rule.re.replace(&text, rule.replace.as_str()).to_string();
            findings.push(make_finding(
                path,
                &offsets,
                mat.start(),
                &rule.rule_id,
                rule.message.clone(),
                text.clone(),
                Some(rewritten.clone()),
                MakeFindingOpts {
                    autofix: Some(true),
                    ..Default::default()
                },
            ));
            edits.push(Edit {
                index: mat.start(),
                len: text.len(),
                replacement: rewritten,
            });
        }

        if !edits.is_empty() {
            out = apply_edits(&out, edits);
            offsets_stale = true;
        }
    }

    (findings, out)
}
