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

//! `n2b win32 <sub>` — scaffolde des projets Bun + Win32 bas-niveau.
//!
//! Miroir de `linux_cmd.rs` pour l'écosystème Microsoft Windows :
//!   - `ffi`    : Rust cdylib (`windows` crate Microsoft) → Bun `dlopen` .dll
//!   - `cc`     : inline C avec `<windows.h>` compilé par TinyCC à runtime
//!   - `pwsh`   : scripts Bun Shell invoquant PowerShell 7 pour ops système
//!   - `init`   : projet complet mêlant les 3 approches + build.ps1 + build.sh (cross)
//!   - `doctor` : vérifie rustc+windows-msvc, cl.exe, link.exe, clang-cl,
//!     cmake, ninja, pwsh, scoop, mingw-w64 (cross-compile)
//!
//! Refs :
//!   - https://github.com/microsoft/windows-rs
//!   - https://bun.com/docs/runtime/ffi
//!   - https://bun.com/docs/runtime/c-compiler
//!   - https://bun.com/docs/runtime/shell
//!   - https://bun.com/docs/runtime/node-api
//!   - https://bun.com/docs/project/building-windows
//!   - https://github.com/marlersoft/zigwin32
//!   - https://www.npmjs.com/package/win32-api
//!   - https://learn.microsoft.com/fr-fr/windows/win32/api/
//!   - https://learn.microsoft.com/fr-fr/windows/dev-environment/rust/rust-for-windows

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

#[path = "win32_cmd_com.rs"]
mod win32_cmd_com;
#[path = "win32_cmd_templates.rs"]
mod win32_cmd_templates;

use win32_cmd_com::*;
use win32_cmd_templates::*;

pub enum Win32Cmd {
    Init {
        name: String,
        dir: Option<PathBuf>,
        force: bool,
    },
    Ffi {
        name: String,
        dir: Option<PathBuf>,
        force: bool,
    },
    Cc {
        name: String,
        dir: Option<PathBuf>,
        force: bool,
    },
    Pwsh {
        name: String,
        dir: Option<PathBuf>,
        force: bool,
    },
    Doctor,
}

pub fn run(cmd: Win32Cmd, quiet: bool) -> Result<()> {
    match cmd {
        Win32Cmd::Init { name, dir, force } => init_all(name, dir, force, quiet),
        Win32Cmd::Ffi { name, dir, force } => scaffold(name, dir, force, quiet, Flavor::Ffi),
        Win32Cmd::Cc { name, dir, force } => scaffold(name, dir, force, quiet, Flavor::Cc),
        Win32Cmd::Pwsh { name, dir, force } => scaffold(name, dir, force, quiet, Flavor::Pwsh),
        Win32Cmd::Doctor => doctor(quiet),
    }
}

enum Flavor {
    Ffi,
    Cc,
    Pwsh,
}

fn scaffold(
    name: String,
    dir: Option<PathBuf>,
    force: bool,
    quiet: bool,
    flavor: Flavor,
) -> Result<()> {
    let target = match dir {
        Some(d) => d.join(&name),
        None => PathBuf::from(&name),
    };
    if target.exists() && !force {
        anyhow::bail!("{} existe déjà — relancer avec --force", target.display());
    }
    std::fs::create_dir_all(&target)?;
    match flavor {
        Flavor::Ffi => scaffold_ffi(&target, &name, quiet)?,
        Flavor::Cc => scaffold_cc(&target, &name, quiet)?,
        Flavor::Pwsh => scaffold_pwsh(&target, &name, quiet)?,
    }
    if !quiet {
        eprintln!(
            "[win32] ✓ {name} scaffolded → cd {} && bun install && bun run build",
            target.display()
        );
    }
    Ok(())
}

