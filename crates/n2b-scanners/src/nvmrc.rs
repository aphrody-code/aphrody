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

/// `.nvmrc` / `.node-version` : advisory. Bun n'utilise pas ces fichiers.
pub fn scan_nvmrc(path: &str, content: &str) -> (Vec<Finding>, String) {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return (Vec::new(), content.to_string());
    }
    let finding = make_finding(
        path,
        &[],
        0,
        "env/nvmrc",
        format!(
            "`.nvmrc`/`.node-version` ({trimmed}) n'est pas utilisé par Bun — tu peux le conserver pour les devs qui restent sur Node, ou le supprimer si l'équipe migre complètement"
        ),
        trimmed.to_string(),
        None,
        MakeFindingOpts {
            autofix: Some(false),
            severity: Some(Severity::Info),
            ..Default::default()
        },
    );
    (vec![finding], content.to_string())
}
