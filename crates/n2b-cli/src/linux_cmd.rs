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

//! `n2b linux <sub>` — scaffolde des projets Bun + Linux bas-niveau.
//!
//! Couvre les 3 voies d'interop Bun ↔ code natif Linux :
//!   - `ffi`    : Rust cdylib → Bun `dlopen` (bun:ffi)
//!   - `cc`     : inline C compilé à runtime par TinyCC (bun:ffi / Bun.cc)
//!   - `shell`  : scripts Bun shell `$\`cmd\`` pour ops système
//!   - `init`   : projet complet mêlant les 3 approches
//!   - `doctor` : vérifie gcc / clang / pkg-config / rustc / tinycc / libc-dev
//!
//! Refs :
//!   - https://bun.com/docs/runtime/ffi
//!   - https://bun.com/docs/runtime/c-compiler
//!   - https://bun.com/docs/runtime/shell
//!   - https://bun.com/docs/runtime/node-api
//!   - https://rust-for-linux.com/
//!   - https://github.com/rust-lang/libc

use anyhow::{Context, Result, anyhow};
use std::path::{Path, PathBuf};

pub enum LinuxCmd {
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
    Shell {
        name: String,
        dir: Option<PathBuf>,
        force: bool,
    },
    Doctor,
}

pub fn run(cmd: LinuxCmd, quiet: bool) -> Result<()> {
    match cmd {
        LinuxCmd::Init { name, dir, force } => init_all(name, dir, force, quiet),
        LinuxCmd::Ffi { name, dir, force } => scaffold(name, dir, force, quiet, Flavor::Ffi),
        LinuxCmd::Cc { name, dir, force } => scaffold(name, dir, force, quiet, Flavor::Cc),
        LinuxCmd::Shell { name, dir, force } => scaffold(name, dir, force, quiet, Flavor::Shell),
        LinuxCmd::Doctor => doctor(quiet),
    }
}

enum Flavor {
    Ffi,
    Cc,
    Shell,
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
        Flavor::Shell => scaffold_shell(&target, &name, quiet)?,
    }
    if !quiet {
        eprintln!(
            "[linux] ✓ {name} scaffolded → cd {} && bun install && bun run build",
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

    // FFI (crate Rust)
    write(
        target.join("crates/ffi/Cargo.toml"),
        &render_ffi_cargo_toml(&format!("{name}_ffi")),
        quiet,
    )?;
    write(target.join("crates/ffi/src/lib.rs"), FFI_LIB_RS, quiet)?;

    // CC (fichier C inline)
    write(target.join("c/hello.c"), CC_HELLO_C, quiet)?;
    write(target.join("c/syscalls.c"), CC_SYSCALLS_C, quiet)?;

    // Shell script Bun
    write(target.join("scripts/sysinfo.ts"), SHELL_SYSINFO_TS, quiet)?;

    // Entrypoints
    write(target.join("src/ffi.ts"), FFI_INDEX_TS, quiet)?;
    write(target.join("src/cc.ts"), CC_INDEX_TS, quiet)?;
    write(target.join("src/main.ts"), &render_main_ts(&name), quiet)?;

    // Package + build
    write(
        target.join("package.json"),
        &render_all_package_json(&name),
        quiet,
    )?;
    write(target.join("README.md"), &render_all_readme(&name), quiet)?;
    write(target.join(".gitignore"), GITIGNORE, quiet)?;
    write(target.join("build.sh"), &render_build_script(&name), quiet)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(target.join("build.sh")) {
            let mut perms = meta.permissions();
            perms.set_mode(0o755);
            let _ = std::fs::set_permissions(target.join("build.sh"), perms);
        }
    }

    if !quiet {
        eprintln!(
            "[linux init] ✓ {name} complet → cd {} && ./build.sh && bun run src/main.ts",
            target.display()
        );
    }
    Ok(())
}

fn scaffold_ffi(dir: &Path, name: &str, quiet: bool) -> Result<()> {
    write(dir.join("Cargo.toml"), &render_ffi_cargo_toml(name), quiet)?;
    write(dir.join("src/lib.rs"), FFI_LIB_RS, quiet)?;
    write(
        dir.join("package.json"),
        &render_ffi_package_json(name),
        quiet,
    )?;
    write(dir.join("index.ts"), FFI_INDEX_TS, quiet)?;
    write(
        dir.join("README.md"),
        &readme(name, "Rust cdylib → Bun FFI (bun:ffi dlopen)"),
        quiet,
    )?;
    write(dir.join(".gitignore"), GITIGNORE, quiet)?;
    Ok(())
}

