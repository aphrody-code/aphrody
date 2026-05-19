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

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

use n2b_core::audit;
use n2b_core::types::{Mode, Report};

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ReportArg {
    Text,
    Json,
    Jsonl,
    Md,
    Markdown,
    Sarif,
}

impl From<ReportArg> for Report {
    fn from(r: ReportArg) -> Self {
        match r {
            ReportArg::Text => Report::Text,
            ReportArg::Json => Report::Json,
            ReportArg::Jsonl => Report::Jsonl,
            ReportArg::Md | ReportArg::Markdown => Report::Markdown,
            ReportArg::Sarif => Report::Sarif,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum StateArg {
    Open,
    Closed,
    All,
}

impl From<StateArg> for audit::ItemState {
    fn from(s: StateArg) -> Self {
        match s {
            StateArg::Open => audit::ItemState::Open,
            StateArg::Closed => audit::ItemState::Closed,
            StateArg::All => audit::ItemState::All,
        }
    }
}

#[derive(Parser, Debug)]
#[command(
    name = "n2b",
    version,
    about = "n2b — outils de dev Rust + migration Node → Bun.",
    long_about = "n2b : scaffold Rust (bin/lib/axum/discord/cdylib/workspace…), \
                  migration Node → Bun, WASM, MUI → MD3, llmstxt, analyse GitHub."
)]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Option<Cmd>,

    // Flags du scan par défaut (conservés au niveau racine pour compat).
    /// Racine du package à analyser (défaut: répertoire courant).
    #[arg(default_value = ".")]
    pub root: PathBuf,

    /// Applique les corrections automatiques sûres (autofix = true).
    /// Mutuellement exclusif avec --aggressive et --migrate.
    #[arg(long, conflicts_with_all = ["aggressive", "migrate"])]
    pub fix: bool,

    /// Applique toutes les corrections, y compris celles marquées `aggressive`
    /// (transformations plus invasives). Mutuellement exclusif avec --fix et --migrate.
    #[arg(long, conflicts_with_all = ["fix", "migrate"])]
    pub aggressive: bool,

    /// Mode migration complète : applique --fix --aggressive ET exécute les
    /// side-effects (bun install, retrait pnpm-lock.yaml, migration
    /// pnpm-workspace.yaml → package.json, ajout @types/bun si requis).
    /// Mutuellement exclusif avec --fix et --aggressive.
    #[arg(long, conflicts_with_all = ["fix", "aggressive"])]
    pub migrate: bool,

    /// Phase 6 §6.3 : appelle `bunpp scaffold <module>` pour chaque module
    /// 🔴 trouvé. Crée des fichiers visibles dans le projet — opt-in
    /// explicite, sous BackupGuard. Requiert `--migrate`.
    #[arg(long, requires = "migrate")]
    pub scaffold_polyfills: bool,

    /// Format du rapport de sortie.
    #[arg(long, value_enum, default_value = "text")]
    pub report: ReportArg,

    /// Glob(s) de chemins à exclure du scan (cumulable, ex: `--ignore "test/**"`).
    #[arg(long)]
    pub ignore: Vec<String>,

    /// Supprime toute sortie sur stdout (le code de retour reste significatif).
    #[arg(long)]
    pub quiet: bool,

    /// Mode AI-agent : désactive les couleurs, logs sur stderr,
    /// stdout réservé au payload structuré. Implique --report=json si text.
    #[arg(long, global = true)]
    pub agent: bool,

    /// **dotnet branch** — active les règles DN0xx (dotnet 10 / NuGet / MSBuild),
    /// WN0xx (Windows 11 idioms — CIM cmdlets, registry, pwsh.exe), NA0xx
    /// (node-api-dotnet), WC0xx (WinClean-specific). Voir README-dotnet.md.
    ///
    /// Wiring complet du scan principal pas encore branché (préserve le
    /// contract externe gelé) — l'activation effective utilise le sous-
    /// command `n2b dotnet <root>` (à venir) ou un appel direct aux modules
    /// `n2b_rules::{dotnet, windows, node_api_dotnet, winclean}`.
    #[arg(long)]
    pub dotnet: bool,
}