fn init_all(name: String, dir: Option<PathBuf>, force: bool, quiet: bool) -> Result<()> {
    let target = match dir {
        Some(d) => d.join(&name),
        None => PathBuf::from(&name),
    };
    if target.exists() && !force {
        anyhow::bail!("{} existe déjà — relancer avec --force", target.display());
    }
    std::fs::create_dir_all(&target)?;

    // FFI : crate Rust avec windows-rs
    write(
        target.join("crates/ffi/Cargo.toml"),
        &render_ffi_cargo_toml(&format!("{name}_ffi"), true),
        quiet,
    )?;
    write(
        target.join("crates/ffi/src/lib.rs"),
        FFI_LIB_RS_COMPLETE,
        quiet,
    )?;

    // CC : fichiers C inline
    write(target.join("c/hello.c"), CC_HELLO_C, quiet)?;
    write(target.join("c/winapi.c"), CC_WINAPI_C, quiet)?;

    // PowerShell scripts Bun
    write(target.join("scripts/sysinfo.ts"), PWSH_SYSINFO_TS, quiet)?;
    write(target.join("scripts/ops.ts"), PWSH_OPS_TS, quiet)?;

    // Entrypoints TS
    write(
        target.join("src/ffi.ts"),
        &render_ffi_index_ts(&name),
        quiet,
    )?;
    write(target.join("src/cc.ts"), CC_INDEX_TS, quiet)?;
    write(target.join("src/pwsh.ts"), PWSH_INDEX_TS, quiet)?;
    write(target.join("src/main.ts"), &render_main_ts(&name), quiet)?;

    // Package + scripts de build
    write(
        target.join("package.json"),
        &render_all_package_json(&name),
        quiet,
    )?;
    write(target.join("README.md"), &render_all_readme(&name), quiet)?;
    write(target.join(".gitignore"), GITIGNORE, quiet)?;
    write(target.join("build.ps1"), &render_build_ps1(&name), quiet)?;
    write(
        target.join("build.sh"),
        &render_build_sh_cross(&name),
        quiet,
    )?;
    write(target.join("cross-compile.md"), CROSS_COMPILE_MD, quiet)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for f in ["build.sh", "build.ps1"] {
            if let Ok(meta) = std::fs::metadata(target.join(f)) {
                let mut perms = meta.permissions();
                perms.set_mode(0o755);
                let _ = std::fs::set_permissions(target.join(f), perms);
            }
        }
    }

    if !quiet {
        eprintln!(
            "[win32 init] ✓ {name} complet → cd {} && ./build.ps1 (Windows) ou ./build.sh (cross)",
            target.display()
        );
    }
    Ok(())
}

fn scaffold_ffi(dir: &Path, name: &str, quiet: bool) -> Result<()> {
    write(
        dir.join("Cargo.toml"),
        &render_ffi_cargo_toml(name, false),
        quiet,
    )?;
    write(dir.join("src/lib.rs"), FFI_LIB_RS_SIMPLE, quiet)?;
    write(
        dir.join("package.json"),
        &render_ffi_package_json(name),
        quiet,
    )?;
    write(
        dir.join("index.ts"),
        &render_ffi_index_ts_simple(name),
        quiet,
    )?;
    write(
        dir.join("README.md"),
        &readme(name, "Rust cdylib (windows-rs) → Bun FFI dlopen .dll"),
        quiet,
    )?;
    write(dir.join(".gitignore"), GITIGNORE, quiet)?;
    write(dir.join("build.ps1"), &render_build_ps1_simple(name), quiet)?;
    write(dir.join("build.sh"), &render_build_sh_simple(name), quiet)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for f in ["build.sh", "build.ps1"] {
            if let Ok(meta) = std::fs::metadata(dir.join(f)) {
                let mut perms = meta.permissions();
                perms.set_mode(0o755);
                let _ = std::fs::set_permissions(dir.join(f), perms);
            }
        }
    }
    Ok(())
}

fn scaffold_cc(dir: &Path, name: &str, quiet: bool) -> Result<()> {
    write(dir.join("hello.c"), CC_HELLO_C, quiet)?;
    write(dir.join("winapi.c"), CC_WINAPI_C, quiet)?;
    write(
        dir.join("package.json"),
        &render_cc_package_json(name),
        quiet,
    )?;
    write(dir.join("index.ts"), CC_INDEX_TS, quiet)?;
    write(
        dir.join("README.md"),
        &readme(
            name,
            "Inline C avec <windows.h> compilé par TinyCC (bun:ffi `cc`)",
        ),
        quiet,
    )?;
    write(dir.join(".gitignore"), GITIGNORE, quiet)?;
    Ok(())
}

fn scaffold_pwsh(dir: &Path, name: &str, quiet: bool) -> Result<()> {
    write(
        dir.join("package.json"),
        &render_pwsh_package_json(name),
        quiet,
    )?;
    write(dir.join("sysinfo.ts"), PWSH_SYSINFO_TS, quiet)?;
    write(dir.join("ops.ts"), PWSH_OPS_TS, quiet)?;
    write(
        dir.join("README.md"),
        &readme(
            name,
            "Ops système via Bun Shell + PowerShell 7 (`$\\`pwsh ...\\``)",
        ),
        quiet,
    )?;
    write(dir.join(".gitignore"), GITIGNORE, quiet)?;
    Ok(())
}

fn write(path: PathBuf, content: &str, quiet: bool) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    std::fs::write(&path, content).with_context(|| format!("write {}", path.display()))?;
    if !quiet {
        eprintln!("[win32]   + {}", path.display());
    }
    Ok(())
}
