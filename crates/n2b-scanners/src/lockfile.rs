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

pub const RIVAL_LOCKFILES: &[&str] = &[
    "package-lock.json",
    "npm-shrinkwrap.json",
    "pnpm-lock.yaml",
    "yarn.lock",
];

pub fn check_lockfile(path: &str, name: &str) -> Option<Finding> {
    if !RIVAL_LOCKFILES.contains(&name) {
        return None;
    }
    Some(make_finding(
        path,
        &[],
        0,
        "lock/rival",
        format!(
            "lockfile concurrent '{name}' présent — exécuter 'bun install' puis supprimer ce fichier"
        ),
        name.to_string(),
        None,
        MakeFindingOpts {
            autofix: Some(false),
            severity: Some(Severity::Warn),
            ..Default::default()
        },
    ))
}
