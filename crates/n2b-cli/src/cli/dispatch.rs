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

/// Table de dispatch : mappe une `Cmd` vers son handler.
use std::process::ExitCode;

use anyhow::Result;
use clap::Parser;

use crate::cli::args::{
    AppSub, BunppSub, Cli, Cmd, LinuxSub, RustSub, WasmSpecSub, WasmSub, Win32Sub,
};
use crate::commands;

/// Point d'entrée principal après le parsing clap.
pub fn run(cli: Cli) -> Result<ExitCode> {
    // Mode agent : coupe les couleurs pour que stderr ne contienne pas d'ANSI.
    if cli.agent {
        colored::control::set_override(false);
    }

    match cli.cmd {
        Some(Cmd::Rules { report }) => commands::rules::run_rules(report.into()),
        Some(Cmd::Prompt {
            root,
            max_findings,
            include_info,
            ignore,
        }) => commands::prompt::run_prompt(root, max_findings, include_info, ignore, cli.agent),
        Some(Cmd::Audit {
            root,
            terms,
            state,
            limit,
            report,
        }) => commands::audit::run_audit(root, terms, state.into(), limit, report.into()),
        Some(Cmd::Dotnet {
            root,
            fix,
            aggressive,
            report,
        }) => commands::dotnet::run(root, fix, aggressive, report),
        Some(Cmd::App { sub }) => {
            let cmd = match sub {
                AppSub::Init {
                    name,
                    flavor,
                    dir,
                    force,
                } => crate::app_cmd::AppCmd::Init {
                    name,
                    flavor,
                    dir,
                    force,
                },
                AppSub::Build {
                    entry,
                    outfile,
                    target,
                    minify,
                    sourcemap,
                } => crate::app_cmd::AppCmd::Build {
                    entry,
                    outfile,
                    target,
                    minify,
                    sourcemap,
                },
                AppSub::Doctor => crate::app_cmd::AppCmd::Doctor,
            };
            crate::app_cmd::run(cmd, cli.quiet)?;
            Ok(ExitCode::SUCCESS)
        }
        Some(Cmd::Win32 { sub }) => {
            let cmd = match sub {
                Win32Sub::Init { name, dir, force } => {
                    crate::win32_cmd::Win32Cmd::Init { name, dir, force }
                }
                Win32Sub::Ffi { name, dir, force } => {
                    crate::win32_cmd::Win32Cmd::Ffi { name, dir, force }
                }
                Win32Sub::Cc { name, dir, force } => {
                    crate::win32_cmd::Win32Cmd::Cc { name, dir, force }
                }
                Win32Sub::Pwsh { name, dir, force } => {
                    crate::win32_cmd::Win32Cmd::Pwsh { name, dir, force }
                }
                Win32Sub::Doctor => crate::win32_cmd::Win32Cmd::Doctor,
            };
            crate::win32_cmd::run(cmd, cli.quiet)?;
            Ok(ExitCode::SUCCESS)
        }
        Some(Cmd::Linux { sub }) => {
            let cmd = match sub {
                LinuxSub::Init { name, dir, force } => {
                    crate::linux_cmd::LinuxCmd::Init { name, dir, force }
                }
                LinuxSub::Ffi { name, dir, force } => {
                    crate::linux_cmd::LinuxCmd::Ffi { name, dir, force }
                }
                LinuxSub::Cc { name, dir, force } => {
                    crate::linux_cmd::LinuxCmd::Cc { name, dir, force }
                }
                LinuxSub::Shell { name, dir, force } => {
                    crate::linux_cmd::LinuxCmd::Shell { name, dir, force }
                }
                LinuxSub::Doctor => crate::linux_cmd::LinuxCmd::Doctor,
            };
            crate::linux_cmd::run(cmd, cli.quiet)?;
            Ok(ExitCode::SUCCESS)
        }
        Some(Cmd::Wasm { sub }) => {
            let cmd = match sub {
                WasmSub::Init {
                    name,
                    template,
                    dir,
                    force,
                } => crate::wasm_cmd::WasmCmd::Init {
                    name,
                    template,
                    dir,
                    force,
                },
                WasmSub::Doctor => crate::wasm_cmd::WasmCmd::Doctor,
                WasmSub::Build {
                    root,
                    target,
                    profile,
                    dev,
                    release: _,
                    out_dir,
                    out_name,
                    scope,
                } => {
                    // Résolution de priorité des flags de profil.
                    // --profile > --dev > (--release ou défaut) = Release.
                    let resolved_profile = if let Some(p) = profile {
                        p
                    } else if dev {
                        crate::wasm_cmd::BuildProfile::Dev
                    } else {
                        crate::wasm_cmd::BuildProfile::Release
                    };
                    crate::wasm_cmd::WasmCmd::Build {
                        root,
                        target,
                        profile: resolved_profile,
                        out_dir,
                        out_name,
                        scope,
                    }
                }
                WasmSub::Opt { path, level } => crate::wasm_cmd::WasmCmd::Opt { path, level },
                WasmSub::Size { path, top } => crate::wasm_cmd::WasmCmd::Size { path, top },
                WasmSub::Spec { sub } => {
                    let spec_cmd = match sub {
                        WasmSpecSub::Testsuite {
                            path,
                            filter,
                            runtime,
                            timeout,
                        } => crate::wasm_cmd::WasmSpecCmd::Testsuite {
                            path,
                            filter,
                            runtime,
                            timeout_secs: timeout,
                        },
                        WasmSpecSub::Features { path } => {
                            crate::wasm_cmd::WasmSpecCmd::Features { path }
                        }
                        WasmSpecSub::Opcodes { proposal, report } => {
                            crate::wasm_cmd::WasmSpecCmd::Opcodes { proposal, report }
                        }
                    };
                    crate::wasm_cmd::WasmCmd::Spec(spec_cmd)
                }
            };
            crate::wasm_cmd::run(cmd, cli.quiet)?;
            Ok(ExitCode::SUCCESS)
        }
        Some(Cmd::Llmstxt {
            url,
            out,
            max_depth,
            max_pages,
            rps,
            concurrency,
            user_agent,
            include,
            exclude,
            no_full,
            summarize,
            model,
            keep_intermediate,
            skip_crawl,
            sitemap,
            export_sitemap,
        }) => {
            n2b_core::llmstxt::run(&n2b_core::llmstxt::LlmstxtOpts {
                url,
                out,
                max_depth,
                max_pages,
                rps,
                concurrency,
                user_agent,
                include,
                exclude,
                full: !no_full,
                summarize,
                model,
                keep_intermediate,
                skip_crawl,
                quiet: cli.quiet,
                sitemap,
                export_sitemap,
            })?;
            Ok(ExitCode::SUCCESS)
        }
        Some(Cmd::Bunpp { sub }) => {
            let cmd = match sub {
                BunppSub::Scaffold {
                    module,
                    root,
                    force,
                } => crate::bunpp_cmd::BunppCmd::Scaffold {
                    module,
                    root,
                    force,
                },
                BunppSub::ScaffoldAll { root, force } => {
                    crate::bunpp_cmd::BunppCmd::ScaffoldAll { root, force }
                }
                BunppSub::Status { root } => crate::bunpp_cmd::BunppCmd::Status { root },
                BunppSub::Sync { root, dry_run } => {
                    crate::bunpp_cmd::BunppCmd::Sync { root, dry_run }
                }
                BunppSub::Doctor => crate::bunpp_cmd::BunppCmd::Doctor,
            };
            crate::bunpp_cmd::run(cmd, cli.quiet)?;
            Ok(ExitCode::SUCCESS)
        }
        Some(Cmd::Bin {
            name,
            flavor,
            dir,
            force,
        }) => {
            crate::bin_cmd::run_bin(crate::bin_cmd::BinOpts {
                name,
                flavor,
                dir,
                force,
                quiet: cli.quiet,
            })?;
            Ok(ExitCode::SUCCESS)
        }
        Some(Cmd::Patch {
            package,
            self_repo,
            root,
            aggressive,
            output,
            patches_dir,
            dry_run,
            ignore,
        }) => {
            crate::patch::run_patch(crate::patch::PatchOpts {
                package,
                self_repo,
                root,
                aggressive,
                output,
                patches_dir,
                dry_run,
                ignore,
                quiet: cli.quiet,
            })?;
            Ok(ExitCode::SUCCESS)
        }
        // Cmd::MuiToMd3 déplacé vers mui-rs/codemod/ — n2b = Node→Bun uniquement
        Some(Cmd::Rust { sub }) => {
            let cmd = match sub {
                RustSub::New {
                    name,
                    flavor,
                    dir,
                    force,
                } => crate::rust_cmd::RustCmd::New {
                    name,
                    flavor,
                    dir,
                    force,
                },
                RustSub::Check { root } => crate::rust_cmd::RustCmd::Check { root },
                RustSub::Deps { root } => crate::rust_cmd::RustCmd::Deps { root },
                RustSub::Doctor => crate::rust_cmd::RustCmd::Doctor,
            };
            crate::rust_cmd::run(cmd, cli.quiet)?;
            Ok(ExitCode::SUCCESS)
        }
        Some(Cmd::Analyze {
            paths,
            issue_limit,
            top_k,
            threshold,
            report,
            ignore,
            apply,
        }) => {
            let cwd = std::env::current_dir()?;
            let paths = if paths.is_empty() {
                crate::analyze::resolve_default_paths(&cwd)
            } else {
                paths
            };
            if paths.is_empty() {
                anyhow::bail!(
                    "aucun chemin fourni et aucun candidat (discord.js/discordx/nextjs) trouvé dans {}",
                    cwd.display()
                );
            }
            crate::analyze::run_analyze(crate::analyze::AnalyzeOpts {
                paths,
                issue_limit,
                top_k,
                threshold,
                report: report.into(),
                apply: apply.map(Into::into),
                ignore,
            })?;
            Ok(ExitCode::SUCCESS)
        }
        None => {
            // Mode scan par défaut.
            commands::scan::run(&cli)
        }
    }
}

/// Parse les arguments et dispatch vers le bon handler.
/// Appelé depuis `main()`.
pub fn run_from_args() -> Result<ExitCode> {
    run(Cli::parse())
}