fn scaffold_cc(dir: &Path, name: &str, quiet: bool) -> Result<()> {
    write(dir.join("hello.c"), CC_HELLO_C, quiet)?;
    write(dir.join("syscalls.c"), CC_SYSCALLS_C, quiet)?;
    write(
        dir.join("package.json"),
        &render_cc_package_json(name),
        quiet,
    )?;
    write(dir.join("index.ts"), CC_INDEX_TS, quiet)?;
    write(
        dir.join("README.md"),
        &readme(name, "Inline C compilé à runtime par TinyCC (bun:ffi `cc`)"),
        quiet,
    )?;
    write(dir.join(".gitignore"), GITIGNORE, quiet)?;
    Ok(())
}

fn scaffold_shell(dir: &Path, name: &str, quiet: bool) -> Result<()> {
    write(
        dir.join("package.json"),
        &render_shell_package_json(name),
        quiet,
    )?;
    write(dir.join("sysinfo.ts"), SHELL_SYSINFO_TS, quiet)?;
    write(
        dir.join("README.md"),
        &readme(name, "Ops système via Bun Shell (`$\\`cmd\\``)"),
        quiet,
    )?;
    write(dir.join(".gitignore"), GITIGNORE, quiet)?;
    Ok(())
}

fn doctor(quiet: bool) -> Result<()> {
    let tools: &[(&str, &str, &str)] = &[
        (
            "rustc",
            "curl https://sh.rustup.rs -sSf | sh",
            "compilateur Rust",
        ),
        ("cargo", "(installé avec rustup)", "package manager Rust"),
        (
            "gcc",
            "sudo apt install build-essential",
            "compilateur C (Bun.cc fallback)",
        ),
        (
            "clang",
            "sudo apt install clang",
            "compilateur C/C++ alternatif",
        ),
        (
            "tcc",
            "sudo apt install tcc",
            "TinyCC — utilisé par bun:ffi cc (intégré à Bun)",
        ),
        (
            "pkg-config",
            "sudo apt install pkg-config",
            "découverte de libs natives",
        ),
        (
            "make",
            "sudo apt install build-essential",
            "build classique",
        ),
        (
            "objdump",
            "sudo apt install binutils",
            "inspection ELF / symboles",
        ),
        ("ldd", "(glibc)", "inspection deps dynamiques"),
        (
            "strace",
            "sudo apt install strace",
            "trace syscalls (debug FFI)",
        ),
        // --- Unix CLI rewrites Rust (uutils) — drop-in modernes pour dev / scripts ---
        (
            "coreutils",
            "cargo install coreutils",
            "uutils/coreutils (ls/cp/cat/… Rust)",
        ),
        (
            "findutils",
            "cargo install findutils",
            "uutils/findutils (find/xargs Rust)",
        ),
        (
            "diffutils",
            "cargo install diffutils",
            "uutils/diffutils (diff/cmp Rust)",
        ),
        (
            "procps",
            "cargo install procps",
            "uutils/procps (ps/top/watch Rust)",
        ),
        (
            "util-linux-rs",
            "cargo install --git https://github.com/uutils/util-linux",
            "uutils/util-linux (mount/fdisk/lscpu/dmesg Rust)",
        ),
    ];
    let mut missing = 0;
    for (bin, install, desc) in tools {
        let ok = which(bin).is_ok();
        if !quiet {
            let mark = if ok { "✓" } else { "✗" };
            println!("  {mark} {:<13} {desc}", bin);
            if !ok {
                println!("       install: {install}");
                missing += 1;
            }
        }
    }
    if missing > 0 && !quiet {
        eprintln!("\n{missing} outil(s) manquant(s)");
    }
    Ok(())
}

// --- Renderers ---

fn render_ffi_cargo_toml(name: &str) -> String {
    format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
libc = "0.2"

[profile.release]
lto = true
codegen-units = 1
strip = "symbols"
"#,
    )
}

