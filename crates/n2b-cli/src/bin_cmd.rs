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
use std::path::PathBuf;

#[path = "bin_cmd_gpu.rs"]
mod bin_cmd_gpu;
#[path = "bin_cmd_templates.rs"]
mod bin_cmd_templates;

use bin_cmd_gpu::*;
use bin_cmd_templates::*;

pub struct BinOpts {
    pub name: String,
    pub dir: Option<PathBuf>,
    pub flavor: BinFlavor,
    pub force: bool,
    pub quiet: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinFlavor {
    /// Plugin natif Bun (Rust → .node via NAPI, bun-native-plugin).
    NativePlugin,
    /// Exemple MDX → JSX (mdxjs-rs via plugin natif Bun).
    Mdx,
    /// Module WASM (wasm-pack / wasm-bindgen) invoqué depuis Bun.
    Wasm,
    /// Module Rust+wgpu compilé en WASM, compute shader WebGPU côté Bun.
    WebGpu,
    /// Plugin Bun natif qui compile/minifie CSS via lightningcss.
    /// Utile pour Tailwind v4, CSS nesting, transpilation color-mix, etc.
    LightningCss,
    /// Plugin Bun JS wrappant @tailwindcss/postcss (Tailwind v4 Oxide via N-API).
    /// Zéro compile Rust — utilise le binding npm officiel.
    Tailwind,
}

impl BinFlavor {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "native" | "native-plugin" | "plugin" => Some(Self::NativePlugin),
            "mdx" => Some(Self::Mdx),
            "wasm" => Some(Self::Wasm),
            "webgpu" | "wgpu" | "gpu" => Some(Self::WebGpu),
            "lightningcss" | "css" | "css-lightning" => Some(Self::LightningCss),
            "tailwind" | "tailwindcss" | "tw" => Some(Self::Tailwind),
            _ => None,
        }
    }
}

pub fn run_bin(opts: BinOpts) -> Result<()> {
    let target_dir = match opts.dir {
        Some(d) => d.join(&opts.name),
        None => PathBuf::from(&opts.name),
    };

    if target_dir.exists() {
        if !opts.force {
            anyhow::bail!(
                "{} existe déjà — relancer avec --force pour écraser",
                target_dir.display()
            );
        }
        if !opts.quiet {
            eprintln!("[bin] écrasement de {}", target_dir.display());
        }
    }

    std::fs::create_dir_all(&target_dir)
        .with_context(|| format!("création de {}", target_dir.display()))?;

    match opts.flavor {
        BinFlavor::NativePlugin => scaffold_native_plugin(&target_dir, &opts.name, opts.quiet)?,
        BinFlavor::Mdx => scaffold_mdx(&target_dir, &opts.name, opts.quiet)?,
        BinFlavor::Wasm => scaffold_wasm(&target_dir, &opts.name, opts.quiet)?,
        BinFlavor::WebGpu => scaffold_webgpu(&target_dir, &opts.name, opts.quiet)?,
        BinFlavor::LightningCss => scaffold_lightningcss(&target_dir, &opts.name, opts.quiet)?,
        BinFlavor::Tailwind => scaffold_tailwind(&target_dir, &opts.name, opts.quiet)?,
    }

    if !opts.quiet {
        eprintln!(
            "[bin] ✓ {} ({:?}) scaffolded → cd {} && bun install && bun run build",
            opts.name,
            opts.flavor,
            target_dir.display()
        );
    }
    Ok(())
}

fn scaffold_native_plugin(dir: &std::path::Path, name: &str, quiet: bool) -> Result<()> {
    write_file(
        dir.join("Cargo.toml"),
        &render_cargo_toml(name, false),
        quiet,
    )?;
    write_file(dir.join("src/lib.rs"), NATIVE_PLUGIN_LIB_RS, quiet)?;
    write_file(
        dir.join("package.json"),
        &render_package_json(name, false),
        quiet,
    )?;
    write_file(dir.join("build.rs"), BUILD_RS, quiet)?;
    write_file(dir.join(".cargo/config.toml"), CARGO_CONFIG, quiet)?;
    write_file(
        dir.join("README.md"),
        &render_readme(name, "Plugin natif Bun (Rust → .node)"),
        quiet,
    )?;
    write_file(dir.join("index.ts"), INDEX_TS_NATIVE, quiet)?;
    write_file(dir.join(".gitignore"), GITIGNORE, quiet)?;
    Ok(())
}

