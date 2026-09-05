// SPDX-License-Identifier: Apache-2.0
//! Package manager for the first-party Aphrody command-line tools.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use clap::{Subcommand, ValueEnum};
use miette::{IntoDiagnostic, WrapErr};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum PackageName {
    Aphrody,
    Bxc,
    N2b,
    #[value(alias = "niers")]
    Nie,
    All,
}

#[derive(Subcommand, Debug, Clone)]
pub(crate) enum PackageAction {
    /// Afficher le catalogue intégré et les dépendances de construction.
    Catalog {
        #[arg(long)]
        json: bool,
    },
    /// Afficher les versions installées et l'état des sources gérées.
    Status {
        package: Option<PackageName>,
        #[arg(long)]
        json: bool,
    },
    /// Cloner, construire et installer un ou plusieurs CLI.
    Install {
        package: PackageName,
        #[arg(long)]
        dry_run: bool,
    },
    /// Mettre à jour les sources, reconstruire et remplacer les binaires.
    Update {
        package: PackageName,
        #[arg(long)]
        dry_run: bool,
    },
    /// Retirer les binaires; --purge retire aussi les sources gérées.
    Uninstall {
        package: PackageName,
        #[arg(long)]
        purge: bool,
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        dry_run: bool,
    },
    /// Vérifier Git, Cargo/Bun, la plateforme et le répertoire d'installation.
    Doctor {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
enum Engine {
    Cargo,
    Bun,
}

#[derive(Debug, Clone, Copy)]
struct Package {
    name: &'static str,
    repository: &'static str,
    engine: Engine,
    cargo_package: Option<&'static str>,
    unix_build: &'static [&'static str],
    windows_build: &'static [&'static str],
    binaries: &'static [&'static str],
}

const PACKAGES: &[Package] = &[
    Package {
        name: "aphrody",
        repository: "https://github.com/aphrody-code/aphrody.git",
        engine: Engine::Cargo,
        cargo_package: Some("aphrody"),
        unix_build: &[],
        windows_build: &[],
        binaries: &["aphrody"],
    },
    Package {
        name: "bxc",
        repository: "https://github.com/aphrody-code/bxc.git",
        engine: Engine::Bun,
        cargo_package: None,
        unix_build: &["run", "build:linux"],
        windows_build: &["run", "build:win"],
        binaries: &["bxc", "bxc-mcp"],
    },
    Package {
        name: "n2b",
        repository: "https://github.com/aphrody-code/n2b.git",
        engine: Engine::Cargo,
        cargo_package: Some("n2b"),
        unix_build: &[],
        windows_build: &[],
        binaries: &["n2b"],
    },
    Package {
        name: "nie",
        repository: "https://github.com/aphrody-code/nie.git",
        engine: Engine::Cargo,
        cargo_package: Some("nie-cli"),
        unix_build: &[],
        windows_build: &[],
        binaries: &["niers"],
    },
];

#[derive(Serialize)]
struct Status<'a> {
    name: &'a str,
    repository: &'a str,
    engine: Engine,
    installed: bool,
    version: Option<String>,
    source_present: bool,
    source: String,
}

pub(crate) fn run(action: PackageAction) -> miette::Result<()> {
    match action {
        PackageAction::Catalog { json } => catalog(json),
        PackageAction::Status { package, json } => {
            status(package.unwrap_or(PackageName::All), json)
        },
        PackageAction::Install { package, dry_run } => change(package, false, dry_run),
        PackageAction::Update { package, dry_run } => change(package, true, dry_run),
        PackageAction::Uninstall { package, purge, yes, dry_run } => {
            uninstall(package, purge, yes, dry_run)
        },
        PackageAction::Doctor { json } => doctor(json),
    }
}

fn selected(name: PackageName) -> Vec<&'static Package> {
    if name == PackageName::All {
        return PACKAGES.iter().collect();
    }
    let wanted = match name {
        PackageName::Aphrody => "aphrody",
        PackageName::Bxc => "bxc",
        PackageName::N2b => "n2b",
        PackageName::Nie => "nie",
        PackageName::All => unreachable!(),
    };
    PACKAGES.iter().filter(|p| p.name == wanted).collect()
}

fn roots() -> miette::Result<(PathBuf, PathBuf)> {
    let home = dirs::home_dir().ok_or_else(|| miette::miette!("profil utilisateur introuvable"))?;
    #[cfg(target_os = "windows")]
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join("AppData").join("Local"))
        .join("Aphrody");
    #[cfg(not(target_os = "windows"))]
    let base = home.join(".local").join("share").join("aphrody");
    #[cfg(target_os = "windows")]
    let bin = base.join("bin");
    #[cfg(not(target_os = "windows"))]
    let bin = home.join(".local").join("bin");
    Ok((base.join("sources"), bin))
}

