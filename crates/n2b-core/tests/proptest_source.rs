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

use n2b_core::scanners::source::scan_source;
use n2b_core::types::{Mode, Report, RunOptions};
use proptest::prelude::*;
use std::path::PathBuf;

fn opts() -> RunOptions {
    RunOptions {
        root: PathBuf::from("."),
        mode: Mode::Check,
        report: Report::Text,
        quiet: true,
        ignore: Vec::new(),
        agent: false,
        dry_run: false,
    }
}

proptest! {
    #[test]
    fn scanner_never_panics_on_arbitrary_source(s in "[\\x20-\\x7E\\n\\t]{0,4096}") {
        // The scanner must either produce findings or return cleanly.
        // It must never panic on arbitrary printable ASCII + whitespace input.
        let _ = scan_source("test.ts", &s, &opts());
    }
}

// Phase 2 — PS1 : la résolution AST garantit qu'une fonction locale dont
// le nom coïncide avec une API connue (marked, v4, exec, …) ne déclenche
// AUCUN finding `api/*`. Ces noms sont génériques et apparaissent
// fréquemment dans du code Node sans aucun lien avec leur dépendance
// d'origine.

/// Helper : true si findings ne contient aucune règle api/* ciblant `from`.
fn has_no_api_finding_from(findings: &[n2b_core::types::Finding], hint: &str) -> bool {
    !findings
        .iter()
        .any(|f| f.rule_id.starts_with("api/") && f.message.contains(hint))
}

#[test]
fn local_function_named_marked_does_not_trigger_api_marked() {
    let src = r##"
        // Pas d'import de 'marked'.
        function marked(input) { return input.toUpperCase(); }
        function marked_parse(s) { return s; }
        const result = marked("hello");
        const parsed = marked.parse("# title");
    "##;
    let (findings, _) = scan_source("ast_test.ts", src, &opts());
    assert!(
        has_no_api_finding_from(&findings, "Bun.markdown"),
        "fonction locale 'marked' ne doit pas déclencher api/marked-* — trouvé: {:?}",
        findings
            .iter()
            .filter(|f| f.rule_id.starts_with("api/marked"))
            .collect::<Vec<_>>()
    );
}

#[test]
fn imported_marked_does_trigger_api_marked() {
    let src = r##"
        import { marked } from 'marked';
        const result = marked("# hello");
    "##;
    let (findings, _) = scan_source("ast_test.ts", src, &opts());
    assert!(
        findings.iter().any(|f| f.rule_id == "api/marked-call"),
        "marked importé depuis 'marked' DOIT déclencher api/marked-call — findings: {:?}",
        findings.iter().map(|f| &f.rule_id).collect::<Vec<_>>()
    );
}

#[test]
fn local_v4_function_does_not_trigger_uuid_v4() {
    let src = r##"
        // Pas d'import de 'uuid'. v4 est juste une fonction locale.
        function v4() { return Math.random(); }
        const id = v4();
        const id2 = uuidv4();  // homonyme aussi non importé
    "##;
    let (findings, _) = scan_source("ast_test.ts", src, &opts());
    assert!(
        !findings.iter().any(|f| f.rule_id == "api/uuid-v4"),
        "v4() local sans import 'uuid' ne doit pas déclencher api/uuid-v4"
    );
}

#[test]
fn local_method_named_exec_does_not_trigger_child_process_exec() {
    let src = r##"
        // Méthode async exec sur une classe — pas l'exec de child_process.
        class Shell {
            async exec(cmd) {
                return await this.run(cmd);
            }
        }
        const re = /pattern/;
        re.exec("input");  // RegExp.exec, pas child_process
    "##;
    let (findings, _) = scan_source("ast_test.ts", src, &opts());
    assert!(
        !findings.iter().any(|f| f.rule_id == "api/exec"),
        "exec() méthode/RegExp sans import 'child_process' ne doit pas déclencher api/exec — findings: {:?}",
        findings
            .iter()
            .filter(|f| f.rule_id == "api/exec")
            .collect::<Vec<_>>()
    );
}

#[test]
fn explicit_child_process_exec_still_triggers() {
    let src = r##"
        const cp = require('child_process');
        cp.exec("ls -la");
        // Préfixe explicite — le pattern reconnaît `child_process.exec(`.
        child_process.exec("pwd");
    "##;
    let (findings, _) = scan_source("ast_test.ts", src, &opts());
    let exec_count = findings.iter().filter(|f| f.rule_id == "api/exec").count();
    assert!(
        exec_count >= 1,
        "child_process.exec() DOIT déclencher api/exec (au moins 1×) — trouvé: {exec_count}",
    );
}

#[test]
fn require_destructured_exec_resolves() {
    let src = r##"
        const { exec } = require('child_process');
        exec("ls");
    "##;
    let (findings, _) = scan_source("ast_test.ts", src, &opts());
    assert!(
        findings.iter().any(|f| f.rule_id == "api/exec"),
        "exec destructuré depuis require('child_process') DOIT déclencher api/exec — findings: {:?}",
        findings.iter().map(|f| &f.rule_id).collect::<Vec<_>>()
    );
}

#[test]
fn local_chalk_object_does_not_trigger_chalk_call() {
    let src = r##"
        // Pas d'import de 'chalk' — c'est juste un objet local.
        const chalk = { red: (s) => `[red]${s}[/red]` };
        console.log(chalk.red("hello"));
    "##;
    let (findings, _) = scan_source("ast_test.ts", src, &opts());
    assert!(
        !findings.iter().any(|f| f.rule_id == "api/chalk-call"),
        "chalk objet local sans import ne doit pas déclencher api/chalk-call"
    );
}

#[test]
fn imported_chalk_does_trigger() {
    let src = r##"
        import chalk from 'chalk';
        console.log(chalk.red("error"));
    "##;
    let (findings, _) = scan_source("ast_test.ts", src, &opts());
    assert!(
        findings.iter().any(|f| f.rule_id == "api/chalk-call"),
        "chalk importé DOIT déclencher api/chalk-call — findings: {:?}",
        findings.iter().map(|f| &f.rule_id).collect::<Vec<_>>()
    );
}