#[derive(Subcommand, Debug)]
pub enum Cmd {
    /// Liste toutes les règles connues avec id, catégorie, docs_url.
    Rules {
        /// Format de sortie.
        #[arg(long, value_enum, default_value = "text")]
        report: ReportArg,
    },

    /// Génère un prompt markdown prêt à coller dans un LLM.
    Prompt {
        /// Racine du package (défaut: .)
        #[arg(default_value = ".")]
        root: PathBuf,

        /// Nombre maximum de findings inclus dans le prompt.
        #[arg(long, default_value_t = 50)]
        max_findings: usize,

        /// Inclure les findings Info (par défaut ignorés).
        #[arg(long)]
        include_info: bool,

        /// Glob d'exclusion.
        #[arg(long)]
        ignore: Vec<String>,
    },

    /// Détecte le repo GitHub et scanne issues + PRs mentionnant bun/node.
    Audit {
        /// Racine du package (défaut: .)
        #[arg(default_value = ".")]
        root: PathBuf,

        /// Termes à chercher (défaut: bun, node).
        #[arg(long = "term", short = 't', value_name = "TERM", num_args = 1..)]
        terms: Vec<String>,

        /// État des items : open, closed, all.
        #[arg(long, value_enum, default_value = "all")]
        state: StateArg,

        /// Nombre maximum d'items par catégorie.
        #[arg(long, default_value_t = 30)]
        limit: usize,

        /// Format du rapport.
        #[arg(long, value_enum, default_value = "text")]
        report: ReportArg,
    },

    /// Scaffolde des apps Bun : CLI, TUI (Ink React), GUI (Electrobun),
    /// ou executable standalone (`bun build --compile` cross-target).
    ///
    ///   n2b app init <name> [--flavor cli|tui|gui|exe]
    ///   n2b app build <entry.ts> [--target bun-linux-x64] [--outfile path]
    ///   n2b app doctor
    App {
        #[command(subcommand)]
        sub: AppSub,
    },

    /// Scaffolde des projets Bun + Windows bas-niveau (Win32).
    /// FFI Rust (windows-rs → bun:ffi dlopen .dll), inline C (<windows.h>
    /// + TinyCC), Bun Shell + PowerShell 7. Cross-compile supporté (cargo-xwin
    ///   ou mingw-w64).
    ///
    ///   n2b win32 init <name>        # projet complet (FFI + CC + PowerShell)
    ///   n2b win32 ffi <name>         # Rust cdylib (windows-rs) + bun:ffi
    ///   n2b win32 cc <name>          # inline C avec <windows.h> via bun:ffi cc
    ///   n2b win32 pwsh <name>        # scripts Bun Shell + PowerShell
    ///   n2b win32 doctor             # check rustc/cl.exe/pwsh/cargo-xwin/mingw
    Win32 {
        #[command(subcommand)]
        sub: Win32Sub,
    },

    /// Scaffolde des projets Bun + Linux bas-niveau :
    /// FFI Rust (bun:ffi dlopen), inline C (bun:ffi cc, TinyCC), Bun Shell.
    ///
    ///   n2b linux init <name>        # projet complet (FFI + CC + Shell)
    ///   n2b linux ffi <name>         # Rust cdylib + bun:ffi
    ///   n2b linux cc <name>          # inline C via bun:ffi cc
    ///   n2b linux shell <name>       # scripts Bun Shell
    ///   n2b linux doctor             # check rustc/gcc/clang/tcc/…
    Linux {
        #[command(subcommand)]
        sub: LinuxSub,
    },

    /// Workflow Rust → WASM → Bun complet :
    /// `init` (scaffold), `doctor` (check tools), `build` (wasm-pack wrapper),
    /// `opt` (wasm-opt), `size` (twiggy top).
    Wasm {
        #[command(subcommand)]
        sub: WasmSub,
    },