fn catalog(json: bool) -> miette::Result<()> {
    let rows: Vec<_> = PACKAGES.iter().map(|p| serde_json::json!({
        "name": p.name, "repository": p.repository, "engine": p.engine,
        "cargo_package": p.cargo_package, "binaries": p.binaries,
        "requires": match p.engine { Engine::Cargo => vec!["git", "cargo", "rustc"], Engine::Bun => vec!["git", "bun"] },
        "os": ["linux", "macos", "windows"]
    })).collect();
    if json {
        println!("{}", serde_json::to_string_pretty(&rows).into_diagnostic()?);
    } else {
        for p in PACKAGES {
            println!("{:<8} {:<5?} {}", p.name, p.engine, p.repository);
        }
    }
    Ok(())
}

fn status(name: PackageName, json: bool) -> miette::Result<()> {
    let (sources, bin) = roots()?;
    let mut rows = Vec::new();
    for p in selected(name) {
        let executable = bin.join(exe(p.binaries[0]));
        let version = if executable.exists() { command_version(&executable) } else { None };
        rows.push(Status {
            name: p.name,
            repository: p.repository,
            engine: p.engine,
            installed: executable.exists(),
            version,
            source_present: sources.join(p.name).join(".git").exists(),
            source: sources.join(p.name).display().to_string(),
        });
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&rows).into_diagnostic()?);
    } else {
        for row in rows {
            println!(
                "{:<8} {:<12} {}",
                row.name,
                row.version.as_deref().unwrap_or("non installé"),
                if row.source_present { "source gérée" } else { "sans source gérée" }
            );
        }
    }
    Ok(())
}

fn change(name: PackageName, update: bool, dry_run: bool) -> miette::Result<()> {
    let (sources, bin) = roots()?;
    for p in selected(name) {
        let source = sources.join(p.name);
        println!(
            "{} {} ({:?})",
            if update { "mise à jour" } else { "installation" },
            p.name,
            p.engine
        );
        ensure_tools(p, dry_run)?;
        if !source.join(".git").exists() {
            run_cmd(
                None,
                "git",
                &["clone", "--depth", "1", p.repository, path_str(&source)?],
                dry_run,
            )?;
        } else if update {
            run_cmd(Some(&source), "git", &["pull", "--ff-only", "origin", "main"], dry_run)?;
        }
        build(p, &source, dry_run)?;
        install_binaries(p, &source, &bin, dry_run)?;
    }
    Ok(())
}

fn ensure_tools(p: &Package, dry_run: bool) -> miette::Result<()> {
    for tool in ["git", match p.engine {
        Engine::Cargo => "cargo",
        Engine::Bun => "bun",
    }] {
        if which::which(tool).is_err() && !dry_run {
            return Err(miette::miette!("dépendance `{tool}` absente"));
        }
    }
    Ok(())
}

fn build(p: &Package, source: &Path, dry_run: bool) -> miette::Result<()> {
    match p.engine {
        Engine::Cargo => run_cmd(
            Some(source),
            "cargo",
            &["build", "--release", "--locked", "-p", p.cargo_package.unwrap()],
            dry_run,
        ),
        Engine::Bun => {
            run_cmd(Some(source), "bun", &["install", "--frozen-lockfile"], dry_run)?;
            let args = if cfg!(target_os = "windows") { p.windows_build } else { p.unix_build };
            run_cmd(Some(source), "bun", args, dry_run)?;
            if cfg!(target_os = "windows") && p.name == "bxc" {
                run_cmd(Some(source), "bun", &["run", "build:mcp:win"], dry_run)?;
            }
            Ok(())
        },
    }
}

fn install_binaries(p: &Package, source: &Path, bin: &Path, dry_run: bool) -> miette::Result<()> {
    if dry_run {
        for name in p.binaries {
            println!("[dry-run] installer {} dans {}", name, bin.display());
        }
        return Ok(());
    }
    fs::create_dir_all(bin).into_diagnostic()?;
    for name in p.binaries {
        let built = built_binary(p, source, name);
        if !built.exists() {
            return Err(miette::miette!("binaire attendu absent: {}", built.display()));
        }
        let destination = bin.join(exe(name));
        fs::copy(&built, &destination)
            .into_diagnostic()
            .wrap_err_with(|| format!("installation de {}", destination.display()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&destination, fs::Permissions::from_mode(0o755))
                .into_diagnostic()?;
        }
        println!("installé: {}", destination.display());
    }
    Ok(())
}