fn render_ffi_package_json(name: &str) -> String {
    format!(
        r#"{{
  "name": "{name}",
  "version": "0.1.0",
  "type": "module",
  "scripts": {{
    "build": "cargo build --release && cp target/release/lib{name}.so ./lib{name}.so",
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

fn render_cc_package_json(name: &str) -> String {
    format!(
        r#"{{
  "name": "{name}",
  "version": "0.1.0",
  "type": "module",
  "scripts": {{
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

fn render_shell_package_json(name: &str) -> String {
    format!(
        r#"{{
  "name": "{name}",
  "version": "0.1.0",
  "type": "module",
  "scripts": {{
    "start": "bun run sysinfo.ts"
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

fn render_all_package_json(name: &str) -> String {
    format!(
        r#"{{
  "name": "{name}",
  "version": "0.1.0",
  "type": "module",
  "scripts": {{
    "build": "./build.sh",
    "start": "bun run src/main.ts",
    "ffi": "bun run src/ffi.ts",
    "cc": "bun run src/cc.ts",
    "sysinfo": "bun run scripts/sysinfo.ts"
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

fn render_all_readme(name: &str) -> String {
    format!(
        r#"# {name}

Projet Linux-native complet : Bun + Rust FFI + C inline + Bun Shell.

## Structure

```
{name}/
├── crates/ffi/       → Rust cdylib (libc, syscalls, algorithmes rapides)
├── c/                → Fichiers C compilés à runtime par TinyCC
├── scripts/          → Scripts Bun Shell (ops système)
├── src/
│   ├── ffi.ts        → Appel du cdylib Rust via bun:ffi dlopen
│   ├── cc.ts         → Appel de C inline via bun:ffi cc
│   └── main.ts       → Orchestration
└── build.sh          → cargo build --release + copie .so
```

## Build

```bash
./build.sh         # compile le cdylib Rust
bun run src/main.ts
```

## Refs

- [Bun FFI](https://bun.com/docs/runtime/ffi)
- [Bun C compiler](https://bun.com/docs/runtime/c-compiler)
- [Bun Shell](https://bun.com/docs/runtime/shell)
- [Bun Node-API](https://bun.com/docs/runtime/node-api)
- [rust-lang/libc](https://github.com/rust-lang/libc)
- [rust-for-linux.com](https://rust-for-linux.com/)
"#,
    )
}

fn render_build_script(name: &str) -> String {
    format!(
        r#"#!/usr/bin/env bash
set -euo pipefail

# Build le cdylib Rust et copie le .so là où Bun FFI le cherche.
cd crates/ffi
cargo build --release
cd ../..

mkdir -p lib
cp crates/ffi/target/release/lib{name}_ffi.so lib/

echo "✓ lib/lib{name}_ffi.so built"
"#,
    )
}

fn render_main_ts(name: &str) -> String {
    format!(
        r#"// Orchestration : appelle FFI, CC et shell dans un pipeline.

import {{ add, getpid }} from "./ffi.ts";
import {{ answer }} from "./cc.ts";
import {{ $ }} from "bun";

console.log("[{name}]");
console.log("  Rust FFI add(2,3) =", add(2, 3));
console.log("  Rust FFI getpid() =", getpid());
console.log("  C inline answer() =", answer());

const uname = await $`uname -a`.text();
console.log("  shell uname        =", uname.trim());
"#,
    )
}

fn readme(name: &str, title: &str) -> String {
    format!(
        r#"# {name}

{title}

Scaffolded by `n2b linux`.

## Build & Run

```bash
bun install
bun run build 2>/dev/null || true
bun run start
```

## Docs

- https://bun.com/docs/runtime/ffi
- https://bun.com/docs/runtime/c-compiler
- https://bun.com/docs/runtime/shell
"#,
    )
}

fn write(path: PathBuf, content: &str, quiet: bool) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    std::fs::write(&path, content).with_context(|| format!("write {}", path.display()))?;
    if !quiet {
        eprintln!("[linux]   + {}", path.display());
    }
    Ok(())
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

// --- Templates ---

const FFI_LIB_RS: &str = r#"//! Rust cdylib exposant des fonctions C-ABI pour Bun.
//!
//! Chaque fn est `#[no_mangle] pub extern "C"` pour être appelable depuis
//! `dlopen` côté Bun. Doc: https://bun.com/docs/runtime/ffi

use libc::{self, c_int};

#[no_mangle]
pub extern "C" fn add(a: c_int, b: c_int) -> c_int {
    a + b
}

/// PID du processus courant (libc::getpid).
#[no_mangle]
pub extern "C" fn get_pid() -> c_int {
    unsafe { libc::getpid() }
}

/// Nombre de CPU logiques (libc::sysconf(_SC_NPROCESSORS_ONLN)).
#[no_mangle]
pub extern "C" fn cpu_count() -> c_int {
    unsafe { libc::sysconf(libc::_SC_NPROCESSORS_ONLN) as c_int }
}

/// Taille de page mémoire (libc::sysconf(_SC_PAGESIZE)).
#[no_mangle]
pub extern "C" fn page_size() -> c_int {
    unsafe { libc::sysconf(libc::_SC_PAGESIZE) as c_int }
}
"#;

const FFI_INDEX_TS: &str = r#"// bun:ffi dlopen du cdylib Rust.
// Le fichier libX.so est produit par `cargo build --release` + copie.

import { dlopen, FFIType, suffix } from "bun:ffi";

const { i32 } = FFIType;

// Résout lib*.so|dylib|dll selon l'OS. En mode n2b linux init, on utilise
// ./lib/libX_ffi.so ; en mode --ffi simple, ./libX.so.
const libPath = `./lib/lib${Bun.env.LIB ?? "ffi"}_ffi.${suffix}`;

const lib = dlopen(libPath, {
  add: { args: [i32, i32], returns: i32 },
  get_pid: { args: [], returns: i32 },
  cpu_count: { args: [], returns: i32 },
  page_size: { args: [], returns: i32 },
});

export const add = (a: number, b: number) => lib.symbols.add(a, b);
export const getpid = () => lib.symbols.get_pid();
export const cpuCount = () => lib.symbols.cpu_count();
export const pageSize = () => lib.symbols.page_size();
"#;

const CC_HELLO_C: &str = r#"// Fichier C compilé à runtime par Bun (TinyCC).
// Pas d'étape de build — c'est compilé au premier appel.

int answer(void) {
    return 42;
}

int add(int a, int b) {
    return a + b;
}
"#;

const CC_SYSCALLS_C: &str = r#"// Exemples de syscalls Linux appelés via libc (standard headers).
// Compilés par TinyCC — les headers sont résolus par le compilateur embarqué.

#include <sys/types.h>
#include <unistd.h>
#include <time.h>

long current_pid(void) {
    return (long)getpid();
}

long current_time(void) {
    return (long)time(0);
}
"#;

const CC_INDEX_TS: &str = r#"// Compile du C à runtime et appelle des fonctions depuis Bun.
// Doc: https://bun.com/docs/runtime/c-compiler

import { cc } from "bun:ffi";
import hello from "./hello.c" with { type: "file" };
import syscalls from "./syscalls.c" with { type: "file" };

const helloLib = cc({
  source: hello,
  symbols: {
    answer: { args: [], returns: "int" },
    add: { args: ["int", "int"], returns: "int" },
  },
});

const sysLib = cc({
  source: syscalls,
  symbols: {
    current_pid: { args: [], returns: "long" },
    current_time: { args: [], returns: "long" },
  },
});

export const answer = () => helloLib.symbols.answer();
export const add = (a: number, b: number) => helloLib.symbols.add(a, b);
export const currentPid = () => Number(sysLib.symbols.current_pid());
export const currentTime = () => Number(sysLib.symbols.current_time());
"#;

const SHELL_SYSINFO_TS: &str = r#"// Ops système via Bun Shell. Aucune dep, entièrement async-await.
// Doc: https://bun.com/docs/runtime/shell

import { $ } from "bun";

console.log("── Linux system overview ──");

const uname = (await $`uname -a`.text()).trim();
console.log("kernel  :", uname);

const uptime = (await $`uptime -p`.text()).trim();
console.log("uptime  :", uptime);

const load = (await $`cat /proc/loadavg`.text()).trim();
console.log("load    :", load);

const mem = await $`free -h | head -2 | tail -1`.text();
console.log("memory  :", mem.trim());

const disk = await $`df -h / | tail -1`.text();
console.log("disk    :", disk.trim());

// Pipeline : top 5 des processus par CPU
console.log("\n── top 5 CPU ──");
await $`ps aux --sort=-%cpu | head -6`;
"#;

const GITIGNORE: &str = r#"target/
node_modules/
*.so
*.dylib
*.dll
lib/
dist/
.DS_Store
"#;