    /// Scaffold un projet binaire natif : plugin Rust pour Bun.build
    /// (bun-native-plugin), exemple MDX→JSX, ou module WASM.
    ///
    ///   n2b bin myplugin                → plugin natif NAPI
    ///   n2b bin mdx-rs --flavor mdx     → plugin MDX→JSX
    ///   n2b bin fast-math --flavor wasm → module wasm-bindgen
    Bin {
        /// Nom du package/dossier à créer.
        name: String,

        /// Type de projet à scaffolder.
        #[arg(long, default_value = "native", value_parser = parse_bin_flavor)]
        flavor: crate::bin_cmd::BinFlavor,

        /// Dossier parent (le projet sera créé dans <dir>/<name>).
        #[arg(long)]
        dir: Option<PathBuf>,

        /// Écraser le dossier existant.
        #[arg(long)]
        force: bool,
    },

    /// Génère ou applique des patches. Deux modes :
    ///   · `n2b patch <pkg>`  → wrapper `bun patch` : applique les règles n2b
    ///                            sur node_modules/<pkg>, puis `bun patch --commit`.
    ///   · `n2b patch --self` → produit un diff unifié du repo courant
    ///                            (n2b.patch) sans modifier les fichiers.
    Patch {
        /// Nom (ou name@version) du package npm à patcher (mode A).
        package: Option<String>,

        /// Mode B : patch le repo courant au lieu d'une dep npm.
        #[arg(long = "self")]
        self_repo: bool,

        /// Racine du projet.
        #[arg(long, default_value = ".")]
        root: PathBuf,

        /// Applique aussi les règles marquées `aggressive`.
        #[arg(long)]
        aggressive: bool,

        /// Fichier de sortie pour le .patch (mode B).
        #[arg(long, default_value = "n2b.patch")]
        output: PathBuf,

        /// Dossier où `bun patch --commit` écrit les patch files (mode A).
        #[arg(long)]
        patches_dir: Option<PathBuf>,

        /// N'écrit rien (mode B imprime le patch sur stdout ; mode A skip
        /// `bun patch --commit`).
        #[arg(long)]
        dry_run: bool,

        /// Glob d'exclusion supplémentaire (cumulable).
        #[arg(long)]
        ignore: Vec<String>,
    },

    /// Automatise la couverture bun++ des gaps Node.js (canary 1.3.13).
    ///
    ///   n2b bunpp scaffold <module>     # génère @bun++/node-<module>
    ///   n2b bunpp scaffold-all          # scaffolde tous les gaps canary
    ///   n2b bunpp status                # % coverage vs gaps canary
    ///   n2b bunpp sync                  # pull issues oven-sh/bun → SYNC_REPORT.md
    ///   n2b bunpp doctor                # vérifie gh/bun/jq
    Bunpp {
        #[command(subcommand)]
        sub: BunppSub,
    },

    /// Génère llms.txt + llms-full.txt depuis une URL (orchestrateur siteone-crawler).
    ///
    ///   n2b llmstxt https://m3.material.io/ --out ./m3-llms
    ///
    /// Spec : https://llmstxt.org/. Nécessite `siteone-crawler` dans PATH.
    Llmstxt {
        /// URL racine à crawler.
        url: String,

        /// Dossier de sortie.
        #[arg(long, short, default_value = "./llmstxt-out")]
        out: PathBuf,

        /// Profondeur max de crawl (0 = illimité).
        #[arg(long, default_value_t = 0)]
        max_depth: usize,

        /// Limite du nombre de pages crawlées (0 = illimité).
        #[arg(long, default_value_t = 0)]
        max_pages: usize,

        /// Requêtes/s (respect du serveur).
        #[arg(long, default_value_t = 4)]
        rps: u32,

        /// Workers concurrents.
        #[arg(long, default_value_t = 8)]
        concurrency: u32,

        /// User-Agent custom.
        #[arg(long)]
        user_agent: Option<String>,

        /// Regex d'inclusion (PCRE, cumulable).
        #[arg(long = "include")]
        include: Vec<String>,

        /// Regex d'exclusion (PCRE, cumulable).
        #[arg(long = "exclude")]
        exclude: Vec<String>,

        /// Désactive la génération de llms-full.txt (activée par défaut).
        #[arg(long = "no-full")]
        no_full: bool,

        /// Résumer les descriptions via Claude (ANTHROPIC_API_KEY requis).
        #[arg(long)]
        summarize: bool,

        /// Modèle Claude si --summarize.
        #[arg(long, default_value = "claude-haiku-4-5")]
        model: String,

        /// Garde l'export intermédiaire siteone (debug).
        #[arg(long)]
        keep_intermediate: bool,

        /// Skip siteone — réutilise un export existant dans --out.
        #[arg(long)]
        skip_crawl: bool,

        /// Auto-utilise `<url>/sitemap.xml` comme seed (ou reconnaît que
        /// l'URL passée est déjà un sitemap). Accélère le crawl sur gros
        /// sites et garantit la couverture.
        #[arg(long)]
        sitemap: bool,

        /// Exporte aussi `sitemap.xml` + `sitemap.txt` dans `--out`.
        #[arg(long)]
        export_sitemap: bool,
    },