fn built_binary(p: &Package, source: &Path, name: &str) -> PathBuf {
    let filename = exe(name);
    match p.engine {
        Engine::Cargo => source.join("target").join("release").join(filename),
        Engine::Bun => source.join("dist").join("standalone").join(
            match (name, std::env::consts::OS, std::env::consts::ARCH) {
                ("bxc", "windows", _) => PathBuf::from("windows").join("bxc-windows-x64.exe"),
                ("bxc", "linux", "aarch64") => "bxc-linux-arm64".into(),
                ("bxc", "linux", _) => "bxc-linux-x64".into(),
                ("bxc", "macos", "aarch64") => "bxc-darwin-arm64".into(),
                ("bxc", "macos", _) => "bxc-darwin-x64".into(),
                ("bxc-mcp", "windows", _) => "bxc-mcp-windows-x64.exe".into(),
                _ => filename.into(),
            },
        ),
    }
}

fn uninstall(name: PackageName, purge: bool, yes: bool, dry_run: bool) -> miette::Result<()> {
    if !dry_run && !yes {
        return Err(miette::miette!(
            "désinstallation refusée: ajouter --yes (ou utiliser --dry-run)"
        ));
    }
    let (sources, bin) = roots()?;
    for p in selected(name) {
        for name in p.binaries {
            let path = bin.join(exe(name));
            if dry_run {
                println!("[dry-run] retirer {}", path.display());
            } else if path.exists() {
                fs::remove_file(&path).into_diagnostic()?;
                println!("retiré: {}", path.display());
            }
        }
        if purge {
            let source = sources.join(p.name);
            if dry_run {
                println!("[dry-run] purger {}", source.display());
            } else if source.exists() {
                fs::remove_dir_all(&source).into_diagnostic()?;
                println!("purgé: {}", source.display());
            }
        }
    }
    Ok(())
}

fn doctor(json: bool) -> miette::Result<()> {
    let (sources, bin) = roots()?;
    let tools: Vec<_> = ["git", "cargo", "rustc", "bun"]
        .iter()
        .map(|name| serde_json::json!({"name": name, "available": which::which(name).is_ok()}))
        .collect();
    let value = serde_json::json!({"os": std::env::consts::OS, "arch": std::env::consts::ARCH,
        "source_root": sources, "bin_dir": bin, "tools": tools});
    if json {
        println!("{}", serde_json::to_string_pretty(&value).into_diagnostic()?);
    } else {
        println!(
            "plateforme: {}/{}\nsources: {}\nbinaires: {}",
            std::env::consts::OS,
            std::env::consts::ARCH,
            sources.display(),
            bin.display()
        );
        for tool in tools {
            println!(
                "{:<8} {}",
                tool["name"].as_str().unwrap_or("?"),
                if tool["available"].as_bool().unwrap_or(false) { "ok" } else { "absent" }
            );
        }
    }
    Ok(())
}

fn run_cmd(cwd: Option<&Path>, program: &str, args: &[&str], dry_run: bool) -> miette::Result<()> {
    if dry_run {
        println!("[dry-run] {} {}", program, args.join(" "));
        return Ok(());
    }
    let mut command = Command::new(program);
    command.args(args);
    if let Some(path) = cwd {
        command.current_dir(path);
    }
    let status =
        command.status().into_diagnostic().wrap_err_with(|| format!("exécution de `{program}`"))?;
    if !status.success() {
        return Err(miette::miette!("`{program}` a échoué avec {}", status.code().unwrap_or(1)));
    }
    Ok(())
}

fn command_version(path: &Path) -> Option<String> {
    let output = Command::new(path).arg("--version").output().ok()?;
    output.status.success().then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn exe(name: &str) -> String {
    if cfg!(target_os = "windows") { format!("{name}.exe") } else { name.to_owned() }
}
fn path_str(path: &Path) -> miette::Result<&str> {
    path.to_str().ok_or_else(|| miette::miette!("chemin non UTF-8: {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_has_unique_first_party_packages() {
        assert_eq!(PACKAGES.len(), 4);
        let mut names: Vec<_> = PACKAGES.iter().map(|p| p.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), PACKAGES.len());
    }
    #[test]
    fn every_package_has_https_repo_and_binary() {
        for p in PACKAGES {
            assert!(p.repository.starts_with("https://github.com/aphrody-code/"));
            assert!(!p.binaries.is_empty());
        }
    }
}
