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

/// Commande `n2b audit` — scanne issues + PRs GitHub mentionnant bun/node.
///
/// Le runtime tokio est scoped à cette fonction (current_thread).
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;

use n2b_core::audit;
use n2b_core::types::Report;

pub fn run_audit(
    root: PathBuf,
    terms: Vec<String>,
    state: audit::ItemState,
    limit: usize,
    report: Report,
) -> Result<ExitCode> {
    let root = root.canonicalize().unwrap_or(root);
    let repo = audit::detect_repo(&root)?;
    let terms = if terms.is_empty() {
        vec!["bun".to_string(), "node".to_string()]
    } else {
        terms
    };

    // Runtime scoped : pas de runtime tokio global pour cette commande.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let result = rt.block_on(audit::run_audit(repo, &terms, state, limit))?;

    match report {
        Report::Json => println!("{}", audit::render_json(&result)),
        _ => print!("{}", audit::render_text(&result)),
    }
    Ok(ExitCode::SUCCESS)
}
