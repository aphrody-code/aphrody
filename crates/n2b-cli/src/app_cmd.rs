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

//! `n2b app <sub>` — scaffolde des applications Bun : CLI, TUI (Ink),
//! GUI (Electrobun), executable standalone via `bun build --compile`.
//!
//! Refs :
//!   - https://github.com/vadimdemedes/ink
//!   - https://bun.com/docs/runtime/auto-install
//!   - https://bun.com/docs/bundler/executables
//!   - https://bun.com/docs/cli/build

use anyhow::{Context, Result, anyhow};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppFlavor {
    /// CLI basic : entry TS, parse process.argv, shebang + bin dans package.json.
    Cli,
    /// TUI React via Ink (Box, Text, useInput, interactive list).
    Tui,
    /// GUI desktop via Electrobun (dep ecosystem, webview Bun+Zig).
    Gui,
    /// Entry minimal + script bun build --compile multi-targets.
    Executable,
}

impl AppFlavor {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "cli" => Some(Self::Cli),
            "tui" | "ink" => Some(Self::Tui),
            "gui" | "electrobun" => Some(Self::Gui),
            "exe" | "executable" | "compile" | "standalone" => Some(Self::Executable),
            _ => None,
        }
    }
}

pub enum AppCmd {
    Init {
        name: String,
        flavor: AppFlavor,
        dir: Option<PathBuf>,
        force: bool,
    },
    Build {
        entry: PathBuf,
        outfile: Option<PathBuf>,
        target: Option<String>,
        minify: bool,
        sourcemap: bool,
    },
    Doctor,
}

pub fn run(cmd: AppCmd, quiet: bool) -> Result<()> {
    match cmd {
        AppCmd::Init {
            name,
            flavor,
            dir,
            force,
        } => init(name, flavor, dir, force, quiet),
        AppCmd::Build {
            entry,
            outfile,
            target,
            minify,
            sourcemap,
        } => build_compile(entry, outfile, target, minify, sourcemap, quiet),
        AppCmd::Doctor => doctor(quiet),
    }
}

fn init(
    name: String,
    flavor: AppFlavor,
    dir: Option<PathBuf>,
    force: bool,
    quiet: bool,
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
        AppFlavor::Cli => scaffold_cli(&target, &name, quiet)?,
        AppFlavor::Tui => scaffold_tui(&target, &name, quiet)?,
        AppFlavor::Gui => scaffold_gui(&target, &name, quiet)?,
        AppFlavor::Executable => scaffold_executable(&target, &name, quiet)?,
    }

    if !quiet {
        eprintln!(
            "[app] ✓ {name} ({:?}) scaffolded → cd {} && bun install && bun start",
            flavor,
            target.display()
        );
    }
    Ok(())
}

fn build_compile(
    entry: PathBuf,
    outfile: Option<PathBuf>,
    target: Option<String>,
    minify: bool,
    sourcemap: bool,
    quiet: bool,
) -> Result<()> {
    if which("bun").is_err() {
        anyhow::bail!("`bun` introuvable dans PATH");
    }
    if !entry.exists() {
        anyhow::bail!("{} introuvable", entry.display());
    }

    let mut args: Vec<String> = vec![
        "build".into(),
        "--compile".into(),
        entry.display().to_string(),
    ];
    if let Some(t) = target {
        args.push("--target".into());
        args.push(t);
    }
    let outfile = outfile.unwrap_or_else(|| PathBuf::from("./dist/app"));
    args.push("--outfile".into());
    args.push(outfile.display().to_string());
    if minify {
        args.push("--minify".into());
    }
    if sourcemap {
        args.push("--sourcemap".into());
    }

    if !quiet {
        eprintln!("[app build] bun {}", args.join(" "));
    }
    let status = Command::new("bun")
        .args(&args)
        .status()
        .context("exécution bun build --compile")?;
    if !status.success() {
        anyhow::bail!("bun build a échoué (exit {})", status);
    }
    if !quiet {
        if let Ok(meta) = std::fs::metadata(&outfile) {
            let size_mb = meta.len() as f64 / 1024.0 / 1024.0;
            eprintln!("[app build] ✓ {} ({:.1} MB)", outfile.display(), size_mb);
        }
    }
    Ok(())
}

