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

//! Orchestre `siteone-crawler` pour télécharger un site entier en Markdown.
//!
//! On invoque :
//!   siteone-crawler --url <url>
//!     --markdown-export-dir <md_dir>
//!     --markdown-export-single-file <md_single>
//!     --max-reqs-per-sec <rps>
//!     --workers <concurrency>
//!     --markdown-remove-links-and-images-from-single-file
//!     --offline-export-no-auto-redirect-html
//!     --include-regex / --ignore-regex

use super::LlmstxtOpts;
use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

pub fn run_siteone(cli: &LlmstxtOpts, md_dir: &Path, md_single: &Path) -> Result<()> {
    let bin = which::which("siteone-crawler")
        .context("siteone-crawler introuvable dans PATH — `cargo install siteone-crawler`")?;

    // Purge d'un éventuel run précédent pour éviter les pages orphelines.
    let _ = std::fs::remove_dir_all(md_dir);
    let _ = std::fs::remove_file(md_single);
    std::fs::create_dir_all(md_dir)?;

    // Pipeline sitemap — si `--sitemap` et que l'URL ne pointe pas déjà sur
    // un sitemap.xml, on substitue l'URL par `<url>/sitemap.xml`. siteone
    // reconnaît nativement ce format et l'utilise comme seed.
    let seed_url = if cli.sitemap && !cli.url.ends_with("sitemap.xml") {
        let base = cli.url.trim_end_matches('/');
        format!("{base}/sitemap.xml")
    } else {
        cli.url.clone()
    };

    let mut cmd = Command::new(&bin);
    cmd.arg(format!("--url={seed_url}"))
        .arg(format!("--markdown-export-dir={}", md_dir.display()))
        .arg(format!(
            "--markdown-export-single-file={}",
            md_single.display()
        ))
        .arg(format!("--max-reqs-per-sec={}", cli.rps))
        .arg(format!("--workers={}", cli.concurrency))
        .arg("--markdown-remove-links-and-images-from-single-file")
        .arg("--offline-export-no-auto-redirect-html")
        .arg("--no-color");

    // Export sitemap — siteone écrit lui-même `sitemap.xml` + `sitemap.txt`
    // dans `<out>/` quand on lui passe les flags dédiés. On coupe ainsi le
    // besoin d'un fallback userland dans 99 % des cas.
    if cli.export_sitemap {
        let parent = md_dir.parent().unwrap_or(md_dir);
        cmd.arg(format!(
            "--sitemap-xml-file={}/sitemap.xml",
            parent.display()
        ))
        .arg(format!(
            "--sitemap-txt-file={}/sitemap.txt",
            parent.display()
        ));
    }

    if cli.max_depth > 0 {
        cmd.arg(format!("--max-depth={}", cli.max_depth));
    }
    if cli.max_pages > 0 {
        cmd.arg(format!("--max-visited-urls={}", cli.max_pages));
    }
    if let Some(ua) = &cli.user_agent {
        cmd.arg(format!("--user-agent={ua}"));
    }
    if cli.quiet {
        cmd.arg("--hide-progress-bar");
    }
    for pat in &cli.include {
        cmd.arg(format!("--include-regex={pat}"));
    }
    for pat in &cli.exclude {
        cmd.arg(format!("--ignore-regex={pat}"));
    }

    let status = cmd
        .status()
        .with_context(|| format!("exécution {}", bin.display()))?;
    if !status.success() {
        anyhow::bail!("siteone-crawler exit {status}");
    }
    Ok(())
}