    // NOTE: `mui-to-md3` a été déplacé vers le workspace mui-rs
    // (voir ~/vps/rust/mui-rs/codemod/). n2b se concentre sur Node→Bun uniquement.
    /// Outils de développement Rust : scaffold, check, deps, doctor.
    ///
    ///   n2b rust new myapp --flavor axum       # serveur Axum
    ///   n2b rust new mybot --flavor discord     # bot Discord (serenity + poise)
    ///   n2b rust new mycrate --flavor cli       # CLI clap full
    ///   n2b rust new myffi --flavor cdylib      # cdylib pour bun:ffi
    ///   n2b rust check [path]                   # cargo check + clippy
    ///   n2b rust deps [path]                    # cargo outdated + audit
    ///   n2b rust doctor                         # vérifie la toolchain
    Rust {
        #[command(subcommand)]
        sub: RustSub,
    },

    /// Scan + audit + crosslink ML (embeddings) sur un ou plusieurs repos.
    Analyze {
        /// Chemins à analyser (défaut: discord.js/discordx/nextjs détectés dans cwd).
        #[arg(value_name = "PATH")]
        paths: Vec<PathBuf>,

        /// Nombre max d'issues/PRs par repo.
        #[arg(long, default_value_t = 60)]
        issue_limit: usize,

        /// Top-K issues à croiser par finding.
        #[arg(long, default_value_t = 3)]
        top_k: usize,

        /// Seuil minimum de similarité cosinus pour considérer un lien.
        #[arg(long, default_value_t = 0.35)]
        threshold: f32,

        /// Format du rapport.
        #[arg(long, value_enum, default_value = "text")]
        report: ReportArg,

        /// Glob d'exclusion supplémentaire (cumulable).
        #[arg(long)]
        ignore: Vec<String>,

        /// Applique aussi les fixes (fix ou aggressive).
        #[arg(long, value_name = "MODE")]
        apply: Option<ApplyArg>,
    },