fn doctor(quiet: bool) -> Result<()> {
    let tools: &[(&str, &str, &str)] = &[
        (
            "bun",
            "curl -fsSL https://bun.sh/install | bash",
            "runtime + bundler Bun",
        ),
        ("tsc", "bun add -g typescript", "TS compiler (optionnel)"),
        (
            "upx",
            "sudo apt install upx",
            "compress binary (bun compile → upx)",
        ),
        // --- Unix CLIs cross-platform via uutils (utile pour scripts CLI multi-OS) ---
        (
            "coreutils",
            "cargo install coreutils",
            "uutils/coreutils (ls/cp/cat/… portables Windows/macOS/Linux)",
        ),
        (
            "findutils",
            "cargo install findutils",
            "uutils/findutils (find/xargs)",
        ),
    ];
    let mut missing = 0;
    for (bin, install, desc) in tools {
        let ok = which(bin).is_ok();
        if !quiet {
            let mark = if ok { "✓" } else { "✗" };
            println!("  {mark} {:<8} {desc}", bin);
            if !ok {
                println!("       install: {install}");
                missing += 1;
            }
        }
    }
    if !quiet && missing > 0 {
        eprintln!("\n{missing} outil(s) manquant(s)");
    }
    if !quiet {
        eprintln!("\nTargets `bun build --compile` :");
        eprintln!("  bun-linux-x64, bun-linux-arm64");
        eprintln!("  bun-darwin-x64, bun-darwin-arm64");
        eprintln!("  bun-windows-x64");
        eprintln!("\nAuto-install en prod : https://bun.com/docs/runtime/auto-install");
    }
    Ok(())
}

// --- Scaffolders ---

fn scaffold_cli(dir: &Path, name: &str, quiet: bool) -> Result<()> {
    write(dir.join("index.ts"), &render_cli_entry(name), quiet)?;
    write(
        dir.join("package.json"),
        &render_cli_package_json(name),
        quiet,
    )?;
    write(
        dir.join("README.md"),
        &readme(
            name,
            "CLI Bun minimale — argv parsing + aide + standalone executable",
        ),
        quiet,
    )?;
    write(dir.join(".gitignore"), GITIGNORE, quiet)?;
    Ok(())
}

fn scaffold_tui(dir: &Path, name: &str, quiet: bool) -> Result<()> {
    write(dir.join("src/App.tsx"), TUI_APP_TSX, quiet)?;
    write(dir.join("src/index.tsx"), TUI_INDEX_TSX, quiet)?;
    write(
        dir.join("package.json"),
        &render_tui_package_json(name),
        quiet,
    )?;
    write(dir.join("tsconfig.json"), TUI_TSCONFIG, quiet)?;
    write(
        dir.join("README.md"),
        &readme(
            name,
            "TUI React via Ink — interactive list, couleurs, keybindings",
        ),
        quiet,
    )?;
    write(dir.join(".gitignore"), GITIGNORE, quiet)?;
    Ok(())
}

fn scaffold_gui(dir: &Path, name: &str, quiet: bool) -> Result<()> {
    write(
        dir.join("src/bun/index.ts"),
        &render_gui_bun_entry(name),
        quiet,
    )?;
    write(dir.join("src/browser/index.html"), GUI_BROWSER_HTML, quiet)?;
    write(dir.join("src/browser/index.ts"), GUI_BROWSER_TS, quiet)?;
    write(dir.join("src/browser/styles.css"), GUI_BROWSER_CSS, quiet)?;
    write(dir.join("electrobun.config.ts"), GUI_CONFIG_TS, quiet)?;
    write(
        dir.join("package.json"),
        &render_gui_package_json(name),
        quiet,
    )?;
    write(
        dir.join("README.md"),
        &readme(
            name,
            "GUI desktop via Electrobun (Bun + Zig WebView, ~10× plus léger qu'Electron)",
        ),
        quiet,
    )?;
    write(dir.join(".gitignore"), GITIGNORE, quiet)?;
    Ok(())
}

