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

// Scanner des hooks Husky (.husky/*) : remplace les commandes pnpm/npm/npx
// par leurs équivalents Bun. Règles : husky/pnpm-command, husky/npx-command.
use n2b_types::types::{Finding, MakeFindingOpts, Severity};
use n2b_util::{line_offsets, make_finding};
use once_cell::sync::Lazy;
use regex::Regex;

struct HuskyRule {
    id: &'static str,
    re: Regex,
    template: &'static str,
    message: &'static str,
}

static RULES: Lazy<Vec<HuskyRule>> = Lazy::new(|| {
    vec![
        HuskyRule {
            id: "husky/pnpm-command",
            re: Regex::new(r"(?m)^(\s*)pnpm\s+(?:run\s+)?([a-zA-Z0-9:_-]+)\b")
                .expect("invariant: pnpm-command regex literal is valid"),
            template: "${1}bun run ${2}",
            message: "hook husky utilise pnpm — remplacer par 'bun run'",
        },
        HuskyRule {
            id: "husky/npm-command",
            re: Regex::new(r"(?m)^(\s*)npm\s+(?:run\s+)?([a-zA-Z0-9:_-]+)\b")
                .expect("invariant: npm-command regex literal is valid"),
            template: "${1}bun run ${2}",
            message: "hook husky utilise npm — remplacer par 'bun run'",
        },
        HuskyRule {
            id: "husky/yarn-command",
            re: Regex::new(r"(?m)^(\s*)yarn\s+(?:run\s+)?([a-zA-Z0-9:_-]+)\b")
                .expect("invariant: yarn-command regex literal is valid"),
            template: "${1}bun run ${2}",
            message: "hook husky utilise yarn — remplacer par 'bun run'",
        },
        HuskyRule {
            id: "husky/npx-command",
            re: Regex::new(r"(?m)^(\s*)npx(?:\s+--no)?\s+(?:--\s+)?")
                .expect("invariant: npx-command regex literal is valid"),
            template: "${1}bunx --bun ",
            message: "hook husky utilise npx — remplacer par 'bunx --bun'",
        },
        HuskyRule {
            id: "husky/pnpm-dlx",
            re: Regex::new(r"(?m)^(\s*)pnpm\s+dlx\s+")
                .expect("invariant: pnpm-dlx regex literal is valid"),
            template: "${1}bunx --bun ",
            message: "hook husky utilise 'pnpm dlx' — remplacer par 'bunx --bun'",
        },
    ]
});

pub fn scan_husky(path: &str, content: &str) -> (Vec<Finding>, String) {
    let offsets = line_offsets(content);
    let mut findings: Vec<Finding> = Vec::new();
    struct Edit {
        index: usize,
        len: usize,
        replacement: String,
    }
    let mut edits: Vec<Edit> = Vec::new();

    for r in RULES.iter() {
        for mat in r.re.captures_iter(content) {
            let whole = mat
                .get(0)
                .expect("invariant: capture group 0 is always present in a match");
            let index = whole.start();
            let original = whole.as_str().to_string();
            let mut replacement = String::new();
            mat.expand(r.template, &mut replacement);
            findings.push(make_finding(
                path,
                &offsets,
                index,
                r.id,
                r.message,
                original.clone(),
                Some(replacement.clone()),
                MakeFindingOpts {
                    autofix: Some(true),
                    severity: Some(Severity::Warn),
                    ..Default::default()
                },
            ));
            edits.push(Edit {
                index,
                len: original.len(),
                replacement,
            });
        }
    }

    let mut out = content.to_string();
    if !edits.is_empty() {
        edits.sort_unstable_by_key(|e| std::cmp::Reverse(e.index));
        for e in edits {
            out.replace_range(e.index..e.index + e.len, &e.replacement);
        }
    }
    (findings, out)
}

/// Détecte si `rel` est un hook husky (.husky/<nom> sans extension).
pub fn is_husky_hook(rel: &str) -> bool {
    let normalized = rel.replace('\\', "/");
    normalized.starts_with(".husky/")
        && !normalized.starts_with(".husky/_/")
        && !normalized.ends_with(".sh")
}
