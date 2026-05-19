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

/// Mode scan par défaut (check / fix / aggressive / migrate).
use std::process::ExitCode;

use anyhow::Result;

use crate::cli::args::{Cli, ReportArg};
use n2b_core::types::{Mode, Report, RunOptions, Severity};
use n2b_core::{report, run};

pub fn run(cli: &Cli) -> Result<ExitCode> {
    let mode = if cli.migrate || cli.aggressive {
        Mode::Aggressive
    } else if cli.fix {
        Mode::Fix
    } else {
        Mode::Check
    };
    let root = cli.root.canonicalize().unwrap_or_else(|_| cli.root.clone());
    // En mode agent, un format text implicite est promu en JSON pour garder
    // stdout parsable ; le user peut forcer jsonl/md via --report.
    let effective_report: Report = if cli.agent && matches!(cli.report, ReportArg::Text) {
        Report::Json
    } else {
        cli.report.into()
    };
    let opts = RunOptions {
        root,
        mode,
        report: effective_report,
        quiet: cli.quiet,
        ignore: cli.ignore.clone(),
        agent: cli.agent,
        dry_run: false,
    };

    let fixes = run::run(&opts)?;

    // Mode --migrate : applique les side-effects après le scan+fix +
    // génère/persiste le report card (Phase 5 §5.4 + §5.6).
    // Phase 6 §6.3 : `--scaffold-polyfills` opt-in pour bunpp scaffold.
    let report_card = if cli.migrate {
        let migrate_opts = crate::commands::migrate::MigrateOpts {
            scaffold_polyfills: cli.scaffold_polyfills,
        };
        crate::commands::migrate::run_migrate_side_effects_with_opts(
            &opts.root,
            &fixes,
            opts.quiet,
            migrate_opts,
        )?;
        let card = n2b_core::report_card::build(&fixes);
        let state = n2b_core::report_card::N2bState::from_card(&card, &fixes);
        if let Err(e) = state.write_to(&opts.root) {
            eprintln!("note: impossible d'écrire .n2b/state.json: {e}");
        }
        Some(card)
    } else {
        None
    };

    let card_value = report_card
        .as_ref()
        .and_then(|c| serde_json::to_value(c).ok());
    match opts.report {
        Report::Json => println!(
            "{}",
            report::render_json_with_card(&fixes, &opts, card_value.as_ref())
        ),
        Report::Jsonl => print!("{}", report::render_jsonl(&fixes, &opts)),
        Report::Markdown => println!("{}", report::render_markdown(&fixes, &opts)),
        Report::Sarif => println!("{}", report::render_sarif(&fixes, &opts)),
        Report::Text if !opts.quiet => {
            print!("{}", report::render_text(&fixes, &opts));
            if let Some(card) = &report_card {
                print!("{}", report::render_report_card_text(card));
            }
        }
        _ => {}
    }

    let has_errors = fixes
        .iter()
        .any(|f| f.findings.iter().any(|x| x.severity == Severity::Error));
    let has_findings = fixes.iter().any(|f| !f.findings.is_empty());

    Ok(if has_errors {
        ExitCode::from(2)
    } else if opts.mode == Mode::Check && has_findings {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}
