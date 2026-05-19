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

use n2b_rules::cli_commands::apply_cli_rules;
use n2b_types::types::{Finding, MakeFindingOpts};
use n2b_util::{line_offsets, make_finding};
use once_cell::sync::Lazy;
use regex::Regex;

static SETUP_NODE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"uses:\s*(?:actions/setup-node@[^\s\n]+)")
        .expect("invariant: SETUP_NODE_RE regex literal is valid")
});
static NODE_VERSION_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"node-version:\s*['"]?[^\n'"]+['"]?"#)
        .expect("invariant: NODE_VERSION_RE regex literal is valid")
});
static CACHE_PM_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"\n\s*cache:\s*['"]?(?:npm|yarn|pnpm)['"]?"#)
        .expect("invariant: CACHE_PM_RE regex literal is valid")
});

pub fn scan_workflow(path: &str, content: &str) -> (Vec<Finding>, String) {
    let mut findings: Vec<Finding> = Vec::new();
    let mut out = content.to_string();

    let offsets = line_offsets(content);
    for mat in SETUP_NODE_RE.find_iter(content) {
        findings.push(make_finding(
            path,
            &offsets,
            mat.start(),
            "ci/setup-node",
            "actions/setup-node → oven-sh/setup-bun@v2",
            mat.as_str().to_string(),
            Some("uses: oven-sh/setup-bun@v2".to_string()),
            MakeFindingOpts {
                autofix: Some(true),
                ..Default::default()
            },
        ));
    }
    out = SETUP_NODE_RE
        .replace_all(&out, "uses: oven-sh/setup-bun@v2")
        .into_owned();

    let offsets = line_offsets(&out);
    for mat in NODE_VERSION_RE.find_iter(&out) {
        findings.push(make_finding(
            path,
            &offsets,
            mat.start(),
            "ci/node-version",
            "remplacer 'node-version' par 'bun-version: latest'",
            mat.as_str().to_string(),
            Some("bun-version: latest".to_string()),
            MakeFindingOpts {
                autofix: Some(true),
                ..Default::default()
            },
        ));
    }
    out = NODE_VERSION_RE
        .replace_all(&out, "bun-version: latest")
        .into_owned();

    out = CACHE_PM_RE.replace_all(&out, "").into_owned();

    let (cli_f, cli_out) = apply_cli_rules(path, &out);
    findings.extend(cli_f);
    (findings, cli_out)
}
