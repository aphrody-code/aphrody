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

use anyhow::{Context, Result, anyhow};
use std::path::{Path, PathBuf};
use std::process::Command;

use n2b_core::run;
use n2b_core::types::{Mode, Report, RunOptions};

pub struct PatchOpts {
    pub package: Option<String>,
    pub self_repo: bool,
    pub root: PathBuf,
    pub aggressive: bool,
    pub output: PathBuf,
    pub patches_dir: Option<PathBuf>,
    pub dry_run: bool,
    pub ignore: Vec<String>,
    pub quiet: bool,
}

pub fn run_patch(opts: PatchOpts) -> Result<()> {
    if opts.self_repo || opts.package.is_none() {
        run_self_patch(opts)
    } else {
        run_npm_patch(opts)
    }
}

/// Mode B : scan le repo, génère un patch unifié agrégé sans modifier les fichiers.
fn run_self_patch(opts: PatchOpts) -> Result<()> {
    let root = opts.root.canonicalize().unwrap_or(opts.root.clone());
    let mode = if opts.aggressive {
        Mode::Aggressive
    } else {
        Mode::Fix
    };
    let run_opts = RunOptions {
        root: root.clone(),
        mode,
        report: Report::Text,
        quiet: true,
        ignore: opts.ignore,
        agent: false,
        dry_run: true,
    };

    // `run::run` applique les transformations en mémoire et retourne les FileFix
    // avec (before, after). En mode dry_run, aucune écriture sur disque n'est faite.
    let fixes = run::run(&run_opts)?;

    let mut patch_text = String::new();
    let mut modified = 0usize;
    for f in &fixes {
        if f.before == f.after {
            continue;
        }
        modified += 1;
        let rel = Path::new(&f.file);
        let a = format!("a/{}", rel.display());
        let b = format!("b/{}", rel.display());
        let diff = similar::TextDiff::from_lines(&f.before, &f.after);
        let unified = diff
            .unified_diff()
            .context_radius(3)
            .header(&a, &b)
            .to_string();
        if !unified.is_empty() {
            patch_text.push_str(&format!("diff --git {a} {b}\n"));
            patch_text.push_str(&unified);
            if !unified.ends_with('\n') {
                patch_text.push('\n');
            }
        }
    }

    if patch_text.is_empty() {
        if !opts.quiet {
            eprintln!("n2b patch : aucun changement à générer");
        }
        return Ok(());
    }

    if opts.dry_run {
        print!("{patch_text}");
    } else {
        std::fs::write(&opts.output, &patch_text)
            .with_context(|| format!("écriture de {}", opts.output.display()))?;
        if !opts.quiet {
            eprintln!(
                "n2b patch : {modified} fichier(s) patchés → {}",
                opts.output.display()
            );
            eprintln!("  Appliquer avec : git apply {}", opts.output.display());
        }
    }
    Ok(())
}

/// Mode A : wrapper autour de `bun patch <pkg>` + apply règles n2b + `bun patch --commit`.
fn run_npm_patch(opts: PatchOpts) -> Result<()> {
    let pkg = opts
        .package
        .as_ref()
        .ok_or_else(|| anyhow!("mode A : --package requis (ex: n2b patch react)"))?;
    let root = opts.root.canonicalize().unwrap_or(opts.root.clone());

    if which("bun").is_err() {
        anyhow::bail!(
            "n2b patch mode npm : le binaire `bun` est requis (https://bun.com/docs/installation)"
        );
    }

    // 1) Préparer le package : `bun patch <pkg>` crée un fresh clone dans node_modules/<pkg>.
    if !opts.quiet {
        eprintln!("[patch] bun patch {pkg}");
    }
    let out = Command::new("bun")
        .args(["patch", pkg])
        .current_dir(&root)
        .output()
        .context("exécution de `bun patch`")?;
    if !out.status.success() {
        anyhow::bail!(
            "bun patch {pkg} a échoué (exit={}): {}",
            out.status,
            String::from_utf8_lossy(&out.stderr)
        );
    }

    // 2) Localiser le dossier du package. bun patch peut imprimer le path sur stdout.
    let pkg_dir = resolve_pkg_dir(&root, pkg, &out.stdout)?;
    if !pkg_dir.exists() {
        anyhow::bail!(
            "impossible de trouver le dossier du package après `bun patch` : {}",
            pkg_dir.display()
        );
    }
    if !opts.quiet {
        eprintln!("[patch] dossier préparé : {}", pkg_dir.display());
    }

    // 3) Appliquer les règles n2b sur le package.
    let mode = if opts.aggressive {
        Mode::Aggressive
    } else {
        Mode::Fix
    };
    let run_opts = RunOptions {
        root: pkg_dir.clone(),
        mode,
        report: Report::Text,
        quiet: true,
        ignore: opts.ignore,
        agent: false,
        dry_run: false,
    };
    let fixes = run::run(&run_opts)?;
    let changed = fixes.iter().filter(|f| f.before != f.after).count();
    if !opts.quiet {
        eprintln!("[patch] {changed} fichier(s) modifiés par les règles n2b");
    }

    if opts.dry_run {
        if !opts.quiet {
            eprintln!("[patch] --dry-run : `bun patch --commit` non exécuté");
        }
        return Ok(());
    }

    if changed == 0 {
        if !opts.quiet {
            eprintln!("[patch] aucun changement, pas de commit");
        }
        return Ok(());
    }

    // 4) `bun patch --commit <pkg> [--patches-dir=<dir>]`.
    let mut args: Vec<String> = vec!["patch".into(), "--commit".into(), pkg.into()];
    if let Some(dir) = &opts.patches_dir {
        args.push(format!("--patches-dir={}", dir.display()));
    }
    if !opts.quiet {
        eprintln!("[patch] bun {}", args.join(" "));
    }
    let out = Command::new("bun")
        .args(&args)
        .current_dir(&root)
        .output()
        .context("exécution de `bun patch --commit`")?;
    if !out.status.success() {
        anyhow::bail!(
            "bun patch --commit {pkg} a échoué (exit={}): {}",
            out.status,
            String::from_utf8_lossy(&out.stderr)
        );
    }
    if !opts.quiet {
        eprintln!(
            "[patch] ✓ patch généré pour {pkg} ({} fichier(s) modifiés)",
            changed
        );
    }
    Ok(())
}

/// Cherche le dossier du package dans node_modules. Priorité à une ligne
/// affichée par `bun patch` contenant node_modules/<pkg>.
fn resolve_pkg_dir(root: &Path, pkg: &str, bun_stdout: &[u8]) -> Result<PathBuf> {
    let s = String::from_utf8_lossy(bun_stdout);
    for line in s.lines() {
        let line = line.trim();
        if line.contains("node_modules/") && line.contains(pkg) {
            let candidate = PathBuf::from(line);
            if candidate.is_absolute() && candidate.exists() {
                return Ok(candidate);
            }
        }
    }
    let name = pkg.split('@').next().unwrap_or(pkg);
    Ok(root.join("node_modules").join(name))
}

fn which(name: &str) -> Result<PathBuf> {
    let path = std::env::var_os("PATH").ok_or_else(|| anyhow!("PATH non défini"))?;
    for dir in std::env::split_paths(&path) {
        let p = dir.join(name);
        if p.is_file() {
            return Ok(p);
        }
    }
    Err(anyhow!("binaire `{name}` introuvable dans PATH"))
}
