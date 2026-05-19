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

//! Scanner Procfile (Heroku, Foreman, Honcho) — détecte les commandes
//! `node script.js` et `npm`/`yarn`/`pnpm` qui doivent passer à `bun`.
//! Phase 4.

use n2b_rules::cli_commands::apply_cli_rules;
use n2b_types::types::Finding;

pub fn is_procfile(name: &str) -> bool {
    name == "Procfile" || name == "Procfile.dev" || name.starts_with("Procfile.")
}

pub fn scan_procfile(path: &str, content: &str) -> (Vec<Finding>, String) {
    // Délègue à apply_cli_rules — le format est shell-like : `web: <command>`.
    apply_cli_rules(path, content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_procfile_predicate() {
        assert!(is_procfile("Procfile"));
        assert!(is_procfile("Procfile.dev"));
        assert!(is_procfile("Procfile.production"));
        assert!(!is_procfile("procfile"));
        assert!(!is_procfile("Procfile-old"));
    }

    #[test]
    fn detects_npm_in_procfile() {
        let src = "web: npm start\nworker: node worker.js\n";
        let (findings, _) = scan_procfile("Procfile", src);
        // apply_cli_rules détecte npm start → bun start
        assert!(findings.iter().any(|f| f.rule_id.starts_with("cli/")));
    }
}
