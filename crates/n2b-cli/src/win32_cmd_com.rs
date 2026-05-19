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

//! Helpers COM / registry / FFI pour `n2b win32`.
//!
//! Séparé de `win32_cmd.rs` pour limiter la taille du fichier principal.
//! Tout ce module est `pub(super)` — il n'expose rien en dehors de `win32_cmd`.

use anyhow::{Result, anyhow};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Doctor
// ---------------------------------------------------------------------------

pub(super) fn doctor(quiet: bool) -> Result<()> {
    let tools: &[(&str, &str, &str)] = &[
        (
            "rustc",
            "winget install Rustlang.Rust.MSVC",
            "compilateur Rust (target x86_64-pc-windows-msvc)",
        ),
        ("cargo", "(installé avec rustup)", "package manager Rust"),
        (
            "cl",
            "VS 2022 Community + workload C++",
            "MSVC compiler (cl.exe)",
        ),
        ("link", "VS 2022 Community", "MSVC linker (link.exe)"),
        (
            "clang-cl",
            "scoop install llvm",
            "clang drop-in pour MSVC ABI",
        ),
        ("cmake", "winget install Kitware.CMake", "build system"),
        ("ninja", "scoop install ninja", "ninja build backend"),
        (
            "pwsh",
            "winget install Microsoft.PowerShell",
            "PowerShell 7 (pwsh.exe)",
        ),
        (
            "scoop",
            "iwr -useb get.scoop.sh | iex",
            "package manager Windows",
        ),
        (
            "x86_64-w64-mingw32-gcc",
            "sudo apt install mingw-w64",
            "cross-compile Linux→Windows (GNU)",
        ),
        (
            "cargo-xwin",
            "cargo install cargo-xwin",
            "cross-compile Linux→Windows (MSVC)",
        ),
        // --- Unix CLI essentiels sur Windows via uutils (cross-platform Rust) ---
        (
            "ls",
            "cargo install coreutils",
            "uutils/coreutils (ls/cp/cat/… natifs Windows)",
        ),
        (
            "find",
            "cargo install findutils",
            "uutils/findutils (find/xargs natifs Windows)",
        ),
        (
            "diff",
            "cargo install diffutils",
            "uutils/diffutils (diff/cmp natifs Windows)",
        ),
        (
            "ps",
            "cargo install procps",
            "uutils/procps (ps/top/watch natifs Windows)",
        ),
    ];
    let mut missing = 0;
    for (bin, install, desc) in tools {
        let ok = which(bin).is_ok();
        if !quiet {
            let mark = if ok { "✓" } else { "✗" };
            println!("  {mark} {:<25} {desc}", bin);
            if !ok {
                println!("       install: {install}");
                missing += 1;
            }
        }
    }
    if !quiet {
        if missing > 0 {
            eprintln!("\n{missing} outil(s) manquant(s)");
        }
        eprintln!("\nRust target :");
        let _ = std::process::Command::new("rustup")
            .args(["target", "list", "--installed"])
            .status();
        eprintln!("\nPour ajouter les cibles Windows :");
        eprintln!("  rustup target add x86_64-pc-windows-msvc   # natif (Windows ou cargo-xwin)");
        eprintln!("  rustup target add x86_64-pc-windows-gnu    # mingw (Linux-friendly)");
        eprintln!("  rustup target add aarch64-pc-windows-msvc  # ARM64 Windows");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Renderers FFI Cargo.toml / package.json
// ---------------------------------------------------------------------------

pub(super) fn render_ffi_cargo_toml(name: &str, richer: bool) -> String {
    let features = if richer {
        r#"[
    "Win32_Foundation",
    "Win32_System_SystemInformation",
    "Win32_System_Threading",
    "Win32_System_ProcessStatus",
    "Win32_UI_WindowsAndMessaging",
    "Win32_Storage_FileSystem",
]"#
    } else {
        r#"[
    "Win32_Foundation",
    "Win32_System_SystemInformation",
    "Win32_System_Threading",
    "Win32_UI_WindowsAndMessaging",
]"#
    };
    format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
windows = {{ version = "0.58", features = {features} }}

[profile.release]
lto = true
codegen-units = 1
strip = "symbols"
"#,
    )
}

pub(super) fn render_ffi_package_json(name: &str) -> String {
    format!(
        r#"{{
  "name": "{name}",
  "version": "0.1.0",
  "type": "module",
  "scripts": {{
    "build": "powershell -NoProfile -File ./build.ps1 || ./build.sh",
    "build:win": "powershell -NoProfile -File ./build.ps1",
    "build:cross": "./build.sh",
    "start": "bun run index.ts"
  }},
  "devDependencies": {{
    "@types/bun": "latest"
  }},
  "engines": {{
    "bun": ">=1.2.0"
  }}
}}
"#,
    )
}

pub(super) fn render_ffi_index_ts(name: &str) -> String {
    format!(
        r#"// Charge le cdylib Windows via bun:ffi dlopen.
// La .dll est produite par ./build.ps1 (natif) ou ./build.sh (cross).

import {{ dlopen, FFIType, suffix }} from "bun:ffi";

const {{ u32, u32: U32 }} = FFIType;

// Sur Windows, suffix === "dll". Sur Linux (debug/test), .so est produit.
const libPath = `./lib/{name}_ffi.${{suffix}}`;

const lib = dlopen(libPath, {{
  add: {{ args: [u32, u32], returns: u32 }},
  tick_count: {{ args: [], returns: u32 }},
  process_id: {{ args: [], returns: u32 }},
  page_size: {{ args: [], returns: u32 }},
  number_of_processors: {{ args: [], returns: u32 }},
}});

export const add = (a: number, b: number) => lib.symbols.add(a, b);
export const tickCount = () => lib.symbols.tick_count();
export const processId = () => lib.symbols.process_id();
export const pageSize = () => lib.symbols.page_size();
export const numberOfProcessors = () => lib.symbols.number_of_processors();
"#,
    )
}

pub(super) fn render_ffi_index_ts_simple(name: &str) -> String {
    format!(
        r#"import {{ dlopen, FFIType, suffix }} from "bun:ffi";
const {{ u32 }} = FFIType;

const lib = dlopen(`./{name}.${{suffix}}`, {{
  add: {{ args: [u32, u32], returns: u32 }},
  tick_count: {{ args: [], returns: u32 }},
  process_id: {{ args: [], returns: u32 }},
}});

export const add = (a: number, b: number) => lib.symbols.add(a, b);
export const tickCount = () => lib.symbols.tick_count();
export const processId = () => lib.symbols.process_id();
"#,
    )
}

// ---------------------------------------------------------------------------
// Utilitaire PATH
// ---------------------------------------------------------------------------

pub(super) fn which(name: &str) -> Result<PathBuf> {
    let path = std::env::var_os("PATH").ok_or_else(|| anyhow!("PATH non défini"))?;
    for dir in std::env::split_paths(&path) {
        let p = dir.join(name);
        if p.is_file() {
            return Ok(p);
        }
        let p_exe = dir.join(format!("{name}.exe"));
        if p_exe.is_file() {
            return Ok(p_exe);
        }
    }
    Err(anyhow!("binaire `{name}` introuvable dans PATH"))
}
