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

use crate::shebang::scan_shebang;
use n2b_rules::{
    bun_apis::apply_bun_api_rules_with_imports, imports_ast::build_import_graph,
    node_imports::apply_node_import_rules,
};
use n2b_types::types::{Finding, Mode, RunOptions};

pub fn scan_source(path: &str, content: &str, opts: &RunOptions) -> (Vec<Finding>, String) {
    let mut all: Vec<Finding> = Vec::new();
    let aggressive = opts.mode == Mode::Aggressive;

    let (f, working) = scan_shebang(path, content);
    all.extend(f);
    let (f, working) = apply_node_import_rules(path, &working, aggressive);
    all.extend(f);
    // Phase 2 : graphe d'imports construit une fois (oxc parse partagé) puis
    // injecté dans le matching api/* pour résoudre PS1.
    let imports = build_import_graph(path, &working);
    let (f, working) = apply_bun_api_rules_with_imports(path, &working, aggressive, &imports);
    all.extend(f);

    if opts.mode == Mode::Check {
        return (all, content.to_string());
    }
    (all, working)
}