fn scaffold_executable(dir: &Path, name: &str, quiet: bool) -> Result<()> {
    write(dir.join("index.ts"), &render_cli_entry(name), quiet)?;
    write(
        dir.join("package.json"),
        &render_exe_package_json(name),
        quiet,
    )?;
    write(dir.join("build-all.sh"), &render_build_all_sh(name), quiet)?;
    write(
        dir.join("README.md"),
        &readme(
            name,
            "Standalone executable via `bun build --compile` — cross-targets Linux/macOS/Windows x64+arm64",
        ),
        quiet,
    )?;
    write(dir.join(".gitignore"), GITIGNORE, quiet)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(dir.join("build-all.sh")) {
            let mut perms = meta.permissions();
            perms.set_mode(0o755);
            let _ = std::fs::set_permissions(dir.join("build-all.sh"), perms);
        }
    }
    Ok(())
}

fn write(path: PathBuf, content: &str, quiet: bool) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    std::fs::write(&path, content).with_context(|| format!("write {}", path.display()))?;
    if !quiet {
        eprintln!("[app]   + {}", path.display());
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

// --- Renderers ---

fn render_cli_entry(name: &str) -> String {
    format!(
        r#"#!/usr/bin/env bun
// CLI scaffolded by n2b app (cli flavor).
// Auto-install : bun lit package.json et installe à la volée les deps
// non résolues (https://bun.com/docs/runtime/auto-install).

import {{ parseArgs }} from "node:util";

const VERSION = "0.1.0";
const USAGE = `{name} v${{VERSION}}

Usage :
  {name} [--name <str>] [--count <n>] [-v|--verbose]
  {name} --help
  {name} --version
`;

const {{ values, positionals }} = parseArgs({{
  args: Bun.argv.slice(2),
  options: {{
    help:    {{ type: "boolean", short: "h" }},
    version: {{ type: "boolean", short: "V" }},
    verbose: {{ type: "boolean", short: "v" }},
    name:    {{ type: "string" }},
    count:   {{ type: "string" }},
  }},
  allowPositionals: true,
  strict: true,
}});

if (values.help) {{ process.stdout.write(USAGE); process.exit(0); }}
if (values.version) {{ console.log(VERSION); process.exit(0); }}

const target = values.name ?? positionals[0] ?? "World";
const count = Number(values.count ?? 1);

for (let i = 0; i < count; i++) {{
  console.log(`Hello, ${{target}}!`);
}}

if (values.verbose) {{
  console.error("[verbose]", {{ values, positionals }});
}}
"#,
    )
}

fn render_cli_package_json(name: &str) -> String {
    format!(
        r#"{{
  "name": "{name}",
  "version": "0.1.0",
  "type": "module",
  "bin": {{ "{name}": "./index.ts" }},
  "scripts": {{
    "start": "bun run index.ts",
    "build": "bun build --compile --minify ./index.ts --outfile ./dist/{name}"
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

fn render_tui_package_json(name: &str) -> String {
    format!(
        r#"{{
  "name": "{name}",
  "version": "0.1.0",
  "type": "module",
  "bin": {{ "{name}": "./src/index.tsx" }},
  "scripts": {{
    "start": "bun run src/index.tsx",
    "build": "bun build --compile ./src/index.tsx --outfile ./dist/{name}"
  }},
  "dependencies": {{
    "ink": "^6.0.0",
    "react": "^19.0.0"
  }},
  "devDependencies": {{
    "@types/bun": "latest",
    "@types/react": "^19.0.0"
  }},
  "engines": {{
    "bun": ">=1.2.0"
  }}
}}
"#,
    )
}

fn render_gui_package_json(name: &str) -> String {
    format!(
        r#"{{
  "name": "{name}",
  "version": "0.1.0",
  "type": "module",
  "scripts": {{
    "start": "electrobun dev",
    "build": "electrobun build",
    "dist": "electrobun dist"
  }},
  "dependencies": {{
    "electrobun": "latest"
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

fn render_exe_package_json(name: &str) -> String {
    format!(
        r#"{{
  "name": "{name}",
  "version": "0.1.0",
  "type": "module",
  "scripts": {{
    "start": "bun run index.ts",
    "build": "bun build --compile --minify ./index.ts --outfile ./dist/{name}",
    "build:all": "./build-all.sh",
    "build:linux-x64":   "bun build --compile --minify --target=bun-linux-x64   ./index.ts --outfile ./dist/{name}-linux-x64",
    "build:linux-arm64": "bun build --compile --minify --target=bun-linux-arm64 ./index.ts --outfile ./dist/{name}-linux-arm64",
    "build:darwin-x64":  "bun build --compile --minify --target=bun-darwin-x64  ./index.ts --outfile ./dist/{name}-darwin-x64",
    "build:darwin-arm64":"bun build --compile --minify --target=bun-darwin-arm64 ./index.ts --outfile ./dist/{name}-darwin-arm64",
    "build:windows-x64": "bun build --compile --minify --target=bun-windows-x64 ./index.ts --outfile ./dist/{name}-windows-x64.exe"
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

fn render_build_all_sh(name: &str) -> String {
    format!(
        r#"#!/usr/bin/env bash
# Build le même index.ts en standalone executable pour toutes les cibles Bun.
set -euo pipefail

mkdir -p dist

declare -a TARGETS=(
  "bun-linux-x64"
  "bun-linux-arm64"
  "bun-darwin-x64"
  "bun-darwin-arm64"
  "bun-windows-x64"
)

for t in "${{TARGETS[@]}}"; do
  ext=""
  [[ "$t" == *"windows"* ]] && ext=".exe"
  out="./dist/{name}-${{t#bun-}}${{ext}}"
  echo "▶ $t → $out"
  bun build --compile --minify --target="$t" ./index.ts --outfile "$out"
  ls -lh "$out" | awk '{{print "  " $5, $9}}'
done

echo ""
echo "✓ 5 targets built in ./dist/"
"#,
    )
}

fn render_gui_bun_entry(name: &str) -> String {
    format!(
        r#"// Electrobun main process — orchestration + webview.
// Doc : https://www.electrobun.dev/

import {{ BrowserWindow, Electrobun }} from "electrobun/bun";

Electrobun.events.on("will-quit", () => {{
  console.log("[{name}] shutting down");
}});

const win = new BrowserWindow({{
  title: "{name}",
  url: "views://browser/index.html",
  frame: {{ width: 1024, height: 720, x: 100, y: 100 }},
}});

win.webview.on("did-navigate", ({{ url }}) => {{
  console.log("[webview] →", url);
}});
"#,
    )
}

fn readme(name: &str, title: &str) -> String {
    format!(
        r#"# {name}

{title}

Scaffolded by `n2b app`.

## Run

```bash
bun install
bun start
```

## Build standalone

```bash
bun run build                 # native target
bun run build:linux-x64       # explicit target
```

## Refs

- [Bun bundler --compile](https://bun.com/docs/bundler/executables)
- [Bun auto-install](https://bun.com/docs/runtime/auto-install)
- [Ink (React for CLIs)](https://github.com/vadimdemedes/ink)
- [Electrobun](https://www.electrobun.dev/)
"#,
    )
}

// --- Templates ---

const TUI_APP_TSX: &str = r#"import React, { useState } from "react";
import { Box, Text, useInput, useApp } from "ink";

const TASKS = ["Install deps", "Scan codebase", "Apply fixes", "Commit", "Push"];

export default function App() {
  const { exit } = useApp();
  const [cursor, setCursor] = useState(0);
  const [done, setDone] = useState<Set<number>>(new Set());

  useInput((input, key) => {
    if (input === "q" || key.escape) exit();
    if (key.upArrow) setCursor((c) => Math.max(0, c - 1));
    if (key.downArrow) setCursor((c) => Math.min(TASKS.length - 1, c + 1));
    if (input === " " || key.return) {
      const next = new Set(done);
      if (next.has(cursor)) next.delete(cursor);
      else next.add(cursor);
      setDone(next);
    }
  });

  return (
    <Box flexDirection="column" padding={1}>
      <Box marginBottom={1}>
        <Text bold color="cyan">n2b TUI demo</Text>
        <Text dimColor> — ↑↓ naviguer · space/enter toggle · q quitter</Text>
      </Box>
      {TASKS.map((task, i) => {
        const isCur = i === cursor;
        const isDone = done.has(i);
        return (
          <Box key={task}>
            <Text color={isCur ? "yellow" : undefined}>
              {isCur ? "▶ " : "  "}
              {isDone ? "[✓]" : "[ ]"} {task}
            </Text>
          </Box>
        );
      })}
      <Box marginTop={1}>
        <Text dimColor>{done.size}/{TASKS.length} done</Text>
      </Box>
    </Box>
  );
}
"#;

const TUI_INDEX_TSX: &str = r#"#!/usr/bin/env bun
import React from "react";
import { render } from "ink";
import App from "./App.js";

render(<App />);
"#;

const TUI_TSCONFIG: &str = r#"{
  "compilerOptions": {
    "target": "ESNext",
    "module": "Preserve",
    "moduleResolution": "bundler",
    "jsx": "react-jsx",
    "allowImportingTsExtensions": true,
    "moduleDetection": "force",
    "verbatimModuleSyntax": true,
    "noEmit": true,
    "strict": true,
    "skipLibCheck": true,
    "types": ["bun"]
  },
  "include": ["src/**/*"]
}
"#;

const GUI_BROWSER_HTML: &str = r#"<!DOCTYPE html>
<html>
  <head>
    <meta charset="utf-8" />
    <title>n2b GUI</title>
    <link rel="stylesheet" href="./styles.css" />
  </head>
  <body>
    <header>
      <h1>Hello from Electrobun + Bun</h1>
      <button id="tick">Tick</button>
      <pre id="out"></pre>
    </header>
    <script type="module" src="./index.ts"></script>
  </body>
</html>
"#;

const GUI_BROWSER_TS: &str = r#"// Browser-side script — runs inside the webview.

const $tick = document.getElementById("tick")!;
const $out = document.getElementById("out")!;

let n = 0;
$tick.addEventListener("click", () => {
  n++;
  $out.textContent = `tick #${n} @ ${new Date().toISOString()}`;
});
"#;

const GUI_BROWSER_CSS: &str = r#"body {
  font-family: system-ui, -apple-system, sans-serif;
  padding: 2rem;
  background: #0e0e11;
  color: #e7e7ea;
}

button {
  padding: .5rem 1rem;
  background: #5b8def;
  color: white;
  border: 0;
  border-radius: .5rem;
  font-weight: 600;
}

pre {
  margin-top: 1rem;
  padding: 1rem;
  background: #1a1a21;
  border-radius: .5rem;
}
"#;

const GUI_CONFIG_TS: &str = r#"// Electrobun config.
// Doc : https://www.electrobun.dev/

export default {
  app: {
    name: "n2b-gui",
    identifier: "com.n2b.gui",
    version: "0.1.0",
  },
  build: {
    bun: { entrypoint: "./src/bun/index.ts" },
    views: {
      browser: {
        entrypoint: "./src/browser/index.html",
      },
    },
  },
};
"#;

const GITIGNORE: &str = r#"node_modules/
dist/
*.exe
*.app
.DS_Store
.electrobun/
"#;