    /// **dotnet branch** — applique les règles DN0xx + WN0xx + NA0xx + WC0xx
    /// (cf README-dotnet.md). Additif : ne touche pas au scan principal ni
    /// aux baselines gelées.
    ///
    ///   n2b dotnet .                       # scan + report text
    ///   n2b dotnet . --fix                 # applique les autofix sûrs
    ///   n2b dotnet . --fix --aggressive    # + règles invasives
    ///   n2b dotnet . --report=sarif        # SARIF pour GitHub Code Scanning
    Dotnet {
        /// Racine du projet à analyser (défaut: répertoire courant).
        #[arg(default_value = ".")]
        root: PathBuf,

        /// Applique les fixes automatiques (réécrit les fichiers in-place).
        #[arg(long)]
        fix: bool,

        /// Active aussi les règles `aggressive` (rewrites invasifs).
        #[arg(long)]
        aggressive: bool,

        /// Format de sortie.
        #[arg(long, value_enum, default_value = "text")]
        report: ReportArg,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ApplyArg {
    Fix,
    Aggressive,
}

impl From<ApplyArg> for Mode {
    fn from(a: ApplyArg) -> Self {
        match a {
            ApplyArg::Fix => Mode::Fix,
            ApplyArg::Aggressive => Mode::Aggressive,
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum RustSub {
    /// Scaffold un nouveau projet Rust avec un template opinioné.
    ///
    /// Flavors disponibles : bin, lib, cdylib, proc-macro, workspace, axum, discord, cli, tauri, leptos, tui, bevy, grpc.
    New {
        /// Nom du projet (= nom du dossier créé).
        name: String,
        /// Template de projet.
        #[arg(long, default_value = "bin", value_parser = parse_rust_flavor)]
        flavor: crate::rust_cmd::RustFlavor,
        /// Dossier parent (le projet sera dans <dir>/<name>).
        #[arg(long)]
        dir: Option<PathBuf>,
        /// Écraser le dossier si existant.
        #[arg(long)]
        force: bool,
    },
    /// `cargo check` puis `cargo clippy -W clippy::all -W clippy::pedantic`.
    Check {
        /// Racine du crate à analyser.
        #[arg(default_value = ".")]
        root: PathBuf,
    },
    /// `cargo outdated` + `cargo audit` en rapport unifié.
    Deps {
        /// Racine du crate à analyser.
        #[arg(default_value = ".")]
        root: PathBuf,
    },
    /// Vérifie l'installation de la toolchain Rust (rustc, clippy, wasm-pack, just…).
    Doctor,
}

pub fn parse_rust_flavor(s: &str) -> Result<crate::rust_cmd::RustFlavor, String> {
    crate::rust_cmd::RustFlavor::parse(s).ok_or_else(|| {
        format!(
            "flavor inconnu '{s}' (valeurs : {})",
            crate::rust_cmd::RustFlavor::variants()
        )
    })
}

pub fn parse_bin_flavor(s: &str) -> Result<crate::bin_cmd::BinFlavor, String> {
    crate::bin_cmd::BinFlavor::parse(s)
        .ok_or_else(|| format!("flavor inconnu '{s}' (valeurs : native, mdx, wasm, webgpu)"))
}

pub fn parse_wasm_template(s: &str) -> Result<crate::wasm_cmd::WasmTemplate, String> {
    crate::wasm_cmd::WasmTemplate::parse(s)
        .ok_or_else(|| format!("template inconnu '{s}' (valeurs : basic, game-of-life, wgpu)"))
}

pub fn parse_app_flavor(s: &str) -> Result<crate::app_cmd::AppFlavor, String> {
    crate::app_cmd::AppFlavor::parse(s)
        .ok_or_else(|| format!("flavor inconnu '{s}' (valeurs : cli, tui, gui, exe)"))
}

#[derive(Subcommand, Debug)]
pub enum AppSub {
    /// Scaffold une app dans le flavor choisi.
    Init {
        name: String,
        #[arg(long, default_value = "cli", value_parser = parse_app_flavor)]
        flavor: crate::app_cmd::AppFlavor,
        #[arg(long)]
        dir: Option<PathBuf>,
        #[arg(long)]
        force: bool,
    },
    /// `bun build --compile` wrapper cross-target.
    Build {
        entry: PathBuf,
        #[arg(long)]
        outfile: Option<PathBuf>,
        /// Cibles : bun-linux-x64, bun-linux-arm64, bun-darwin-x64,
        /// bun-darwin-arm64, bun-windows-x64.
        #[arg(long)]
        target: Option<String>,
        /// Active la minification du bundle de sortie.
        #[arg(long)]
        minify: bool,
        /// Génère une source map à côté du bundle.
        #[arg(long)]
        sourcemap: bool,
    },
    /// Vérifie que bun / tsc / upx sont installés + liste les cibles bun build.
    Doctor,
}

#[derive(Subcommand, Debug)]
pub enum Win32Sub {
    /// Projet complet : FFI + CC + PowerShell dans un même repo.
    Init {
        name: String,
        #[arg(long)]
        dir: Option<PathBuf>,
        #[arg(long)]
        force: bool,
    },
    /// Scaffold Rust cdylib (windows-rs) + bun:ffi dlopen.
    Ffi {
        name: String,
        #[arg(long)]
        dir: Option<PathBuf>,
        #[arg(long)]
        force: bool,
    },
    /// Scaffold inline C avec <windows.h> compilé par TinyCC.
    Cc {
        name: String,
        #[arg(long)]
        dir: Option<PathBuf>,
        #[arg(long)]
        force: bool,
    },
    /// Scaffold scripts Bun Shell invoquant PowerShell 7 pour ops Windows.
    Pwsh {
        name: String,
        #[arg(long)]
        dir: Option<PathBuf>,
        #[arg(long)]
        force: bool,
    },
    /// Vérifie rustc, cl.exe, clang-cl, pwsh, cargo-xwin, mingw-w64.
    Doctor,
}

#[derive(Subcommand, Debug)]
pub enum LinuxSub {
    /// Projet complet : FFI + CC + Shell dans un même repo.
    Init {
        name: String,
        #[arg(long)]
        dir: Option<PathBuf>,
        #[arg(long)]
        force: bool,
    },
    /// Scaffold Rust cdylib + bun:ffi dlopen.
    Ffi {
        name: String,
        #[arg(long)]
        dir: Option<PathBuf>,
        #[arg(long)]
        force: bool,
    },
    /// Scaffold inline C compilé à runtime (bun:ffi cc, TinyCC).
    Cc {
        name: String,
        #[arg(long)]
        dir: Option<PathBuf>,
        #[arg(long)]
        force: bool,
    },
    /// Scaffold scripts Bun Shell pour ops système.
    Shell {
        name: String,
        #[arg(long)]
        dir: Option<PathBuf>,
        #[arg(long)]
        force: bool,
    },
    /// Vérifie que rustc / gcc / clang / tcc / pkg-config sont installés.
    Doctor,
}

#[derive(Subcommand, Debug)]
pub enum BunppSub {
    /// Scaffolde un seul polyfill `@bun++/node-<module>` (ou `@bun++/<pkg>`).
    Scaffold {
        /// Nom du module (`node:sqlite`, `sqlite`, `node-util-ext`…).
        module: String,
        /// Racine bun++ (contient `packages/`).
        #[arg(long, default_value = "./bun++")]
        root: PathBuf,
        #[arg(long)]
        force: bool,
    },
    /// Scaffolde tous les polyfills canary manquants (liste figée).
    ScaffoldAll {
        #[arg(long, default_value = "./bun++")]
        root: PathBuf,
        #[arg(long)]
        force: bool,
    },
    /// Rapporte la couverture bun++ vs gaps canary.
    Status {
        #[arg(long, default_value = "./bun++")]
        root: PathBuf,
    },
    /// Pull les issues oven-sh/bun liées aux gaps connus (nécessite `gh`).
    Sync {
        #[arg(long, default_value = "./bun++")]
        root: PathBuf,
        /// Affiche le rapport sur stdout au lieu d'écrire `SYNC_REPORT.md`.
        #[arg(long)]
        dry_run: bool,
    },
    /// Vérifie que gh / bun / jq sont installés.
    Doctor,
}

#[derive(Subcommand, Debug)]
pub enum WasmSub {
    /// Scaffold un nouveau projet Rust→WASM.
    Init {
        name: String,
        #[arg(long, default_value = "basic", value_parser = parse_wasm_template)]
        template: crate::wasm_cmd::WasmTemplate,
        #[arg(long)]
        dir: Option<PathBuf>,
        #[arg(long)]
        force: bool,
    },
    /// Vérifie que wasm-pack / cargo-generate / wasm-opt / twiggy / wasm2wat sont installés.
    Doctor,
    /// Build via wasm-pack avec les bonnes options par défaut.
    ///
    /// Exemples :
    ///   n2b wasm build                          # release + bundler (défauts)
    ///   n2b wasm build --target web --dev       # dev, cible web
    ///   n2b wasm build --profile profiling      # profiling
    ///   n2b wasm build --out-dir dist/pkg --scope myorg
    #[command(group(
        clap::ArgGroup::new("profile_group")
            .args(["profile", "dev", "release"])
            .multiple(false)
    ))]
    Build {
        /// Répertoire racine du crate Rust à builder (défaut : répertoire courant).
        #[arg(default_value = ".")]
        root: PathBuf,

        /// Cible de sortie wasm-pack.
        #[arg(long, default_value = "bundler", value_enum)]
        target: crate::wasm_cmd::WasmTarget,

        /// Profil de compilation (dev / profiling / release).
        #[arg(long, value_enum, group = "profile_group")]
        profile: Option<crate::wasm_cmd::BuildProfile>,

        /// Alias de --profile dev (compatibilité ascendante).
        #[arg(long, group = "profile_group")]
        dev: bool,

        /// Alias de --profile release (compatibilité ascendante).
        #[arg(long, group = "profile_group")]
        release: bool,

        /// Répertoire de sortie du paquet wasm-pack (défaut : `pkg`).
        #[arg(long)]
        out_dir: Option<PathBuf>,

        /// Préfixe des fichiers générés (défaut : nom du crate).
        #[arg(long)]
        out_name: Option<String>,

        /// Scope npm pour le paquet publié (`@scope/package`).
        #[arg(long)]
        scope: Option<String>,
    },
    /// Passe wasm-opt (-Oz par défaut) sur un fichier .wasm, en place.
    Opt {
        path: PathBuf,
        #[arg(long, default_value = "-Oz")]
        level: String,
    },
    /// Affiche les N symboles les plus volumineux (twiggy top).
    Size {
        path: PathBuf,
        #[arg(long, default_value_t = 20)]
        top: usize,
    },
    /// Référence WebAssembly spec officielle : testsuite, feature detection, opcodes.
    ///
    /// Exemples :
    ///   n2b wasm spec testsuite --path /path/to/spec
    ///   n2b wasm spec testsuite --path /path/to/spec --filter simd
    ///   n2b wasm spec features foo.wasm
    ///   n2b wasm spec opcodes
    ///   n2b wasm spec opcodes --proposal bulk-memory --report md
    Spec {
        #[command(subcommand)]
        sub: WasmSpecSub,
    },
}

#[derive(Subcommand, Debug)]
pub enum WasmSpecSub {
    /// Lance la testsuite WebAssembly officielle (.wast) contre Bun ou wasmtime.
    ///
    /// Sans `wat2wasm` (apt install wabt) : mode count-only (compte les modules).
    /// Avec `wat2wasm` + `bun` : valide chaque module via `new WebAssembly.Module(bytes)`.
    Testsuite {
        /// Chemin vers la racine du clone WebAssembly/spec
        /// (doit contenir `test/core/`).
        #[arg(long, default_value = "./spec")]
        path: PathBuf,

        /// Filtre par sous-proposition :
        /// `core`, `simd`, `gc`, `threads`, `bulk-memory`, `exceptions`,
        /// `memory64`, `multi-memory`, `relaxed-simd`.
        #[arg(long)]
        filter: Option<String>,

        /// Runtime à utiliser (`bun` ou `wasmtime` — wasmtime prévu en V2).
        #[arg(long, default_value = "bun")]
        runtime: String,

        /// Timeout par fichier `.wast` en secondes (réservé V2).
        #[arg(long, default_value_t = 30)]
        timeout: u64,
    },

    /// Analyse un binaire `.wasm` et liste les propositions WebAssembly utilisées.
    ///
    /// Détecte : bulk-memory, reference-types, tail-calls, SIMD (v128),
    /// exception-handling, GC (structs/arrays), multi-memory, memory64, threads.
    Features {
        /// Chemin du binaire `.wasm` à analyser.
        path: PathBuf,
    },

    /// Affiche la table de référence des opcodes WebAssembly.
    ///
    /// Par défaut : tous les opcodes. Filtrer avec `--proposal`.
    Opcodes {
        /// Proposition à filtrer :
        /// `mvp`, `bulk-memory`, `reference-types`, `tail-calls`,
        /// `exception-handling`, `simd`, `relaxed-simd`, `gc`,
        /// `multi-memory`, `memory64`, `threads`.
        #[arg(long)]
        proposal: Option<String>,

        /// Format de sortie : `text` | `md` | `json`.
        #[arg(long, default_value = "text")]
        report: String,
    },
}
