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

//! Templates de scaffolding pour `n2b win32`.
//!
//! Contient toutes les constantes de templates string et les fonctions
//! `render_*` spécifiques à chaque flavor.

// ---------------------------------------------------------------------------
// Renderers package.json
// ---------------------------------------------------------------------------

pub(super) fn render_cc_package_json(name: &str) -> String {
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

pub(super) fn render_pwsh_package_json(name: &str) -> String {
    format!(
        r#"{{
  "name": "{name}",
  "version": "0.1.0",
  "type": "module",
  "scripts": {{
    "start": "bun run sysinfo.ts",
    "ops": "bun run ops.ts"
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

pub(super) fn render_all_package_json(name: &str) -> String {
    format!(
        r#"{{
  "name": "{name}",
  "version": "0.1.0",
  "type": "module",
  "scripts": {{
    "build": "powershell -NoProfile -File ./build.ps1 || ./build.sh",
    "build:win": "powershell -NoProfile -File ./build.ps1",
    "build:cross": "./build.sh",
    "start": "bun run src/main.ts",
    "ffi": "bun run src/ffi.ts",
    "cc": "bun run src/cc.ts",
    "pwsh": "bun run src/pwsh.ts",
    "sysinfo": "bun run scripts/sysinfo.ts",
    "ops": "bun run scripts/ops.ts"
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

// ---------------------------------------------------------------------------
// Renderers README / build scripts
// ---------------------------------------------------------------------------

pub(super) fn render_all_readme(name: &str) -> String {
    format!(
        r#"# {name}

Projet Windows-native complet : Bun + Rust FFI (windows-rs) + C inline
+ Bun Shell (PowerShell 7).

## Structure

```
{name}/
├── crates/ffi/         → Rust cdylib (windows 0.58, GetTickCount, MessageBoxW…)
├── c/                  → Fichiers C compilés à runtime par TinyCC (<windows.h>)
├── scripts/            → Scripts Bun Shell utilisant pwsh.exe
├── src/
│   ├── ffi.ts          → dlopen({name}_ffi.dll) via bun:ffi
│   ├── cc.ts           → cc() compile winapi.c à runtime
│   ├── pwsh.ts         → Wrapper typé PowerShell via $`pwsh -Command …`
│   └── main.ts         → Orchestration
├── build.ps1           → Build natif Windows (cargo build --release)
├── build.sh            → Cross-compile depuis Linux (mingw ou cargo-xwin)
└── cross-compile.md    → Guide cross-compilation
```

## Build

### Natif sur Windows
```powershell
./build.ps1
bun run src/main.ts
```

### Cross-compile depuis Linux / macOS
```bash
# Option 1 : MinGW (GNU ABI) — simple, pas de dep Windows
rustup target add x86_64-pc-windows-gnu
sudo apt install mingw-w64
./build.sh

# Option 2 : cargo-xwin (MSVC ABI) — compatible .dll Windows officielles
cargo install cargo-xwin
cargo xwin build --release --target x86_64-pc-windows-msvc
```

Voir `cross-compile.md` pour le détail.

## Tester sur Linux sans Windows

Le scaffold compile via `build.sh` et produit un `.dll`. Pour l'exécuter,
utilisez Wine :

```bash
wine64 bun.exe src/main.ts    # avec une version Windows de Bun
```

Ou copiez le projet + `.dll` vers une VM/machine Windows.

## Docs

- [bun:ffi](https://bun.com/docs/runtime/ffi)
- [bun:cc](https://bun.com/docs/runtime/c-compiler)
- [Bun Shell](https://bun.com/docs/runtime/shell)
- [Bun Node-API](https://bun.com/docs/runtime/node-api)
- [microsoft/windows-rs](https://github.com/microsoft/windows-rs)
- [Rust for Windows](https://learn.microsoft.com/fr-fr/windows/dev-environment/rust/rust-for-windows)
- [Win32 API reference](https://learn.microsoft.com/fr-fr/windows/win32/api/)
- [Building Bun on Windows](https://bun.com/docs/project/building-windows)
"#,
    )
}

pub(super) fn render_build_ps1(name: &str) -> String {
    format!(
        r#"# Build natif Windows via cargo (MSVC).
# Requiert : VS 2022 Community + "Desktop Development with C++" + rustc.

$ErrorActionPreference = "Stop"

Push-Location "$PSScriptRoot/crates/ffi"
cargo build --release
Pop-Location

New-Item -ItemType Directory -Force -Path "$PSScriptRoot/lib" | Out-Null
Copy-Item "$PSScriptRoot/crates/ffi/target/release/{name}_ffi.dll" `
  -Destination "$PSScriptRoot/lib/{name}_ffi.dll" -Force

Write-Host "✓ lib/{name}_ffi.dll built"
"#,
    )
}

pub(super) fn render_build_ps1_simple(name: &str) -> String {
    format!(
        r#"# Build natif Windows via cargo (MSVC).
$ErrorActionPreference = "Stop"
cargo build --release
Copy-Item "target/release/{name}.dll" -Destination "./{name}.dll" -Force
Write-Host "✓ {name}.dll built"
"#,
    )
}

pub(super) fn render_build_sh_cross(name: &str) -> String {
    format!(
        r#"#!/usr/bin/env bash
# Cross-compile depuis Linux/macOS vers Windows.
# Essaie cargo-xwin (MSVC ABI) d'abord, fallback MinGW (GNU ABI).
set -euo pipefail

mkdir -p lib
cd crates/ffi

if command -v cargo-xwin >/dev/null 2>&1; then
    echo "▶ cargo-xwin build (MSVC ABI)"
    cargo xwin build --release --target x86_64-pc-windows-msvc
    cp target/x86_64-pc-windows-msvc/release/{name}_ffi.dll ../../lib/
elif rustup target list --installed | grep -q x86_64-pc-windows-gnu; then
    echo "▶ cargo build --target x86_64-pc-windows-gnu (MinGW ABI)"
    cargo build --release --target x86_64-pc-windows-gnu
    cp target/x86_64-pc-windows-gnu/release/{name}_ffi.dll ../../lib/
else
    echo "✗ Ni cargo-xwin ni la target x86_64-pc-windows-gnu ne sont disponibles."
    echo "  Installer :"
    echo "    cargo install cargo-xwin                              # MSVC ABI (recommandé)"
    echo "    rustup target add x86_64-pc-windows-gnu && apt install mingw-w64"
    exit 1
fi

echo "✓ lib/{name}_ffi.dll built"
"#,
    )
}

pub(super) fn render_build_sh_simple(name: &str) -> String {
    format!(
        r#"#!/usr/bin/env bash
set -euo pipefail
if command -v cargo-xwin >/dev/null 2>&1; then
    cargo xwin build --release --target x86_64-pc-windows-msvc
    cp target/x86_64-pc-windows-msvc/release/{name}.dll ./{name}.dll
else
    cargo build --release --target x86_64-pc-windows-gnu
    cp target/x86_64-pc-windows-gnu/release/{name}.dll ./{name}.dll
fi
echo "✓ {name}.dll built"
"#,
    )
}

pub(super) fn render_main_ts(name: &str) -> String {
    format!(
        r#"// Orchestration Bun + Win32 : FFI cdylib + inline C + PowerShell.

import {{ add, tickCount, processId, numberOfProcessors }} from "./ffi.ts";
import {{ answer, currentPid }} from "./cc.ts";
import {{ osName, cpuName, ramGB }} from "./pwsh.ts";

console.log("[{name}]");
console.log("");
console.log("── Rust FFI (windows-rs) ──");
console.log("  add(2, 3)               =", add(2, 3));
console.log("  GetTickCount()          =", tickCount(), "ms since boot");
console.log("  GetCurrentProcessId()   =", processId());
console.log("  nb processors           =", numberOfProcessors());
console.log("");
console.log("── Inline C (TinyCC) ──");
console.log("  answer()                =", answer());
console.log("  GetCurrentProcessId()   =", currentPid());
console.log("");
console.log("── PowerShell via Bun Shell ──");
console.log("  OS                      =", await osName());
console.log("  CPU                     =", await cpuName());
console.log("  RAM                     =", await ramGB(), "GB");
"#,
    )
}

pub(super) fn readme(name: &str, title: &str) -> String {
    format!(
        r#"# {name}

{title}

Scaffolded by `n2b win32`.

## Build & Run

### Natif sur Windows
```powershell
bun install
./build.ps1
bun run start
```

### Cross-compile depuis Linux / macOS
```bash
bun install
./build.sh
bun run start     # avec Bun for Windows (via Wine si besoin)
```

## Docs

- https://github.com/microsoft/windows-rs
- https://bun.com/docs/runtime/ffi
- https://bun.com/docs/runtime/c-compiler
- https://bun.com/docs/runtime/shell
- https://learn.microsoft.com/fr-fr/windows/win32/api/
"#,
    )
}

// ---------------------------------------------------------------------------
// Templates de code (constantes string)
// ---------------------------------------------------------------------------

pub(super) const FFI_LIB_RS_SIMPLE: &str = r#"//! Rust cdylib Windows minimal : add + GetTickCount + GetCurrentProcessId.
//!
//! Compilation :
//!   Windows    : cargo build --release
//!   Linux/mac  : cargo xwin build --release --target x86_64-pc-windows-msvc
//!              : ou cargo build --release --target x86_64-pc-windows-gnu

use std::os::raw::c_uint;
use windows::Win32::System::SystemInformation::GetTickCount;
use windows::Win32::System::Threading::GetCurrentProcessId;

#[no_mangle]
pub extern "C" fn add(a: c_uint, b: c_uint) -> c_uint {
    a + b
}

#[no_mangle]
pub extern "C" fn tick_count() -> c_uint {
    unsafe { GetTickCount() }
}

#[no_mangle]
pub extern "C" fn process_id() -> c_uint {
    unsafe { GetCurrentProcessId() }
}
"#;

pub(super) const FFI_LIB_RS_COMPLETE: &str = r#"//! Rust cdylib riche : Win32 exposé via la crate `windows` (Microsoft).
//!
//! Features Cargo activées :
//!   Win32_Foundation, Win32_System_{SystemInformation,Threading,ProcessStatus},
//!   Win32_UI_WindowsAndMessaging, Win32_Storage_FileSystem.

use std::os::raw::{c_char, c_uint};
use windows::core::PCWSTR;
use windows::Win32::System::SystemInformation::{
    GetSystemInfo, GetTickCount, SYSTEM_INFO,
};
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONINFORMATION, MB_OK};

#[no_mangle]
pub extern "C" fn add(a: c_uint, b: c_uint) -> c_uint {
    a + b
}

#[no_mangle]
pub extern "C" fn tick_count() -> c_uint {
    unsafe { GetTickCount() }
}

#[no_mangle]
pub extern "C" fn process_id() -> c_uint {
    unsafe { GetCurrentProcessId() }
}

#[no_mangle]
pub extern "C" fn page_size() -> c_uint {
    let mut info = SYSTEM_INFO::default();
    unsafe { GetSystemInfo(&mut info) };
    info.dwPageSize
}

#[no_mangle]
pub extern "C" fn number_of_processors() -> c_uint {
    let mut info = SYSTEM_INFO::default();
    unsafe { GetSystemInfo(&mut info) };
    info.dwNumberOfProcessors
}

/// Affiche une MessageBox. `text` et `title` sont des pointeurs UTF-16 nul-terminés
/// côté appelant ; passer un pointeur Bun (via CString) nécessite une conversion
/// UTF-8 → UTF-16. Exposée à titre de démo ; prefer l'appeler depuis code Rust.
///
/// # Safety
/// `text` et `title` doivent être des pointeurs UTF-16 nul-terminés valides.
#[no_mangle]
pub unsafe extern "C" fn message_box_w(text: *const u16, title: *const u16) -> c_uint {
    unsafe {
        MessageBoxW(
            None,
            PCWSTR(text),
            PCWSTR(title),
            MB_OK | MB_ICONINFORMATION,
        )
        .0 as c_uint
    }
}

// Évite un warning sur c_char inutilisé.
#[allow(dead_code)]
fn _unused(_: c_char) {}
"#;

pub(super) const CC_HELLO_C: &str = r#"// Fichier C compilé à runtime par Bun (TinyCC).

int answer(void) {
    return 42;
}

int add(int a, int b) {
    return a + b;
}
"#;

pub(super) const CC_WINAPI_C: &str = r#"// Appels Win32 depuis C compilé par TinyCC à runtime.
// Les headers <windows.h> sont fournis par le compilateur embarqué.

#include <windows.h>

unsigned long current_pid(void) {
    return GetCurrentProcessId();
}

unsigned long tick_count(void) {
    return GetTickCount();
}

// Exemple : ouvrir une MessageBox. Blocage UI — uniquement en démo.
int message_box(void) {
    return MessageBoxA(NULL, "Hello from inline C!", "n2b win32 cc", MB_OK);
}
"#;

pub(super) const CC_INDEX_TS: &str = r#"// Compile du C Win32 à runtime et appelle les fonctions depuis Bun.

import { cc } from "bun:ffi";
import hello from "./hello.c" with { type: "file" };
import winapi from "./winapi.c" with { type: "file" };

const helloLib = cc({
  source: hello,
  symbols: {
    answer: { args: [], returns: "int" },
    add: { args: ["int", "int"], returns: "int" },
  },
});

const winLib = cc({
  source: winapi,
  symbols: {
    current_pid: { args: [], returns: "u32" },
    tick_count: { args: [], returns: "u32" },
    // message_box: { args: [], returns: "int" }, // décommente pour pop-up
  },
});

export const answer = () => helloLib.symbols.answer();
export const add = (a: number, b: number) => helloLib.symbols.add(a, b);
export const currentPid = () => winLib.symbols.current_pid();
export const tickCount = () => winLib.symbols.tick_count();
"#;

pub(super) const PWSH_SYSINFO_TS: &str = r#"// Info système Windows via Bun Shell + PowerShell 7.
// Doc : https://bun.com/docs/runtime/shell

import { $ } from "bun";

const PS = "powershell";  // ou "pwsh" si PowerShell 7 installé

async function run(cmd: string): Promise<string> {
  return (await $`${PS} -NoProfile -NonInteractive -Command ${cmd}`.text()).trim();
}

console.log("── Windows system overview ──");
console.log("OS      :", await run("(Get-CimInstance Win32_OperatingSystem).Caption"));
console.log("Version :", await run("(Get-CimInstance Win32_OperatingSystem).Version"));
console.log("CPU     :", await run("(Get-CimInstance Win32_Processor).Name"));
console.log(
  "RAM     :",
  await run("[math]::Round((Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory/1GB,1)"),
  "GB",
);
console.log(
  "Disk C: :",
  await run(
    "[math]::Round((Get-PSDrive C).Used/1GB,1).ToString() + '/' + [math]::Round(((Get-PSDrive C).Used+(Get-PSDrive C).Free)/1GB,1).ToString()",
  ),
  "GB",
);
console.log("Uptime  :", await run("((Get-Date) - (Get-CimInstance Win32_OperatingSystem).LastBootUpTime).ToString()"));

console.log("\n── Top 5 processus par CPU ──");
await $`${PS} -NoProfile -Command "Get-Process | Sort-Object CPU -Descending | Select-Object -First 5 Name,Id,CPU | Format-Table"`;
"#;

pub(super) const PWSH_OPS_TS: &str = r#"// Exemples d'opérations admin Windows via Bun Shell.

import { $ } from "bun";

// Liste les services en running
console.log("── Services actifs ──");
await $`powershell -NoProfile -Command "Get-Service | Where-Object Status -eq 'Running' | Select -First 10 Name,DisplayName"`;

// Événements récents (journal Application)
console.log("\n── Derniers événements Application (5) ──");
await $`powershell -NoProfile -Command "Get-EventLog -LogName Application -Newest 5 | Format-Table TimeGenerated,Source,EntryType -Auto"`;

// Variables d'environnement critiques
console.log("\n── Env ──");
console.log("USERPROFILE :", Bun.env.USERPROFILE);
console.log("PATH (head) :", (Bun.env.PATH ?? "").split(";").slice(0, 5).join(";"));
"#;

pub(super) const PWSH_INDEX_TS: &str = r#"// Wrapper typé pour les 3 infos système basiques (utilisé par main.ts).

import { $ } from "bun";

const PS = "powershell";

async function run(cmd: string): Promise<string> {
  return (await $`${PS} -NoProfile -Command ${cmd}`.text()).trim();
}

export const osName   = () => run("(Get-CimInstance Win32_OperatingSystem).Caption");
export const cpuName  = () => run("(Get-CimInstance Win32_Processor).Name");
export const ramGB    = async () =>
  Number(await run("[math]::Round((Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory/1GB,1)"));
"#;

pub(super) const CROSS_COMPILE_MD: &str = r#"# Cross-compilation Linux/macOS → Windows

Deux approches supportées par ce scaffold :

## Option 1 — cargo-xwin (MSVC ABI, recommandé)

Reproduit exactement ce que produit Visual Studio :

```bash
cargo install cargo-xwin
rustup target add x86_64-pc-windows-msvc
cargo xwin build --release --target x86_64-pc-windows-msvc
```

cargo-xwin télécharge automatiquement les headers Windows SDK + CRT MSVC
sous licence (accepte l'EULA au premier run).

**Avantages** : ABI MSVC native, compatible avec .dll Windows officielles,
linkable par code compilé avec `cl.exe`.

## Option 2 — MinGW-w64 (GNU ABI)

Plus simple, pas de téléchargement EULA :

```bash
rustup target add x86_64-pc-windows-gnu
sudo apt install mingw-w64   # Debian/Ubuntu
brew install mingw-w64       # macOS
cargo build --release --target x86_64-pc-windows-gnu
```

**Avantages** : setup trivial, pas d'accord EULA.
**Limitations** : ABI différente (GNU), quelques incompat rares avec crates
qui link contre MSVCRT.

## Tester sans machine Windows

1. **Wine64** (couvre 90% des cas FFI Bun) :
   ```bash
   apt install wine
   wine64 bun.exe src/main.ts
   ```

2. **VM / Docker Windows** : GitHub Actions `windows-latest` runner est
   gratuit pour les repos publics — utile pour CI.

3. **Remote** : Azure DevBox, GitHub Codespaces avec image Windows.

## Refs

- https://github.com/rust-cross/cargo-xwin
- https://rust-lang.github.io/rustup/cross-compilation.html
- https://bun.com/docs/project/building-windows
"#;

pub(super) const GITIGNORE: &str = r#"target/
node_modules/
*.dll
*.so
*.dylib
lib/
dist/
.DS_Store
"#;