fn scaffold_tailwind(dir: &std::path::Path, name: &str, quiet: bool) -> Result<()> {
    write_file(
        dir.join("package.json"),
        &render_tailwind_package_json(name),
        quiet,
    )?;
    write_file(dir.join("plugin.ts"), TAILWIND_PLUGIN_TS, quiet)?;
    write_file(dir.join("build.ts"), TAILWIND_BUILD_TS, quiet)?;
    write_file(
        dir.join("postcss.config.mjs"),
        TAILWIND_POSTCSS_CONFIG,
        quiet,
    )?;
    write_file(dir.join("src/app.css"), TAILWIND_APP_CSS, quiet)?;
    write_file(dir.join("src/index.html"), TAILWIND_INDEX_HTML, quiet)?;
    write_file(
        dir.join("README.md"),
        &render_readme(
            name,
            "Plugin Bun JS wrapping @tailwindcss/postcss (Tailwind v4 Oxide via N-API)",
        ),
        quiet,
    )?;
    write_file(dir.join(".gitignore"), GITIGNORE_WASM, quiet)?;
    Ok(())
}

fn scaffold_lightningcss(dir: &std::path::Path, name: &str, quiet: bool) -> Result<()> {
    write_file(
        dir.join("Cargo.toml"),
        &render_lightningcss_cargo_toml(name),
        quiet,
    )?;
    write_file(dir.join("src/lib.rs"), LIGHTNINGCSS_LIB_RS, quiet)?;
    write_file(
        dir.join("package.json"),
        &render_package_json(name, false),
        quiet,
    )?;
    write_file(dir.join("build.rs"), BUILD_RS, quiet)?;
    write_file(dir.join(".cargo/config.toml"), CARGO_CONFIG, quiet)?;
    write_file(
        dir.join("README.md"),
        &render_readme(
            name,
            "Plugin Bun natif : minify/transpile CSS via lightningcss (Rust)",
        ),
        quiet,
    )?;
    write_file(dir.join("index.ts"), INDEX_TS_LIGHTNINGCSS, quiet)?;
    write_file(dir.join("example.css"), EXAMPLE_CSS, quiet)?;
    write_file(dir.join(".gitignore"), GITIGNORE, quiet)?;
    Ok(())
}

fn scaffold_mdx(dir: &std::path::Path, name: &str, quiet: bool) -> Result<()> {
    write_file(
        dir.join("Cargo.toml"),
        &render_cargo_toml(name, true),
        quiet,
    )?;
    write_file(dir.join("src/lib.rs"), MDX_LIB_RS, quiet)?;
    write_file(
        dir.join("package.json"),
        &render_package_json(name, false),
        quiet,
    )?;
    write_file(dir.join("build.rs"), BUILD_RS, quiet)?;
    write_file(dir.join(".cargo/config.toml"), CARGO_CONFIG, quiet)?;
    write_file(
        dir.join("README.md"),
        &render_readme(name, "Plugin MDX → JSX pour Bun.build (via mdxjs-rs)"),
        quiet,
    )?;
    write_file(dir.join("index.ts"), INDEX_TS_MDX, quiet)?;
    write_file(dir.join("example.mdx"), EXAMPLE_MDX, quiet)?;
    write_file(dir.join(".gitignore"), GITIGNORE, quiet)?;
    Ok(())
}

fn scaffold_wasm(dir: &std::path::Path, name: &str, quiet: bool) -> Result<()> {
    write_file(dir.join("Cargo.toml"), &render_wasm_cargo_toml(name), quiet)?;
    write_file(dir.join("src/lib.rs"), WASM_LIB_RS, quiet)?;
    write_file(
        dir.join("package.json"),
        &render_wasm_package_json(name),
        quiet,
    )?;
    write_file(
        dir.join("README.md"),
        &render_readme(
            name,
            "Module WASM (wasm-pack / wasm-bindgen) utilisable depuis Bun",
        ),
        quiet,
    )?;
    write_file(dir.join("index.ts"), INDEX_TS_WASM, quiet)?;
    write_file(dir.join(".gitignore"), GITIGNORE_WASM, quiet)?;
    Ok(())
}

pub(crate) fn write_file(path: PathBuf, content: &str, quiet: bool) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    std::fs::write(&path, content).with_context(|| format!("écrire {}", path.display()))?;
    if !quiet {
        eprintln!("[bin]   + {}", path.display());
    }
    Ok(())
}

#[allow(dead_code)]
pub fn valid_name(s: &str) -> Result<()> {
    if s.is_empty() {
        return Err(anyhow!("nom vide"));
    }
    if !s
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(anyhow!(
            "nom '{s}' contient des caractères invalides (autorisés : a-z 0-9 - _)"
        ));
    }
    Ok(())
}
